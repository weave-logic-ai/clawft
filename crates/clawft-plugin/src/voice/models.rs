//! Model download and cache management (WEFT-215).
//!
//! Downloads STT/TTS/VAD models over HTTP, verifies SHA-256 integrity,
//! reports progress on stderr, and reuses a verified on-disk cache.
//!
//! # Integrity (WEFT-215 + WEFT-209 / SC-7)
//!
//! * **Download path (this module):** every fetch requires a non-placeholder
//!   64-hex SHA-256 expected hash. Placeholder / missing hashes are refused —
//!   we never treat "file exists" as integrity.
//! * **Runtime path (SC-7):** model *bundles* may additionally ship a signed
//!   manifest (`model.manifest.json` + `model.manifest.sig`) verified by
//!   `clawft-service-whisper::manifest::verify_model_dir` against a trusted
//!   public key under `~/.clawft/trust-roots/voice/`.
//!
//! [`ModelInfo::sha256_hint`] is therefore an `Option<String>`:
//! * `Some(<64-hex>)` — hard-pin; required for `ensure_model` / cache reuse.
//! * `None` or any placeholder string — refused (no download, not "cached").
//!
//! Catalog entries without pinned hashes are listed for discovery but
//! `weft voice setup` skips them until operators / release tooling pin real
//! digests (or SC-7 signed manifests provide them).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::PluginError;

/// Prefix / markers that must never be accepted as a real hash.
const PLACEHOLDER_MARKERS: &[&str] = &[
    "PLACEHOLDER",
    "placeholder",
    "TODO",
    "FIXME",
    "xxx",
    "XXX",
    "0000000000000000000000000000000000000000000000000000000000000000",
];

/// Manages voice model downloads, caching, and integrity verification.
pub struct ModelDownloadManager {
    cache_dir: PathBuf,
    /// Optional injected HTTP client (tests pin a fixture base via absolute URLs
    /// on [`ModelInfo`]; the client is still useful for custom timeouts / TLS).
    #[cfg(feature = "voice")]
    http: reqwest::Client,
}

/// Information about a downloadable model.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Model identifier (e.g., "sherpa-onnx-streaming-zipformer-en-20M").
    pub id: String,
    /// Download URL (absolute).
    pub url: String,
    /// Optional static SHA-256 pin. `None` / placeholder → refuse download
    /// and refuse to treat an on-disk file as cached. See module docs.
    pub sha256_hint: Option<String>,
    /// File size in bytes (for progress reporting when Content-Length is absent).
    pub size_bytes: u64,
}

/// Result of a single ensure/download attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnsureOutcome {
    /// File already on disk and hash-verified.
    Cached(PathBuf),
    /// Freshly downloaded and hash-verified.
    Downloaded(PathBuf),
}

impl EnsureOutcome {
    /// Local path of the verified model file.
    pub fn path(&self) -> &Path {
        match self {
            EnsureOutcome::Cached(p) | EnsureOutcome::Downloaded(p) => p,
        }
    }
}

/// Return `true` when `hash` is missing, a known placeholder, or not a
/// 64-character lowercase/uppercase hex SHA-256 digest.
pub fn is_placeholder_hash(hash: Option<&str>) -> bool {
    let Some(h) = hash.map(str::trim).filter(|s| !s.is_empty()) else {
        return true;
    };
    if h.len() != 64 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }
    PLACEHOLDER_MARKERS
        .iter()
        .any(|m| h.eq_ignore_ascii_case(m) || h.contains(m))
}

/// Human-readable byte count for progress lines.
fn human_bytes(n: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let n = n as f64;
    if n >= GB {
        format!("{:.1} GiB", n / GB)
    } else if n >= MB {
        format!("{:.1} MiB", n / MB)
    } else if n >= KB {
        format!("{:.1} KiB", n / KB)
    } else {
        format!("{n:.0} B")
    }
}

/// Progress reporter that writes a single rewriting line to **stderr**.
///
/// Call once more after the download finishes (or use [`finish_stderr_progress`])
/// so the cursor advances past the progress line.
pub fn stderr_progress_line(model_id: &str) -> impl FnMut(u64, u64) + '_ {
    move |downloaded: u64, total: u64| {
        if total > 0 {
            let pct = (downloaded.saturating_mul(100)) / total;
            eprint!(
                "\r  downloading {model_id}: {pct:>3}% ({} / {})   ",
                human_bytes(downloaded),
                human_bytes(total)
            );
        } else {
            eprint!(
                "\r  downloading {model_id}: {}   ",
                human_bytes(downloaded)
            );
        }
        let _ = std::io::stderr().flush();
    }
}

/// End a stderr progress line with a trailing newline.
pub fn finish_stderr_progress() {
    eprintln!();
}

impl ModelDownloadManager {
    /// Create a new manager with the given cache directory.
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            #[cfg(feature = "voice")]
            http: reqwest::Client::new(),
        }
    }

    /// Create a manager with a custom HTTP client (timeouts, interceptors).
    #[cfg(feature = "voice")]
    pub fn with_client(cache_dir: PathBuf, http: reqwest::Client) -> Self {
        Self { cache_dir, http }
    }

    /// Default model cache directory: `~/.clawft/models/voice/`.
    pub fn default_cache_dir() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home)
                .join(".clawft")
                .join("models")
                .join("voice")
        } else {
            PathBuf::from(".clawft/models/voice")
        }
    }

    /// Get the cache directory path.
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Local path where a model file is stored (`cache_dir / model.id`).
    pub fn model_path(&self, model_id: &str) -> PathBuf {
        self.cache_dir.join(model_id)
    }

    /// Check if a model is already cached **and** its SHA-256 matches.
    ///
    /// Returns `false` when:
    /// * the expected hash is a placeholder / missing,
    /// * the file is absent,
    /// * the on-disk digest does not match the pin.
    pub fn is_cached(&self, model: &ModelInfo) -> bool {
        if is_placeholder_hash(model.sha256_hint.as_deref()) {
            return false;
        }
        let path = self.model_path(&model.id);
        if !path.is_file() {
            return false;
        }
        let expected = model.sha256_hint.as_deref().unwrap_or("");
        match sha256_hex_file(&path) {
            Ok(got) => got.eq_ignore_ascii_case(expected),
            Err(_) => false,
        }
    }

    /// Verify an on-disk file against an expected SHA-256 hex digest.
    pub fn verify_sha256(path: &Path, expected: &str) -> Result<bool, PluginError> {
        if is_placeholder_hash(Some(expected)) {
            return Err(PluginError::ExecutionFailed(format!(
                "refusing SHA-256 verify with placeholder/invalid hash for {}",
                path.display()
            )));
        }
        let got = sha256_hex_file(path)?;
        Ok(got.eq_ignore_ascii_case(expected.trim()))
    }

    /// Ensure `model` is present in the cache with a matching SHA-256.
    ///
    /// * Refuses placeholder / missing expected hashes.
    /// * Reuses a verified cache hit (no network).
    /// * Otherwise streams HTTP → temp file with optional progress, verifies,
    ///   then renames into place. A hash mismatch deletes the partial file.
    ///
    /// `progress` is invoked as `(bytes_downloaded, total_bytes)`;
    /// `total_bytes` may come from `Content-Length` or `model.size_bytes`.
    #[cfg(feature = "voice")]
    pub async fn ensure_model(
        &self,
        model: &ModelInfo,
        progress: Option<&mut dyn FnMut(u64, u64)>,
    ) -> Result<EnsureOutcome, PluginError> {
        let expected = model.sha256_hint.as_deref().map(str::trim).unwrap_or("");
        if is_placeholder_hash(Some(expected)) {
            return Err(PluginError::ExecutionFailed(format!(
                "refusing to download model '{}': missing or placeholder SHA-256 \
                 (pin a real 64-hex digest in ModelInfo::sha256_hint)",
                model.id
            )));
        }

        let path = self.model_path(&model.id);
        if path.is_file() {
            if Self::verify_sha256(&path, expected)? {
                return Ok(EnsureOutcome::Cached(path));
            }
            // Stale / tampered cache — remove and re-fetch.
            let _ = std::fs::remove_file(&path);
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let partial = path.with_extension("partial");
        if partial.exists() {
            let _ = std::fs::remove_file(&partial);
        }

        match progress {
            Some(cb) => {
                self.download_streaming(model, &partial, expected, Some(cb))
                    .await?
            }
            None => {
                self.download_streaming(model, &partial, expected, None)
                    .await?
            }
        }

        // Atomic-ish publish: rename partial → final after verify.
        // (verify already ran inside download_streaming against `partial`.)
        std::fs::rename(&partial, &path).map_err(|e| {
            PluginError::ExecutionFailed(format!(
                "failed to publish model '{}' to {}: {e}",
                model.id,
                path.display()
            ))
        })?;

        Ok(EnsureOutcome::Downloaded(path))
    }

    /// Download all models that have a real SHA-256 pin. Entries with
    /// placeholder / missing hashes are collected as skip reasons rather than
    /// hard-failing the whole batch (callers decide policy).
    #[cfg(feature = "voice")]
    pub async fn ensure_models_report(
        &self,
        models: &[ModelInfo],
    ) -> Vec<(String, Result<EnsureOutcome, String>)> {
        let mut out = Vec::with_capacity(models.len());
        for model in models {
            if is_placeholder_hash(model.sha256_hint.as_deref()) {
                out.push((
                    model.id.clone(),
                    Err(
                        "skipped: missing or placeholder SHA-256 (refusing download)".into(),
                    ),
                ));
                continue;
            }
            let mut line = stderr_progress_line(&model.id);
            let progress: &mut dyn FnMut(u64, u64) = &mut line;
            let result = self.ensure_model(model, Some(progress)).await;
            finish_stderr_progress();
            out.push((model.id.clone(), result.map_err(|e| e.to_string())));
        }
        out
    }

    #[cfg(feature = "voice")]
    async fn download_streaming(
        &self,
        model: &ModelInfo,
        dest: &Path,
        expected_sha256: &str,
        mut progress: Option<&mut dyn FnMut(u64, u64)>,
    ) -> Result<(), PluginError> {
        use futures_util::StreamExt;
        use std::io::Write as _;

        let response = self.http.get(&model.url).send().await.map_err(|e| {
            PluginError::ExecutionFailed(format!(
                "model '{}' download failed: {e}",
                model.id
            ))
        })?;

        if !response.status().is_success() {
            return Err(PluginError::ExecutionFailed(format!(
                "model '{}' download HTTP {}: {}",
                model.id,
                response.status(),
                model.url
            )));
        }

        let total = response
            .content_length()
            .filter(|&n| n > 0)
            .unwrap_or(model.size_bytes);

        let mut file = std::fs::File::create(dest).map_err(|e| {
            PluginError::ExecutionFailed(format!(
                "cannot create partial file {}: {e}",
                dest.display()
            ))
        })?;

        let mut hasher = Sha256::new();
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| {
                PluginError::ExecutionFailed(format!(
                    "model '{}' stream error: {e}",
                    model.id
                ))
            })?;
            file.write_all(&chunk).map_err(|e| {
                PluginError::ExecutionFailed(format!(
                    "model '{}' write error: {e}",
                    model.id
                ))
            })?;
            hasher.update(&chunk);
            downloaded = downloaded.saturating_add(chunk.len() as u64);
            if let Some(cb) = progress.as_mut() {
                cb(downloaded, total);
            }
        }
        file.flush()?;

        let got = hex::encode(hasher.finalize());
        if !got.eq_ignore_ascii_case(expected_sha256) {
            let _ = std::fs::remove_file(dest);
            return Err(PluginError::ExecutionFailed(format!(
                "model '{}' SHA-256 mismatch after download: expected {expected_sha256}, got {got}",
                model.id
            )));
        }
        Ok(())
    }

    /// List available STT models. Hashes must be pinned before download.
    pub fn available_stt_models() -> Vec<ModelInfo> {
        vec![ModelInfo {
            id: "sherpa-onnx-streaming-zipformer-en-20M".into(),
            url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-streaming-zipformer-en-20M.tar.bz2".into(),
            // Unpinned: refuse download until a real digest is committed.
            sha256_hint: None,
            size_bytes: 20_000_000,
        }]
    }

    /// List available TTS models. See [`Self::available_stt_models`] re: hash pinning.
    pub fn available_tts_models() -> Vec<ModelInfo> {
        vec![ModelInfo {
            id: "vits-piper-en_US-amy-medium".into(),
            url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-amy-medium.tar.bz2".into(),
            sha256_hint: None,
            size_bytes: 40_000_000,
        }]
    }

    /// List available VAD models. See [`Self::available_stt_models`] re: hash pinning.
    pub fn available_vad_models() -> Vec<ModelInfo> {
        vec![ModelInfo {
            id: "silero-vad-v5".into(),
            url: "https://github.com/snakers4/silero-vad/raw/master/files/silero_vad.onnx".into(),
            sha256_hint: None,
            size_bytes: 2_000_000,
        }]
    }

    /// Union of STT + TTS + VAD catalog entries (order: STT, TTS, VAD).
    pub fn all_canonical_models() -> Vec<ModelInfo> {
        let mut v = Self::available_stt_models();
        v.extend(Self::available_tts_models());
        v.extend(Self::available_vad_models());
        v
    }
}

/// Streaming SHA-256 of a file (hex-encoded, lowercase).
pub fn sha256_hex_file(path: &Path) -> Result<String, PluginError> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// SHA-256 of in-memory bytes (hex-encoded, lowercase). Useful for tests.
pub fn sha256_hex_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

// ── Tests (fixture HTTP server) ──────────────────────────────────────────

#[cfg(all(test, feature = "voice"))]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn fixture_body() -> &'static [u8] {
        b"weft-215-voice-model-fixture-bytes-v1"
    }

    fn fixture_sha() -> String {
        sha256_hex_bytes(fixture_body())
    }

    fn model_for(server: &MockServer, id: &str, sha: Option<String>) -> ModelInfo {
        ModelInfo {
            id: id.into(),
            url: format!("{}/models/{id}", server.uri()),
            sha256_hint: sha,
            size_bytes: fixture_body().len() as u64,
        }
    }

    async fn mount_fixture(server: &MockServer, id: &str) {
        Mock::given(method("GET"))
            .and(path(format!("/models/{id}")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(fixture_body())
                    .insert_header("content-length", fixture_body().len().to_string()),
            )
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn download_and_verify_from_fixture_server() {
        let server = MockServer::start().await;
        let id = "fixture-vad";
        mount_fixture(&server, id).await;

        let dir = tempfile::tempdir().unwrap();
        let mgr = ModelDownloadManager::new(dir.path().to_path_buf());
        let model = model_for(&server, id, Some(fixture_sha()));

        let mut ticks = Vec::new();
        let mut progress = |done: u64, total: u64| {
            ticks.push((done, total));
        };
        let progress_ref: &mut dyn FnMut(u64, u64) = &mut progress;

        let outcome = mgr
            .ensure_model(&model, Some(progress_ref))
            .await
            .expect("download ok");
        assert!(matches!(outcome, EnsureOutcome::Downloaded(_)));
        assert!(outcome.path().is_file());
        assert_eq!(std::fs::read(outcome.path()).unwrap(), fixture_body());
        assert!(mgr.is_cached(&model));
        assert!(
            !ticks.is_empty(),
            "progress callback should fire at least once"
        );
        let (done, total) = *ticks.last().unwrap();
        assert_eq!(done, fixture_body().len() as u64);
        assert_eq!(total, fixture_body().len() as u64);
    }

    #[tokio::test]
    async fn cache_reuse_skips_second_fetch() {
        let server = MockServer::start().await;
        let id = "fixture-cached";
        // Only one successful GET is mounted; a second fetch would 404.
        Mock::given(method("GET"))
            .and(path(format!("/models/{id}")))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(fixture_body()))
            .expect(1)
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let mgr = ModelDownloadManager::new(dir.path().to_path_buf());
        let model = model_for(&server, id, Some(fixture_sha()));

        let first = mgr.ensure_model(&model, None).await.unwrap();
        assert!(matches!(first, EnsureOutcome::Downloaded(_)));

        let second = mgr.ensure_model(&model, None).await.unwrap();
        assert!(matches!(second, EnsureOutcome::Cached(_)));
        assert_eq!(first.path(), second.path());
    }

    #[tokio::test]
    async fn refuses_placeholder_hash() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let mgr = ModelDownloadManager::new(dir.path().to_path_buf());

        for bad in [
            None,
            Some("PLACEHOLDER_SHA256_SILERO_VAD".into()),
            Some("todo".into()),
            Some("abcd".into()), // too short
            Some("g".repeat(64)), // not hex
        ] {
            let model = ModelInfo {
                id: "bad-hash".into(),
                url: format!("{}/models/bad", server.uri()),
                sha256_hint: bad.clone(),
                size_bytes: 1,
            };
            assert!(
                is_placeholder_hash(bad.as_deref()),
                "expected placeholder for {bad:?}"
            );
            assert!(!mgr.is_cached(&model));
            let err = mgr
                .ensure_model(&model, None)
                .await
                .expect_err("must refuse");
            let msg = err.to_string();
            assert!(
                msg.contains("placeholder") || msg.contains("missing"),
                "unexpected error: {msg}"
            );
        }
    }

    #[tokio::test]
    async fn rejects_tampered_payload() {
        let server = MockServer::start().await;
        let id = "tampered";
        Mock::given(method("GET"))
            .and(path(format!("/models/{id}")))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"not-the-expected-bytes"))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let mgr = ModelDownloadManager::new(dir.path().to_path_buf());
        let model = model_for(&server, id, Some(fixture_sha()));

        let err = mgr
            .ensure_model(&model, None)
            .await
            .expect_err("hash mismatch");
        assert!(
            err.to_string().contains("SHA-256 mismatch"),
            "got: {err}"
        );
        assert!(!mgr.model_path(id).exists());
        assert!(!mgr.model_path(id).with_extension("partial").exists());
    }

    #[tokio::test]
    async fn is_cached_false_when_on_disk_hash_wrong() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = ModelDownloadManager::new(dir.path().to_path_buf());
        let path = mgr.model_path("stale");
        std::fs::write(&path, b"stale-bytes").unwrap();
        let model = ModelInfo {
            id: "stale".into(),
            url: "http://127.0.0.1:1/unused".into(),
            sha256_hint: Some(fixture_sha()),
            size_bytes: 1,
        };
        assert!(!mgr.is_cached(&model));
    }

    #[test]
    fn placeholder_detection_accepts_real_hex() {
        let real = "a".repeat(64);
        assert!(!is_placeholder_hash(Some(&real)));
        assert!(is_placeholder_hash(None));
        assert!(is_placeholder_hash(Some("PLACEHOLDER_SHA256_STREAMING_STT")));
        assert!(is_placeholder_hash(Some("")));
    }

    #[test]
    fn canonical_catalog_is_discoverable() {
        let all = ModelDownloadManager::all_canonical_models();
        assert!(all.len() >= 3);
        // Until hashes are release-pinned, catalog entries must not pretend
        // to be integrity-safe (is_cached / ensure refuse placeholders).
        for m in &all {
            assert!(
                is_placeholder_hash(m.sha256_hint.as_deref()),
                "catalog entry {} still needs a real pin or must stay unpinned",
                m.id
            );
        }
    }
}
