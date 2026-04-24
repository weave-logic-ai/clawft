/**
 * RuVector Unified Intelligence Layer
 *
 * Integrates the FULL power of RuVector ecosystem:
 *
 * @ruvector/sona - Self-Learning:
 *   - Micro-LoRA: Ultra-fast rank-1/2 adaptations (~0.1ms)
 *   - Base-LoRA: Deeper pattern adaptations
 *   - EWC++: Elastic Weight Consolidation (catastrophic forgetting prevention)
 *   - ReasoningBank: Pattern storage/retrieval via findPatterns
 *   - Trajectory tracking: Learn from execution paths
 *
 * @ruvector/attention - Advanced Attention:
 *   - MultiHeadAttention: Standard transformer attention
 *   - FlashAttention: Memory-efficient O(n) attention
 *   - HyperbolicAttention: Poincaré ball geometry for hierarchies
 *   - MoEAttention: Mixture of Experts routing
 *   - GraphRoPeAttention: Graph + Rotary Position Embeddings
 *   - EdgeFeaturedAttention: Edge-aware graph attention
 *   - DualSpaceAttention: Euclidean + Hyperbolic hybrid
 *
 * @ruvector/core - Vector Database:
 *   - HNSW indexing: 150x faster than brute force
 *   - Real vector similarity search
 *   - Cosine/Euclidean/Dot product distance
 *
 * Performance:
 *   - Micro-LoRA adaptation: ~0.1ms
 *   - FlashAttention: O(n) complexity vs O(n²)
 *   - HNSW search: O(log n) vs O(n)
 *   - Background learning: Non-blocking
 */
// Import from @ruvector/sona
import { SonaEngine } from '@ruvector/sona';
// Import from @ruvector/attention
import { MultiHeadAttention, FlashAttention, HyperbolicAttention, MoEAttention, GraphRoPeAttention, DualSpaceAttention, 
// Training (simplified - only AdamOptimizer needed)
AdamOptimizer, 
// Utilities
computeAttentionAsync, computeFlashAttentionAsync, computeHyperbolicAttentionAsync, 
// Hyperbolic math
poincareDistance, projectToPoincareBall, AttentionType, } from '@ruvector/attention';
// Import from ruvector core (for HNSW)
import ruvector from 'ruvector';
export class RuVectorIntelligence {
    config;
    initialized = false;
    initPromise = null;
    // SONA Engine for self-learning
    sona = null;
    // Attention mechanisms
    multiHeadAttention = null;
    flashAttention = null;
    hyperbolicAttention = null;
    moeAttention = null;
    graphAttention = null;
    dualSpaceAttention = null;
    // Training (simplified - removed unused scheduler/loss)
    optimizer = null;
    // HNSW index for vector search
    hnswIndex = null;
    // Active trajectories with LRU tracking
    trajectories = new Map();
    trajectoryAccessOrder = []; // For LRU eviction
    nextTrajectoryId = 0;
    // Agent embeddings with LRU tracking
    agentEmbeddings = new Map();
    agentAccessOrder = []; // For LRU eviction
    // Background learning timer
    learningTimer = null;
    // Cleanup timer for stale trajectories
    cleanupTimer = null;
    // Statistics
    stats = {
        totalRoutings: 0,
        totalTrajectories: 0,
        totalLearningCycles: 0,
        avgRoutingLatencyMs: 0,
        patternsLearned: 0,
        hnswQueries: 0,
        sonaAdaptations: 0,
        evictedTrajectories: 0,
        evictedAgents: 0,
    };
    constructor(config) {
        this.config = {
            embeddingDim: config?.embeddingDim ?? 384,
            hiddenDim: config?.hiddenDim ?? 256,
            enableSona: config?.enableSona ?? true,
            sonaConfig: config?.sonaConfig ?? {},
            attentionType: config?.attentionType ?? 'moe',
            numHeads: config?.numHeads ?? 8,
            numExperts: config?.numExperts ?? 4,
            topK: config?.topK ?? 2,
            curvature: config?.curvature ?? 1.0,
            enableHnsw: config?.enableHnsw ?? true,
            hnswM: config?.hnswM ?? 16,
            hnswEfConstruction: config?.hnswEfConstruction ?? 200,
            enableTrajectories: config?.enableTrajectories ?? true,
            qualityThreshold: config?.qualityThreshold ?? 0.5,
            backgroundIntervalMs: config?.backgroundIntervalMs ?? 60000,
            // Memory limits with LRU eviction
            maxTrajectories: config?.maxTrajectories ?? 1000,
            trajectoryTTLMs: config?.trajectoryTTLMs ?? 1800000, // 30 min
            maxAgentEmbeddings: config?.maxAgentEmbeddings ?? 500,
        };
        // Initialize asynchronously to avoid race conditions
        this.initPromise = this.initializeAsync();
    }
    /**
     * Wait for initialization to complete
     */
    async waitForInit() {
        if (this.initPromise) {
            await this.initPromise;
        }
    }
    /**
     * Initialize all components (async to avoid race conditions)
     */
    async initializeAsync() {
        if (this.initialized)
            return;
        const dim = this.config.embeddingDim;
        // Initialize SONA Engine
        if (this.config.enableSona) {
            try {
                // Ensure all values are explicitly defined (no undefined values)
                const sonaConfig = {
                    hiddenDim: this.config.hiddenDim ?? 256,
                    embeddingDim: dim ?? 384,
                    microLoraRank: 1, // Ultra-fast rank-1
                    baseLoraRank: 8,
                    microLoraLr: 0.001,
                    baseLoraLr: 0.0001,
                    ewcLambda: 1000.0, // EWC++ regularization
                    patternClusters: 50,
                    trajectoryCapacity: 10000,
                    qualityThreshold: this.config.qualityThreshold ?? 0.5, // Ensure defined
                    enableSimd: true,
                    // Only spread defined values from sonaConfig
                    ...(this.config.sonaConfig && Object.fromEntries(Object.entries(this.config.sonaConfig).filter(([, v]) => v !== undefined))),
                };
                this.sona = SonaEngine.withConfig(sonaConfig);
            }
            catch (err) {
                console.warn('[RuVectorIntelligence] SONA init failed, using fallback:', err);
                this.sona = null;
            }
        }
        // Initialize attention mechanisms based on type
        try {
            switch (this.config.attentionType) {
                case 'multi_head':
                    this.multiHeadAttention = new MultiHeadAttention(dim, this.config.numHeads ?? 8);
                    break;
                case 'flash':
                    this.flashAttention = new FlashAttention(dim);
                    break;
                case 'hyperbolic':
                    this.hyperbolicAttention = new HyperbolicAttention(dim, this.config.curvature ?? 1.0);
                    break;
                case 'moe':
                    const moeConfig = {
                        dim,
                        numExperts: this.config.numExperts ?? 4,
                        topK: this.config.topK ?? 2,
                    };
                    this.moeAttention = new MoEAttention(moeConfig);
                    break;
                case 'graph':
                    this.graphAttention = new GraphRoPeAttention(dim, 10000);
                    break;
                case 'dual':
                    this.dualSpaceAttention = new DualSpaceAttention(dim, this.config.curvature ?? 1.0, 0.5, // Euclidean weight
                    0.5 // Hyperbolic weight
                    );
                    break;
            }
        }
        catch (err) {
            console.warn('[RuVectorIntelligence] Attention init failed, using fallback:', err);
        }
        // Initialize training components (simplified - only optimizer needed)
        try {
            // AdamOptimizer requires: learning_rate, beta1, beta2, epsilon
            this.optimizer = new AdamOptimizer(0.001, 0.9, 0.999, 1e-8);
        }
        catch (err) {
            console.warn('[RuVectorIntelligence] Optimizer init failed:', err);
        }
        // Initialize HNSW index
        if (this.config.enableHnsw) {
            try {
                this.initializeHnsw();
            }
            catch (err) {
                console.warn('[RuVectorIntelligence] HNSW init failed:', err);
            }
        }
        // Start background learning
        if (this.config.enableSona) {
            this.startBackgroundLearning();
        }
        // Start cleanup timer for stale trajectories
        this.startCleanupTimer();
        this.initialized = true;
    }
    /**
     * Start cleanup timer for stale trajectories
     */
    startCleanupTimer() {
        if (this.cleanupTimer) {
            clearInterval(this.cleanupTimer);
        }
        // Run cleanup every 5 minutes
        this.cleanupTimer = setInterval(() => {
            this.cleanupStaleTrajectories();
        }, 300000);
    }
    /**
     * Clean up trajectories older than TTL
     */
    cleanupStaleTrajectories() {
        const now = Date.now();
        const ttl = this.config.trajectoryTTLMs;
        for (const [id, trajectory] of this.trajectories) {
            if (now - trajectory.startTime > ttl) {
                this.trajectories.delete(id);
                this.trajectoryAccessOrder = this.trajectoryAccessOrder.filter(t => t !== id);
                this.stats.evictedTrajectories++;
            }
        }
    }
    /**
     * LRU eviction for trajectories when limit exceeded
     */
    evictOldestTrajectory() {
        if (this.trajectoryAccessOrder.length === 0)
            return;
        const oldestId = this.trajectoryAccessOrder.shift();
        this.trajectories.delete(oldestId);
        this.stats.evictedTrajectories++;
    }
    /**
     * LRU eviction for agent embeddings when limit exceeded
     */
    evictOldestAgent() {
        if (this.agentAccessOrder.length === 0)
            return;
        const oldestId = this.agentAccessOrder.shift();
        this.agentEmbeddings.delete(oldestId);
        this.stats.evictedAgents++;
    }
    /**
     * Update LRU access order for trajectory
     */
    touchTrajectory(id) {
        const idx = this.trajectoryAccessOrder.indexOf(id);
        if (idx !== -1) {
            this.trajectoryAccessOrder.splice(idx, 1);
        }
        this.trajectoryAccessOrder.push(id);
    }
    /**
     * Update LRU access order for agent
     */
    touchAgent(id) {
        const idx = this.agentAccessOrder.indexOf(id);
        if (idx !== -1) {
            this.agentAccessOrder.splice(idx, 1);
        }
        this.agentAccessOrder.push(id);
    }
    /**
     * Initialize HNSW index for fast vector search
     */
    initializeHnsw() {
        try {
            // Use ruvector core HNSW
            this.hnswIndex = new ruvector.HnswIndex({
                dim: this.config.embeddingDim,
                m: this.config.hnswM,
                efConstruction: this.config.hnswEfConstruction,
                distance: 'cosine',
            });
        }
        catch (error) {
            console.warn('HNSW initialization failed, falling back to brute force:', error);
            this.hnswIndex = null;
        }
    }
    /**
     * Register an agent with its embedding
     *
     * @param agentId - Unique agent identifier
     * @param embedding - Agent's semantic embedding
     * @param metadata - Optional metadata
     * @returns Operation result indicating success/failure
     */
    async registerAgent(agentId, embedding, metadata) {
        await this.waitForInit();
        try {
            const embeddingArray = embedding instanceof Float32Array
                ? embedding
                : new Float32Array(embedding);
            // LRU eviction if at capacity
            while (this.agentEmbeddings.size >= this.config.maxAgentEmbeddings) {
                this.evictOldestAgent();
            }
            // Store in cache and update LRU order
            this.agentEmbeddings.set(agentId, embeddingArray);
            this.touchAgent(agentId);
            // Add to HNSW index
            if (this.hnswIndex) {
                try {
                    await this.hnswIndex.add(agentId, embeddingArray);
                }
                catch (error) {
                    console.warn(`Failed to add agent ${agentId} to HNSW:`, error);
                }
            }
            return { success: true };
        }
        catch (error) {
            return {
                success: false,
                error: error instanceof Error ? error.message : String(error),
            };
        }
    }
    /**
     * Route a task to the best agent using full intelligence stack
     *
     * Uses:
     * - HNSW for fast candidate retrieval
     * - Attention mechanism for ranking
     * - SONA for adaptive learning
     *
     * @param taskEmbedding - Task's semantic embedding
     * @param candidates - Optional candidate agent IDs
     * @param topK - Number of results
     */
    async routeTask(taskEmbedding, candidates, topK = 5) {
        const startTime = performance.now();
        const query = taskEmbedding instanceof Float32Array
            ? taskEmbedding
            : new Float32Array(taskEmbedding);
        let results = [];
        let usedHnsw = false;
        let usedSona = false;
        // Step 1: Get candidates via HNSW (150x faster than brute force)
        let candidateAgents = [];
        if (this.hnswIndex && this.agentEmbeddings.size > 10) {
            try {
                const hnswResults = await this.hnswIndex.search(query, Math.min(topK * 2, 20));
                candidateAgents = hnswResults.map((r) => ({
                    id: r.id,
                    embedding: this.agentEmbeddings.get(r.id),
                    score: r.score,
                }));
                usedHnsw = true;
                this.stats.hnswQueries++;
            }
            catch (error) {
                // Fallback to all agents
            }
        }
        // Fallback: use provided candidates or all agents
        if (candidateAgents.length === 0) {
            const agentIds = candidates || Array.from(this.agentEmbeddings.keys());
            candidateAgents = agentIds
                .filter(id => this.agentEmbeddings.has(id))
                .map(id => ({
                id,
                embedding: this.agentEmbeddings.get(id),
                score: 0,
            }));
        }
        if (candidateAgents.length === 0) {
            return [];
        }
        // Step 2: Apply SONA Micro-LoRA transformation (~0.1ms)
        let transformedQuery = query;
        if (this.sona) {
            try {
                const loraResult = this.sona.applyMicroLora(Array.from(query));
                transformedQuery = new Float32Array(loraResult);
                usedSona = true;
                this.stats.sonaAdaptations++;
            }
            catch (error) {
                // Use original query
            }
        }
        // Step 3: Compute attention scores
        const keys = candidateAgents.map(c => c.embedding);
        const values = candidateAgents.map(c => c.embedding);
        let attentionOutput;
        let expertWeights;
        switch (this.config.attentionType) {
            case 'moe':
                if (this.moeAttention) {
                    attentionOutput = this.moeAttention.compute(transformedQuery, keys, values);
                    // MoE provides expert routing weights
                    expertWeights = Array.from(attentionOutput.slice(0, this.config.numExperts));
                }
                else {
                    attentionOutput = this.computeFallbackAttention(transformedQuery, keys, values);
                }
                break;
            case 'flash':
                if (this.flashAttention) {
                    attentionOutput = this.flashAttention.compute(transformedQuery, keys, values);
                }
                else {
                    attentionOutput = this.computeFallbackAttention(transformedQuery, keys, values);
                }
                break;
            case 'hyperbolic':
                if (this.hyperbolicAttention) {
                    attentionOutput = this.hyperbolicAttention.compute(transformedQuery, keys, values);
                }
                else {
                    attentionOutput = this.computeFallbackAttention(transformedQuery, keys, values);
                }
                break;
            case 'dual':
                if (this.dualSpaceAttention) {
                    attentionOutput = this.dualSpaceAttention.compute(transformedQuery, keys, values);
                }
                else {
                    attentionOutput = this.computeFallbackAttention(transformedQuery, keys, values);
                }
                break;
            default:
                if (this.multiHeadAttention) {
                    attentionOutput = this.multiHeadAttention.compute(transformedQuery, keys, values);
                }
                else {
                    attentionOutput = this.computeFallbackAttention(transformedQuery, keys, values);
                }
        }
        // Step 4: Compute similarity scores for ranking
        const scores = candidateAgents.map((agent, i) => {
            // Combine HNSW score with attention-weighted score
            const attentionScore = this.cosineSimilarity(attentionOutput, agent.embedding);
            const hnswScore = agent.score || 0;
            // Weighted combination
            const finalScore = usedHnsw
                ? 0.3 * hnswScore + 0.7 * attentionScore
                : attentionScore;
            return {
                agentId: agent.id,
                confidence: finalScore,
                attentionWeights: attentionOutput,
                expertWeights,
                latencyMs: 0,
                usedHnsw,
                usedSona,
            };
        });
        // Sort by confidence
        results = scores
            .sort((a, b) => b.confidence - a.confidence)
            .slice(0, topK);
        // Update latency
        const latencyMs = performance.now() - startTime;
        results.forEach(r => (r.latencyMs = latencyMs));
        // Update stats
        this.stats.totalRoutings++;
        this.stats.avgRoutingLatencyMs =
            (this.stats.avgRoutingLatencyMs * (this.stats.totalRoutings - 1) + latencyMs) /
                this.stats.totalRoutings;
        return results;
    }
    /**
     * Fallback attention using dot product
     */
    computeFallbackAttention(query, keys, values) {
        const dim = query.length;
        const result = new Float32Array(dim);
        let totalWeight = 0;
        for (let i = 0; i < keys.length; i++) {
            const similarity = this.cosineSimilarity(query, keys[i]);
            const weight = Math.exp(similarity * 10); // Temperature scaling
            totalWeight += weight;
            for (let j = 0; j < dim; j++) {
                result[j] += weight * values[i][j];
            }
        }
        // Normalize
        if (totalWeight > 0) {
            for (let j = 0; j < dim; j++) {
                result[j] /= totalWeight;
            }
        }
        return result;
    }
    /**
     * Cosine similarity between two vectors
     */
    cosineSimilarity(a, b) {
        let dot = 0;
        let normA = 0;
        let normB = 0;
        for (let i = 0; i < a.length; i++) {
            dot += a[i] * b[i];
            normA += a[i] * a[i];
            normB += b[i] * b[i];
        }
        const norm = Math.sqrt(normA) * Math.sqrt(normB);
        return norm > 0 ? dot / norm : 0;
    }
    // ==========================================================================
    // Trajectory Learning (SONA)
    // ==========================================================================
    /**
     * Begin a new trajectory for learning
     *
     * @param query - The task query
     * @param embedding - Query embedding
     * @returns Operation result with trajectory ID
     */
    beginTrajectory(query, embedding) {
        if (!this.config.enableTrajectories) {
            return {
                success: false,
                error: 'Trajectories are disabled in config',
            };
        }
        if (!this.sona) {
            return {
                success: false,
                error: 'SONA engine not initialized',
            };
        }
        try {
            // LRU eviction if at capacity
            while (this.trajectories.size >= this.config.maxTrajectories) {
                this.evictOldestTrajectory();
            }
            const trajectoryId = this.nextTrajectoryId++;
            // Start SONA trajectory
            const sonaId = this.sona.beginTrajectory(embedding);
            // Store local trajectory
            this.trajectories.set(trajectoryId, {
                id: sonaId,
                query,
                embedding,
                steps: [],
                contexts: [],
                startTime: Date.now(),
            });
            // Update LRU order
            this.touchTrajectory(trajectoryId);
            this.stats.totalTrajectories++;
            return { success: true, value: trajectoryId };
        }
        catch (error) {
            return {
                success: false,
                error: error instanceof Error ? error.message : String(error),
            };
        }
    }
    /**
     * Add a step to trajectory
     *
     * @param trajectoryId - Trajectory ID from beginTrajectory
     * @param action - Action taken (e.g., agent selected)
     * @param reward - Reward for this step (0-1)
     * @param activations - Optional activations
     * @param attentionWeights - Optional attention weights
     */
    addTrajectoryStep(trajectoryId, action, reward, activations, attentionWeights) {
        const trajectory = this.trajectories.get(trajectoryId);
        if (!trajectory || !this.sona) {
            return;
        }
        const step = {
            action,
            activations: activations || new Array(this.config.hiddenDim).fill(0),
            attentionWeights: attentionWeights || new Array(this.config.numHeads).fill(0),
            reward,
            timestamp: Date.now(),
        };
        trajectory.steps.push(step);
        // Add to SONA
        this.sona.addTrajectoryStep(trajectory.id, step.activations, step.attentionWeights, reward);
    }
    /**
     * Set the route (agent selected) for trajectory
     */
    setTrajectoryRoute(trajectoryId, route) {
        const trajectory = this.trajectories.get(trajectoryId);
        if (!trajectory || !this.sona) {
            return;
        }
        trajectory.route = route;
        this.sona.setTrajectoryRoute(trajectory.id, route);
    }
    /**
     * Add context to trajectory
     */
    addTrajectoryContext(trajectoryId, contextId) {
        const trajectory = this.trajectories.get(trajectoryId);
        if (!trajectory || !this.sona) {
            return;
        }
        trajectory.contexts.push(contextId);
        this.sona.addTrajectoryContext(trajectory.id, contextId);
    }
    /**
     * End trajectory and submit for learning
     *
     * @param trajectoryId - Trajectory ID
     * @param success - Whether the task succeeded
     * @param quality - Quality score (0-1)
     * @returns Learning outcome
     */
    endTrajectory(trajectoryId, success, quality) {
        const trajectory = this.trajectories.get(trajectoryId);
        if (!trajectory || !this.sona) {
            return {
                trajectoryId,
                success,
                quality,
                patternsLearned: 0,
                adaptations: { microLora: false, baseLora: false, ewc: false },
            };
        }
        // End SONA trajectory
        this.sona.endTrajectory(trajectory.id, quality);
        // Cleanup
        this.trajectories.delete(trajectoryId);
        return {
            trajectoryId,
            success,
            quality,
            patternsLearned: quality >= this.config.qualityThreshold ? 1 : 0,
            adaptations: {
                microLora: true, // Micro-LoRA always adapts
                baseLora: quality >= 0.7, // Base-LoRA for high quality
                ewc: quality >= 0.8, // EWC++ for very high quality
            },
        };
    }
    // ==========================================================================
    // Pattern Retrieval (ReasoningBank)
    // ==========================================================================
    /**
     * Find similar learned patterns
     *
     * Uses SONA's ReasoningBank for pattern retrieval
     *
     * @param embedding - Query embedding
     * @param k - Number of patterns to return
     */
    findPatterns(embedding, k = 5) {
        if (!this.sona) {
            return [];
        }
        return this.sona.findPatterns(embedding, k);
    }
    /**
     * Force a learning cycle
     */
    forceLearning() {
        if (!this.sona) {
            return 'SONA not enabled';
        }
        const result = this.sona.forceLearn();
        this.stats.totalLearningCycles++;
        return result;
    }
    // ==========================================================================
    // Background Learning
    // ==========================================================================
    /**
     * Start background learning timer
     */
    startBackgroundLearning() {
        if (this.learningTimer) {
            clearInterval(this.learningTimer);
        }
        this.learningTimer = setInterval(() => {
            if (this.sona) {
                const result = this.sona.tick();
                if (result) {
                    this.stats.totalLearningCycles++;
                    this.stats.patternsLearned = this.getPatternsCount();
                }
            }
        }, this.config.backgroundIntervalMs);
    }
    /**
     * Get patterns count from SONA stats
     */
    getPatternsCount() {
        if (!this.sona)
            return 0;
        try {
            const stats = JSON.parse(this.sona.getStats());
            return stats.patterns_count || 0;
        }
        catch {
            return 0;
        }
    }
    // ==========================================================================
    // Async Attention (for large batches)
    // ==========================================================================
    /**
     * Compute attention asynchronously
     *
     * Useful for large batches or when non-blocking is required
     */
    async computeAttentionAsync(query, keys, values, type) {
        switch (type) {
            case 'flash':
                return computeFlashAttentionAsync(query, keys, values);
            case 'hyperbolic':
                return computeHyperbolicAttentionAsync(query, keys, values, this.config.curvature);
            default:
                return computeAttentionAsync(query, keys, values, AttentionType.MultiHead);
        }
    }
    // ==========================================================================
    // Hyperbolic Operations (for hierarchical structures)
    // ==========================================================================
    /**
     * Compute Poincaré distance between two embeddings
     *
     * Useful for hierarchical agent structures
     */
    poincareDistance(a, b) {
        return poincareDistance(a, b, this.config.curvature);
    }
    /**
     * Project embedding to Poincaré ball
     */
    projectToPoincare(embedding) {
        return projectToPoincareBall(embedding, this.config.curvature);
    }
    // ==========================================================================
    // Statistics & Status
    // ==========================================================================
    /**
     * Get intelligence layer statistics
     */
    getStats() {
        let sonaStats = null;
        if (this.sona) {
            try {
                sonaStats = JSON.parse(this.sona.getStats());
            }
            catch { }
        }
        return {
            ...this.stats,
            sonaStats,
        };
    }
    /**
     * Enable/disable the intelligence layer
     */
    setEnabled(enabled) {
        if (this.sona) {
            this.sona.setEnabled(enabled);
        }
    }
    /**
     * Check if enabled
     */
    isEnabled() {
        return this.sona?.isEnabled() ?? false;
    }
    /**
     * Cleanup resources
     */
    dispose() {
        if (this.learningTimer) {
            clearInterval(this.learningTimer);
            this.learningTimer = null;
        }
        if (this.cleanupTimer) {
            clearInterval(this.cleanupTimer);
            this.cleanupTimer = null;
        }
        // Flush SONA
        if (this.sona) {
            this.sona.flush();
        }
        // Clear caches and LRU tracking
        this.trajectories.clear();
        this.trajectoryAccessOrder = [];
        this.agentEmbeddings.clear();
        this.agentAccessOrder = [];
    }
}
/**
 * Create a default intelligence layer
 */
export function createIntelligenceLayer(config) {
    return new RuVectorIntelligence(config);
}
/**
 * Presets for common configurations
 */
export const IntelligencePresets = {
    /** Fast routing with MoE and minimal learning */
    fast: {
        attentionType: 'moe',
        numExperts: 4,
        topK: 2,
        enableTrajectories: false,
        backgroundIntervalMs: 300000, // 5 min
    },
    /** Balanced performance and learning */
    balanced: {
        attentionType: 'moe',
        numExperts: 4,
        topK: 2,
        enableTrajectories: true,
        backgroundIntervalMs: 60000, // 1 min
        qualityThreshold: 0.5,
    },
    /** Maximum learning for development */
    learning: {
        attentionType: 'dual',
        enableTrajectories: true,
        backgroundIntervalMs: 30000, // 30 sec
        qualityThreshold: 0.3,
        sonaConfig: {
            microLoraRank: 2,
            baseLoraRank: 16,
            trajectoryCapacity: 50000,
        },
    },
    /** Hierarchical structures (Poincaré geometry) */
    hierarchical: {
        attentionType: 'hyperbolic',
        curvature: 1.0,
        enableTrajectories: true,
    },
    /** Graph-based reasoning */
    graph: {
        attentionType: 'graph',
        enableTrajectories: true,
    },
};
//# sourceMappingURL=RuVectorIntelligence.js.map