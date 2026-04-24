/**
 * Real-Time Federation System using Supabase
 *
 * Leverages Supabase Real-Time for:
 * - Live agent coordination
 * - Instant memory synchronization
 * - Real-time presence tracking
 * - Collaborative multi-agent workflows
 * - Event-driven agent communication
 */
export interface RealtimeConfig {
    url: string;
    anonKey: string;
    serviceRoleKey?: string;
    presenceHeartbeat?: number;
    memorySync?: boolean;
    broadcastLatency?: 'low' | 'high';
}
export interface AgentPresence {
    agent_id: string;
    tenant_id: string;
    status: 'online' | 'busy' | 'offline';
    task?: string;
    started_at: string;
    last_heartbeat: string;
    metadata?: Record<string, any>;
}
export interface MemoryEvent {
    type: 'memory_added' | 'memory_updated' | 'memory_retrieved';
    tenant_id: string;
    agent_id: string;
    session_id: string;
    content: string;
    embedding?: number[];
    metadata?: Record<string, any>;
    timestamp: string;
}
export interface CoordinationMessage {
    from_agent: string;
    to_agent?: string;
    type: 'task_assignment' | 'task_complete' | 'request_help' | 'share_knowledge' | 'status_update';
    payload: any;
    timestamp: string;
}
export interface TaskAssignment {
    task_id: string;
    assigned_to: string;
    description: string;
    priority: 'low' | 'medium' | 'high' | 'critical';
    deadline?: string;
    dependencies?: string[];
}
export declare class RealtimeFederationHub {
    private client;
    private config;
    private channels;
    private presenceChannel?;
    private agentId;
    private tenantId;
    private heartbeatInterval?;
    private eventHandlers;
    constructor(config: RealtimeConfig, agentId: string, tenantId?: string);
    /**
     * Initialize real-time federation
     */
    initialize(): Promise<void>;
    /**
     * Set up presence tracking for all agents in tenant
     */
    private setupPresence;
    /**
     * Set up real-time memory synchronization
     */
    private setupMemorySync;
    /**
     * Set up coordination channel for agent-to-agent communication
     */
    private setupCoordination;
    /**
     * Broadcast message to all agents in tenant
     */
    broadcast(type: CoordinationMessage['type'], payload: any): Promise<void>;
    /**
     * Send direct message to specific agent
     */
    sendMessage(toAgent: string, type: CoordinationMessage['type'], payload: any): Promise<void>;
    /**
     * Assign task to another agent
     */
    assignTask(task: TaskAssignment): Promise<void>;
    /**
     * Report task completion
     */
    reportTaskComplete(taskId: string, result: any): Promise<void>;
    /**
     * Request help from other agents
     */
    requestHelp(problem: string, context?: any): Promise<void>;
    /**
     * Share knowledge with other agents
     */
    shareKnowledge(knowledge: string, metadata?: any): Promise<void>;
    /**
     * Update agent status
     */
    updateStatus(status: 'online' | 'busy' | 'offline', task?: string): Promise<void>;
    /**
     * Get list of active agents
     */
    getActiveAgents(presenceState?: any): AgentPresence[];
    /**
     * Start heartbeat to maintain presence
     */
    private startHeartbeat;
    /**
     * Stop heartbeat
     */
    private stopHeartbeat;
    /**
     * Subscribe to events
     */
    on(event: string, handler: Function): void;
    /**
     * Unsubscribe from events
     */
    off(event: string, handler: Function): void;
    /**
     * Emit event to handlers
     */
    private emit;
    /**
     * Get real-time statistics
     */
    getStats(): Promise<any>;
    /**
     * Shutdown and cleanup
     */
    shutdown(): Promise<void>;
}
/**
 * Create real-time federation hub from environment
 */
export declare function createRealtimeHub(agentId: string, tenantId?: string): RealtimeFederationHub;
//# sourceMappingURL=realtime-federation.d.ts.map