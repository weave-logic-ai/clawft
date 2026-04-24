/**
 * Attention-Based Multi-Agent Coordination
 *
 * Uses attention mechanisms for intelligent agent consensus, task routing,
 * and topology-aware coordination in multi-agent swarms.
 *
 * @module attention-coordinator
 * @version 2.0.0-alpha
 */
/**
 * Attention-based multi-agent coordinator
 *
 * @example Basic Consensus
 * ```typescript
 * const coordinator = new AttentionCoordinator(attentionService);
 *
 * const agentOutputs = [
 *   { agentId: 'agent-1', embedding: emb1, value: result1 },
 *   { agentId: 'agent-2', embedding: emb2, value: result2 },
 *   // ... more agents
 * ];
 *
 * const result = await coordinator.coordinateAgents(agentOutputs);
 * console.log(`Consensus: ${result.consensus}`);
 * console.log(`Top agents: ${result.topAgents}`);
 * ```
 *
 * @example Expert Routing (MoE)
 * ```typescript
 * const task = {
 *   id: 'task-1',
 *   embedding: taskEmbedding,
 *   description: 'Optimize database queries'
 * };
 *
 * const experts = await coordinator.routeToExperts(task, allAgents, 3);
 * console.log(`Selected experts: ${experts.selectedExperts.map(e => e.type)}`);
 * ```
 *
 * @example Topology-Aware Coordination
 * ```typescript
 * const result = await coordinator.topologyAwareCoordination(
 *   agentOutputs,
 *   'mesh', // or 'hierarchical', 'ring', 'star'
 *   graphStructure
 * );
 * ```
 */
export class AttentionCoordinator {
    attentionService;
    constructor(attentionService) {
        this.attentionService = attentionService;
    }
    /**
     * Coordinate agents using attention-based consensus
     *
     * Uses multi-head attention to weight agent contributions based on
     * relevance and confidence. Better than simple voting or averaging.
     *
     * @param agentOutputs - Outputs from multiple agents
     * @param mechanism - Attention mechanism to use (default: 'flash')
     * @returns Coordination result with weighted consensus
     */
    async coordinateAgents(agentOutputs, mechanism = 'flash') {
        if (agentOutputs.length === 0) {
            throw new Error('No agent outputs to coordinate');
        }
        const startTime = Date.now();
        // Stack embeddings into tensors
        const Q = this.stackEmbeddings(agentOutputs.map((o) => o.embedding));
        const K = Q; // Self-attention over agent outputs
        const V = Q;
        // Run attention mechanism
        let attentionResult;
        switch (mechanism) {
            case 'flash':
                attentionResult = await this.attentionService.flashAttention(Q, K, V);
                break;
            case 'multi-head':
                attentionResult = await this.attentionService.multiHeadAttention(Q, K, V);
                break;
            default:
                attentionResult = await this.attentionService.flashAttention(Q, K, V);
        }
        // Extract attention weights for each agent
        const weights = this.extractAttentionWeights(attentionResult.attentionWeights || attentionResult.output, agentOutputs.length);
        // Compute weighted consensus
        const consensus = this.weightedConsensus(agentOutputs, weights);
        // Identify top contributing agents
        const topAgents = agentOutputs
            .map((output, i) => ({ agentId: output.agentId, weight: weights[i] }))
            .sort((a, b) => b.weight - a.weight)
            .slice(0, 3)
            .map((a) => a.agentId);
        const executionTimeMs = Date.now() - startTime;
        return {
            consensus,
            attentionWeights: weights,
            mechanism,
            executionTimeMs,
            topAgents,
        };
    }
    /**
     * Route tasks to specialized experts using MoE attention
     *
     * Uses Mixture-of-Experts attention to select top-k agents
     * best suited for a given task.
     *
     * @param task - Task to route
     * @param agents - Available specialized agents
     * @param topK - Number of experts to select (default: 3)
     * @returns Selected experts with routing scores
     */
    async routeToExperts(task, agents, topK = 3) {
        if (agents.length === 0) {
            throw new Error('No agents available for routing');
        }
        const startTime = Date.now();
        // Task embedding as query
        const Q = task.embedding;
        // Agent specializations as keys and values
        const K = this.stackEmbeddings(agents.map((a) => a.specialization));
        const V = K;
        // MoE attention for expert selection
        const attentionResult = await this.attentionService.moeAttention(Q, K, V, agents.length);
        // Extract routing scores
        const scores = this.extractRoutingScores(attentionResult.attentionWeights || attentionResult.output, agents.length);
        // Select top-k experts
        const expertsWithScores = agents.map((agent, i) => ({
            agent,
            score: scores[i],
        }));
        expertsWithScores.sort((a, b) => b.score - a.score);
        const selectedExperts = expertsWithScores.slice(0, topK).map((e) => e.agent);
        const routingScores = expertsWithScores.slice(0, topK).map((e) => e.score);
        const executionTimeMs = Date.now() - startTime;
        return {
            selectedExperts,
            routingScores,
            mechanism: 'moe',
            executionTimeMs,
        };
    }
    /**
     * Topology-aware agent coordination using GraphRoPE
     *
     * Uses graph-aware positional embeddings to coordinate agents
     * based on their position in the swarm topology (mesh, hierarchical, etc.)
     *
     * @param agentOutputs - Agent outputs
     * @param topology - Swarm topology type
     * @param graphStructure - Graph structure of agent network
     * @returns Coordination result with topology-aware weights
     */
    async topologyAwareCoordination(agentOutputs, topology, graphStructure) {
        const startTime = Date.now();
        // Build graph structure from topology if not provided
        const graph = graphStructure || this.buildTopologyGraph(agentOutputs, topology);
        // Stack embeddings
        const Q = this.stackEmbeddings(agentOutputs.map((o) => o.embedding));
        const K = Q;
        const V = Q;
        // GraphRoPE attention preserves topological relationships
        const attentionResult = await this.attentionService.graphRoPEAttention(Q, K, V, graph);
        // Extract topology-aware weights
        const weights = this.extractAttentionWeights(attentionResult.attentionWeights || attentionResult.output, agentOutputs.length);
        // Apply topology bias (agents closer in topology get higher weights)
        const biasedWeights = this.applyTopologyBias(weights, topology, graph);
        // Weighted consensus
        const consensus = this.weightedConsensus(agentOutputs, biasedWeights);
        // Top agents
        const topAgents = agentOutputs
            .map((output, i) => ({ agentId: output.agentId, weight: biasedWeights[i] }))
            .sort((a, b) => b.weight - a.weight)
            .slice(0, 3)
            .map((a) => a.agentId);
        const executionTimeMs = Date.now() - startTime;
        return {
            consensus,
            attentionWeights: biasedWeights,
            mechanism: 'graph-rope',
            executionTimeMs,
            topAgents,
        };
    }
    /**
     * Hierarchical coordination for queen-worker swarms
     *
     * Uses hyperbolic attention to model hierarchical relationships
     * where queen/coordinator agents have higher curvature.
     *
     * @param queenOutputs - Outputs from queen/coordinator agents
     * @param workerOutputs - Outputs from worker agents
     * @param curvature - Hyperbolic curvature (-1.0 = strong hierarchy)
     * @returns Hierarchical coordination result
     */
    async hierarchicalCoordination(queenOutputs, workerOutputs, curvature = -1.0) {
        const startTime = Date.now();
        // Queens and workers in separate groups
        const allOutputs = [...queenOutputs, ...workerOutputs];
        // Stack embeddings
        const Q = this.stackEmbeddings(queenOutputs.map((o) => o.embedding)); // Queens query
        const K = this.stackEmbeddings(allOutputs.map((o) => o.embedding)); // All agents as keys
        const V = K;
        // Hyperbolic attention models hierarchical relationships
        const attentionResult = await this.attentionService.hyperbolicAttention(Q, K, V, curvature);
        // Extract weights
        const weights = this.extractAttentionWeights(attentionResult.attentionWeights || attentionResult.output, allOutputs.length);
        // Queens have higher influence in hierarchical structure
        const hierarchicalWeights = weights.map((w, i) => {
            const isQueen = i < queenOutputs.length;
            return isQueen ? w * 1.5 : w; // Boost queen weights
        });
        // Normalize
        const sumWeights = hierarchicalWeights.reduce((a, b) => a + b, 0);
        const normalizedWeights = hierarchicalWeights.map((w) => w / sumWeights);
        // Weighted consensus
        const consensus = this.weightedConsensus(allOutputs, normalizedWeights);
        // Top agents
        const topAgents = allOutputs
            .map((output, i) => ({ agentId: output.agentId, weight: normalizedWeights[i] }))
            .sort((a, b) => b.weight - a.weight)
            .slice(0, 3)
            .map((a) => a.agentId);
        const executionTimeMs = Date.now() - startTime;
        return {
            consensus,
            attentionWeights: normalizedWeights,
            mechanism: 'hyperbolic',
            executionTimeMs,
            topAgents,
        };
    }
    // ============================================
    // UTILITY FUNCTIONS
    // ============================================
    /**
     * Stack multiple embeddings into single tensor
     */
    stackEmbeddings(embeddings) {
        const total = embeddings.reduce((sum, e) => sum + e.length, 0);
        const stacked = new Float32Array(total);
        let offset = 0;
        for (const emb of embeddings) {
            stacked.set(emb, offset);
            offset += emb.length;
        }
        return stacked;
    }
    /**
     * Extract attention weights from attention output
     */
    extractAttentionWeights(output, numAgents) {
        // Simplified: average across dimensions
        const dim = output.length / numAgents;
        const weights = [];
        for (let i = 0; i < numAgents; i++) {
            const slice = output.slice(i * dim, (i + 1) * dim);
            const avgWeight = slice.reduce((a, b) => a + b, 0) / slice.length;
            weights.push(avgWeight);
        }
        // Normalize to sum to 1
        const sum = weights.reduce((a, b) => a + b, 0);
        return weights.map((w) => w / sum);
    }
    /**
     * Extract routing scores for expert selection
     */
    extractRoutingScores(output, numAgents) {
        // For MoE, use softmax-like scoring
        const scores = this.extractAttentionWeights(output, numAgents);
        // Apply softmax for clearer expert selection
        const expScores = scores.map((s) => Math.exp(s * 10)); // Temperature = 10
        const sumExp = expScores.reduce((a, b) => a + b, 0);
        return expScores.map((s) => s / sumExp);
    }
    /**
     * Compute weighted consensus from agent outputs
     */
    weightedConsensus(outputs, weights) {
        // If outputs are numeric, compute weighted average
        if (typeof outputs[0].value === 'number') {
            return outputs.reduce((sum, output, i) => sum + output.value * weights[i], 0);
        }
        // If outputs are arrays, compute element-wise weighted average
        if (Array.isArray(outputs[0].value)) {
            const length = outputs[0].value.length;
            const weighted = new Array(length).fill(0);
            for (let i = 0; i < outputs.length; i++) {
                for (let j = 0; j < length; j++) {
                    weighted[j] += outputs[i].value[j] * weights[i];
                }
            }
            return weighted;
        }
        // For objects or complex types, return most weighted output
        const maxWeightIndex = weights.indexOf(Math.max(...weights));
        return outputs[maxWeightIndex].value;
    }
    /**
     * Build graph structure from swarm topology
     */
    buildTopologyGraph(outputs, topology) {
        const nodes = outputs.map((o) => o.embedding);
        const edges = [];
        switch (topology) {
            case 'mesh':
                // Fully connected mesh
                for (let i = 0; i < outputs.length; i++) {
                    for (let j = i + 1; j < outputs.length; j++) {
                        edges.push([i, j]);
                    }
                }
                break;
            case 'hierarchical':
                // Queen (0) connects to all workers
                for (let i = 1; i < outputs.length; i++) {
                    edges.push([0, i]);
                }
                break;
            case 'ring':
                // Ring topology
                for (let i = 0; i < outputs.length; i++) {
                    edges.push([i, (i + 1) % outputs.length]);
                }
                break;
            case 'star':
                // Star topology (central coordinator)
                const center = 0;
                for (let i = 1; i < outputs.length; i++) {
                    edges.push([center, i]);
                }
                break;
        }
        return { nodes, edges };
    }
    /**
     * Apply topology bias to attention weights
     */
    applyTopologyBias(weights, topology, graph) {
        // Agents with more connections get slight boost
        const connectionCounts = new Array(weights.length).fill(0);
        for (const [src, dst] of graph.edges) {
            connectionCounts[src]++;
            connectionCounts[dst]++;
        }
        const maxConnections = Math.max(...connectionCounts);
        return weights.map((w, i) => {
            const connectivityBoost = connectionCounts[i] / maxConnections;
            return w * (1 + connectivityBoost * 0.1); // 10% boost for well-connected agents
        });
    }
}
/**
 * Create attention coordinator from attention service
 */
export function createAttentionCoordinator(attentionService) {
    return new AttentionCoordinator(attentionService);
}
//# sourceMappingURL=attention-coordinator.js.map