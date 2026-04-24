import { QuicClient, QuicConnectionPool, QuicConnection, QuicStats } from '../transport/quic.js';
export type SwarmTopology = 'mesh' | 'hierarchical' | 'ring' | 'star';
export type AgentRole = 'coordinator' | 'worker' | 'aggregator' | 'validator';
export interface SwarmAgent {
    id: string;
    role: AgentRole;
    host: string;
    port: number;
    capabilities: string[];
    metadata?: Record<string, any>;
}
export interface SwarmMessage {
    id: string;
    from: string;
    to: string | string[];
    type: 'task' | 'result' | 'state' | 'heartbeat' | 'sync';
    payload: any;
    timestamp: number;
    ttl?: number;
}
export interface SwarmState {
    swarmId: string;
    topology: SwarmTopology;
    agents: Map<string, SwarmAgent>;
    connections: Map<string, QuicConnection>;
    stats: SwarmStats;
}
export interface SwarmStats {
    totalAgents: number;
    activeAgents: number;
    totalMessages: number;
    messagesPerSecond: number;
    averageLatency: number;
    quicStats: QuicStats;
}
export interface QuicCoordinatorConfig {
    swarmId: string;
    topology: SwarmTopology;
    maxAgents: number;
    quicClient: QuicClient;
    connectionPool: QuicConnectionPool;
    heartbeatInterval?: number;
    statesSyncInterval?: number;
    enableCompression?: boolean;
}
/**
 * QuicCoordinator - Manages multi-agent swarm coordination over QUIC
 *
 * Features:
 * - Agent-to-agent communication via QUIC streams
 * - Topology-aware message routing (mesh, hierarchical, ring, star)
 * - Connection pooling for efficient resource usage
 * - Real-time state synchronization
 * - Per-agent statistics tracking
 */
export declare class QuicCoordinator {
    private config;
    private state;
    private messageQueue;
    private heartbeatTimer?;
    private syncTimer?;
    private messageStats;
    constructor(config: QuicCoordinatorConfig);
    /**
     * Start the coordinator (heartbeat and state sync)
     */
    start(): Promise<void>;
    /**
     * Stop the coordinator
     */
    stop(): Promise<void>;
    /**
     * Register an agent in the swarm
     */
    registerAgent(agent: SwarmAgent): Promise<void>;
    /**
     * Unregister an agent from the swarm
     */
    unregisterAgent(agentId: string): Promise<void>;
    /**
     * Send message to one or more agents
     */
    sendMessage(message: SwarmMessage): Promise<void>;
    /**
     * Broadcast message to all agents (except sender)
     */
    broadcast(message: Omit<SwarmMessage, 'to'>): Promise<void>;
    /**
     * Get current swarm state
     */
    getState(): Promise<SwarmState>;
    /**
     * Get agent statistics
     */
    getAgentStats(agentId: string): {
        sent: number;
        received: number;
        avgLatency: number;
    } | null;
    /**
     * Get all agent statistics
     */
    getAllAgentStats(): Map<string, {
        sent: number;
        received: number;
        avgLatency: number;
    }>;
    /**
     * Synchronize state across all agents
     */
    syncState(): Promise<void>;
    /**
     * Establish topology-specific connections
     */
    private establishTopologyConnections;
    /**
     * Resolve message recipients based on topology
     */
    private resolveRecipients;
    /**
     * Apply topology-specific routing rules
     */
    private applyTopologyRouting;
    /**
     * Send message to specific agent
     */
    private sendToAgent;
    /**
     * Serialize message to bytes
     */
    private serializeMessage;
    /**
     * Start heartbeat timer
     */
    private startHeartbeat;
    /**
     * Start state sync timer
     */
    private startStateSync;
    /**
     * Update message statistics
     */
    private updateMessageStats;
    /**
     * Calculate current swarm statistics
     */
    private calculateStats;
}
//# sourceMappingURL=quic-coordinator.d.ts.map