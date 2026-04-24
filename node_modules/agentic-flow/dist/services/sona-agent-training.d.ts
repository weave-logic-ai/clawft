/**
 * SONA Agent Training Service
 *
 * Train specialized models for specific agents, tasks, and codebases
 * Uses @ruvector/sona for continuous learning and adaptation
 */
import { EventEmitter } from 'events';
export interface AgentConfig {
    name: string;
    purpose: 'simple' | 'complex' | 'diverse';
    hiddenDim?: number;
    microLoraRank?: number;
    baseLoraRank?: number;
    patternClusters?: number;
    trajectoryCapacity?: number;
    qualityThreshold?: number;
    ewcLambda?: number;
    route?: string;
}
export interface TrainingExample {
    embedding: number[];
    prompt?: string;
    output?: string;
    hiddenStates?: number[];
    attention?: number[];
    quality: number;
    context?: Record<string, any>;
}
export interface CodebaseFile {
    path: string;
    language: string;
    content: string;
    chunks?: Array<{
        code: string;
        type: 'function' | 'class' | 'interface' | 'module';
        embedding?: number[];
    }>;
}
export interface AgentStats {
    name: string;
    purpose: string;
    trainingCount: number;
    avgQuality: number;
    patterns: number;
    lastTrained?: Date;
    config: any;
}
/**
 * SONA Agent Factory - Create and manage specialized agents
 */
export declare class AgentFactory extends EventEmitter {
    private agents;
    private baseConfig;
    constructor(baseConfig?: Partial<AgentConfig>);
    /**
     * Create a new specialized agent
     */
    createAgent(name: string, config?: Partial<AgentConfig>): any;
    /**
     * Train an agent on specific examples
     */
    trainAgent(name: string, examples: TrainingExample[]): Promise<number>;
    /**
     * Get an agent's engine for inference
     */
    getAgent(name: string): any;
    /**
     * Get agent statistics
     */
    getAgentStats(name: string): AgentStats | null;
    /**
     * List all agents
     */
    listAgents(): AgentStats[];
    /**
     * Find similar patterns for a query
     */
    findPatterns(agentName: string, queryEmbedding: number[], k?: number): Promise<any[]>;
    /**
     * Apply agent-specific adaptation to embedding
     */
    applyAdaptation(agentName: string, embedding: number[]): Promise<number[]>;
}
/**
 * Codebase-Specific Agent Trainer
 */
export declare class CodebaseTrainer {
    private engine;
    private indexed;
    constructor(config?: Partial<AgentConfig>);
    /**
     * Index an entire codebase for pattern learning
     */
    indexCodebase(files: CodebaseFile[]): Promise<number>;
    /**
     * Query with codebase-aware adaptation
     */
    query(queryText: string, k?: number): Promise<{
        adapted: number[];
        relevantPatterns: any[];
    }>;
    /**
     * Get codebase statistics
     */
    getStats(): any;
    /**
     * Chunk code into trainable segments
     */
    private chunkCode;
    /**
     * Find end of code block (simplified)
     */
    private findBlockEnd;
    /**
     * Mock embedding (replace with actual embedding service in production)
     */
    private mockEmbedding;
    private hashCode;
}
/**
 * Pre-configured agent templates
 */
export declare const AgentTemplates: {
    /**
     * Code Assistant - Complex reasoning, code-specific patterns
     */
    codeAssistant: () => AgentConfig;
    /**
     * Chat Agent - Simple conversational patterns
     */
    chatBot: () => AgentConfig;
    /**
     * Data Analyst - Diverse data patterns
     */
    dataAnalyst: () => AgentConfig;
    /**
     * RAG Agent - Large capacity for document retrieval
     */
    ragAgent: () => AgentConfig;
    /**
     * Task Planner - Complex reasoning with strong memory
     */
    taskPlanner: () => AgentConfig;
    /**
     * Domain Expert - Learns specific domain
     */
    domainExpert: (domain: string) => AgentConfig;
};
//# sourceMappingURL=sona-agent-training.d.ts.map