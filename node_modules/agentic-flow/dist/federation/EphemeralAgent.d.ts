/**
 * Ephemeral Agent - Short-lived agent with federated memory access
 *
 * Features:
 * - Automatic lifecycle management (spawn → execute → learn → destroy)
 * - Federated memory sync via QUIC
 * - Tenant isolation with JWT authentication
 * - Memory persistence after agent destruction
 */
type AgentDB = any;
export interface EphemeralAgentConfig {
    tenantId: string;
    lifetime?: number;
    hubEndpoint?: string;
    memoryPath?: string;
    enableEncryption?: boolean;
    syncInterval?: number;
}
export interface AgentContext {
    agentId: string;
    tenantId: string;
    db: AgentDB;
    spawnTime: number;
    expiresAt: number;
}
export declare class EphemeralAgent {
    private config;
    private context?;
    private hub?;
    private security;
    private cleanupTimer?;
    private syncTimer?;
    constructor(config: EphemeralAgentConfig);
    /**
     * Spawn a new ephemeral agent with federated memory access
     */
    static spawn(config: EphemeralAgentConfig): Promise<EphemeralAgent>;
    /**
     * Initialize agent: setup DB, connect to hub, start lifecycle timers
     */
    private initialize;
    /**
     * Execute a task within the agent context
     * Automatically syncs memory before and after execution
     */
    execute<T>(task: (db: AgentDB, context: AgentContext) => Promise<T>): Promise<T>;
    /**
     * Query memories from federated database
     */
    queryMemories(task: string, k?: number): Promise<any[]>;
    /**
     * Store a learning episode to persistent memory
     */
    storeEpisode(episode: {
        task: string;
        input: string;
        output: string;
        reward: number;
        critique?: string;
    }): Promise<void>;
    /**
     * Sync local memory with federation hub
     */
    private syncWithHub;
    /**
     * Get remaining lifetime in seconds
     */
    getRemainingLifetime(): number;
    /**
     * Destroy agent and cleanup resources
     * Memory persists in federation hub
     */
    destroy(): Promise<void>;
    /**
     * Check if agent is still alive
     */
    isAlive(): boolean;
    /**
     * Get agent info
     */
    getInfo(): AgentContext | null;
}
export {};
//# sourceMappingURL=EphemeralAgent.d.ts.map