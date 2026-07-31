//! The six pipeline stages (DESIGN.md §2).
//!
//! Each stage is a free async fn taking the config + job workspace and
//! mutating the `JobRecord`'s metrics/artifacts. Stage order and error
//! handling live in [`crate::pipeline::run_job`].

use crate::config::PipelineConfig;
use crate::job::{JobDirs, JobRecord};
use crate::run::{RunError, run_tool, stage_log};
use crate::session::{JobInput, materialize_frames_to_images, parse_session};
use std::path::Path;
use tokio::io::AsyncWriteExt;

/// Stage 1 — ffprobe the input video into metrics.
///
/// For image-set sessions, skips ffprobe and records frame count / optional
/// camera size hints from `session.json` (no video duration).
pub async fn probe(
    cfg: &PipelineConfig,
    dirs: &JobDirs,
    rec: &mut JobRecord,
    input: &JobInput,
) -> Result<(), RunError> {
    match input {
        JobInput::ImageSet(session_dir) => {
            probe_image_set(dirs, rec, session_dir).await
        }
        JobInput::Video(path) => probe_video(cfg, dirs, rec, path).await,
    }
}

async fn probe_video(
    cfg: &PipelineConfig,
    dirs: &JobDirs,
    rec: &mut JobRecord,
    input: &Path,
) -> Result<(), RunError> {
    let args = vec![
        "-v".into(),
        "error".into(),
        "-select_streams".into(),
        "v:0".into(),
        "-show_entries".into(),
        "stream=width,height:format=duration".into(),
        "-of".into(),
        "json".into(),
        input.display().to_string(),
    ];
    let out = run_tool(&cfg.tools.ffprobe, &args, &dirs.root, &stage_log(&dirs.logs(), "probe")).await?;
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out.output) {
        rec.metrics.video_duration_s = v["format"]["duration"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok());
        rec.metrics.video_width = v["streams"][0]["width"].as_u64().map(|w| w as u32);
        rec.metrics.video_height = v["streams"][0]["height"].as_u64().map(|h| h as u32);
    }
    Ok(())
}

async fn probe_image_set(
    dirs: &JobDirs,
    rec: &mut JobRecord,
    session_dir: &Path,
) -> Result<(), RunError> {
    let log = stage_log(&dirs.logs(), "probe");
    let layout = parse_session(session_dir).map_err(|e| {
        RunError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            e.to_string(),
        ))
    })?;
    rec.metrics.n_frames = Some(layout.frame_paths.len() as u32);
    if let Some(meta) = &layout.session_json
        && let Some(cam) = meta.get("camera")
    {
        rec.metrics.video_width = cam
            .get("width")
            .and_then(|v| v.as_u64())
            .map(|w| w as u32);
        rec.metrics.video_height = cam
            .get("height")
            .and_then(|v| v.as_u64())
            .map(|h| h as u32);
    }
    let mut f = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .await?;
    f.write_all(
        format!(
            "\n# image-set probe: {} frames, {} pose lines (ffprobe skipped)\n",
            layout.frame_paths.len(),
            layout.poses.len()
        )
        .as_bytes(),
    )
    .await?;
    f.flush().await?;
    Ok(())
}

/// Stage 2 — produce `dataset/images/`.
///
/// **Video:** ffmpeg frame extraction. fps is chosen so a `max_frames` cap is
/// respected. Phone videos often carry a Display Matrix rotation; we rely on
/// ffmpeg's default autorotate (do **not** pass `-noautorotate`).
///
/// **Image-set:** skip ffmpeg; copy/link `session/frames/*` into
/// `dataset/images/` as `frame_%05d.ext`. COLMAP still runs feature extract
/// on those images (SfM stage).
pub async fn frames(
    cfg: &PipelineConfig,
    dirs: &JobDirs,
    rec: &mut JobRecord,
    input: &JobInput,
) -> Result<(), RunError> {
    match input {
        JobInput::ImageSet(session_dir) => frames_from_session(dirs, rec, session_dir).await,
        JobInput::Video(path) => frames_from_video(cfg, dirs, rec, path).await,
    }
}

async fn frames_from_session(
    dirs: &JobDirs,
    rec: &mut JobRecord,
    session_dir: &Path,
) -> Result<(), RunError> {
    let log = stage_log(&dirs.logs(), "frames");
    let layout = parse_session(session_dir).map_err(|e| {
        RunError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            e.to_string(),
        ))
    })?;
    let n = materialize_frames_to_images(&layout, &dirs.images())
        .map_err(|e| RunError::Io(std::io::Error::other(e.to_string())))?;
    rec.metrics.n_frames = Some(n);
    // Ensure sparse/ exists for COLMAP mapper output (empty until sfm).
    std::fs::create_dir_all(dirs.sparse())?;
    let mut f = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .await?;
    f.write_all(
        format!(
            "\n# image-set frames: materialized {n} images from {} → dataset/images/ (ffmpeg skipped)\n",
            session_dir.display()
        )
        .as_bytes(),
    )
    .await?;
    f.flush().await?;
    Ok(())
}

async fn frames_from_video(
    cfg: &PipelineConfig,
    dirs: &JobDirs,
    rec: &mut JobRecord,
    input: &Path,
) -> Result<(), RunError> {
    let duration = rec.metrics.video_duration_s.unwrap_or(0.0);
    let mut fps = cfg.frames.fps as f64;
    if duration > 0.0 && fps * duration > cfg.frames.max_frames as f64 {
        fps = cfg.frames.max_frames as f64 / duration;
    }

    let mut vf = format!("fps={fps:.4}");
    if cfg.frames.max_edge > 0 {
        // Downscale longest edge to max_edge, preserve aspect, keep even dims.
        let e = cfg.frames.max_edge;
        vf.push_str(&format!(
            ",scale='if(gt(iw,ih),min({e},iw),-2)':'if(gt(ih,iw),min({e},ih),-2)'"
        ));
    }

    let out_pattern = dirs.images().join("frame_%05d.jpg");
    let args = vec![
        "-y".into(),
        "-i".into(),
        input.display().to_string(),
        "-vf".into(),
        vf,
        "-q:v".into(),
        cfg.frames.jpeg_q.to_string(),
        out_pattern.display().to_string(),
    ];
    run_tool(&cfg.tools.ffmpeg, &args, &dirs.root, &stage_log(&dirs.logs(), "frames")).await?;

    let n = std::fs::read_dir(dirs.images())?
        .filter(|e| e.as_ref().map(|e| e.path().extension().is_some_and(|x| x == "jpg")).unwrap_or(false))
        .count() as u32;
    rec.metrics.n_frames = Some(n);
    Ok(())
}

/// Stage 3 — COLMAP SfM: features → sequential/exhaustive matching → mapper.
/// Output lands in `dataset/sparse/0` (the layout Brush consumes directly).
pub async fn sfm(cfg: &PipelineConfig, dirs: &JobDirs, rec: &mut JobRecord) -> Result<(), RunError> {
    let log = stage_log(&dirs.logs(), "sfm");
    let db = dirs.colmap_db().display().to_string();
    let images = dirs.images().display().to_string();
    let sparse = dirs.sparse().display().to_string();
    let colmap = &cfg.tools.colmap;

    let mut feat = vec![
        "feature_extractor".to_string(),
        "--database_path".into(), db.clone(),
        "--image_path".into(), images.clone(),
        "--ImageReader.camera_model".into(), cfg.sfm.camera_model.clone(),
    ];
    if cfg.sfm.single_camera {
        feat.extend(["--ImageReader.single_camera".into(), "1".into()]);
    }
    run_tool(colmap, &feat, &dirs.root, &log).await?;

    let matcher: Vec<String> = if cfg.sfm.sequential {
        let mut m = vec![
            "sequential_matcher".to_string(),
            "--database_path".into(), db.clone(),
        ];
        if cfg.sfm.loop_detection {
            m.extend(["--SequentialMatching.loop_detection".into(), "1".into()]);
        }
        m
    } else {
        vec!["exhaustive_matcher".to_string(), "--database_path".into(), db.clone()]
    };
    run_tool(colmap, &matcher, &dirs.root, &log).await?;

    // Mapper: glomap if configured (much faster global SfM), else colmap.
    let (mapper_tool, mapper_args) = match &cfg.tools.glomap {
        Some(glomap) => (
            glomap.as_str(),
            vec![
                "mapper".to_string(),
                "--database_path".into(), db.clone(),
                "--image_path".into(), images.clone(),
                "--output_path".into(), sparse.clone(),
            ],
        ),
        None => (
            colmap.as_str(),
            vec![
                "mapper".to_string(),
                "--database_path".into(), db.clone(),
                "--image_path".into(), images.clone(),
                "--output_path".into(), sparse.clone(),
            ],
        ),
    };
    run_tool(mapper_tool, &mapper_args, &dirs.root, &log).await?;

    // COLMAP may emit multiple disconnected reconstructions (sparse/0, sparse/1, …).
    // Brush loads *every* model under sparse/ — dual coordinate systems look like an
    // "exploded" splat with only one coherent pocket. Keep the largest model only.
    keep_largest_sparse_model(colmap, dirs, &log).await?;

    // Count registered images / points via `colmap model_analyzer` (best-effort).
    // model_analyzer prints to stdout+stderr; run_tool merges both into `output`.
    let model0 = dirs.sparse().join("0");
    if model0.exists()
        && let Ok(out) = run_tool(
            colmap,
            &["model_analyzer".to_string(), "--path".into(), model0.display().to_string()],
            &dirs.root,
            &log,
        )
        .await
    {
        parse_model_analyzer(&out.output, rec);
    }
    Ok(())
}

/// Prefer the reconstruction with the most registered images (tie-break: points).
/// Renames it to `sparse/0` and removes sibling models.
async fn keep_largest_sparse_model(
    colmap: &str,
    dirs: &JobDirs,
    log: &Path,
) -> Result<(), RunError> {
    let sparse = dirs.sparse();
    if !sparse.is_dir() {
        return Ok(());
    }

    let mut models: Vec<(u32, u32, std::path::PathBuf)> = Vec::new();
    let rd = std::fs::read_dir(&sparse)?;
    for ent in rd.flatten() {
        let path = ent.path();
        if !path.is_dir() {
            continue;
        }
        // Expect numeric COLMAP model dirs: 0, 1, 2, …
        if ent
            .file_name()
            .to_str()
            .is_none_or(|s| !s.chars().all(|c| c.is_ascii_digit()))
        {
            continue;
        }
        let (n_img, n_pts) = analyze_model_counts(colmap, &path, dirs, log).await;
        models.push((n_img, n_pts, path));
    }

    if models.len() <= 1 {
        return Ok(());
    }

    models.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    let (best_img, best_pts, best_path) = models[0].clone();
    tracing::warn!(
        models = models.len(),
        best_images = best_img,
        best_points = best_pts,
        best = %best_path.display(),
        "COLMAP produced multiple reconstructions; keeping largest only for train"
    );

    // Park winner, wipe siblings, move winner to sparse/0.
    let park = sparse.join("_best_keep");
    if park.exists() {
        let _ = std::fs::remove_dir_all(&park);
    }
    std::fs::rename(&best_path, &park)?;
    for (_, _, path) in models.into_iter().skip(1) {
        let _ = std::fs::remove_dir_all(path);
    }
    // Place winner at sparse/0.
    let dest = sparse.join("0");
    if dest.exists() {
        let _ = std::fs::remove_dir_all(&dest);
    }
    std::fs::rename(&park, &dest)?;

    // Append a note into the sfm log for operators.
    if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(log) {
        use std::io::Write;
        let _ = writeln!(
            f,
            "\n# keep_largest_sparse_model: kept sparse/0 with ~{best_img} images / ~{best_pts} points; removed other models\n"
        );
    }
    Ok(())
}

async fn analyze_model_counts(
    colmap: &str,
    model: &Path,
    dirs: &JobDirs,
    log: &Path,
) -> (u32, u32) {
    let mut n_img = 0u32;
    let mut n_pts = 0u32;
    if let Ok(out) = run_tool(
        colmap,
        &[
            "model_analyzer".to_string(),
            "--path".into(),
            model.display().to_string(),
        ],
        &dirs.root,
        log,
    )
    .await
    {
        for line in out.output.lines() {
            let line = line.trim();
            // COLMAP 3.x: "Registered images: N" or log-prefixed variants.
            if let Some(v) = line
                .rsplit_once("Registered images:")
                .map(|(_, r)| r.trim())
                && let Ok(n) = v.parse()
            {
                n_img = n;
            }
            if let Some(v) = line.rsplit_once("Points:").map(|(_, r)| r.trim())
                // Avoid matching "Mean track length" etc. — Points: is its own line.
                && let Ok(n) = v.split_whitespace().next().unwrap_or("").parse()
            {
                n_pts = n;
            }
        }
    }
    // Fallback: points3D.bin size as weak signal if analyzer failed.
    if n_img == 0 && n_pts == 0
        && let Ok(meta) = std::fs::metadata(model.join("points3D.bin"))
    {
        n_pts = (meta.len() / 64).min(u32::MAX as u64) as u32;
    }
    (n_img, n_pts)
}

fn parse_model_analyzer(output: &str, rec: &mut JobRecord) {
    for line in output.lines() {
        let line = line.trim();
        if let Some(v) = line.rsplit_once("Registered images:").map(|(_, r)| r.trim())
            && let Ok(n) = v.parse::<u32>()
        {
            rec.metrics.n_registered = Some(n);
        }
        if let Some(v) = line.rsplit_once("Points:").map(|(_, r)| r.trim())
            && let Ok(n) = v.split_whitespace().next().unwrap_or("").parse::<u64>()
        {
            rec.metrics.n_points = Some(n);
        }
    }
}

/// Stage 4 — Brush training. Consumes the COLMAP dataset dir, exports
/// `artifacts/splat.ply`. Flag names match brush-cli's clap definitions
/// (total-train-iters, max-resolution, max-splats, sh-degree, export-*).
pub async fn train(cfg: &PipelineConfig, dirs: &JobDirs, rec: &mut JobRecord) -> Result<(), RunError> {
    let t = &cfg.train;
    let mut args = vec![
        dirs.dataset().display().to_string(),
        // brush v0.3.0 headless CLI names this --total-steps (was --total-train-iters
        // on the older brush-cli git build this stage was first written against).
        "--total-steps".into(), t.total_iters.to_string(),
        "--max-resolution".into(), t.max_resolution.to_string(),
        "--max-splats".into(), t.max_splats.to_string(),
        "--sh-degree".into(), t.sh_degree.to_string(),
        // Export only at the end, straight into artifacts/.
        "--export-every".into(), t.total_iters.to_string(),
        "--export-path".into(), dirs.artifacts().display().to_string(),
        "--export-name".into(), "splat.ply".into(),
        "--eval-every".into(), t.total_iters.to_string(),
    ];
    args.extend(t.extra_args.iter().cloned());
    run_tool(&cfg.tools.brush, &args, &dirs.root, &stage_log(&dirs.logs(), "train")).await?;

    let ply = dirs.artifacts().join("splat.ply");
    if let Ok(meta) = std::fs::metadata(&ply) {
        rec.metrics.splat_ply_bytes = Some(meta.len());
    }
    rec.artifacts.insert("ply".into(), "splat.ply".into());
    Ok(())
}

/// Stage 5 — compress ply → sog via splat-transform. Skipped (with a log
/// line) when no tool is configured; the raw ply is still a deliverable.
pub async fn compress(cfg: &PipelineConfig, dirs: &JobDirs, rec: &mut JobRecord) -> Result<(), RunError> {
    let Some(tool) = &cfg.tools.splat_transform else {
        tracing::info!("compress: no splat_transform configured, skipping");
        return Ok(());
    };
    let out_name = format!("splat.{}", cfg.compress.format);
    let args = vec![
        dirs.artifacts().join("splat.ply").display().to_string(),
        dirs.artifacts().join(&out_name).display().to_string(),
    ];
    run_tool(tool, &args, &dirs.root, &stage_log(&dirs.logs(), "compress")).await?;

    if let Ok(meta) = std::fs::metadata(dirs.artifacts().join(&out_name)) {
        rec.metrics.splat_sog_bytes = Some(meta.len());
    }
    rec.artifacts.insert(cfg.compress.format.clone(), out_name);
    Ok(())
}

/// Stage 6 — write `world_model.json` (ADR-078 W0) + `artifacts/manifest.json`.
///
/// On successful package, the job exports a scene-level world-model stub
/// (`SPLAT_SCENE` tag) with optional AABB from COLMAP sparse points / PLY.
/// Counts stay at 0 until structure stages (W1+). BVH is not published yet
/// (`bvh_published: false`) — splat-pipeline stays free of clawft-bvh.
pub async fn package(_cfg: &PipelineConfig, dirs: &JobDirs, rec: &mut JobRecord) -> Result<(), RunError> {
    let world_model = crate::world_model::write_world_model(dirs, &rec.job_id, &mut rec.artifacts)?;

    rec.artifacts.insert("manifest".into(), "manifest.json".into());
    let manifest = serde_json::json!({
        "job_id": rec.job_id,
        "source": rec.source,
        "artifacts": rec.artifacts,
        "metrics": rec.metrics,
        "timings_ms": rec.timings_ms,
        // Portable summary (full document also in artifacts/world_model.json).
        "world_model": {
            "frame": world_model["frame"],
            "scale_m": world_model["scale_m"],
            "objects": world_model["objects"],
            "surfaces": world_model["surfaces"],
            "volumes": world_model["volumes"],
            "bvh_published": world_model["bvh_published"],
        },
    });
    std::fs::write(
        dirs.artifacts().join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).expect("manifest serializes"),
    )?;
    Ok(())
}
