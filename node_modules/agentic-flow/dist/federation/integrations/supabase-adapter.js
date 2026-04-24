/**
 * Supabase Database Adapter for Federation Hub
 *
 * Provides PostgreSQL backend using Supabase for:
 * - Hub persistence
 * - Agent metadata
 * - Memory storage (optional pgvector)
 * - Real-time subscriptions
 */
import { createClient } from '@supabase/supabase-js';
export class SupabaseFederationAdapter {
    client;
    config;
    constructor(config) {
        this.config = config;
        // Use service role key for server-side operations
        const key = config.serviceRoleKey || config.anonKey;
        this.client = createClient(config.url, key);
    }
    /**
     * Initialize Supabase schema for federation
     */
    async initialize() {
        console.log('🔧 Initializing Supabase Federation Schema...');
        // Check if tables exist, create if needed
        await this.ensureTables();
        if (this.config.vectorBackend === 'pgvector') {
            await this.ensureVectorExtension();
        }
        console.log('✅ Supabase Federation Schema Ready');
    }
    /**
     * Ensure required tables exist
     */
    async ensureTables() {
        // Note: In production, use Supabase migrations
        // This is a runtime check for development
        const { data, error } = await this.client
            .from('agent_sessions')
            .select('id')
            .limit(1);
        if (error && error.code === 'PGRST116') {
            console.log('⚠️  Tables not found. Please run Supabase migrations.');
            console.log('📖 See: docs/supabase/migrations/');
        }
    }
    /**
     * Ensure pgvector extension is enabled
     */
    async ensureVectorExtension() {
        try {
            // This requires service role key with proper permissions
            const { error } = await this.client.rpc('exec_sql', {
                sql: 'CREATE EXTENSION IF NOT EXISTS vector;'
            });
            if (error) {
                console.warn('⚠️  pgvector extension check failed:', error.message);
                console.log('📖 Enable manually: CREATE EXTENSION vector;');
            }
        }
        catch (err) {
            console.warn('⚠️  Could not verify pgvector extension');
        }
    }
    /**
     * Store agent memory in Supabase
     */
    async storeMemory(memory) {
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
        if (error) {
            throw new Error(`Failed to store memory: ${error.message}`);
        }
    }
    /**
     * Query memories by tenant and agent
     */
    async queryMemories(tenantId, agentId, limit = 100) {
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
        if (error) {
            throw new Error(`Failed to query memories: ${error.message}`);
        }
        return data;
    }
    /**
     * Semantic search using pgvector
     */
    async semanticSearch(embedding, tenantId, limit = 10) {
        if (this.config.vectorBackend !== 'pgvector') {
            throw new Error('pgvector backend not enabled');
        }
        // Use pgvector cosine similarity search
        const { data, error } = await this.client.rpc('search_memories', {
            query_embedding: embedding,
            query_tenant_id: tenantId,
            match_count: limit,
        });
        if (error) {
            throw new Error(`Semantic search failed: ${error.message}`);
        }
        return data;
    }
    /**
     * Register agent session
     */
    async registerSession(sessionId, tenantId, agentId, metadata) {
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
        if (error) {
            throw new Error(`Failed to register session: ${error.message}`);
        }
    }
    /**
     * Update session status
     */
    async updateSessionStatus(sessionId, status) {
        const updates = { status };
        if (status !== 'active') {
            updates.ended_at = new Date().toISOString();
        }
        const { error } = await this.client
            .from('agent_sessions')
            .update(updates)
            .eq('session_id', sessionId);
        if (error) {
            throw new Error(`Failed to update session: ${error.message}`);
        }
    }
    /**
     * Get active sessions for tenant
     */
    async getActiveSessions(tenantId) {
        const { data, error } = await this.client
            .from('agent_sessions')
            .select('*')
            .eq('tenant_id', tenantId)
            .eq('status', 'active')
            .order('started_at', { ascending: false });
        if (error) {
            throw new Error(`Failed to get sessions: ${error.message}`);
        }
        return data || [];
    }
    /**
     * Subscribe to real-time memory updates
     */
    subscribeToMemories(tenantId, callback) {
        const subscription = this.client
            .channel(`memories:${tenantId}`)
            .on('postgres_changes', {
            event: 'INSERT',
            schema: 'public',
            table: 'agent_memories',
            filter: `tenant_id=eq.${tenantId}`,
        }, callback)
            .subscribe();
        // Return unsubscribe function
        return () => {
            subscription.unsubscribe();
        };
    }
    /**
     * Clean up expired memories
     */
    async cleanupExpiredMemories() {
        const { data, error } = await this.client
            .from('agent_memories')
            .delete()
            .lt('expires_at', new Date().toISOString())
            .select('id');
        if (error) {
            throw new Error(`Cleanup failed: ${error.message}`);
        }
        return data?.length || 0;
    }
    /**
     * Get hub statistics
     */
    async getStats(tenantId) {
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
        return {
            total_memories: totalMemories || 0,
            active_sessions: activeSessions || 0,
            backend: 'supabase',
            vector_backend: this.config.vectorBackend,
            timestamp: new Date().toISOString(),
        };
    }
    /**
     * Close connection
     */
    async close() {
        // Supabase client doesn't need explicit closing
        console.log('✅ Supabase connection closed');
    }
}
/**
 * Create Supabase adapter from environment variables
 */
export function createSupabaseAdapter() {
    const url = process.env.SUPABASE_URL;
    const anonKey = process.env.SUPABASE_ANON_KEY;
    const serviceRoleKey = process.env.SUPABASE_SERVICE_ROLE_KEY;
    if (!url || !anonKey) {
        throw new Error('Missing Supabase credentials. Set SUPABASE_URL and SUPABASE_ANON_KEY');
    }
    return new SupabaseFederationAdapter({
        url,
        anonKey,
        serviceRoleKey,
        vectorBackend: process.env.FEDERATION_VECTOR_BACKEND || 'hybrid',
        syncInterval: parseInt(process.env.FEDERATION_SYNC_INTERVAL || '60000'),
    });
}
//# sourceMappingURL=supabase-adapter.js.map