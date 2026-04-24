/**
 * Single Agent Debug Streaming
 *
 * Provides detailed visibility into a single agent's operations,
 * lifecycle, and internal state changes.
 *
 * Features:
 * - Agent lifecycle tracking (spawn → work → shutdown)
 * - Task execution tracing
 * - Memory operations
 * - Communication events
 * - Decision logging
 * - State transitions
 * - Performance metrics
 * - Timeline visualization
 */
import { EventEmitter } from 'events';
import { DebugLevel } from './debug-stream.js';
export { DebugLevel } from './debug-stream.js';
export interface AgentDebugConfig {
    agentId: string;
    tenantId?: string;
    level?: DebugLevel;
    format?: 'human' | 'json' | 'compact' | 'timeline';
    output?: 'console' | 'file' | 'both';
    outputFile?: string;
    colorize?: boolean;
    trackState?: boolean;
    trackDecisions?: boolean;
    trackCommunication?: boolean;
    timeline?: boolean;
}
export interface AgentState {
    phase: 'spawning' | 'initializing' | 'ready' | 'working' | 'idle' | 'shutting_down' | 'dead';
    task?: string;
    taskId?: string;
    startTime: string;
    lastActivity?: string;
    metadata?: Record<string, any>;
}
export interface AgentDecision {
    timestamp: string;
    context: string;
    options: any[];
    selected: any;
    reasoning?: string;
    confidence?: number;
}
export interface AgentTask {
    taskId: string;
    description: string;
    startTime: string;
    endTime?: string;
    status: 'pending' | 'running' | 'completed' | 'failed';
    steps: AgentTaskStep[];
    result?: any;
    error?: Error;
}
export interface AgentTaskStep {
    step: number;
    operation: string;
    startTime: string;
    endTime?: string;
    duration?: number;
    data?: any;
    error?: Error;
}
export declare class AgentDebugStream extends EventEmitter {
    private config;
    private debug;
    private state;
    private decisions;
    private tasks;
    private communications;
    private timeline;
    private performanceMetrics;
    constructor(config: AgentDebugConfig);
    /**
     * Log agent lifecycle phase change
     */
    logAgentPhase(phase: AgentState['phase'], data?: any): void;
    /**
     * Log agent initialization
     */
    logInitialization(config: any): void;
    /**
     * Log agent ready
     */
    logReady(capabilities?: any): void;
    /**
     * Start tracking a task
     */
    startTask(taskId: string, description: string, data?: any): void;
    /**
     * Log task step
     */
    logTaskStep(taskId: string, step: number, operation: string, data?: any): void;
    /**
     * Complete task step
     */
    completeTaskStep(taskId: string, step: number, duration: number, data?: any): void;
    /**
     * Complete task
     */
    completeTask(taskId: string, result?: any): void;
    /**
     * Fail task
     */
    failTask(taskId: string, error: Error): void;
    /**
     * Log a decision
     */
    logDecision(context: string, options: any[], selected: any, reasoning?: string, confidence?: number): void;
    /**
     * Log communication
     */
    logCommunication(type: 'send' | 'receive', target: string, message: any): void;
    /**
     * Log memory operation
     */
    logMemoryOperation(operation: 'store' | 'retrieve' | 'search', data: any, duration?: number): void;
    /**
     * Log thought/reasoning
     */
    logThought(thought: string, context?: any): void;
    /**
     * Log agent shutdown
     */
    logShutdown(reason?: string): void;
    /**
     * Track performance metric
     */
    private trackPerformance;
    /**
     * Print agent summary
     */
    printSummary(): void;
    /**
     * Print timeline
     */
    printTimeline(): void;
    /**
     * Get current state
     */
    getState(): AgentState;
    /**
     * Get task history
     */
    getTasks(): AgentTask[];
    /**
     * Get decisions
     */
    getDecisions(): AgentDecision[];
    /**
     * Get communications
     */
    getCommunications(): any[];
    /**
     * Close debug stream
     */
    close(): void;
}
/**
 * Create agent debug stream
 */
export declare function createAgentDebugStream(config: AgentDebugConfig): AgentDebugStream;
/**
 * Create from environment variables
 */
export declare function createAgentDebugStreamFromEnv(agentId: string, tenantId?: string): AgentDebugStream;
//# sourceMappingURL=agent-debug-stream.d.ts.map