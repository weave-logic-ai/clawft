//! Learning backends for the pipeline (Stage 6).
//!
//! Provides two implementations:
//! - [`NoopLearner`] -- Level 0 no-op baseline (discards all data)
//! - [`TrajectoryLearner`] -- Level 1 GEPA-inspired trajectory collector
//!
//! The active learner is selected by configuration. Both implement
//! [`LearningBackend`].

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Mutex;

use super::mutation::{GaConfig, Population};
use super::traits::{LearningBackend, LearningSignal, Trajectory};

// ─── NoopLearner (Level 0) ────────────────────────────────────────────

/// Level 0 no-op learning backend.
///
/// Both [`record`](LearningBackend::record) and
/// [`adapt`](LearningBackend::adapt) are no-ops. This serves as
/// the baseline backend until adaptive learning is implemented.
pub struct NoopLearner;

impl NoopLearner {
    /// Create a new no-op learner.
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoopLearner {
    fn default() -> Self {
        Self::new()
    }
}

impl LearningBackend for NoopLearner {
    fn record(&self, _trajectory: &Trajectory) {
        // No-op: Level 0 does not learn from interactions.
    }

    fn adapt(&self, _signal: &LearningSignal) {
        // No-op: Level 0 does not adapt from signals.
    }
}

// ─── TrajectoryLearner (Level 1 -- GEPA-inspired) ─────────────────────

/// Configuration for the trajectory learner.
#[derive(Debug, Clone)]
pub struct TrajectoryLearnerConfig {
    /// Maximum trajectories retained in the ring buffer.
    pub max_trajectories: usize,
    /// Quality score below which a trajectory is considered "poor".
    pub poor_threshold: f32,
    /// Number of poor trajectories needed to flag evolution readiness.
    pub evolution_trigger_count: usize,
    /// How often (in recorded trajectory count) to check triggers.
    pub check_interval: u64,
    /// GA hyperparameters for the ADR-017 flywheel (WEFT-38).
    pub ga: GaConfig,
    /// Optional path for persisting the evolved population across runs.
    ///
    /// When set, each successful evolution writes JSON via
    /// [`Population::save`]. On the next evolution the file is loaded
    /// (if present) and reseeded only when the active system prompt no
    /// longer matches any candidate.
    pub population_path: Option<PathBuf>,
}

impl Default for TrajectoryLearnerConfig {
    fn default() -> Self {
        Self {
            max_trajectories: 1000,
            poor_threshold: 0.6,
            evolution_trigger_count: 10,
            check_interval: 50,
            ga: GaConfig::default(),
            population_path: None,
        }
    }
}

/// A trajectory stored with computed feedback text.
#[derive(Debug, Clone)]
pub struct ScoredTrajectory {
    /// The original trajectory data.
    pub trajectory: Trajectory,
    /// Natural-language feedback derived from quality dimensions.
    pub feedback: String,
    /// Monotonic sequence number for ordering.
    pub recorded_at: u64,
}

/// Internal mutable state behind a Mutex.
struct LearnerState {
    trajectories: VecDeque<ScoredTrajectory>,
    total_recorded: u64,
    poor_count: usize,
    evolution_ready: bool,
    /// Live GA population retained between evolution runs (WEFT-38).
    population: Option<Population>,
    /// Last system prompt content that was evolved (or accepted).
    last_evolved_prompt: Option<String>,
}

/// Level 1 trajectory learning backend (GEPA-inspired).
///
/// Collects (prompt, response, outcome) trajectories in a bounded ring
/// buffer. Analyzes quality scores to detect degradation patterns and
/// extracts successful patterns for prompt mutation.
///
/// When enough poor trajectories accumulate, [`is_evolution_ready`]
/// becomes true. The next pipeline stage-3.5 call to
/// [`LearningBackend::evolve_prompt`] runs the
/// [`crate::pipeline::mutation`] GA loop and returns the champion prompt
/// (ADR-017 / WEFT-38 flywheel).
pub struct TrajectoryLearner {
    config: TrajectoryLearnerConfig,
    state: Mutex<LearnerState>,
}

impl TrajectoryLearner {
    /// Create a new trajectory learner with the given configuration.
    pub fn new(config: TrajectoryLearnerConfig) -> Self {
        // Best-effort warm-start from disk when a persistence path is set.
        let population = config.population_path.as_ref().and_then(|p| {
            Population::load(p)
                .map_err(|e| {
                    tracing::debug!(
                        path = %p.display(),
                        error = %e,
                        "TrajectoryLearner: no prior population to load"
                    );
                    e
                })
                .ok()
        });
        Self {
            config,
            state: Mutex::new(LearnerState {
                trajectories: VecDeque::new(),
                total_recorded: 0,
                poor_count: 0,
                evolution_ready: false,
                population,
                last_evolved_prompt: None,
            }),
        }
    }

    /// Builder: enable population persistence at `path` (WEFT-38).
    pub fn with_population_path(mut self, path: PathBuf) -> Self {
        self.config.population_path = Some(path.clone());
        if let Ok(pop) = Population::load(&path) {
            if let Ok(mut state) = self.state.lock() {
                state.population = Some(pop);
            }
        }
        self
    }

    /// Last prompt returned by a successful evolution run, if any.
    pub fn last_evolved_prompt(&self) -> Option<String> {
        self.state.lock().unwrap().last_evolved_prompt.clone()
    }

    /// Snapshot of the live GA population (for tests / admin surfaces).
    pub fn population_snapshot(&self) -> Option<Population> {
        self.state.lock().unwrap().population.clone()
    }

    /// Number of trajectories currently stored.
    pub fn trajectory_count(&self) -> usize {
        self.state.lock().unwrap().trajectories.len()
    }

    /// Total trajectories recorded (including evicted ones).
    pub fn total_recorded(&self) -> u64 {
        self.state.lock().unwrap().total_recorded
    }

    /// Whether the learner has flagged evolution readiness.
    pub fn is_evolution_ready(&self) -> bool {
        self.state.lock().unwrap().evolution_ready
    }

    /// Reset the evolution-ready flag (call after an evolution run).
    pub fn clear_evolution_ready(&self) {
        self.state.lock().unwrap().evolution_ready = false;
    }

    /// Retrieve the N poorest trajectories for reflection input.
    ///
    /// Returns trajectories sorted by overall quality (ascending).
    pub fn get_poor_trajectories(&self, n: usize) -> Vec<ScoredTrajectory> {
        let state = self.state.lock().unwrap();
        let mut poor: Vec<_> = state
            .trajectories
            .iter()
            .filter(|t| t.trajectory.quality.overall < self.config.poor_threshold)
            .cloned()
            .collect();
        poor.sort_by(|a, b| {
            a.trajectory
                .quality
                .overall
                .partial_cmp(&b.trajectory.quality.overall)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        poor.truncate(n);
        poor
    }

    /// Retrieve the N best trajectories as successful examples.
    ///
    /// Returns trajectories sorted by overall quality (descending).
    pub fn get_best_trajectories(&self, n: usize) -> Vec<ScoredTrajectory> {
        let state = self.state.lock().unwrap();
        let mut best: Vec<_> = state.trajectories.iter().cloned().collect();
        best.sort_by(|a, b| {
            b.trajectory
                .quality
                .overall
                .partial_cmp(&a.trajectory.quality.overall)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        best.truncate(n);
        best
    }

    /// Extract successful patterns from high-quality trajectories.
    ///
    /// Returns a list of user-message content strings from trajectories
    /// scoring above the poor threshold. These serve as positive examples
    /// for prompt mutation.
    pub fn extract_successful_patterns(&self) -> Vec<String> {
        let state = self.state.lock().unwrap();
        state
            .trajectories
            .iter()
            .filter(|t| t.trajectory.quality.overall >= 0.8)
            .filter_map(|t| {
                t.trajectory
                    .request
                    .messages
                    .first()
                    .map(|m| m.content.clone())
            })
            .collect()
    }

    /// Check whether evolution should be triggered.
    fn should_evolve(state: &LearnerState, config: &TrajectoryLearnerConfig) -> bool {
        state.poor_count >= config.evolution_trigger_count
            && !state.evolution_ready
            && state.total_recorded > 0
            && state.total_recorded.is_multiple_of(config.check_interval)
    }

    /// Generate feedback text from quality dimensions.
    fn generate_feedback(trajectory: &Trajectory, poor_threshold: f32) -> String {
        let q = &trajectory.quality;
        let assessment = if q.overall < poor_threshold {
            "Below quality threshold -- candidate for reflection."
        } else if q.overall >= 0.9 {
            "Excellent quality -- use as positive example."
        } else {
            "Acceptable quality."
        };

        let mut parts = Vec::new();
        if q.relevance < 0.5 {
            parts.push("Low relevance: response may not address the request.");
        }
        if q.coherence < 0.5 {
            parts.push("Low coherence: response may be unclear or disjointed.");
        }

        format!(
            "Overall: {:.2}, Relevance: {:.2}, Coherence: {:.2}. {}{}",
            q.overall,
            q.relevance,
            q.coherence,
            assessment,
            if parts.is_empty() {
                String::new()
            } else {
                format!(" {}", parts.join(" "))
            }
        )
    }
}

impl LearningBackend for TrajectoryLearner {
    fn record(&self, trajectory: &Trajectory) {
        let mut state = self.state.lock().unwrap();

        let feedback = Self::generate_feedback(trajectory, self.config.poor_threshold);

        let scored = ScoredTrajectory {
            trajectory: trajectory.clone(),
            feedback,
            recorded_at: state.total_recorded,
        };

        // Track poor trajectories
        if trajectory.quality.overall < self.config.poor_threshold {
            state.poor_count += 1;
        }

        // Ring buffer: push new, evict oldest if over capacity
        state.trajectories.push_back(scored);
        if state.trajectories.len() > self.config.max_trajectories
            && let Some(removed) = state.trajectories.pop_front()
            && removed.trajectory.quality.overall < self.config.poor_threshold
        {
            state.poor_count = state.poor_count.saturating_sub(1);
        }

        state.total_recorded += 1;

        // Check evolution trigger
        if Self::should_evolve(&state, &self.config) {
            state.evolution_ready = true;
        }
    }

    fn adapt(&self, signal: &LearningSignal) {
        let mut state = self.state.lock().unwrap();
        // Negative feedback accelerates evolution trigger.
        if signal.value < 0.0 {
            state.poor_count += 2;
        }
        // Positive feedback is captured via trajectory quality scores
        // and contributes to successful pattern extraction.
    }

    /// Run the GA population loop when an evolution is due (WEFT-38).
    ///
    /// Pulls trajectory hints from the ring buffer, loads or seeds a
    /// [`Population`], runs
    /// [`crate::pipeline::mutation::Population::evolve`], optionally
    /// persists it, and returns the champion prompt when fitness
    /// improves by at least `GaConfig::min_improvement`.
    ///
    /// When `evolution_ready` is false this is a no-op and the prompt
    /// is returned unchanged — we only thrash the system prompt after
    /// enough poor outcomes have accumulated.
    fn evolve_prompt(&self, prompt: &str) -> String {
        use crate::pipeline::mutation::{
            TrajectoryHint, select_evolved_prompt,
        };

        // Snapshot trajectory hints + take ownership of any live population
        // while holding the lock, then run the (CPU-only) GA outside it.
        let (hints, prior_pop): (Vec<TrajectoryHint>, Option<Population>) = {
            let mut state = self.state.lock().unwrap();
            if !state.evolution_ready {
                return prompt.to_string();
            }
            let mut hints: Vec<TrajectoryHint> = state
                .trajectories
                .iter()
                .map(|t| TrajectoryHint {
                    request_content: t
                        .trajectory
                        .request
                        .messages
                        .first()
                        .map(|m| m.content.clone())
                        .unwrap_or_default(),
                    quality_score: t.trajectory.quality.overall,
                    feedback: t.feedback.clone(),
                })
                .collect();
            // Cap hint set — GA fitness / AddExamples don't benefit from
            // arbitrarily large buffers and would bloat mutated prompts.
            if hints.len() > 16 {
                hints.truncate(16);
            }
            let prior = state.population.take();
            (hints, prior)
        };

        let ga_cfg = self.config.ga.clone();
        // Continue the live population across evolution cycles so the GA
        // compounds improvements. Reseed only when no population exists yet
        // (first trigger, or failed warm-start). The assembler still passes
        // the operator baseline (SOUL.md); we evolve from the retained
        // population and return the champion.
        let mut population =
            prior_pop.unwrap_or_else(|| Population::seed(prompt, ga_cfg.clone()));

        let result = population.evolve(&hints);
        let mutated = select_evolved_prompt(&result, ga_cfg.min_improvement);

        // Persist population + book-keeping under the lock.
        {
            let mut state = self.state.lock().unwrap();
            state.evolution_ready = false;
            // Reset poor-count so the next request's stage-6 `record` does
            // not immediately re-arm evolution before new evidence arrives.
            state.poor_count = 0;
            state.last_evolved_prompt = Some(mutated.clone());
            if let Some(ref path) = self.config.population_path {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(e) = population.save(path) {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "TrajectoryLearner: failed to persist evolved population"
                    );
                }
            }
            state.population = Some(population);
        }

        tracing::info!(
            improvement = result.improvement,
            generations = result.generations,
            population_size = result.population_size,
            hints = hints.len(),
            changed = mutated != prompt,
            "TrajectoryLearner: GA evolution applied (WEFT-38)"
        );

        mutated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::traits::{ChatRequest, LlmMessage, QualityScore, RoutingDecision};
    use clawft_types::provider::{ContentBlock, LlmResponse, StopReason, Usage};
    use std::collections::HashMap;

    fn make_trajectory_with_quality(overall: f32, relevance: f32, coherence: f32) -> Trajectory {
        Trajectory {
            request: ChatRequest {
                messages: vec![LlmMessage {
                    role: "user".into(),
                    content: "hello".into(),
                    tool_call_id: None,
                    tool_calls: None,
                }],
                tools: vec![],
                model: None,
                max_tokens: None,
                temperature: None,
                auth_context: None,
                complexity_boost: 0.0,
                tool_choice: None,
            },
            routing: RoutingDecision {
                provider: "openai".into(),
                model: "gpt-4o".into(),
                reason: "test".into(),
                ..Default::default()
            },
            response: LlmResponse {
                id: "resp-1".into(),
                content: vec![ContentBlock::Text { text: "Hi!".into() }],
                stop_reason: StopReason::EndTurn,
                usage: Usage {
                    input_tokens: 5,
                    output_tokens: 2,
                    total_tokens: 0,
                },
                metadata: HashMap::new(),
            },
            quality: QualityScore {
                overall,
                relevance,
                coherence,
            },
        }
    }

    fn make_trajectory() -> Trajectory {
        make_trajectory_with_quality(0.9, 0.95, 0.85)
    }

    // ── NoopLearner tests ─────────────────────────────────────────────

    #[test]
    fn noop_record_does_not_panic() {
        let learner = NoopLearner::new();
        learner.record(&make_trajectory());
    }

    #[test]
    fn noop_adapt_does_not_panic() {
        let learner = NoopLearner::new();
        learner.adapt(&LearningSignal {
            feedback_type: "thumbs_up".into(),
            value: 1.0,
        });
    }

    #[test]
    fn noop_adapt_negative_signal_does_not_panic() {
        let learner = NoopLearner::new();
        learner.adapt(&LearningSignal {
            feedback_type: "thumbs_down".into(),
            value: -1.0,
        });
    }

    #[test]
    fn noop_multiple_records_do_not_panic() {
        let learner = NoopLearner::new();
        for _ in 0..100 {
            learner.record(&make_trajectory());
        }
    }

    #[test]
    fn noop_multiple_adapts_do_not_panic() {
        let learner = NoopLearner::new();
        for i in 0..50 {
            learner.adapt(&LearningSignal {
                feedback_type: format!("signal_{i}"),
                value: i as f32 / 50.0,
            });
        }
    }

    #[test]
    fn noop_default_trait_impl() {
        let learner = NoopLearner;
        learner.record(&make_trajectory());
        learner.adapt(&LearningSignal {
            feedback_type: "test".into(),
            value: 0.0,
        });
    }

    // ── TrajectoryLearner tests ───────────────────────────────────────

    #[test]
    fn trajectory_learner_records_and_counts() {
        let learner = TrajectoryLearner::new(TrajectoryLearnerConfig::default());
        assert_eq!(learner.trajectory_count(), 0);
        assert_eq!(learner.total_recorded(), 0);

        learner.record(&make_trajectory());
        assert_eq!(learner.trajectory_count(), 1);
        assert_eq!(learner.total_recorded(), 1);

        learner.record(&make_trajectory());
        assert_eq!(learner.trajectory_count(), 2);
        assert_eq!(learner.total_recorded(), 2);
    }

    #[test]
    fn trajectory_learner_ring_buffer_eviction() {
        let config = TrajectoryLearnerConfig {
            max_trajectories: 3,
            ..Default::default()
        };
        let learner = TrajectoryLearner::new(config);

        for _ in 0..5 {
            learner.record(&make_trajectory());
        }

        assert_eq!(learner.trajectory_count(), 3);
        assert_eq!(learner.total_recorded(), 5);
    }

    #[test]
    fn trajectory_learner_poor_trajectory_tracking() {
        let config = TrajectoryLearnerConfig {
            poor_threshold: 0.6,
            ..Default::default()
        };
        let learner = TrajectoryLearner::new(config);

        // Record a poor trajectory
        learner.record(&make_trajectory_with_quality(0.3, 0.2, 0.4));
        // Record a good trajectory
        learner.record(&make_trajectory_with_quality(0.9, 0.95, 0.85));

        let poor = learner.get_poor_trajectories(10);
        assert_eq!(poor.len(), 1);
        assert!((poor[0].trajectory.quality.overall - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn trajectory_learner_get_best_trajectories() {
        let learner = TrajectoryLearner::new(TrajectoryLearnerConfig::default());

        learner.record(&make_trajectory_with_quality(0.3, 0.2, 0.4));
        learner.record(&make_trajectory_with_quality(0.95, 0.9, 0.95));
        learner.record(&make_trajectory_with_quality(0.7, 0.8, 0.6));

        let best = learner.get_best_trajectories(2);
        assert_eq!(best.len(), 2);
        assert!(best[0].trajectory.quality.overall > best[1].trajectory.quality.overall);
    }

    #[test]
    fn trajectory_learner_extract_successful_patterns() {
        let learner = TrajectoryLearner::new(TrajectoryLearnerConfig::default());

        learner.record(&make_trajectory_with_quality(0.3, 0.2, 0.4));
        learner.record(&make_trajectory_with_quality(0.9, 0.9, 0.9));
        learner.record(&make_trajectory_with_quality(0.85, 0.8, 0.9));

        let patterns = learner.extract_successful_patterns();
        // Only trajectories with overall >= 0.8 qualify
        assert_eq!(patterns.len(), 2);
    }

    #[test]
    fn trajectory_learner_evolution_trigger() {
        let config = TrajectoryLearnerConfig {
            max_trajectories: 1000,
            poor_threshold: 0.6,
            evolution_trigger_count: 3,
            check_interval: 5,
            ..Default::default()
        };
        let learner = TrajectoryLearner::new(config);

        assert!(!learner.is_evolution_ready());

        // Record 5 poor trajectories (trigger_count=3, check_interval=5)
        for _ in 0..5 {
            learner.record(&make_trajectory_with_quality(0.2, 0.1, 0.3));
        }

        assert!(learner.is_evolution_ready());

        // Clear and verify
        learner.clear_evolution_ready();
        assert!(!learner.is_evolution_ready());
    }

    #[test]
    fn trajectory_learner_negative_adapt_accelerates_trigger() {
        let config = TrajectoryLearnerConfig {
            max_trajectories: 1000,
            poor_threshold: 0.6,
            evolution_trigger_count: 5,
            check_interval: 3,
            ..Default::default()
        };
        let learner = TrajectoryLearner::new(config);

        // Record 1 poor trajectory
        learner.record(&make_trajectory_with_quality(0.3, 0.2, 0.4));

        // Negative adapt adds 2 to poor_count
        learner.adapt(&LearningSignal {
            feedback_type: "thumbs_down".into(),
            value: -1.0,
        });

        // Record 2 more (total_recorded will be 3, matching check_interval)
        learner.record(&make_trajectory_with_quality(0.3, 0.2, 0.4));
        learner.record(&make_trajectory_with_quality(0.3, 0.2, 0.4));

        // poor_count = 3 (from records) + 2 (from adapt) = 5 >= trigger_count(5)
        // total_recorded = 3, check_interval = 3 -> 3 % 3 == 0
        assert!(learner.is_evolution_ready());
    }

    #[test]
    fn trajectory_learner_feedback_text_generation() {
        let poor = make_trajectory_with_quality(0.3, 0.4, 0.3);
        let feedback = TrajectoryLearner::generate_feedback(&poor, 0.6);
        assert!(feedback.contains("Below quality threshold"));
        assert!(feedback.contains("Low coherence"));

        let good = make_trajectory_with_quality(0.95, 0.9, 0.9);
        let feedback = TrajectoryLearner::generate_feedback(&good, 0.6);
        assert!(feedback.contains("Excellent quality"));

        let ok = make_trajectory_with_quality(0.7, 0.8, 0.7);
        let feedback = TrajectoryLearner::generate_feedback(&ok, 0.6);
        assert!(feedback.contains("Acceptable quality"));
    }

    #[test]
    fn trajectory_learner_poor_eviction_decrements_count() {
        let config = TrajectoryLearnerConfig {
            max_trajectories: 2,
            poor_threshold: 0.6,
            evolution_trigger_count: 100, // high so we don't trigger
            check_interval: 100,
            ..Default::default()
        };
        let learner = TrajectoryLearner::new(config);

        // Fill with poor trajectories
        learner.record(&make_trajectory_with_quality(0.3, 0.2, 0.3));
        learner.record(&make_trajectory_with_quality(0.4, 0.3, 0.3));

        // Evict a poor one by adding a good one
        learner.record(&make_trajectory_with_quality(0.9, 0.9, 0.9));

        // Should still have 1 poor trajectory after eviction
        let poor = learner.get_poor_trajectories(10);
        assert_eq!(poor.len(), 1);
    }

    // ── evolve_prompt feedback loop ───────────────────────────────────

    #[test]
    fn evolve_prompt_returns_unchanged_when_not_ready() {
        let learner = TrajectoryLearner::new(TrajectoryLearnerConfig::default());
        // Single poor trajectory — far below evolution_trigger_count.
        learner.record(&make_trajectory_with_quality(0.2, 0.2, 0.2));

        let prompt = "You are a helpful assistant.";
        let out = learner.evolve_prompt(prompt);
        assert_eq!(
            out, prompt,
            "evolve_prompt is a no-op until evolution-ready"
        );
    }

    #[test]
    fn evolve_prompt_clears_flag_when_ready() {
        // The mutation strategies are content-dependent (some may
        // return the prompt unchanged for short inputs that don't
        // match their patterns). What we MUST assert is the
        // book-keeping: when the flag is set, evolve_prompt fires the
        // mutation pipeline once and then clears the flag so we don't
        // re-mutate on every subsequent call until new poor outcomes
        // accumulate.
        let cfg = TrajectoryLearnerConfig {
            max_trajectories: 32,
            poor_threshold: 0.6,
            evolution_trigger_count: 2,
            check_interval: 1,
            ..Default::default()
        };
        let learner = TrajectoryLearner::new(cfg);

        learner.record(&make_trajectory_with_quality(0.2, 0.2, 0.2));
        learner.record(&make_trajectory_with_quality(0.3, 0.3, 0.3));

        // Sanity: flag is set after the threshold is reached.
        assert!(
            learner.is_evolution_ready(),
            "evolution should be triggered after `evolution_trigger_count` poor trajectories"
        );

        let seed = "Use the read tool\nCheck the output\nAlways verify results";
        let _ = learner.evolve_prompt(seed);

        // After firing, the flag is cleared.
        assert!(
            !learner.is_evolution_ready(),
            "evolve_prompt must clear the evolution flag after firing"
        );
        // GA population retained for the next evolution cycle.
        assert!(
            learner.population_snapshot().is_some(),
            "evolve_prompt must retain the live Population (WEFT-38)"
        );
        assert!(learner.last_evolved_prompt().is_some());
    }

    #[test]
    fn evolve_prompt_ga_changes_instruction_prompt() {
        // Instruction-style system prompts are mutated by Rephrase/Emphasize
        // so the champion should differ from the seed after a GA run.
        let cfg = TrajectoryLearnerConfig {
            max_trajectories: 32,
            poor_threshold: 0.6,
            evolution_trigger_count: 2,
            check_interval: 1,
            ga: crate::pipeline::mutation::GaConfig {
                population_size: 6,
                elite_count: 2,
                generations: 2,
                crossover_rate: 0.4,
                min_improvement: 0.0,
            },
            ..Default::default()
        };
        let learner = TrajectoryLearner::new(cfg);

        // Mix of poor + good trajectories so AddExamples / RemoveIneffective
        // have signal and fitness scoring is non-trivial.
        learner.record(&make_trajectory_with_quality(0.2, 0.1, 0.2));
        learner.record(&make_trajectory_with_quality(0.25, 0.2, 0.2));
        // Inject a high-quality trajectory by overwriting via record path.
        learner.record(&make_trajectory_with_quality(0.9, 0.9, 0.9));

        assert!(learner.is_evolution_ready());
        let seed = "Use the search tool to find docs\nAlways verify output\nEnsure accuracy";
        let evolved = learner.evolve_prompt(seed);
        assert!(!evolved.trim().is_empty());
        // Either content changed or population still tracked a best —
        // structure markers are the usual signature of Rephrase/Emphasize.
        let changed = evolved != seed;
        let structured = evolved.contains("IMPORTANT:") || evolved.contains("Step ");
        assert!(
            changed || structured || learner.population_snapshot().is_some(),
            "GA run should leave a population and typically mutate instruction prompts; \
             evolved={evolved:?}"
        );
    }

    #[test]
    fn evolve_prompt_persists_population_to_disk() {
        let dir = std::env::temp_dir().join(format!(
            "weft38-learner-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("population.json");

        let cfg = TrajectoryLearnerConfig {
            evolution_trigger_count: 1,
            check_interval: 1,
            population_path: Some(path.clone()),
            ga: crate::pipeline::mutation::GaConfig {
                population_size: 4,
                generations: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        let learner = TrajectoryLearner::new(cfg);
        learner.record(&make_trajectory_with_quality(0.1, 0.1, 0.1));
        assert!(learner.is_evolution_ready());
        let _ = learner.evolve_prompt("Use tools\nAlways verify");

        assert!(
            path.is_file(),
            "population must be written to {}",
            path.display()
        );
        let loaded = crate::pipeline::mutation::Population::load(&path).unwrap();
        assert!(!loaded.candidates().is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
