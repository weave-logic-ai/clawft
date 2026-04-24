/**
 * Supabase Database Adapter with Debug Streaming
 *
 * Enhanced version of SupabaseFederationAdapter with comprehensive
 * debug logging and performance tracking.
 */
import { DebugStream, DebugLevel } from '../debug/debug-stream.js';
export interface SupabaseConfigDebug {
    url: string;
    anonKey: string;
    serviceRoleKey?: string;
    vectorBackend?: 'pgvector' | 'agentdb' | 'hybrid';
    syncInterval?: number;
    debug?: {
        enabled?: boolean;
        level?: DebugLevel;
        output?: 'console' | 'file' | 'both';
        format?: 'human' | 'json' | 'compact';
        outputFile?: string;
    };
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
export declare class SupabaseFederationAdapterDebug {
    private client;
    private config;
    private debug;
    constructor(config: SupabaseConfigDebug);
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
     * Get hub statistics
     */
    getStats(tenantId?: string): Promise<any>;
    /**
     * Get debug stream for external use
     */
    getDebugStream(): DebugStream;
    /**
     * Print performance metrics
     */
    printMetrics(): void;
    /**
     * Close connection
     */
    close(): Promise<void>;
}
/**
 * Create Supabase adapter from environment variables with debug support
 */
export declare function createSupabaseAdapterDebug(): SupabaseFederationAdapterDebug;
//# sourceMappingURL=supabase-adapter-debug.d.ts.map