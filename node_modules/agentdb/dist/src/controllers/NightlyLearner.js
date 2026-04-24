/**
 * Nightly Learner - Automated Causal Discovery and Consolidation
 *
 * Runs as a background job to:
 * 1. Discover new causal edges from episode patterns
 * 2. Run A/B experiments on promising hypotheses
 * 3. Calculate uplift for completed experiments
 * 4. Prune low-confidence edges
 * 5. Update rerank weights based on performance
 *
 * Based on doubly robust learner:
 * τ̂(x) = μ1(x) − μ0(x) + [a*(y−μ1(x)) / e(x)] − [(1−a)*(y−μ0(x)) / (1−e(x))]
 *
 * v2.0.0-alpha.3 Features:
 * - FlashAttention for memory-efficient episodic consolidation
 * - Block-wise computation for large episode buffers
 * - Feature flag: ENABLE_FLASH_CONSOLIDATION (default: false)
 * - 100% backward compatible with fallback to standard consolidation
 */
import { CausalMemoryGraph } from './CausalMemoryGraph.js';
import { ReflexionMemory } from './ReflexionMemory.js';
import { SkillLibrary } from './SkillLibrary.js';
import { AttentionService } from '../services/AttentionService.js';
export class NightlyLearner {
    config;
    db;
    causalGraph;
    reflexion;
    skillLibrary;
    embedder;
    attentionService;
    constructor(db, embedder, config = {
        minSimilarity: 0.7,
        minSampleSize: 30,
        confidenceThreshold: 0.6,
        upliftThreshold: 0.05,
        pruneOldEdges: true,
        edgeMaxAgeDays: 90,
        autoExperiments: true,
        experimentBudget: 10,
        ENABLE_FLASH_CONSOLIDATION: false,
    }) {
        this.config = config;
        this.db = db;
        this.embedder = embedder;
        this.causalGraph = new CausalMemoryGraph(db);
        this.reflexion = new ReflexionMemory(db, embedder);
        this.skillLibrary = new SkillLibrary(db, embedder);
        // Initialize AttentionService if FlashAttention enabled
        if (this.config.ENABLE_FLASH_CONSOLIDATION) {
            this.attentionService = new AttentionService(db, {
                flash: {
                    enabled: true,
                    ...this.config.flashConfig,
                },
            });
        }
    }
    /**
     * Main learning job - runs all discovery and consolidation tasks
     */
    async run() {
        console.log('\n🌙 Nightly Learner Starting...\n');
        const startTime = Date.now();
        const report = {
            timestamp: startTime,
            executionTimeMs: 0,
            edgesDiscovered: 0,
            edgesPruned: 0,
            experimentsCompleted: 0,
            experimentsCreated: 0,
            avgUplift: 0,
            avgConfidence: 0,
            recommendations: []
        };
        try {
            // Step 1: Discover new causal edges
            console.log('📊 Discovering causal edges from episode patterns...');
            report.edgesDiscovered = await this.discoverCausalEdges();
            console.log(`   ✓ Discovered ${report.edgesDiscovered} new edges\n`);
            // Step 2: Complete running experiments
            console.log('🧪 Completing A/B experiments...');
            report.experimentsCompleted = await this.completeExperiments();
            console.log(`   ✓ Completed ${report.experimentsCompleted} experiments\n`);
            // Step 3: Create new experiments (if enabled)
            if (this.config.autoExperiments) {
                console.log('🔬 Creating new A/B experiments...');
                report.experimentsCreated = await this.createExperiments();
                console.log(`   ✓ Created ${report.experimentsCreated} new experiments\n`);
            }
            // Step 4: Prune low-confidence edges
            if (this.config.pruneOldEdges) {
                console.log('🧹 Pruning low-confidence edges...');
                report.edgesPruned = await this.pruneEdges();
                console.log(`   ✓ Pruned ${report.edgesPruned} edges\n`);
            }
            // Step 5: Calculate statistics
            const stats = this.calculateStats();
            report.avgUplift = stats.avgUplift;
            report.avgConfidence = stats.avgConfidence;
            // Step 6: Generate recommendations
            report.recommendations = this.generateRecommendations(report);
            report.executionTimeMs = Date.now() - startTime;
            console.log('✅ Nightly Learner Completed\n');
            this.printReport(report);
            return report;
        }
        catch (error) {
            console.error('❌ Nightly Learner Failed:', error);
            throw error;
        }
    }
    /**
     * Discover causal edges using doubly robust learner
     *
     * τ̂(x) = μ1(x) − μ0(x) + [a*(y−μ1(x)) / e(x)] − [(1−a)*(y−μ0(x)) / (1−e(x))]
     *
     * Where:
     * - μ1(x) = outcome model for treatment
     * - μ0(x) = outcome model for control
     * - e(x) = propensity score (probability of treatment)
     * - a = treatment indicator
     * - y = observed outcome
     *
     * v2: Uses FlashAttention for memory-efficient consolidation if enabled
     */
    async discover(config) {
        const edges = [];
        const count = await this.discoverCausalEdges();
        // If dryRun, return empty array since we didn't actually create edges
        if (config.dryRun) {
            return edges;
        }
        // Return discovered edges (for non-dry runs)
        // Note: In a real implementation, we'd track and return the actual edges created
        return edges;
    }
    /**
     * Consolidate episodic memories using FlashAttention (v2 feature)
     *
     * Processes large episode buffers efficiently using block-wise computation.
     * Identifies patterns and relationships across episodes for causal edge discovery.
     *
     * @param sessionId - Session to consolidate (optional, processes all if not provided)
     * @returns Number of edges discovered through consolidation
     */
    async consolidateEpisodes(sessionId) {
        if (!this.attentionService) {
            // Fallback: Use standard discovery without attention
            const edgesDiscovered = await this.discoverCausalEdges();
            return {
                edgesDiscovered,
                episodesProcessed: 0,
            };
        }
        // Get episodes to consolidate
        const episodes = sessionId
            ? this.db.prepare(`
          SELECT id, task, output, reward FROM episodes
          WHERE session_id = ?
          ORDER BY ts ASC
        `).all(sessionId)
            : this.db.prepare(`
          SELECT id, task, output, reward FROM episodes
          ORDER BY ts ASC
          LIMIT 1000
        `).all();
        if (episodes.length === 0) {
            return { edgesDiscovered: 0, episodesProcessed: 0 };
        }
        // Generate embeddings for all episodes
        const episodeEmbeddings = [];
        for (const episode of episodes) {
            const text = `${episode.task}: ${episode.output}`;
            const embedding = await this.embedder.embed(text);
            episodeEmbeddings.push(embedding);
        }
        // Prepare queries (each episode is a query)
        const dim = 384;
        const queries = new Float32Array(episodes.length * dim);
        const keys = new Float32Array(episodes.length * dim);
        const values = new Float32Array(episodes.length * dim);
        episodeEmbeddings.forEach((embedding, idx) => {
            queries.set(embedding, idx * dim);
            keys.set(embedding, idx * dim);
            values.set(embedding, idx * dim);
        });
        // Apply FlashAttention for memory-efficient consolidation
        const attentionResult = await this.attentionService.flashAttention(queries, keys, values);
        // Analyze attention output to discover causal relationships
        let edgesDiscovered = 0;
        const consolidatedEmbeddings = attentionResult.output;
        // For each episode, find similar episodes in consolidated space
        for (let i = 0; i < episodes.length; i++) {
            const queryEmb = consolidatedEmbeddings.slice(i * dim, (i + 1) * dim);
            // Find top-k similar episodes
            const similarities = [];
            for (let j = 0; j < episodes.length; j++) {
                if (i === j)
                    continue;
                const keyEmb = consolidatedEmbeddings.slice(j * dim, (j + 1) * dim);
                const score = this.cosineSimilarity(queryEmb, keyEmb);
                if (score >= this.config.minSimilarity) {
                    similarities.push({ idx: j, score });
                }
            }
            // Sort by similarity
            similarities.sort((a, b) => b.score - a.score);
            // Create causal edges for top matches
            for (const { idx, score } of similarities.slice(0, 5)) {
                // Only create edge if temporal sequence is correct
                if (idx > i) {
                    const uplift = episodes[idx].reward - episodes[i].reward;
                    if (Math.abs(uplift) >= this.config.upliftThreshold) {
                        await this.causalGraph.addCausalEdge({
                            fromMemoryId: episodes[i].id,
                            fromMemoryType: 'episode',
                            toMemoryId: episodes[idx].id,
                            toMemoryType: 'episode',
                            similarity: score,
                            uplift,
                            confidence: score,
                            sampleSize: 1,
                            mechanism: 'flash_attention_consolidation',
                            metadata: {
                                consolidationMethod: 'flash_attention',
                                blockSize: this.config.flashConfig?.blockSize || 256,
                            },
                        });
                        edgesDiscovered++;
                    }
                }
            }
        }
        return {
            edgesDiscovered,
            episodesProcessed: episodes.length,
            metrics: attentionResult.metrics,
        };
    }
    /**
     * Helper: Cosine similarity between two vectors
     */
    cosineSimilarity(a, b) {
        let dotProduct = 0;
        let normA = 0;
        let normB = 0;
        for (let i = 0; i < a.length; i++) {
            dotProduct += a[i] * b[i];
            normA += a[i] * a[i];
            normB += b[i] * b[i];
        }
        const denom = Math.sqrt(normA) * Math.sqrt(normB);
        return denom === 0 ? 0 : dotProduct / denom;
    }
    async discoverCausalEdges() {
        let discovered = 0;
        // Find episode pairs with high similarity and temporal sequence
        const candidatePairs = this.db.prepare(`
      SELECT
        e1.id as from_id,
        e1.task as from_task,
        e1.reward as from_reward,
        e2.id as to_id,
        e2.task as to_task,
        e2.reward as to_reward,
        e2.ts - e1.ts as time_diff
      FROM episodes e1
      JOIN episodes e2 ON e1.session_id = e2.session_id
      WHERE e1.id != e2.id
        AND e2.ts > e1.ts
        AND e2.ts - e1.ts < 3600 -- Within 1 hour
      ORDER BY e1.id, e2.ts
      LIMIT 1000
    `).all();
        // Better-sqlite3 best practice: Prepare statements OUTSIDE loops for better performance
        const checkExistingStmt = this.db.prepare(`
      SELECT id FROM causal_edges
      WHERE from_memory_id = ? AND to_memory_id = ?
    `);
        for (const pair of candidatePairs) {
            // Check if edge already exists
            const existing = checkExistingStmt.get(pair.from_id, pair.to_id);
            if (existing)
                continue;
            // Calculate propensity score e(x) - probability of treatment
            // Simplified: use frequency of from_task in session
            const propensity = this.calculatePropensity(pair.from_id);
            // Calculate outcome models μ1(x) and μ0(x)
            const mu1 = this.calculateOutcomeModel(pair.from_task, true); // With treatment
            const mu0 = this.calculateOutcomeModel(pair.from_task, false); // Without treatment
            // Calculate doubly robust estimator
            const a = 1; // This is a treated observation
            const y = pair.to_reward;
            const doublyRobustEstimate = (mu1 - mu0) + (a * (y - mu1) / propensity);
            // Calculate confidence based on sample size and variance
            const sampleSize = this.getSampleSize(pair.from_task);
            const confidence = this.calculateConfidence(sampleSize, doublyRobustEstimate);
            // Only add if meets thresholds
            if (Math.abs(doublyRobustEstimate) >= this.config.upliftThreshold && confidence >= this.config.confidenceThreshold) {
                const edge = {
                    fromMemoryId: pair.from_id,
                    fromMemoryType: 'episode',
                    toMemoryId: pair.to_id,
                    toMemoryType: 'episode',
                    similarity: 0.8, // Simplified - would use embedding similarity in production
                    uplift: doublyRobustEstimate,
                    confidence,
                    sampleSize,
                    mechanism: `${pair.from_task} → ${pair.to_task} (doubly robust)`,
                    metadata: {
                        propensity,
                        mu1,
                        mu0,
                        discoveredAt: Date.now()
                    }
                };
                this.causalGraph.addCausalEdge(edge);
                discovered++;
            }
        }
        return discovered;
    }
    /**
     * Calculate propensity score e(x) - probability of treatment given context
     */
    calculatePropensity(episodeId) {
        const episode = this.db.prepare('SELECT task, session_id FROM episodes WHERE id = ?').get(episodeId);
        // Count occurrences of this task type in session
        const counts = this.db.prepare(`
      SELECT
        COUNT(*) as total,
        SUM(CASE WHEN task = ? THEN 1 ELSE 0 END) as task_count
      FROM episodes
      WHERE session_id = ?
    `).get(episode.task, episode.session_id);
        const propensity = counts.task_count / Math.max(counts.total, 1);
        // Clip to avoid division by zero
        return Math.max(0.01, Math.min(0.99, propensity));
    }
    /**
     * Calculate outcome model μ(x) - expected outcome given treatment status
     */
    calculateOutcomeModel(task, treated) {
        // Get average reward for episodes with/without this task in their history
        const avgReward = this.db.prepare(`
      SELECT AVG(reward) as avg_reward
      FROM episodes
      WHERE ${treated ? '' : 'NOT'} EXISTS (
        SELECT 1 FROM episodes e2
        WHERE e2.session_id = episodes.session_id
          AND e2.task = ?
          AND e2.ts < episodes.ts
      )
    `).get(task);
        return avgReward?.avg_reward || 0.5;
    }
    /**
     * Get sample size for a task type
     */
    getSampleSize(task) {
        const count = this.db.prepare(`
      SELECT COUNT(*) as count
      FROM episodes
      WHERE task = ?
    `).get(task);
        return count.count;
    }
    /**
     * Calculate confidence based on sample size and effect size
     */
    calculateConfidence(sampleSize, uplift) {
        // Simplified confidence calculation
        // In production, use proper statistical methods (bootstrap, etc.)
        const sampleFactor = Math.min(sampleSize / 100, 1.0); // Max at 100 samples
        const effectSizeFactor = Math.min(Math.abs(uplift) / 0.5, 1.0); // Max at 0.5 uplift
        return sampleFactor * effectSizeFactor;
    }
    /**
     * Complete running A/B experiments and calculate uplift
     */
    async completeExperiments() {
        // Better-sqlite3 best practice: Prepare statements OUTSIDE loops for better performance
        const runningExperiments = this.db.prepare(`
      SELECT id, start_time, sample_size
      FROM causal_experiments
      WHERE status = 'running'
        AND sample_size >= ?
    `).all(this.config.minSampleSize);
        let completed = 0;
        for (const exp of runningExperiments) {
            try {
                this.causalGraph.calculateUplift(exp.id);
                completed++;
            }
            catch (error) {
                console.error(`   ⚠ Failed to calculate uplift for experiment ${exp.id}:`, error);
            }
        }
        return completed;
    }
    /**
     * Create new A/B experiments for promising hypotheses
     */
    async createExperiments() {
        const currentExperiments = this.db.prepare(`
      SELECT COUNT(*) as count
      FROM causal_experiments
      WHERE status = 'running'
    `).get();
        const available = this.config.experimentBudget - currentExperiments.count;
        if (available <= 0) {
            return 0;
        }
        // Find promising task pairs that don't have experiments yet
        const candidates = this.db.prepare(`
      SELECT DISTINCT
        e1.task as treatment_task,
        e1.id as treatment_id,
        COUNT(e2.id) as potential_outcomes
      FROM episodes e1
      JOIN episodes e2 ON e1.session_id = e2.session_id
      WHERE e2.ts > e1.ts
        AND NOT EXISTS (
          SELECT 1 FROM causal_experiments
          WHERE treatment_id = e1.id
        )
      GROUP BY e1.task, e1.id
      HAVING COUNT(e2.id) >= ?
      ORDER BY COUNT(e2.id) DESC
      LIMIT ?
    `).all(this.config.minSampleSize, available);
        let created = 0;
        for (const candidate of candidates) {
            const expId = this.causalGraph.createExperiment({
                name: `Auto: ${candidate.treatment_task} Impact`,
                hypothesis: `${candidate.treatment_task} affects downstream outcomes`,
                treatmentId: candidate.treatment_id,
                treatmentType: 'episode',
                startTime: Date.now(),
                sampleSize: 0,
                status: 'running',
                metadata: {
                    autoGenerated: true,
                    potentialOutcomes: candidate.potential_outcomes
                }
            });
            created++;
        }
        return created;
    }
    /**
     * Prune old or low-confidence edges
     */
    async pruneEdges() {
        const maxAgeMs = this.config.edgeMaxAgeDays * 24 * 60 * 60 * 1000;
        const cutoffTime = Date.now() / 1000 - maxAgeMs / 1000;
        const result = this.db.prepare(`
      DELETE FROM causal_edges
      WHERE confidence < ?
        OR created_at < ?
    `).run(this.config.confidenceThreshold, cutoffTime);
        return result.changes;
    }
    /**
     * Calculate overall statistics
     */
    calculateStats() {
        const stats = this.db.prepare(`
      SELECT
        AVG(ABS(uplift)) as avg_uplift,
        AVG(confidence) as avg_confidence
      FROM causal_edges
      WHERE uplift IS NOT NULL
    `).get();
        return {
            avgUplift: stats?.avg_uplift || 0,
            avgConfidence: stats?.avg_confidence || 0
        };
    }
    /**
     * Generate recommendations based on learning results
     */
    generateRecommendations(report) {
        const recommendations = [];
        if (report.edgesDiscovered === 0) {
            recommendations.push('No new causal edges discovered. Consider collecting more diverse episode data.');
        }
        if (report.avgUplift < 0.1) {
            recommendations.push('Average uplift is low. Review task sequences for optimization opportunities.');
        }
        if (report.avgConfidence < 0.7) {
            recommendations.push('Average confidence is below target. Increase sample sizes or refine hypothesis selection.');
        }
        if (report.experimentsCompleted > 0) {
            recommendations.push(`${report.experimentsCompleted} experiments completed. Review results for actionable insights.`);
        }
        if (report.edgesPruned > report.edgesDiscovered) {
            recommendations.push('More edges pruned than discovered. Consider adjusting confidence thresholds.');
        }
        return recommendations;
    }
    /**
     * Print report to console
     */
    printReport(report) {
        console.log('═══════════════════════════════════════════════════════════');
        console.log('  Nightly Learner Report');
        console.log('═══════════════════════════════════════════════════════════\n');
        console.log(`  Execution Time: ${report.executionTimeMs}ms`);
        console.log(`  Timestamp: ${new Date(report.timestamp).toISOString()}\n`);
        console.log('  Results:');
        console.log(`    • Edges Discovered: ${report.edgesDiscovered}`);
        console.log(`    • Edges Pruned: ${report.edgesPruned}`);
        console.log(`    • Experiments Completed: ${report.experimentsCompleted}`);
        console.log(`    • Experiments Created: ${report.experimentsCreated}\n`);
        console.log('  Statistics:');
        console.log(`    • Avg Uplift: ${report.avgUplift.toFixed(3)}`);
        console.log(`    • Avg Confidence: ${report.avgConfidence.toFixed(3)}\n`);
        if (report.recommendations.length > 0) {
            console.log('  Recommendations:');
            report.recommendations.forEach(rec => console.log(`    • ${rec}`));
            console.log('');
        }
        console.log('═══════════════════════════════════════════════════════════\n');
    }
    /**
     * Update learner configuration
     */
    updateConfig(config) {
        this.config = { ...this.config, ...config };
    }
}
//# sourceMappingURL=NightlyLearner.js.map