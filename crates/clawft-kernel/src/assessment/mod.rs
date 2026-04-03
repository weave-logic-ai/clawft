//! Project assessment service for WeftOS kernel.
//!
//! Provides automated codebase analysis: file scanning, complexity
//! detection, >500-line warnings, TODO tracking, and optional
//! tree-sitter symbol extraction. Supports peer linking for
//! cross-project comparison and a pluggable analyzer registry
//! (ADR-023) with 5 built-in analyzers.

pub mod analyzer;
pub mod analyzers;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::health::HealthStatus;
use crate::service::{ServiceType, SystemService};

pub use analyzer::{AnalysisContext, Analyzer, AnalyzerRegistry, AssessmentDiff};

// ── Trigger configuration ──────────────────────────────────────

/// Top-level configuration loaded from `.weftos/weave.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AssessmentConfig {
    /// Project metadata for multi-company namespace isolation.
    #[serde(default)]
    pub project: Option<ProjectConfig>,
    /// Assessment-specific settings.
    #[serde(default)]
    pub assessment: Option<AssessmentSettings>,
}

/// Project-level metadata used for namespace isolation and peer matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Human-readable project name.
    pub name: String,
    /// Organisation slug — only projects sharing the same org can link.
    #[serde(default)]
    pub org: Option<String>,
    /// Deployment environment label (development, staging, production).
    #[serde(default)]
    pub environment: Option<String>,
}

/// Assessment section of weave.toml.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AssessmentSettings {
    /// Schema version.
    #[serde(default)]
    pub version: Option<u32>,
    /// Source file configuration.
    #[serde(default)]
    pub sources: Option<SourcesConfig>,
    /// Trigger configuration.
    #[serde(default)]
    pub triggers: Option<TriggersConfig>,
    /// Reporting options.
    #[serde(default)]
    pub reporting: Option<ReportingConfig>,
}

/// Source file patterns.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourcesConfig {
    #[serde(default)]
    pub files: Option<FilePatterns>,
}

/// Glob patterns for file inclusion/exclusion.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilePatterns {
    #[serde(default)]
    pub patterns: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Trigger configuration block.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TriggersConfig {
    /// Filesystem-watch trigger.
    #[serde(default)]
    pub filesystem: Option<FilesystemTrigger>,
    /// Cron-based scheduled trigger.
    #[serde(default)]
    pub scheduled: Option<ScheduledTrigger>,
}

/// Filesystem watcher trigger settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemTrigger {
    pub enabled: bool,
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    #[serde(default)]
    pub patterns: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

fn default_debounce_ms() -> u64 {
    2000
}

/// Scheduled (cron) trigger settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTrigger {
    pub enabled: bool,
    #[serde(default = "default_cron")]
    pub cron: String,
    #[serde(default = "default_scope")]
    pub scope: String,
}

fn default_cron() -> String {
    "0 2 * * *".to_string()
}

fn default_scope() -> String {
    "full".to_string()
}

/// Reporting configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReportingConfig {
    #[serde(default)]
    pub default_format: Option<String>,
    #[serde(default)]
    pub save_artifacts: Option<bool>,
}

// ── Report types ────────────────────────────────────────────────

/// Full assessment report produced by a scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentReport {
    /// When the assessment was run.
    pub timestamp: DateTime<Utc>,
    /// Scope that was used (full, commit, ci, dependency).
    pub scope: String,
    /// Root directory that was scanned.
    pub project: String,
    /// Number of files scanned.
    pub files_scanned: usize,
    /// Aggregate summary metrics.
    pub summary: AssessmentSummary,
    /// Individual findings (warnings, issues).
    pub findings: Vec<Finding>,
    /// Which analyzers were executed.
    pub analyzers_run: Vec<String>,
}

/// Aggregate metrics from an assessment.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssessmentSummary {
    pub total_files: usize,
    pub lines_of_code: usize,
    pub rust_files: usize,
    pub typescript_files: usize,
    pub config_files: usize,
    pub doc_files: usize,
    pub dependency_files: usize,
    pub complexity_warnings: usize,
    pub coherence_score: f64,
    pub symbols_extracted: usize,
    pub avg_complexity: f64,
}

/// A single finding from the assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub severity: String,
    pub category: String,
    pub file: String,
    pub line: Option<usize>,
    pub message: String,
}

/// A linked peer project for cross-project comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub name: String,
    pub location: String,
    pub linked_at: DateTime<Utc>,
    pub last_assessment: Option<AssessmentReport>,
}

/// Comparison between local and remote peer assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub local: AssessmentReport,
    pub remote_name: String,
    pub remote: AssessmentReport,
    pub shared_deps: Vec<String>,
}

// ── Service ─────────────────────────────────────────────────────

/// Project assessment service.
///
/// Scans a project directory for code quality signals, complexity
/// warnings, and structural issues. Uses a pluggable `AnalyzerRegistry`
/// so that external crates can register custom analyzers. Optionally
/// uses tree-sitter for Rust symbol extraction and complexity analysis.
pub struct AssessmentService {
    started: AtomicBool,
    latest: Mutex<Option<AssessmentReport>>,
    peers: Mutex<Vec<PeerInfo>>,
    /// Path to the previous report JSON for diff computation.
    previous_report_path: Mutex<Option<PathBuf>>,
}

impl AssessmentService {
    pub fn new() -> Self {
        Self {
            started: AtomicBool::new(false),
            latest: Mutex::new(None),
            peers: Mutex::new(Vec::new()),
            previous_report_path: Mutex::new(None),
        }
    }

    /// Set the path to a previous assessment report JSON file.
    ///
    /// When set, `run_assessment` will load it and pass it to analyzers
    /// via `AnalysisContext`, and `diff_latest` will use it to compute
    /// an `AssessmentDiff`.
    pub fn set_previous_report_path(&self, path: PathBuf) {
        *self.previous_report_path.lock().unwrap() = Some(path);
    }

    /// Load the previous report from the configured path, if any.
    fn load_previous_report(&self) -> Option<AssessmentReport> {
        let guard = self.previous_report_path.lock().unwrap();
        let path = guard.as_ref()?;
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Compute a diff between the latest report and the previous one.
    pub fn diff_latest(&self) -> Option<AssessmentDiff> {
        let current = self.get_latest()?;
        let previous = self.load_previous_report()?;
        Some(analyzer::diff_reports(&current, &previous))
    }

    /// Run the full assessment pipeline on `project_dir`.
    ///
    /// `scope` selects what to scan:
    /// - `"full"` — all files under project_dir
    /// - `"commit"` — only files changed in the last git commit
    /// - `"ci"` — CI config files only
    /// - `"dependency"` — dependency manifests only
    ///
    /// `format` is reserved for future output formatting (currently ignored).
    pub fn run_assessment(
        &self,
        project_dir: &Path,
        scope: &str,
        _format: &str,
    ) -> Result<AssessmentReport, String> {
        self.run_assessment_with_registry(
            project_dir,
            scope,
            _format,
            AnalyzerRegistry::with_defaults(),
        )
    }

    /// Run assessment with a custom analyzer registry.
    pub fn run_assessment_with_registry(
        &self,
        project_dir: &Path,
        scope: &str,
        _format: &str,
        registry: AnalyzerRegistry,
    ) -> Result<AssessmentReport, String> {
        let files = match scope {
            "commit" => collect_git_changed_files(project_dir)?,
            "ci" => collect_files_filtered(project_dir, |p| is_ci_file(p)),
            "dependency" => collect_files_filtered(project_dir, |p| is_dependency_file(p)),
            _ => collect_all_files(project_dir),
        };

        let mut summary = AssessmentSummary::default();
        #[allow(unused_mut)]
        let mut total_complexity_sum: f64 = 0.0;
        #[allow(unused_mut)]
        let mut complexity_count: usize = 0;

        // Classify files and compute line counts + tree-sitter metrics
        let mut ts_findings = Vec::new();
        for path in &files {
            let rel = path.strip_prefix(project_dir).unwrap_or(path);
            let _rel_str = rel.display().to_string();

            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            match ext {
                "rs" => summary.rust_files += 1,
                "ts" | "tsx" => summary.typescript_files += 1,
                "toml" | "yaml" | "yml" | "json" if is_config_file(path) => {
                    summary.config_files += 1;
                }
                "md" | "txt" | "adoc" => summary.doc_files += 1,
                _ => {}
            }
            if is_dependency_file(path) {
                summary.dependency_files += 1;
            }

            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let line_count = content.lines().count();
            summary.lines_of_code += line_count;

            // Tree-sitter analysis for Rust files
            #[cfg(feature = "treesitter")]
            if ext == "rs" {
                if let Ok(tree) = clawft_plugin_treesitter::analysis::parse_source(
                    &content,
                    clawft_plugin_treesitter::types::Language::Rust,
                ) {
                    let symbols = clawft_plugin_treesitter::analysis::extract_symbols(
                        &tree,
                        &content,
                        clawft_plugin_treesitter::types::Language::Rust,
                    );
                    summary.symbols_extracted += symbols.len();

                    let metrics = clawft_plugin_treesitter::analysis::calculate_complexity(
                        &tree,
                        &content,
                        clawft_plugin_treesitter::types::Language::Rust,
                    );
                    for func in &metrics.functions {
                        total_complexity_sum += func.complexity as f64;
                        complexity_count += 1;
                        if func.complexity > 10 {
                            summary.complexity_warnings += 1;
                            ts_findings.push(Finding {
                                severity: "warning".into(),
                                category: "complexity".into(),
                                file: _rel_str.clone(),
                                line: Some(func.start_line),
                                message: format!(
                                    "Function '{}' has cyclomatic complexity {}",
                                    func.name, func.complexity
                                ),
                            });
                        }
                    }
                }
            }
        }

        // Run pluggable analyzers
        let previous_report = self.load_previous_report();
        let context = AnalysisContext {
            scope: scope.to_string(),
            previous_report,
        };
        let analyzer_ids = registry.analyzer_ids();
        let mut findings = registry.run_all(project_dir, &files, &context);

        // Append tree-sitter findings
        findings.append(&mut ts_findings);

        // Update summary with analyzer-produced complexity warnings
        let warning_count = findings.iter().filter(|f| f.severity == "warning").count();
        // Add complexity warnings from the size category produced by ComplexityAnalyzer
        summary.complexity_warnings += findings
            .iter()
            .filter(|f| f.category == "size" && f.severity == "warning")
            .count();

        summary.total_files = files.len();
        summary.avg_complexity = if complexity_count > 0 {
            total_complexity_sum / complexity_count as f64
        } else {
            0.0
        };
        // Simple coherence score: ratio of files without warnings
        summary.coherence_score = if summary.total_files > 0 {
            1.0 - (warning_count as f64 / summary.total_files as f64).min(1.0)
        } else {
            1.0
        };

        let report = AssessmentReport {
            timestamp: Utc::now(),
            scope: scope.to_string(),
            project: project_dir.display().to_string(),
            files_scanned: files.len(),
            summary,
            findings,
            analyzers_run: analyzer_ids,
        };

        *self.latest.lock().unwrap() = Some(report.clone());
        debug!(
            scope = scope,
            files = report.files_scanned,
            "assessment complete"
        );
        Ok(report)
    }

    /// Load and parse `.weftos/weave.toml` from the given project directory.
    ///
    /// Returns `None` if the file does not exist or cannot be parsed.
    pub fn load_config(&self, project: &Path) -> Option<AssessmentConfig> {
        let config_path = project.join(".weftos/weave.toml");
        let content = std::fs::read_to_string(&config_path).ok()?;
        match toml::from_str::<AssessmentConfig>(&content) {
            Ok(cfg) => Some(cfg),
            Err(e) => {
                tracing::warn!(
                    path = %config_path.display(),
                    error = %e,
                    "failed to parse assessment config"
                );
                None
            }
        }
    }

    /// Returns the latest assessment report, if any.
    pub fn get_latest(&self) -> Option<AssessmentReport> {
        self.latest.lock().unwrap().clone()
    }

    /// Returns all linked peers.
    pub fn list_peers(&self) -> Vec<PeerInfo> {
        self.peers.lock().unwrap().clone()
    }

    /// Link a peer project for comparison.
    pub fn link_peer(&self, name: String, location: String) -> Result<(), String> {
        let mut peers = self.peers.lock().unwrap();
        if peers.iter().any(|p| p.name == name) {
            return Err(format!("peer '{name}' already linked"));
        }
        peers.push(PeerInfo {
            name,
            location,
            linked_at: Utc::now(),
            last_assessment: None,
        });
        Ok(())
    }

    /// Run assessment on a peer and compare with the latest local assessment.
    pub fn compare_with_peer(&self, peer_name: &str) -> Result<ComparisonReport, String> {
        let local = self
            .get_latest()
            .ok_or_else(|| "no local assessment available; run an assessment first".to_string())?;

        let peers = self.peers.lock().unwrap();
        let peer = peers
            .iter()
            .find(|p| p.name == peer_name)
            .ok_or_else(|| format!("peer '{peer_name}' not found"))?
            .clone();
        drop(peers);

        let peer_dir = PathBuf::from(&peer.location);
        if !peer_dir.exists() {
            return Err(format!(
                "peer directory '{}' does not exist",
                peer.location
            ));
        }

        let remote = self.run_assessment(&peer_dir, &local.scope, "json")?;

        // Update peer's last_assessment
        {
            let mut peers = self.peers.lock().unwrap();
            if let Some(p) = peers.iter_mut().find(|p| p.name == peer_name) {
                p.last_assessment = Some(remote.clone());
            }
        }

        // Find shared dependency files by name
        let local_deps: std::collections::HashSet<String> = local
            .findings
            .iter()
            .filter(|f| f.category == "dependency")
            .map(|f| f.file.clone())
            .collect();
        let remote_deps: std::collections::HashSet<String> = remote
            .findings
            .iter()
            .filter(|f| f.category == "dependency")
            .map(|f| f.file.clone())
            .collect();
        let shared_deps: Vec<String> = local_deps.intersection(&remote_deps).cloned().collect();

        Ok(ComparisonReport {
            local,
            remote_name: peer_name.to_string(),
            remote,
            shared_deps,
        })
    }
}

impl Default for AssessmentService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SystemService for AssessmentService {
    fn name(&self) -> &str {
        "assessment"
    }

    fn service_type(&self) -> ServiceType {
        ServiceType::Custom("assessment".into())
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.started.store(true, Ordering::Relaxed);
        tracing::info!("assessment service started");
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.started.store(false, Ordering::Relaxed);
        tracing::info!("assessment service stopped");
        Ok(())
    }

    async fn health_check(&self) -> HealthStatus {
        if self.started.load(Ordering::Relaxed) {
            HealthStatus::Healthy
        } else {
            HealthStatus::Degraded("not started".into())
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────

fn collect_all_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_dir(dir, &mut files, &|_| true);
    files
}

fn collect_files_filtered<F: Fn(&Path) -> bool>(dir: &Path, predicate: F) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_dir(dir, &mut files, &predicate);
    files
}

fn walk_dir<F: Fn(&Path) -> bool>(dir: &Path, out: &mut Vec<PathBuf>, predicate: &F) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // Skip hidden dirs and common noise
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
        }
        if path.is_dir() {
            walk_dir(&path, out, predicate);
        } else if predicate(&path) {
            out.push(path);
        }
    }
}

fn collect_git_changed_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD~1"])
        .current_dir(dir)
        .output()
        .map_err(|e| format!("failed to run git diff: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "git diff failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| dir.join(l))
        .filter(|p| p.exists())
        .collect();
    Ok(files)
}

fn is_ci_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let path_str = path.display().to_string();
    name.contains("ci")
        || name.contains("CI")
        || path_str.contains(".github/workflows")
        || name == "Jenkinsfile"
        || name == ".gitlab-ci.yml"
}

fn is_dependency_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    matches!(
        name,
        "Cargo.toml"
            | "Cargo.lock"
            | "package.json"
            | "package-lock.json"
            | "yarn.lock"
            | "pnpm-lock.yaml"
            | "go.mod"
            | "go.sum"
            | "requirements.txt"
            | "Pipfile"
            | "Pipfile.lock"
    )
}

fn is_config_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    // Dependency files are not config files
    if is_dependency_file(path) {
        return false;
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext, "toml" | "yaml" | "yml" | "json")
        || name == ".editorconfig"
        || name == ".rustfmt.toml"
        || name == "clippy.toml"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        // Create a Rust file (>500 lines)
        let mut big = String::new();
        for i in 0..510 {
            big.push_str(&format!("// line {i}\n"));
        }
        fs::write(dir.path().join("big.rs"), &big).unwrap();

        // Create a small Rust file with a TODO
        fs::write(
            dir.path().join("small.rs"),
            "fn main() {\n    // TODO: implement\n}\n",
        )
        .unwrap();

        // Create a config file
        fs::write(dir.path().join("config.toml"), "[section]\nkey = 1\n").unwrap();

        // Create a markdown doc
        fs::write(dir.path().join("README.md"), "# Readme\n").unwrap();

        // Create a dep file
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\n[dependencies]\nserde = \"1.0\"\n",
        )
        .unwrap();

        dir
    }

    #[test]
    fn full_assessment_scans_files() {
        let dir = setup_test_dir();
        let svc = AssessmentService::new();
        let report = svc
            .run_assessment(dir.path(), "full", "json")
            .unwrap();

        assert_eq!(report.scope, "full");
        assert!(report.files_scanned >= 4);
        assert!(report.summary.rust_files >= 2);
        assert!(report.summary.doc_files >= 1);
        assert!(report.summary.dependency_files >= 1);
    }

    #[test]
    fn detects_large_files() {
        let dir = setup_test_dir();
        let svc = AssessmentService::new();
        let report = svc.run_assessment(dir.path(), "full", "json").unwrap();

        let size_warnings: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.category == "size")
            .collect();
        assert!(!size_warnings.is_empty(), "should detect >500 line file");
    }

    #[test]
    fn detects_todos() {
        let dir = setup_test_dir();
        let svc = AssessmentService::new();
        let report = svc.run_assessment(dir.path(), "full", "json").unwrap();

        let todos: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.category == "todo")
            .collect();
        assert!(!todos.is_empty(), "should detect TODO comments");
    }

    #[test]
    fn get_latest_returns_last() {
        let dir = setup_test_dir();
        let svc = AssessmentService::new();
        assert!(svc.get_latest().is_none());

        svc.run_assessment(dir.path(), "full", "json").unwrap();
        assert!(svc.get_latest().is_some());
    }

    #[test]
    fn load_config_parses_weave_toml() {
        let dir = tempfile::tempdir().unwrap();
        let weftos = dir.path().join(".weftos");
        fs::create_dir_all(&weftos).unwrap();
        fs::write(
            weftos.join("weave.toml"),
            r#"
[project]
name = "acme-app"
org = "acme"
environment = "development"

[assessment]
version = 1

[assessment.triggers.filesystem]
enabled = true
debounce_ms = 3000
patterns = ["**/*.rs"]
exclude = ["target/**"]

[assessment.triggers.scheduled]
enabled = false
cron = "0 4 * * *"
scope = "full"
"#,
        )
        .unwrap();

        let svc = AssessmentService::new();
        let cfg = svc.load_config(dir.path()).expect("should parse config");

        let project = cfg.project.expect("project section");
        assert_eq!(project.name, "acme-app");
        assert_eq!(project.org.as_deref(), Some("acme"));
        assert_eq!(project.environment.as_deref(), Some("development"));

        let triggers = cfg.assessment.unwrap().triggers.unwrap();
        let fs_trigger = triggers.filesystem.unwrap();
        assert!(fs_trigger.enabled);
        assert_eq!(fs_trigger.debounce_ms, 3000);
        assert_eq!(fs_trigger.patterns, vec!["**/*.rs"]);

        let sched = triggers.scheduled.unwrap();
        assert!(!sched.enabled);
        assert_eq!(sched.cron, "0 4 * * *");
    }

    #[test]
    fn load_config_returns_none_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let svc = AssessmentService::new();
        assert!(svc.load_config(dir.path()).is_none());
    }

    #[test]
    fn peer_link_and_list() {
        let svc = AssessmentService::new();
        assert!(svc.list_peers().is_empty());

        svc.link_peer("other".into(), "/tmp/other".into()).unwrap();
        assert_eq!(svc.list_peers().len(), 1);
        assert_eq!(svc.list_peers()[0].name, "other");

        // Duplicate link fails
        assert!(svc.link_peer("other".into(), "/tmp/other2".into()).is_err());
    }

    #[test]
    fn dependency_scope_filters() {
        let dir = setup_test_dir();
        let svc = AssessmentService::new();
        let report = svc
            .run_assessment(dir.path(), "dependency", "json")
            .unwrap();

        // Should only scan dependency files
        assert_eq!(report.files_scanned, 1); // Cargo.toml
    }

    #[test]
    fn coherence_score_is_bounded() {
        let dir = setup_test_dir();
        let svc = AssessmentService::new();
        let report = svc.run_assessment(dir.path(), "full", "json").unwrap();

        assert!(report.summary.coherence_score >= 0.0);
        assert!(report.summary.coherence_score <= 1.0);
    }

    #[test]
    fn report_includes_analyzers_run() {
        let dir = setup_test_dir();
        let svc = AssessmentService::new();
        let report = svc.run_assessment(dir.path(), "full", "json").unwrap();

        assert!(!report.analyzers_run.is_empty());
        assert!(report.analyzers_run.contains(&"complexity".to_string()));
        assert!(report.analyzers_run.contains(&"dependency".to_string()));
        assert!(report.analyzers_run.contains(&"security".to_string()));
        assert!(report.analyzers_run.contains(&"topology".to_string()));
        assert!(report.analyzers_run.contains(&"data_source".to_string()));
    }

    #[test]
    fn dependency_analyzer_finds_deps() {
        let dir = setup_test_dir();
        let svc = AssessmentService::new();
        let report = svc.run_assessment(dir.path(), "full", "json").unwrap();

        let dep_findings: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.category == "dependency")
            .collect();
        assert!(
            !dep_findings.is_empty(),
            "should detect dependencies in Cargo.toml"
        );
    }

    #[test]
    fn diff_reports_computes_deltas() {
        let prev = AssessmentReport {
            timestamp: Utc::now(),
            scope: "full".into(),
            project: "/test".into(),
            files_scanned: 2,
            summary: AssessmentSummary {
                complexity_warnings: 1,
                coherence_score: 0.8,
                ..Default::default()
            },
            findings: vec![
                Finding {
                    severity: "warning".into(),
                    category: "size".into(),
                    file: "old.rs".into(),
                    line: None,
                    message: "File has 600 lines (>500 limit)".into(),
                },
            ],
            analyzers_run: vec!["complexity".into()],
        };

        let curr = AssessmentReport {
            timestamp: Utc::now(),
            scope: "full".into(),
            project: "/test".into(),
            files_scanned: 3,
            summary: AssessmentSummary {
                complexity_warnings: 2,
                coherence_score: 0.7,
                ..Default::default()
            },
            findings: vec![
                Finding {
                    severity: "warning".into(),
                    category: "size".into(),
                    file: "new.rs".into(),
                    line: None,
                    message: "File has 700 lines (>500 limit)".into(),
                },
            ],
            analyzers_run: vec!["complexity".into()],
        };

        let diff = analyzer::diff_reports(&curr, &prev);
        assert!(diff.files_added.contains(&"new.rs".to_string()));
        assert!(diff.files_removed.contains(&"old.rs".to_string()));
        assert_eq!(diff.findings_new.len(), 1);
        assert_eq!(diff.findings_resolved.len(), 1);
        assert_eq!(diff.complexity_delta, 1);
        assert!((diff.coherence_delta - (-0.1)).abs() < 0.001);
    }

    #[test]
    fn custom_registry_runs_only_registered() {
        let dir = setup_test_dir();
        let svc = AssessmentService::new();

        // Empty registry — no analyzer findings
        let registry = AnalyzerRegistry::new();
        let report = svc
            .run_assessment_with_registry(dir.path(), "full", "json", registry)
            .unwrap();

        assert!(report.analyzers_run.is_empty());
        assert!(report.findings.is_empty());
    }
}
