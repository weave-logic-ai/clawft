/**
 * Supabase Database Adapter for Federation Hub
 *
 * Provides PostgreSQL backend using Supabase for:
 * - Hub persistence
 * - Agent metadata
 * - Memory storage (optional pgvector)
 * - Real-time subscriptions
 */
export interface SupabaseConfig {
    url: string;
    anonKey: string;
    serviceRoleKey?: string;
    vectorBackend?: 'pgvector' | 'agentdb' | 'hybrid';
    syncInterval?: number;
}
export interface AgentMemory {
    id: string;
    tenant_id: string;
    agent_id: string;
    session_id: string;
    content: string;
    embedding?: number[];
    metadata?: Record<string, any>;
    created_at?: string;
    expires_at?: string;
}
export declare class SupabaseFederationAdapter {
    private client;
    private config;
    constructor(config: SupabaseConfig);
    /**
     * Initialize Supabase schema for federation
     */
    initialize(): Promise<void>;
    /**
     * Ensure required tables exist
     */
    private ensureTables;
    /**
     * Ensure pgvector extension is enabled
     */
    private ensureVectorExtension;
    /**
     * Store agent memory in Supabase
     */
    storeMemory(memory: AgentMemory): Promise<void>;
    /**
     * Query memories by tenant and agent
     */
    queryMemories(tenantId: string, agentId?: string, limit?: number): Promise<AgentMemory[]>;
    /**
     * Semantic search using pgvector
     */
    semanticSearch(embedding: number[], tenantId: string, limit?: number): Promise<AgentMemory[]>;
    /**
     * Register agent session
     */
    registerSession(sessionId: string, tenantId: string, agentId: string, metadata?: Record<string, any>): Promise<void>;
    /**
     * Update session status
     */
    updateSessionStatus(sessionId: string, status: 'active' | 'completed' | 'failed'): Promise<void>;
    /**
     * Get active sessions for tenant
     */
    getActiveSessions(tenantId: string): Promise<any[]>;
    /**
     * Subscribe to real-time memory updates
     */
    subscribeToMemories(tenantId: string, callback: (payload: any) => void): () => void;
    /**
     * Clean up expired memories
     */
    cleanupExpiredMemories(): Promise<number>;
    /**
     * Get hub statistics
     */
    getStats(tenantId?: string): Promise<any>;
    /**
     * Close connection
     */
    close(): Promise<void>;
}
/**
 * Create Supabase adapter from environment variables
 */
export declare function createSupabaseAdapter(): SupabaseFederationAdapter;
//# sourceMappingURL=supabase-adapter.d.ts.map