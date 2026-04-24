/**
 * E2B Swarm Orchestrator - Multi-agent swarm with E2B sandbox isolation
 *
 * Spawns specialized agents in isolated E2B sandboxes for:
 * - Parallel code execution
 * - Secure multi-agent coordination
 * - Resource-isolated task processing
 */
import { E2BSandboxManager } from "./e2b-sandbox.js";
/**
 * E2B Agent capabilities
 */
export type E2BAgentCapability = 'python-executor' | 'javascript-executor' | 'shell-executor' | 'data-analyst' | 'code-reviewer' | 'test-runner' | 'security-scanner' | 'performance-profiler' | 'ml-trainer' | 'api-tester';
/**
 * E2B Agent configuration
 */
export interface E2BAgentConfig {
    id: string;
    name: string;
    capability: E2BAgentCapability;
    template?: string;
    timeout?: number;
    envVars?: Record<string, string>;
    packages?: string[];
}
/**
 * E2B Agent instance
 */
export interface E2BAgent {
    id: string;
    name: string;
    capability: E2BAgentCapability;
    sandbox: E2BSandboxManager;
    status: 'initializing' | 'ready' | 'busy' | 'error' | 'terminated';
    tasksCompleted: number;
    totalExecutionTime: number;
    errors: number;
    createdAt: number;
}
/**
 * Task for E2B agent execution
 */
export interface E2BTask {
    id: string;
    type: 'python' | 'javascript' | 'shell' | 'file-write' | 'file-read';
    code: string;
    targetAgent?: string;
    capability?: E2BAgentCapability;
    priority?: 'low' | 'medium' | 'high' | 'critical';
    timeout?: number;
    metadata?: Record<string, any>;
}
/**
 * Task result
 */
export interface E2BTaskResult {
    taskId: string;
    agentId: string;
    success: boolean;
    output: string;
    error?: string;
    executionTime: number;
    metadata?: Record<string, any>;
}
/**
 * Swarm metrics
 */
export interface E2BSwarmMetrics {
    totalAgents: number;
    activeAgents: number;
    tasksCompleted: number;
    tasksInProgress: number;
    totalExecutionTime: number;
    averageExecutionTime: number;
    errorRate: number;
    agentUtilization: Record<string, number>;
}
/**
 * E2B Swarm Orchestrator
 */
export declare class E2BSwarmOrchestrator {
    private agents;
    private taskQueue;
    private taskResults;
    private isRunning;
    private config;
    constructor(config?: Partial<E2BSwarmOrchestrator['config']>);
    /**
     * Initialize the swarm
     */
    initialize(): Promise<boolean>;
    /**
     * Spawn a new E2B agent with specific capability
     */
    spawnAgent(config: E2BAgentConfig): Promise<E2BAgent | null>;
    /**
     * Spawn multiple agents in parallel
     */
    spawnAgents(configs: E2BAgentConfig[]): Promise<E2BAgent[]>;
    /**
     * Execute a task on the swarm
     */
    executeTask(task: E2BTask): Promise<E2BTaskResult>;
    /**
     * Execute multiple tasks in parallel
     */
    executeTasks(tasks: E2BTask[]): Promise<E2BTaskResult[]>;
    /**
     * Select best agent for a task
     */
    private selectAgent;
    /**
     * Get ready agents
     */
    private getReadyAgents;
    /**
     * Get template for capability
     */
    private getTemplateForCapability;
    /**
     * Get swarm metrics
     */
    getMetrics(): E2BSwarmMetrics;
    /**
     * Get all agents
     */
    getAgents(): E2BAgent[];
    /**
     * Get agent by ID
     */
    getAgent(id: string): E2BAgent | undefined;
    /**
     * Terminate an agent
     */
    terminateAgent(id: string): Promise<boolean>;
    /**
     * Shutdown the swarm
     */
    shutdown(): Promise<void>;
    /**
     * Health check
     */
    healthCheck(): Promise<{
        healthy: boolean;
        agents: Array<{
            id: string;
            status: string;
            healthy: boolean;
        }>;
    }>;
}
/**
 * Create default swarm with standard agent types
 */
export declare function createDefaultE2BSwarm(): Promise<E2BSwarmOrchestrator>;
/**
 * Quick helper to run code in E2B swarm
 */
export declare function runInSwarm(swarm: E2BSwarmOrchestrator, code: string, type?: 'python' | 'javascript' | 'shell'): Promise<E2BTaskResult>;
//# sourceMappingURL=e2b-swarm.d.ts.map