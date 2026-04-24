/**
 * Attention-Based Multi-Agent Coordination
 *
 * Uses attention mechanisms for intelligent agent consensus, task routing,
 * and topology-aware coordination in multi-agent swarms.
 *
 * @module attention-coordinator
 * @version 2.0.0-alpha
 */
import type { AttentionType, GraphContext } from '../types/agentdb.js';
/**
 * Agent output with embedding
 */
export interface AgentOutput {
    agentId: string;
    agentType: string;
    embedding: Float32Array;
    value: any;
    confidence?: number;
    metadata?: Record<string, any>;
}
/**
 * Specialized agent with domain expertise
 */
export interface SpecializedAgent {
    id: string;
    type: string;
    specialization: Float32Array;
    capabilities: string[];
    load?: number;
}
/**
 * Task with embedding
 */
export interface Task {
    id: string;
    embedding: Float32Array;
    description: string;
    requirements?: string[];
}
/**
 * Swarm topology types
 */
export type SwarmTopology = 'mesh' | 'hierarchical' | 'ring' | 'star';
/**
 * Coordination result
 */
export interface CoordinationResult {
    consensus: any;
    attentionWeights: number[];
    mechanism: AttentionType;
    executionTimeMs: number;
    topAgents: string[];
}
/**
 * Expert routing result
 */
export interface ExpertRoutingResult {
    selectedExperts: SpecializedAgent[];
    routingScores: number[];
    mechanism: 'moe' | 'attention';
    executionTimeMs: number;
}
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
export declare class AttentionCoordinator {
    private attentionService;
    constructor(attentionService: any);
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
    coordinateAgents(agentOutputs: AgentOutput[], mechanism?: AttentionType): Promise<CoordinationResult>;
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
    routeToExperts(task: Task, agents: SpecializedAgent[], topK?: number): Promise<ExpertRoutingResult>;
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
    topologyAwareCoordination(agentOutputs: AgentOutput[], topology: SwarmTopology, graphStructure?: GraphContext): Promise<CoordinationResult>;
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
    hierarchicalCoordination(queenOutputs: AgentOutput[], workerOutputs: AgentOutput[], curvature?: number): Promise<CoordinationResult>;
    /**
     * Stack multiple embeddings into single tensor
     */
    private stackEmbeddings;
    /**
     * Extract attention weights from attention output
     */
    private extractAttentionWeights;
    /**
     * Extract routing scores for expert selection
     */
    private extractRoutingScores;
    /**
     * Compute weighted consensus from agent outputs
     */
    private weightedConsensus;
    /**
     * Build graph structure from swarm topology
     */
    private buildTopologyGraph;
    /**
     * Apply topology bias to attention weights
     */
    private applyTopologyBias;
}
/**
 * Create attention coordinator from attention service
 */
export declare function createAttentionCoordinator(attentionService: any): AttentionCoordinator;
//# sourceMappingURL=attention-coordinator.d.ts.map