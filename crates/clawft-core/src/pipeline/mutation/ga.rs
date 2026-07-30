//! Genetic-algorithm population loop for GEPA prompt evolution (WEFT-38).
//!
//! Provides a deterministic, LLM-free generation cycle:
//! selection (tournament) → mutation / crossover → fitness → elitism.
//!
//! Triggered from [`crate::pipeline::learner::TrajectoryLearner::evolve_prompt`]
//! when `evolution_ready` is set (ADR-017 flywheel / production stage 3.5).

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    MutationStrategy, TrajectoryHint, auto_select_strategy, mutate_prompt,
};

/// Configuration for one GA evolution run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaConfig {
    /// Active population size (elites + offspring). Default 8.
    pub population_size: usize,
    /// Number of top candidates retained each generation. Default 2.
    pub elite_count: usize,
    /// Generations to run per `evolve` call. Default 1.
    pub generations: u32,
    /// Probability (0.0–1.0) of applying crossover instead of pure mutation.
    /// Implemented as a deterministic threshold on candidate id. Default 0.3.
    pub crossover_rate: f32,
    /// Minimum fitness delta vs seed required to accept an evolved prompt.
    /// Default 0.0 (any non-worse best is accepted; caller may still prefer
    /// the original when improvement is zero).
    pub min_improvement: f32,
}

impl Default for GaConfig {
    fn default() -> Self {
        Self {
            population_size: 8,
            elite_count: 2,
            generations: 1,
            crossover_rate: 0.3,
            min_improvement: 0.0,
        }
    }
}

/// A prompt candidate in the evolution population.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptCandidate {
    /// Unique identifier within a population (stable across generations).
    pub id: u64,
    /// Prompt text being evolved.
    pub content: String,
    /// Parent candidate id (`None` for the seed).
    pub parent_id: Option<u64>,
    /// Generation number (0 = seed).
    pub generation: u32,
    /// Aggregate fitness score in `[0.0, 1.0]`.
    pub fitness: f32,
    /// Strategy label that produced this candidate (`None` for seed).
    pub strategy: Option<String>,
}

/// Result of a GA evolution run.
#[derive(Debug, Clone)]
pub struct EvolutionResult {
    /// Seed / original candidate that opened the run.
    pub original: PromptCandidate,
    /// Best candidate after evolution (may equal original if no improvement).
    pub best: PromptCandidate,
    /// `best.fitness - original.fitness`.
    pub improvement: f32,
    /// Generations actually executed.
    pub generations: u32,
    /// Population size after the last generation.
    pub population_size: usize,
}

/// Active set of prompt candidates under evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Population {
    candidates: Vec<PromptCandidate>,
    next_id: u64,
    generation: u32,
    config: GaConfig,
}

impl Population {
    /// Seed a population from a base system prompt.
    ///
    /// Fills the population with the seed plus deterministic mutants of it
    /// (one per [`MutationStrategy`], then cycling) so the first generation
    /// has diversity without requiring prior trajectory data.
    pub fn seed(base_prompt: &str, config: GaConfig) -> Self {
        let size = config.population_size.max(1);
        let mut next_id = 0u64;
        let seed = PromptCandidate {
            id: next_id,
            content: base_prompt.to_string(),
            parent_id: None,
            generation: 0,
            fitness: 0.5, // neutral until scored
            strategy: None,
        };
        next_id += 1;

        let strategies = [
            MutationStrategy::Rephrase,
            MutationStrategy::AddExamples,
            MutationStrategy::RemoveIneffective,
            MutationStrategy::Emphasize,
        ];
        let mut candidates = Vec::with_capacity(size);
        candidates.push(seed);

        let mut i = 0usize;
        while candidates.len() < size {
            let strategy = strategies[i % strategies.len()];
            let parent = &candidates[0];
            let content = mutate_prompt(&parent.content, &[], strategy);
            candidates.push(PromptCandidate {
                id: next_id,
                content,
                parent_id: Some(parent.id),
                generation: 0,
                fitness: 0.5,
                strategy: Some(format!("{strategy:?}")),
            });
            next_id += 1;
            i += 1;
        }

        Self {
            candidates,
            next_id,
            generation: 0,
            config,
        }
    }

    /// Current generation index.
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// Snapshot of all candidates (sorted by fitness descending).
    pub fn candidates(&self) -> Vec<PromptCandidate> {
        let mut c = self.candidates.clone();
        c.sort_by(|a, b| {
            b.fitness
                .partial_cmp(&a.fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        c
    }

    /// Best candidate by fitness (stable: first on ties = seed if equal).
    pub fn best(&self) -> &PromptCandidate {
        self.candidates
            .iter()
            .max_by(|a, b| {
                a.fitness
                    .partial_cmp(&b.fitness)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| b.id.cmp(&a.id)) // prefer lower id (seed) on ties
            })
            .expect("population always has ≥1 candidate")
    }

    /// Re-score every candidate against the current trajectory hints.
    pub fn rescore(&mut self, trajectories: &[TrajectoryHint]) {
        for c in &mut self.candidates {
            c.fitness = score_candidate(&c.content, trajectories);
        }
    }

    /// Run `config.generations` of selection → mutate/crossover → elitism.
    ///
    /// Returns the evolution result relative to the best candidate **before**
    /// this call (the “original” for the run — usually the prior champion or
    /// the seed, not necessarily the raw assembler baseline). When
    /// improvement is below `min_improvement`, `best` is still the post-run
    /// champion (caller may reject it).
    pub fn evolve(&mut self, trajectories: &[TrajectoryHint]) -> EvolutionResult {
        self.rescore(trajectories);
        let original = self.best().clone();
        let gens = self.config.generations.max(1);

        for _ in 0..gens {
            self.evolve_one_generation(trajectories);
        }

        let best = self.best().clone();
        let improvement = best.fitness - original.fitness;
        EvolutionResult {
            original,
            best,
            improvement,
            generations: gens,
            population_size: self.candidates.len(),
        }
    }

    /// One-shot helper: seed from `prompt`, then [`Self::evolve`].
    ///
    /// `EvolutionResult::original` is the fitness-best member of the
    /// **seeded** population (may already include structure mutations from
    /// seed diversification), not always the raw `prompt` string.
    pub fn seed_and_evolve(
        prompt: &str,
        trajectories: &[TrajectoryHint],
        config: GaConfig,
    ) -> (Self, EvolutionResult) {
        let mut pop = Self::seed(prompt, config);
        let result = pop.evolve(trajectories);
        (pop, result)
    }

    fn evolve_one_generation(&mut self, trajectories: &[TrajectoryHint]) {
        self.rescore(trajectories);
        self.generation = self.generation.saturating_add(1);
        let elite_n = self.config.elite_count.min(self.candidates.len()).max(1);
        let size = self.config.population_size.max(elite_n);

        // Elitism: keep top elite_n
        let mut ranked = self.candidates.clone();
        ranked.sort_by(|a, b| {
            b.fitness
                .partial_cmp(&a.fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut next: Vec<PromptCandidate> = ranked.into_iter().take(elite_n).collect();

        // Fill with offspring from tournament selection
        let strategy_cycle = [
            MutationStrategy::Rephrase,
            MutationStrategy::AddExamples,
            MutationStrategy::RemoveIneffective,
            MutationStrategy::Emphasize,
        ];
        let mut offspring_i = 0usize;
        while next.len() < size {
            let parent_a = tournament_select(&self.candidates, offspring_i);
            let use_crossover = ((parent_a.id as f32 * 0.6180339887) % 1.0)
                < self.config.crossover_rate
                && self.candidates.len() >= 2;

            let (content, strategy_label, parent_id) = if use_crossover {
                let parent_b = tournament_select(&self.candidates, offspring_i + 1);
                let merged = crossover(&parent_a.content, &parent_b.content);
                (
                    merged,
                    "Crossover".to_string(),
                    Some(parent_a.id),
                )
            } else {
                // Prefer trajectory-informed strategy, then cycle for diversity
                let preferred = auto_select_strategy(trajectories);
                let strategy = if offspring_i % 2 == 0 {
                    preferred
                } else {
                    strategy_cycle[offspring_i % strategy_cycle.len()]
                };
                let mutated = mutate_prompt(&parent_a.content, trajectories, strategy);
                (mutated, format!("{strategy:?}"), Some(parent_a.id))
            };

            let fitness = score_candidate(&content, trajectories);
            next.push(PromptCandidate {
                id: self.next_id,
                content,
                parent_id,
                generation: self.generation,
                fitness,
                strategy: Some(strategy_label),
            });
            self.next_id = self.next_id.saturating_add(1);
            offspring_i += 1;
        }

        self.candidates = next;
    }

    /// Persist population as JSON to `path` (parent dirs must exist).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;
        std::fs::write(path, json)
    }

    /// Load a previously saved population.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let bytes = std::fs::read(path)?;
        serde_json::from_slice(&bytes).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })
    }
}

/// Run a one-shot GA cycle over a fresh population seeded from `prompt`.
///
/// Production entry point for the ADR-017 flywheel: call when the learner
/// has set `evolution_ready`. Returns the evolved best prompt content when
/// improvement ≥ `config.min_improvement`; otherwise returns the original
/// prompt text.
pub fn evolve(
    prompt: &str,
    trajectories: &[TrajectoryHint],
    config: &GaConfig,
) -> EvolutionResult {
    let mut pop = Population::seed(prompt, config.clone());
    pop.evolve(trajectories)
}

/// Accept the evolved prompt only when fitness improves enough.
pub fn select_evolved_prompt(result: &EvolutionResult, min_improvement: f32) -> String {
    if result.improvement >= min_improvement && result.best.content != result.original.content {
        result.best.content.clone()
    } else if result.improvement >= min_improvement {
        // Same text but scored higher — still return best content
        result.best.content.clone()
    } else {
        result.original.content.clone()
    }
}

// ─── Internals ────────────────────────────────────────────────────────

/// Deterministic tournament of size 2: pick two candidates by rotating
/// index and keep the fitter one.
fn tournament_select(candidates: &[PromptCandidate], salt: usize) -> PromptCandidate {
    let n = candidates.len();
    debug_assert!(n > 0);
    let a = &candidates[salt % n];
    let b = &candidates[(salt.wrapping_mul(3).wrapping_add(1)) % n];
    if b.fitness > a.fitness {
        b.clone()
    } else {
        a.clone()
    }
}

/// Simple line-level crossover: first half of lines from `a`, second half
/// from `b`. Falls back to `a` when either side is empty.
fn crossover(a: &str, b: &str) -> String {
    let a_lines: Vec<&str> = a.lines().collect();
    let b_lines: Vec<&str> = b.lines().collect();
    if a_lines.is_empty() {
        return b.to_string();
    }
    if b_lines.is_empty() {
        return a.to_string();
    }
    let mid_a = (a_lines.len() + 1) / 2;
    let mid_b = b_lines.len() / 2;
    let mut out = Vec::with_capacity(mid_a + (b_lines.len() - mid_b));
    out.extend_from_slice(&a_lines[..mid_a]);
    out.extend_from_slice(&b_lines[mid_b..]);
    // Deduplicate consecutive identical lines after the join.
    let mut deduped = Vec::new();
    for line in out {
        if deduped.last().copied() != Some(line) {
            deduped.push(line);
        }
    }
    let joined = deduped.join("\n");
    if joined.trim().is_empty() {
        a.to_string()
    } else {
        joined
    }
}

/// Heuristic fitness of a prompt given trajectory history.
///
/// Higher is better. Components (clamped to `[0, 1]`):
/// - Base 0.5
/// - Bonus for structure markers (`IMPORTANT:`, `Step N:`, Successful Examples)
/// - Bonus for embedding high-quality request snippets
/// - Penalty for embedding low-quality feedback phrases
fn score_candidate(content: &str, trajectories: &[TrajectoryHint]) -> f32 {
    let mut score: f32 = 0.5;
    let lower = content.to_lowercase();

    if lower.contains("important:") {
        score += 0.08;
    }
    if lower.contains("step ") {
        score += 0.06;
    }
    if lower.contains("successful examples") {
        score += 0.12;
    }

    for t in trajectories {
        if t.quality_score >= 0.8 {
            let snippet = t.request_content.chars().take(40).collect::<String>();
            if !snippet.is_empty() && content.contains(&snippet) {
                score += 0.05 * t.quality_score;
            }
        }
        if t.quality_score < 0.5 {
            for phrase in t
                .feedback
                .split(['.', ',', ';'])
                .map(str::trim)
                .filter(|s| s.len() > 5 && s.len() < 60)
            {
                if lower.contains(&phrase.to_lowercase()) {
                    score -= 0.08;
                }
            }
        }
    }

    // Mild preference for non-empty, non-huge prompts
    let len = content.len();
    if len == 0 {
        score = 0.0;
    } else if len > 8_000 {
        score -= 0.1;
    }

    score.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_hints() -> Vec<TrajectoryHint> {
        vec![
            TrajectoryHint {
                request_content: "Write a function to sort a list".into(),
                quality_score: 0.9,
                feedback: "Excellent quality.".into(),
            },
            TrajectoryHint {
                request_content: "Explain recursion with examples".into(),
                quality_score: 0.85,
                feedback: "Good explanation.".into(),
            },
            TrajectoryHint {
                request_content: "Fix the bug".into(),
                quality_score: 0.2,
                feedback: "Low relevance: response may not address the request.".into(),
            },
        ]
    }

    #[test]
    fn seed_fills_population() {
        let pop = Population::seed("Use the tools carefully\nAlways verify output", GaConfig {
            population_size: 6,
            ..Default::default()
        });
        assert_eq!(pop.candidates().len(), 6);
        assert_eq!(pop.best().generation, 0);
        assert!(pop.candidates().iter().any(|c| c.parent_id.is_none()));
    }

    #[test]
    fn evolve_runs_and_returns_result() {
        let prompt = "Use the read tool\nCheck results\nWrite the answer";
        let hints = sample_hints();
        let result = evolve(
            prompt,
            &hints,
            &GaConfig {
                population_size: 6,
                elite_count: 2,
                generations: 2,
                crossover_rate: 0.5,
                min_improvement: 0.0,
            },
        );
        assert_eq!(result.generations, 2);
        assert_eq!(result.population_size, 6);
        assert!(!result.best.content.trim().is_empty());
        // Original is the pre-generation champion (may already be a seed
        // mutant with higher heuristic fitness than the raw prompt).
        assert!(!result.original.content.trim().is_empty());
        assert!(result.best.fitness >= result.original.fitness - f32::EPSILON);
    }

    #[test]
    fn select_evolved_prompt_respects_min_improvement() {
        let original = PromptCandidate {
            id: 0,
            content: "orig".into(),
            parent_id: None,
            generation: 0,
            fitness: 0.5,
            strategy: None,
        };
        let best = PromptCandidate {
            id: 1,
            content: "better".into(),
            parent_id: Some(0),
            generation: 1,
            fitness: 0.52,
            strategy: Some("Rephrase".into()),
        };
        let result = EvolutionResult {
            original: original.clone(),
            best: best.clone(),
            improvement: 0.02,
            generations: 1,
            population_size: 4,
        };
        assert_eq!(select_evolved_prompt(&result, 0.05), "orig");
        assert_eq!(select_evolved_prompt(&result, 0.01), "better");
    }

    #[test]
    fn population_persist_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "weft38-pop-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("population.json");

        let mut pop = Population::seed(
            "Use tools\nAlways verify",
            GaConfig {
                population_size: 4,
                ..Default::default()
            },
        );
        let _ = pop.evolve(&sample_hints());
        pop.save(&path).unwrap();

        let loaded = Population::load(&path).unwrap();
        assert_eq!(loaded.candidates().len(), pop.candidates().len());
        assert_eq!(loaded.generation(), pop.generation());
        assert_eq!(loaded.best().content, pop.best().content);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn crossover_merges_lines() {
        let a = "line a1\nline a2\nline a3";
        let b = "line b1\nline b2\nline b3";
        let m = crossover(a, b);
        assert!(m.contains("line a1"));
        assert!(m.contains("line b"));
        assert!(!m.trim().is_empty());
    }

    #[test]
    fn score_prefers_structured_prompts() {
        let hints = sample_hints();
        let plain = score_candidate("You are helpful.", &hints);
        let structured = score_candidate(
            "IMPORTANT: Use tools\nStep 1: Check output\n## Successful Examples\nExample 1: Write a function to sort a list",
            &hints,
        );
        assert!(structured > plain);
    }
}
