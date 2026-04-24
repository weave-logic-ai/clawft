/**
 * Supabase Database Adapter with Debug Streaming
 *
 * Enhanced version of SupabaseFederationAdapter with comprehensive
 * debug logging and performance tracking.
 */
import { createClient } from '@supabase/supabase-js';
import { DebugLevel, createDebugStream } from '../debug/debug-stream.js';
export class SupabaseFederationAdapterDebug {
    client;
    config;
    debug;
    constructor(config) {
        this.config = config;
        // Initialize debug stream
        this.debug = createDebugStream({
            level: config.debug?.level ?? DebugLevel.BASIC,
            output: config.debug?.output ?? 'console',
            format: config.debug?.format ?? 'human',
            outputFile: config.debug?.outputFile,
        });
        const startTime = Date.now();
        // Use service role key for server-side operations
        const key = config.serviceRoleKey || config.anonKey;
        this.client = createClient(config.url, key);
        this.debug.logConnection('client_created', {
            url: config.url,
            hasServiceRole: !!config.serviceRoleKey,
            vectorBackend: config.vectorBackend,
        });
        const duration = Date.now() - startTime;
        this.debug.logTrace('constructor_complete', { duration });
    }
    /**
     * Initialize Supabase schema for federation
     */
    async initialize() {
        const startTime = Date.now();
        this.debug.logConnection('initialize_start', {
            vectorBackend: this.config.vectorBackend,
        });
        try {
            // Check if tables exist
            await this.ensureTables();
            if (this.config.vectorBackend === 'pgvector') {
                await this.ensureVectorExtension();
            }
            const duration = Date.now() - startTime;
            this.debug.logConnection('initialize_complete', { duration }, duration);
        }
        catch (error) {
            const duration = Date.now() - startTime;
            this.debug.logConnection('initialize_error', { duration }, duration, error);
            throw error;
        }
    }
    /**
     * Ensure required tables exist
     */
    async ensureTables() {
        const startTime = Date.now();
        this.debug.logTrace('checking_tables');
        const tables = ['agent_sessions', 'agent_memories', 'agent_tasks', 'agent_events'];
        const results = {};
        for (const table of tables) {
            const tableStart = Date.now();
            try {
                const { data, error } = await this.client
                    .from(table)
                    .select('id')
                    .limit(1);
                const exists = !error || error.code !== 'PGRST116';
                results[table] = exists;
                const tableDuration = Date.now() - tableStart;
                this.debug.logDatabase('table_check', {
                    table,
                    exists,
                }, tableDuration);
                if (!exists) {
                    this.debug.logDatabase('table_missing', { table });
                }
            }
            catch (error) {
                const tableDuration = Date.now() - tableStart;
                this.debug.logDatabase('table_check_error', { table }, tableDuration, error);
            }
        }
        const duration = Date.now() - startTime;
        this.debug.logDatabase('tables_checked', { results }, duration);
    }
    /**
     * Ensure pgvector extension is enabled
     */
    async ensureVectorExtension() {
        const startTime = Date.now();
        this.debug.logTrace('checking_pgvector');
        try {
            const { error } = await this.client.rpc('exec_sql', {
                sql: 'CREATE EXTENSION IF NOT EXISTS vector;'
            });
            const duration = Date.now() - startTime;
            if (error) {
                this.debug.logDatabase('pgvector_check_failed', {
                    message: error.message,
                }, duration, error);
            }
            else {
                this.debug.logDatabase('pgvector_ready', {}, duration);
            }
        }
        catch (err) {
            const duration = Date.now() - startTime;
            this.debug.logDatabase('pgvector_error', {}, duration, err);
        }
    }
    /**
     * Store agent memory in Supabase
     */
    async storeMemory(memory) {
        const startTime = Date.now();
        this.debug.logMemory('store_start', memory.agent_id, memory.tenant_id, {
            id: memory.id,
            content_length: memory.content.length,
            has_embedding: !!memory.embedding,
            embedding_dims: memory.embedding?.length,
        });
        try {
            const { error } = await this.client
                .from('agent_memories')
                .insert({
                id: memory.id,
                tenant_id: memory.tenant_id,
                agent_id: memory.agent_id,
                session_id: memory.session_id,
                content: memory.content,
                embedding: memory.embedding,
                metadata: memory.metadata,
                created_at: memory.created_at || new Date().toISOString(),
                expires_at: memory.expires_at,
            });
            const duration = Date.now() - startTime;
            if (error) {
                this.debug.logMemory('store_error', memory.agent_id, memory.tenant_id, {
                    error: error.message,
                }, duration);
                throw new Error(`Failed to store memory: ${error.message}`);
            }
            this.debug.logMemory('store_complete', memory.agent_id, memory.tenant_id, {
                id: memory.id,
            }, duration);
        }
        catch (error) {
            const duration = Date.now() - startTime;
            this.debug.logMemory('store_failed', memory.agent_id, memory.tenant_id, {}, duration);
            throw error;
        }
    }
    /**
     * Query memories by tenant and agent
     */
    async queryMemories(tenantId, agentId, limit = 100) {
        const startTime = Date.now();
        this.debug.logMemory('query_start', agentId, tenantId, {
            limit,
            hasAgentFilter: !!agentId,
        });
        try {
            let query = this.client
                .from('agent_memories')
                .select('*')
                .eq('tenant_id', tenantId)
                .order('created_at', { ascending: false })
                .limit(limit);
            if (agentId) {
                query = query.eq('agent_id', agentId);
            }
            const { data, error } = await query;
            const duration = Date.now() - startTime;
            if (error) {
                this.debug.logMemory('query_error', agentId, tenantId, {
                    error: error.message,
                }, duration);
                throw new Error(`Failed to query memories: ${error.message}`);
            }
            this.debug.logMemory('query_complete', agentId, tenantId, {
                count: data?.length || 0,
            }, duration);
            return data;
        }
        catch (error) {
            const duration = Date.now() - startTime;
            this.debug.logMemory('query_failed', agentId, tenantId, {}, duration);
            throw error;
        }
    }
    /**
     * Semantic search using pgvector
     */
    async semanticSearch(embedding, tenantId, limit = 10) {
        const startTime = Date.now();
        this.debug.logMemory('semantic_search_start', undefined, tenantId, {
            embedding_dims: embedding.length,
            limit,
        });
        if (this.config.vectorBackend !== 'pgvector') {
            this.debug.logMemory('semantic_search_disabled', undefined, tenantId, {
                backend: this.config.vectorBackend,
            });
            throw new Error('pgvector backend not enabled');
        }
        try {
            const { data, error } = await this.client.rpc('search_memories', {
                query_embedding: embedding,
                query_tenant_id: tenantId,
                match_count: limit,
            });
            const duration = Date.now() - startTime;
            if (error) {
                this.debug.logMemory('semantic_search_error', undefined, tenantId, {
                    error: error.message,
                }, duration);
                throw new Error(`Semantic search failed: ${error.message}`);
            }
            this.debug.logMemory('semantic_search_complete', undefined, tenantId, {
                results: data?.length || 0,
            }, duration);
            return data;
        }
        catch (error) {
            const duration = Date.now() - startTime;
            this.debug.logMemory('semantic_search_failed', undefined, tenantId, {}, duration);
            throw error;
        }
    }
    /**
     * Register agent session
     */
    async registerSession(sessionId, tenantId, agentId, metadata) {
        const startTime = Date.now();
        this.debug.logDatabase('register_session_start', {
            sessionId,
            tenantId,
            agentId,
        });
        try {
            const { error } = await this.client
                .from('agent_sessions')
                .insert({
                session_id: sessionId,
                tenant_id: tenantId,
                agent_id: agentId,
                metadata,
                started_at: new Date().toISOString(),
                status: 'active',
            });
            const duration = Date.now() - startTime;
            if (error) {
                this.debug.logDatabase('register_session_error', {
                    sessionId,
                    error: error.message,
                }, duration, error);
                throw new Error(`Failed to register session: ${error.message}`);
            }
            this.debug.logDatabase('register_session_complete', {
                sessionId,
            }, duration);
        }
        catch (error) {
            const duration = Date.now() - startTime;
            this.debug.logDatabase('register_session_failed', { sessionId }, duration);
            throw error;
        }
    }
    /**
     * Update session status
     */
    async updateSessionStatus(sessionId, status) {
        const startTime = Date.now();
        this.debug.logDatabase('update_session_start', {
            sessionId,
            status,
        });
        try {
            const updates = { status };
            if (status !== 'active') {
                updates.ended_at = new Date().toISOString();
            }
            const { error } = await this.client
                .from('agent_sessions')
                .update(updates)
                .eq('session_id', sessionId);
            const duration = Date.now() - startTime;
            if (error) {
                this.debug.logDatabase('update_session_error', {
                    sessionId,
                    error: error.message,
                }, duration, error);
                throw new Error(`Failed to update session: ${error.message}`);
            }
            this.debug.logDatabase('update_session_complete', {
                sessionId,
                status,
            }, duration);
        }
        catch (error) {
            const duration = Date.now() - startTime;
            this.debug.logDatabase('update_session_failed', { sessionId }, duration);
            throw error;
        }
    }
    /**
     * Get hub statistics
     */
    async getStats(tenantId) {
        const startTime = Date.now();
        this.debug.logDatabase('get_stats_start', { tenantId });
        try {
            // Total memories
            let memoriesQuery = this.client
                .from('agent_memories')
                .select('id', { count: 'exact', head: true });
            if (tenantId) {
                memoriesQuery = memoriesQuery.eq('tenant_id', tenantId);
            }
            const { count: totalMemories } = await memoriesQuery;
            // Active sessions
            let sessionsQuery = this.client
                .from('agent_sessions')
                .select('session_id', { count: 'exact', head: true })
                .eq('status', 'active');
            if (tenantId) {
                sessionsQuery = sessionsQuery.eq('tenant_id', tenantId);
            }
            const { count: activeSessions } = await sessionsQuery;
            const duration = Date.now() - startTime;
            const stats = {
                total_memories: totalMemories || 0,
                active_sessions: activeSessions || 0,
                backend: 'supabase',
                vector_backend: this.config.vectorBackend,
                timestamp: new Date().toISOString(),
            };
            this.debug.logDatabase('get_stats_complete', stats, duration);
            return stats;
        }
        catch (error) {
            const duration = Date.now() - startTime;
            this.debug.logDatabase('get_stats_error', {}, duration, error);
            throw error;
        }
    }
    /**
     * Get debug stream for external use
     */
    getDebugStream() {
        return this.debug;
    }
    /**
     * Print performance metrics
     */
    printMetrics() {
        this.debug.printMetrics();
    }
    /**
     * Close connection
     */
    async close() {
        const startTime = Date.now();
        this.debug.logConnection('close_start');
        this.debug.close();
        const duration = Date.now() - startTime;
        this.debug.logConnection('close_complete', {}, duration);
    }
}
/**
 * Create Supabase adapter from environment variables with debug support
 */
export function createSupabaseAdapterDebug() {
    const url = process.env.SUPABASE_URL;
    const anonKey = process.env.SUPABASE_ANON_KEY;
    const serviceRoleKey = process.env.SUPABASE_SERVICE_ROLE_KEY;
    if (!url || !anonKey) {
        throw new Error('Missing Supabase credentials. Set SUPABASE_URL and SUPABASE_ANON_KEY');
    }
    const debugLevel = process.env.DEBUG_LEVEL?.toUpperCase() || 'BASIC';
    return new SupabaseFederationAdapterDebug({
        url,
        anonKey,
        serviceRoleKey,
        vectorBackend: process.env.FEDERATION_VECTOR_BACKEND || 'hybrid',
        syncInterval: parseInt(process.env.FEDERATION_SYNC_INTERVAL || '60000'),
        debug: {
            enabled: process.env.DEBUG_ENABLED !== 'false',
            level: DebugLevel[debugLevel] || DebugLevel.BASIC,
            output: process.env.DEBUG_OUTPUT || 'console',
            format: process.env.DEBUG_FORMAT || 'human',
            outputFile: process.env.DEBUG_OUTPUT_FILE,
        },
    });
}
//# sourceMappingURL=supabase-adapter-debug.js.map