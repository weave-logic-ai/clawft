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
import { createClient } from '@supabase/supabase-js';
export class RealtimeFederationHub {
    client;
    config;
    channels = new Map();
    presenceChannel;
    agentId;
    tenantId;
    heartbeatInterval;
    eventHandlers = new Map();
    constructor(config, agentId, tenantId = 'default') {
        this.config = config;
        this.agentId = agentId;
        this.tenantId = tenantId;
        const key = config.serviceRoleKey || config.anonKey;
        this.client = createClient(config.url, key, {
            realtime: {
                params: {
                    eventsPerSecond: config.broadcastLatency === 'low' ? 10 : 2,
                },
            },
        });
    }
    /**
     * Initialize real-time federation
     */
    async initialize() {
        console.log('🌐 Initializing Real-Time Federation Hub...');
        console.log(`   Agent: ${this.agentId}`);
        console.log(`   Tenant: ${this.tenantId}`);
        // Set up presence tracking
        await this.setupPresence();
        // Set up memory sync channel
        if (this.config.memorySync !== false) {
            await this.setupMemorySync();
        }
        // Set up coordination channel
        await this.setupCoordination();
        // Start heartbeat
        this.startHeartbeat();
        console.log('✅ Real-Time Federation Hub Active');
    }
    /**
     * Set up presence tracking for all agents in tenant
     */
    async setupPresence() {
        const channelName = `presence:${this.tenantId}`;
        this.presenceChannel = this.client.channel(channelName, {
            config: {
                presence: {
                    key: this.agentId,
                },
            },
        });
        // Track agent join
        this.presenceChannel.on('presence', { event: 'join' }, ({ key, newPresences }) => {
            console.log(`🟢 Agent joined: ${key}`);
            this.emit('agent:join', { agent_id: key, presences: newPresences });
        });
        // Track agent leave
        this.presenceChannel.on('presence', { event: 'leave' }, ({ key, leftPresences }) => {
            console.log(`🔴 Agent left: ${key}`);
            this.emit('agent:leave', { agent_id: key, presences: leftPresences });
        });
        // Track agent sync (periodic updates)
        this.presenceChannel.on('presence', { event: 'sync' }, () => {
            const state = this.presenceChannel.presenceState();
            this.emit('agents:sync', { agents: this.getActiveAgents(state) });
        });
        // Subscribe and track presence
        await this.presenceChannel.subscribe(async (status) => {
            if (status === 'SUBSCRIBED') {
                await this.presenceChannel.track({
                    agent_id: this.agentId,
                    tenant_id: this.tenantId,
                    status: 'online',
                    started_at: new Date().toISOString(),
                    last_heartbeat: new Date().toISOString(),
                });
            }
        });
    }
    /**
     * Set up real-time memory synchronization
     */
    async setupMemorySync() {
        const channelName = `memories:${this.tenantId}`;
        const channel = this.client.channel(channelName);
        // Listen for new memories from database
        channel
            .on('postgres_changes', {
            event: 'INSERT',
            schema: 'public',
            table: 'agent_memories',
            filter: `tenant_id=eq.${this.tenantId}`,
        }, (payload) => {
            const memory = {
                type: 'memory_added',
                tenant_id: payload.new.tenant_id,
                agent_id: payload.new.agent_id,
                session_id: payload.new.session_id,
                content: payload.new.content,
                embedding: payload.new.embedding,
                metadata: payload.new.metadata,
                timestamp: payload.new.created_at,
            };
            this.emit('memory:added', memory);
        })
            .on('postgres_changes', {
            event: 'UPDATE',
            schema: 'public',
            table: 'agent_memories',
            filter: `tenant_id=eq.${this.tenantId}`,
        }, (payload) => {
            const memory = {
                type: 'memory_updated',
                tenant_id: payload.new.tenant_id,
                agent_id: payload.new.agent_id,
                session_id: payload.new.session_id,
                content: payload.new.content,
                embedding: payload.new.embedding,
                metadata: payload.new.metadata,
                timestamp: new Date().toISOString(),
            };
            this.emit('memory:updated', memory);
        });
        await channel.subscribe();
        this.channels.set('memory-sync', channel);
        console.log('💾 Real-time memory sync enabled');
    }
    /**
     * Set up coordination channel for agent-to-agent communication
     */
    async setupCoordination() {
        const channelName = `coordination:${this.tenantId}`;
        const channel = this.client.channel(channelName);
        // Listen for broadcast messages
        channel.on('broadcast', { event: 'coordination' }, ({ payload }) => {
            const message = payload;
            // Only process if message is for us or broadcast
            if (!message.to_agent || message.to_agent === this.agentId) {
                this.emit('message:received', message);
                // Emit specific event types
                this.emit(`message:${message.type}`, message);
            }
        });
        await channel.subscribe();
        this.channels.set('coordination', channel);
        console.log('📡 Agent coordination channel active');
    }
    /**
     * Broadcast message to all agents in tenant
     */
    async broadcast(type, payload) {
        const channel = this.channels.get('coordination');
        if (!channel) {
            throw new Error('Coordination channel not initialized');
        }
        const message = {
            from_agent: this.agentId,
            type,
            payload,
            timestamp: new Date().toISOString(),
        };
        await channel.send({
            type: 'broadcast',
            event: 'coordination',
            payload: message,
        });
    }
    /**
     * Send direct message to specific agent
     */
    async sendMessage(toAgent, type, payload) {
        const channel = this.channels.get('coordination');
        if (!channel) {
            throw new Error('Coordination channel not initialized');
        }
        const message = {
            from_agent: this.agentId,
            to_agent: toAgent,
            type,
            payload,
            timestamp: new Date().toISOString(),
        };
        await channel.send({
            type: 'broadcast',
            event: 'coordination',
            payload: message,
        });
    }
    /**
     * Assign task to another agent
     */
    async assignTask(task) {
        await this.sendMessage(task.assigned_to, 'task_assignment', task);
        console.log(`📋 Task assigned: ${task.task_id} → ${task.assigned_to}`);
    }
    /**
     * Report task completion
     */
    async reportTaskComplete(taskId, result) {
        await this.broadcast('task_complete', {
            task_id: taskId,
            result,
            completed_by: this.agentId,
        });
        console.log(`✅ Task completed: ${taskId}`);
    }
    /**
     * Request help from other agents
     */
    async requestHelp(problem, context) {
        await this.broadcast('request_help', {
            problem,
            context,
            from: this.agentId,
        });
        console.log(`🆘 Help requested: ${problem}`);
    }
    /**
     * Share knowledge with other agents
     */
    async shareKnowledge(knowledge, metadata) {
        await this.broadcast('share_knowledge', {
            knowledge,
            metadata,
            from: this.agentId,
        });
        console.log(`💡 Knowledge shared: ${knowledge.substring(0, 50)}...`);
    }
    /**
     * Update agent status
     */
    async updateStatus(status, task) {
        if (!this.presenceChannel)
            return;
        await this.presenceChannel.track({
            agent_id: this.agentId,
            tenant_id: this.tenantId,
            status,
            task,
            last_heartbeat: new Date().toISOString(),
        });
        await this.broadcast('status_update', {
            agent_id: this.agentId,
            status,
            task,
        });
    }
    /**
     * Get list of active agents
     */
    getActiveAgents(presenceState) {
        if (!this.presenceChannel)
            return [];
        const state = presenceState || this.presenceChannel.presenceState();
        const agents = [];
        for (const [agentId, presences] of Object.entries(state)) {
            const presence = presences[0];
            agents.push({
                agent_id: agentId,
                tenant_id: presence.tenant_id,
                status: presence.status,
                task: presence.task,
                started_at: presence.started_at,
                last_heartbeat: presence.last_heartbeat,
                metadata: presence.metadata,
            });
        }
        return agents;
    }
    /**
     * Start heartbeat to maintain presence
     */
    startHeartbeat() {
        const interval = this.config.presenceHeartbeat || 30000;
        this.heartbeatInterval = setInterval(async () => {
            if (this.presenceChannel) {
                await this.presenceChannel.track({
                    agent_id: this.agentId,
                    tenant_id: this.tenantId,
                    last_heartbeat: new Date().toISOString(),
                });
            }
        }, interval);
    }
    /**
     * Stop heartbeat
     */
    stopHeartbeat() {
        if (this.heartbeatInterval) {
            clearInterval(this.heartbeatInterval);
            this.heartbeatInterval = undefined;
        }
    }
    /**
     * Subscribe to events
     */
    on(event, handler) {
        if (!this.eventHandlers.has(event)) {
            this.eventHandlers.set(event, new Set());
        }
        this.eventHandlers.get(event).add(handler);
    }
    /**
     * Unsubscribe from events
     */
    off(event, handler) {
        const handlers = this.eventHandlers.get(event);
        if (handlers) {
            handlers.delete(handler);
        }
    }
    /**
     * Emit event to handlers
     */
    emit(event, data) {
        const handlers = this.eventHandlers.get(event);
        if (handlers) {
            handlers.forEach((handler) => {
                try {
                    handler(data);
                }
                catch (error) {
                    console.error(`Error in event handler for ${event}:`, error);
                }
            });
        }
    }
    /**
     * Get real-time statistics
     */
    async getStats() {
        const activeAgents = this.getActiveAgents();
        return {
            tenant_id: this.tenantId,
            agent_id: this.agentId,
            active_agents: activeAgents.length,
            agents: activeAgents,
            channels: Array.from(this.channels.keys()),
            heartbeat_interval: this.config.presenceHeartbeat || 30000,
            memory_sync: this.config.memorySync !== false,
            timestamp: new Date().toISOString(),
        };
    }
    /**
     * Shutdown and cleanup
     */
    async shutdown() {
        console.log('🛑 Shutting down Real-Time Federation Hub...');
        // Stop heartbeat
        this.stopHeartbeat();
        // Update status to offline
        if (this.presenceChannel) {
            await this.presenceChannel.track({
                agent_id: this.agentId,
                tenant_id: this.tenantId,
                status: 'offline',
            });
            await this.presenceChannel.untrack();
        }
        // Unsubscribe from all channels
        for (const [name, channel] of this.channels) {
            await channel.unsubscribe();
            console.log(`   Unsubscribed from ${name}`);
        }
        if (this.presenceChannel) {
            await this.presenceChannel.unsubscribe();
        }
        this.channels.clear();
        this.eventHandlers.clear();
        console.log('✅ Real-Time Federation Hub shutdown complete');
    }
}
/**
 * Create real-time federation hub from environment
 */
export function createRealtimeHub(agentId, tenantId = 'default') {
    const url = process.env.SUPABASE_URL;
    const anonKey = process.env.SUPABASE_ANON_KEY;
    const serviceRoleKey = process.env.SUPABASE_SERVICE_ROLE_KEY;
    if (!url || !anonKey) {
        throw new Error('Missing Supabase credentials. Set SUPABASE_URL and SUPABASE_ANON_KEY');
    }
    return new RealtimeFederationHub({
        url,
        anonKey,
        serviceRoleKey,
        presenceHeartbeat: parseInt(process.env.FEDERATION_HEARTBEAT_INTERVAL || '30000'),
        memorySync: process.env.FEDERATION_MEMORY_SYNC !== 'false',
        broadcastLatency: process.env.FEDERATION_BROADCAST_LATENCY || 'low',
    }, agentId, tenantId);
}
//# sourceMappingURL=realtime-federation.js.map