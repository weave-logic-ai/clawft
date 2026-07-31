//! ADR-078 / WEFT-708 — W0 world-model export on package.
//!
//! Writes a portable `world_model.json` stub when a splat job finishes.
//! Object/surface/volume counts stay at 0 until W1+ structure stages land.
//! BVH publish is deferred (`bvh_published: false`); live leaves need
//! clawft-bvh + daemon wiring (tag `SPLAT_SCENE` = 0x5350_0001).

use crate::job::JobDirs;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

/// Scene-level AABB in reconstruction coordinates (`scene_local` frame).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneAabb {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl SceneAabb {
    fn from_point(p: [f64; 3]) -> Self {
        Self { min: p, max: p }
    }

    fn expand(&mut self, p: [f64; 3]) {
        for i in 0..3 {
            self.min[i] = self.min[i].min(p[i]);
            self.max[i] = self.max[i].max(p[i]);
        }
    }

    fn is_valid(self) -> bool {
        self.min[0] <= self.max[0]
            && self.min[1] <= self.max[1]
            && self.min[2] <= self.max[2]
            && self.min.iter().chain(self.max.iter()).all(|v| v.is_finite())
    }

    fn to_json(self) -> Value {
        json!({
            "min": self.min,
            "max": self.max,
        })
    }
}

/// Build the W0 world-model stub (ADR-078 / splat-to-world-model.md §10).
pub fn build_world_model_stub(
    job_id: &str,
    aabb: Option<SceneAabb>,
    artifact_names: &BTreeMap<String, String>,
) -> Value {
    // Relative paths under artifacts/ for known appearance products.
    let mut scene_artifacts = serde_json::Map::new();
    if let Some(sog) = artifact_names.get("sog") {
        scene_artifacts.insert("sog".into(), json!(sog));
    }
    if let Some(ply) = artifact_names.get("ply") {
        scene_artifacts.insert("ply".into(), json!(ply));
    }

    json!({
        "frame": "scene_local",
        "scale_m": null,
        "objects": 0,
        "surfaces": 0,
        "volumes": 0,
        // W0 does not insert into a live BVH tree — export only.
        "bvh_published": false,
        "scene": {
            "tag": "SPLAT_SCENE",
            "tag_u32": 0x5350_0001u32,
            "job_id": job_id,
            "aabb": aabb.map(SceneAabb::to_json),
            "artifacts": scene_artifacts,
        }
    })
}

/// Best-effort scene AABB from COLMAP sparse cloud, then splat PLY.
pub fn compute_scene_aabb(dirs: &JobDirs) -> Option<SceneAabb> {
    let model0 = dirs.sparse().join("0");
    if let Some(a) = aabb_from_colmap_model(&model0) {
        return Some(a);
    }
    aabb_from_ply(&dirs.artifacts().join("splat.ply"))
}

fn aabb_from_colmap_model(model: &Path) -> Option<SceneAabb> {
    if let Some(a) = aabb_from_points3d_txt(&model.join("points3D.txt")) {
        return Some(a);
    }
    aabb_from_points3d_bin(&model.join("points3D.bin"))
}

/// COLMAP `points3D.txt` lines: `POINT3D_ID X Y Z R G B ERROR TRACK…`
fn aabb_from_points3d_txt(path: &Path) -> Option<SceneAabb> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut aabb: Option<SceneAabb> = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let _id = parts.next()?;
        let x: f64 = parts.next()?.parse().ok()?;
        let y: f64 = parts.next()?.parse().ok()?;
        let z: f64 = parts.next()?.parse().ok()?;
        let p = [x, y, z];
        match &mut aabb {
            Some(a) => a.expand(p),
            None => aabb = Some(SceneAabb::from_point(p)),
        }
    }
    aabb.filter(|a| a.is_valid())
}

/// COLMAP little-endian `points3D.bin` (see COLMAP Reconstruction format).
fn aabb_from_points3d_bin(path: &Path) -> Option<SceneAabb> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    if buf.len() < 8 {
        return None;
    }
    let mut off = 0usize;
    let n = read_u64_le(&buf, &mut off)?;
    let mut aabb: Option<SceneAabb> = None;
    for _ in 0..n {
        // point3D_id
        let _ = read_u64_le(&buf, &mut off)?;
        let x = read_f64_le(&buf, &mut off)?;
        let y = read_f64_le(&buf, &mut off)?;
        let z = read_f64_le(&buf, &mut off)?;
        // r,g,b
        off = off.checked_add(3)?;
        if off > buf.len() {
            return None;
        }
        let _error = read_f64_le(&buf, &mut off)?;
        let track_len = read_u64_le(&buf, &mut off)? as usize;
        // each track: image_id u32 + point2D_idx u32
        off = off.checked_add(track_len.checked_mul(8)?)?;
        if off > buf.len() {
            return None;
        }
        let p = [x, y, z];
        match &mut aabb {
            Some(a) => a.expand(p),
            None => aabb = Some(SceneAabb::from_point(p)),
        }
    }
    aabb.filter(|a| a.is_valid())
}

/// Minimal ASCII PLY bounds (Brush export). Binary PLY left for later.
fn aabb_from_ply(path: &Path) -> Option<SceneAabb> {
    let text = std::fs::read_to_string(path).ok()?;
    if !text.starts_with("ply") {
        return None;
    }
    let mut lines = text.lines();
    let mut n_verts: Option<usize> = None;
    let mut props: Vec<String> = Vec::new();
    let mut ascii = false;
    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.starts_with("element vertex") {
            n_verts = line.split_whitespace().nth(2)?.parse().ok();
        } else if line.starts_with("property") {
            // property <type> <name>
            if let Some(name) = line.split_whitespace().nth(2) {
                props.push(name.to_string());
            }
        } else if line.starts_with("format ascii") {
            ascii = true;
        } else if line == "end_header" {
            break;
        }
    }
    if !ascii {
        return None;
    }
    let n_verts = n_verts?;
    let ix = props.iter().position(|p| p == "x")?;
    let iy = props.iter().position(|p| p == "y")?;
    let iz = props.iter().position(|p| p == "z")?;
    let mut aabb: Option<SceneAabb> = None;
    for _ in 0..n_verts {
        let line = lines.next()?;
        let vals: Vec<&str> = line.split_whitespace().collect();
        if vals.len() <= ix.max(iy).max(iz) {
            continue;
        }
        let x: f64 = vals.get(ix)?.parse().ok()?;
        let y: f64 = vals.get(iy)?.parse().ok()?;
        let z: f64 = vals.get(iz)?.parse().ok()?;
        let p = [x, y, z];
        match &mut aabb {
            Some(a) => a.expand(p),
            None => aabb = Some(SceneAabb::from_point(p)),
        }
    }
    aabb.filter(|a| a.is_valid())
}

fn read_u64_le(buf: &[u8], off: &mut usize) -> Option<u64> {
    let end = off.checked_add(8)?;
    if end > buf.len() {
        return None;
    }
    let mut b = [0u8; 8];
    b.copy_from_slice(&buf[*off..end]);
    *off = end;
    Some(u64::from_le_bytes(b))
}

fn read_f64_le(buf: &[u8], off: &mut usize) -> Option<f64> {
    let end = off.checked_add(8)?;
    if end > buf.len() {
        return None;
    }
    let mut b = [0u8; 8];
    b.copy_from_slice(&buf[*off..end]);
    *off = end;
    Some(f64::from_le_bytes(b))
}

/// Write `artifacts/world_model.json` and register it on the job record.
pub fn write_world_model(
    dirs: &JobDirs,
    job_id: &str,
    artifacts: &mut BTreeMap<String, String>,
) -> std::io::Result<Value> {
    let aabb = compute_scene_aabb(dirs);
    let wm = build_world_model_stub(job_id, aabb, artifacts);
    let path = dirs.artifacts().join("world_model.json");
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&wm).expect("world_model serializes"),
    )?;
    artifacts.insert("world_model".into(), "world_model.json".into());
    Ok(wm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::{JobDirs, JobRecord};
    use std::path::PathBuf;

    fn temp_job(id: &str) -> (TempHold, JobDirs) {
        let root = std::env::temp_dir().join(format!(
            "clawft-splat-wm-{}-{}-{}",
            id,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let dirs = JobDirs::new(&root, id);
        dirs.create_all().unwrap();
        (TempHold { root }, dirs)
    }

    /// Minimal hold so temp root isn't dropped mid-test without tempfile crate.
    struct TempHold {
        root: PathBuf,
    }
    impl Drop for TempHold {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn stub_has_w0_fields() {
        let mut arts = BTreeMap::new();
        arts.insert("ply".into(), "splat.ply".into());
        arts.insert("sog".into(), "splat.sog".into());
        let aabb = SceneAabb {
            min: [-1.0, 0.0, -2.0],
            max: [1.0, 2.0, 0.5],
        };
        let v = build_world_model_stub("job-abc", Some(aabb), &arts);
        assert_eq!(v["frame"], "scene_local");
        assert_eq!(v["objects"], 0);
        assert_eq!(v["surfaces"], 0);
        assert_eq!(v["volumes"], 0);
        assert_eq!(v["bvh_published"], false);
        assert_eq!(v["scale_m"], Value::Null);
        assert_eq!(v["scene"]["tag"], "SPLAT_SCENE");
        assert_eq!(v["scene"]["job_id"], "job-abc");
        assert_eq!(v["scene"]["tag_u32"], 0x5350_0001);
        assert_eq!(v["scene"]["aabb"]["min"][0], -1.0);
        assert_eq!(v["scene"]["artifacts"]["sog"], "splat.sog");
        assert_eq!(v["scene"]["artifacts"]["ply"], "splat.ply");
    }

    #[test]
    fn stub_null_aabb_when_unknown() {
        let arts = BTreeMap::new();
        let v = build_world_model_stub("j", None, &arts);
        assert_eq!(v["scene"]["aabb"], Value::Null);
    }

    #[test]
    fn points3d_txt_aabb() {
        let (_hold, dirs) = temp_job("txt");
        let model = dirs.sparse().join("0");
        std::fs::create_dir_all(&model).unwrap();
        std::fs::write(
            model.join("points3D.txt"),
            "# comment\n\
             1 0.0 0.0 0.0 255 0 0 0.1\n\
             2 2.0 4.0 -1.0 0 255 0 0.1\n",
        )
        .unwrap();
        let a = compute_scene_aabb(&dirs).expect("aabb");
        assert_eq!(a.min, [0.0, 0.0, -1.0]);
        assert_eq!(a.max, [2.0, 4.0, 0.0]);
    }

    #[test]
    fn points3d_bin_aabb() {
        let (_hold, dirs) = temp_job("bin");
        let model = dirs.sparse().join("0");
        std::fs::create_dir_all(&model).unwrap();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1u64.to_le_bytes()); // n points
        bytes.extend_from_slice(&42u64.to_le_bytes()); // id
        bytes.extend_from_slice(&1.5f64.to_le_bytes());
        bytes.extend_from_slice(&(-0.5f64).to_le_bytes());
        bytes.extend_from_slice(&3.0f64.to_le_bytes());
        bytes.extend_from_slice(&[10, 20, 30]); // rgb
        bytes.extend_from_slice(&0.01f64.to_le_bytes()); // error
        bytes.extend_from_slice(&0u64.to_le_bytes()); // track_len
        std::fs::write(model.join("points3D.bin"), bytes).unwrap();
        let a = compute_scene_aabb(&dirs).expect("aabb");
        assert_eq!(a.min, [1.5, -0.5, 3.0]);
        assert_eq!(a.max, [1.5, -0.5, 3.0]);
    }

    #[test]
    fn write_world_model_registers_artifact() {
        let (_hold, dirs) = temp_job("write");
        let mut rec = JobRecord::new("job-w".into(), "test".into(), None, 0);
        rec.artifacts.insert("ply".into(), "splat.ply".into());
        // touch empty ply so package still works without AABB
        std::fs::write(dirs.artifacts().join("splat.ply"), b"not a ply").unwrap();
        let wm = write_world_model(&dirs, &rec.job_id, &mut rec.artifacts).unwrap();
        assert_eq!(rec.artifacts.get("world_model").map(String::as_str), Some("world_model.json"));
        let on_disk = std::fs::read_to_string(dirs.artifacts().join("world_model.json")).unwrap();
        let parsed: Value = serde_json::from_str(&on_disk).unwrap();
        assert_eq!(parsed["scene"]["job_id"], "job-w");
        assert_eq!(parsed["bvh_published"], false);
        assert_eq!(wm["objects"], 0);
    }

    #[tokio::test]
    async fn package_stage_writes_world_model_and_manifest() {
        use crate::config::PipelineConfig;
        use crate::stages;

        let (_hold, dirs) = temp_job("pkg");
        let mut rec = JobRecord::new("job-pkg".into(), "upload".into(), None, 1);
        rec.artifacts.insert("ply".into(), "splat.ply".into());
        rec.artifacts.insert("sog".into(), "splat.sog".into());
        std::fs::write(dirs.artifacts().join("splat.ply"), b"ply\n").unwrap();

        // Seed sparse points so AABB is non-null.
        let model = dirs.sparse().join("0");
        std::fs::create_dir_all(&model).unwrap();
        std::fs::write(
            model.join("points3D.txt"),
            "1 -1.0 -2.0 -3.0 0 0 0 0\n2 1.0 2.0 3.0 0 0 0 0\n",
        )
        .unwrap();

        let cfg = PipelineConfig::default();
        stages::package(&cfg, &dirs, &mut rec).await.expect("package");

        assert_eq!(
            rec.artifacts.get("world_model").map(String::as_str),
            Some("world_model.json")
        );
        let wm: Value = serde_json::from_str(
            &std::fs::read_to_string(dirs.artifacts().join("world_model.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(wm["scene"]["tag"], "SPLAT_SCENE");
        assert_eq!(wm["scene"]["aabb"]["min"][0], -1.0);
        assert_eq!(wm["scene"]["aabb"]["max"][2], 3.0);
        assert_eq!(wm["objects"], 0);

        let manifest: Value = serde_json::from_str(
            &std::fs::read_to_string(dirs.artifacts().join("manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["artifacts"]["world_model"], "world_model.json");
        // Summary also embedded for quick clients.
        assert_eq!(manifest["world_model"]["bvh_published"], false);
        assert_eq!(manifest["world_model"]["objects"], 0);
    }
}
