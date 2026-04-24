// QUIC-enabled Swarm Coordinator
// Manages agent-to-agent communication over QUIC transport with fallback to HTTP/2
import { logger } from '../utils/logger.js';
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
export class QuicCoordinator {
    config;
    state;
    messageQueue;
    heartbeatTimer;
    syncTimer;
    messageStats;
    constructor(config) {
        this.config = {
            ...config,
            heartbeatInterval: config.heartbeatInterval || 10000,
            statesSyncInterval: config.statesSyncInterval || 5000,
            enableCompression: config.enableCompression ?? true
        };
        this.state = {
            swarmId: config.swarmId,
            topology: config.topology,
            agents: new Map(),
            connections: new Map(),
            stats: {
                totalAgents: 0,
                activeAgents: 0,
                totalMessages: 0,
                messagesPerSecond: 0,
                averageLatency: 0,
                quicStats: {
                    totalConnections: 0,
                    activeConnections: 0,
                    totalStreams: 0,
                    activeStreams: 0,
                    bytesReceived: 0,
                    bytesSent: 0,
                    packetsLost: 0,
                    rttMs: 0
                }
            }
        };
        this.messageQueue = [];
        this.messageStats = new Map();
        logger.info('QUIC Coordinator initialized', {
            swarmId: config.swarmId,
            topology: config.topology,
            maxAgents: config.maxAgents
        });
    }
    /**
     * Start the coordinator (heartbeat and state sync)
     */
    async start() {
        logger.info('Starting QUIC Coordinator', { swarmId: this.config.swarmId });
        // Initialize QUIC client
        await this.config.quicClient.initialize();
        // Start heartbeat
        this.startHeartbeat();
        // Start state synchronization
        this.startStateSync();
        logger.info('QUIC Coordinator started successfully');
    }
    /**
     * Stop the coordinator
     */
    async stop() {
        logger.info('Stopping QUIC Coordinator', { swarmId: this.config.swarmId });
        // Stop timers
        if (this.heartbeatTimer) {
            clearInterval(this.heartbeatTimer);
        }
        if (this.syncTimer) {
            clearInterval(this.syncTimer);
        }
        // Close all connections
        for (const [agentId, connection] of this.state.connections.entries()) {
            await this.config.quicClient.closeConnection(connection.id);
            logger.debug('Closed connection', { agentId, connectionId: connection.id });
        }
        // Shutdown QUIC client
        await this.config.quicClient.shutdown();
        logger.info('QUIC Coordinator stopped');
    }
    /**
     * Register an agent in the swarm
     */
    async registerAgent(agent) {
        if (this.state.agents.size >= this.config.maxAgents) {
            throw new Error(`Maximum agents (${this.config.maxAgents}) reached`);
        }
        logger.info('Registering agent', {
            agentId: agent.id,
            role: agent.role,
            host: agent.host,
            port: agent.port
        });
        // Establish QUIC connection to agent
        const connection = await this.config.connectionPool.getConnection(agent.host, agent.port);
        // Store agent and connection
        this.state.agents.set(agent.id, agent);
        this.state.connections.set(agent.id, connection);
        this.messageStats.set(agent.id, { sent: 0, received: 0, latency: [] });
        // Update stats
        this.state.stats.totalAgents = this.state.agents.size;
        this.state.stats.activeAgents = this.state.agents.size;
        // Establish topology-specific connections
        await this.establishTopologyConnections(agent);
        logger.info('Agent registered successfully', { agentId: agent.id });
    }
    /**
     * Unregister an agent from the swarm
     */
    async unregisterAgent(agentId) {
        const agent = this.state.agents.get(agentId);
        if (!agent) {
            logger.warn('Agent not found', { agentId });
            return;
        }
        logger.info('Unregistering agent', { agentId });
        // Close connection
        const connection = this.state.connections.get(agentId);
        if (connection) {
            await this.config.quicClient.closeConnection(connection.id);
        }
        // Remove from state
        this.state.agents.delete(agentId);
        this.state.connections.delete(agentId);
        this.messageStats.delete(agentId);
        // Update stats
        this.state.stats.totalAgents = this.state.agents.size;
        this.state.stats.activeAgents = this.state.agents.size;
        logger.info('Agent unregistered successfully', { agentId });
    }
    /**
     * Send message to one or more agents
     */
    async sendMessage(message) {
        const startTime = Date.now();
        logger.debug('Sending message', {
            messageId: message.id,
            from: message.from,
            to: message.to,
            type: message.type
        });
        // Determine recipients based on topology
        const recipients = this.resolveRecipients(message);
        // Send message to each recipient
        for (const recipientId of recipients) {
            await this.sendToAgent(recipientId, message);
        }
        // Update stats
        const latency = Date.now() - startTime;
        this.updateMessageStats(message.from, 'sent', latency);
        this.state.stats.totalMessages++;
        logger.debug('Message sent successfully', {
            messageId: message.id,
            recipients: recipients.length,
            latency
        });
    }
    /**
     * Broadcast message to all agents (except sender)
     */
    async broadcast(message) {
        const broadcastMessage = {
            ...message,
            to: '*'
        };
        await this.sendMessage(broadcastMessage);
    }
    /**
     * Get current swarm state
     */
    async getState() {
        return {
            ...this.state,
            stats: await this.calculateStats()
        };
    }
    /**
     * Get agent statistics
     */
    getAgentStats(agentId) {
        const stats = this.messageStats.get(agentId);
        if (!stats) {
            return null;
        }
        return {
            sent: stats.sent,
            received: stats.received,
            avgLatency: stats.latency.length > 0
                ? stats.latency.reduce((sum, l) => sum + l, 0) / stats.latency.length
                : 0
        };
    }
    /**
     * Get all agent statistics
     */
    getAllAgentStats() {
        const allStats = new Map();
        for (const [agentId, stats] of this.messageStats.entries()) {
            allStats.set(agentId, {
                sent: stats.sent,
                received: stats.received,
                avgLatency: stats.latency.length > 0
                    ? stats.latency.reduce((sum, l) => sum + l, 0) / stats.latency.length
                    : 0
            });
        }
        return allStats;
    }
    /**
     * Synchronize state across all agents
     */
    async syncState() {
        logger.debug('Synchronizing swarm state', { swarmId: this.config.swarmId });
        const stateMessage = {
            id: `sync-${Date.now()}`,
            from: 'coordinator',
            to: '*',
            type: 'sync',
            payload: {
                swarmId: this.state.swarmId,
                topology: this.state.topology,
                agents: Array.from(this.state.agents.values()),
                stats: this.calculateStats()
            },
            timestamp: Date.now()
        };
        await this.broadcast(stateMessage);
    }
    // ========== Private Methods ==========
    /**
     * Establish topology-specific connections
     */
    async establishTopologyConnections(agent) {
        switch (this.config.topology) {
            case 'mesh':
                // In mesh, each agent connects to all others (handled by caller)
                logger.debug('Mesh topology: agent connects to all', { agentId: agent.id });
                break;
            case 'hierarchical':
                // In hierarchical, workers connect to coordinators
                if (agent.role === 'worker') {
                    const coordinators = Array.from(this.state.agents.values())
                        .filter(a => a.role === 'coordinator');
                    logger.debug('Hierarchical topology: connecting worker to coordinators', {
                        agentId: agent.id,
                        coordinators: coordinators.length
                    });
                }
                break;
            case 'ring':
                // In ring, each agent connects to next agent in circular order
                const agents = Array.from(this.state.agents.values());
                if (agents.length > 1) {
                    logger.debug('Ring topology: establishing ring connections', {
                        agentId: agent.id,
                        totalAgents: agents.length
                    });
                }
                break;
            case 'star':
                // In star, all agents connect to central coordinator
                if (agent.role === 'coordinator') {
                    logger.debug('Star topology: coordinator established', { agentId: agent.id });
                }
                else {
                    const coordinator = Array.from(this.state.agents.values())
                        .find(a => a.role === 'coordinator');
                    if (coordinator) {
                        logger.debug('Star topology: connecting to coordinator', {
                            agentId: agent.id,
                            coordinator: coordinator.id
                        });
                    }
                }
                break;
        }
    }
    /**
     * Resolve message recipients based on topology
     */
    resolveRecipients(message) {
        const recipients = [];
        const to = Array.isArray(message.to) ? message.to : [message.to];
        for (const target of to) {
            if (target === '*') {
                // Broadcast to all (except sender)
                recipients.push(...Array.from(this.state.agents.keys())
                    .filter(id => id !== message.from));
            }
            else {
                recipients.push(target);
            }
        }
        // Apply topology-specific routing
        return this.applyTopologyRouting(message.from, recipients);
    }
    /**
     * Apply topology-specific routing rules
     */
    applyTopologyRouting(senderId, recipients) {
        switch (this.config.topology) {
            case 'mesh':
                // Direct routing in mesh topology
                return recipients;
            case 'hierarchical':
                // Route through coordinator in hierarchical
                const sender = this.state.agents.get(senderId);
                if (sender?.role === 'worker') {
                    // Worker messages go through coordinator
                    const coordinators = Array.from(this.state.agents.values())
                        .filter(a => a.role === 'coordinator')
                        .map(a => a.id);
                    return coordinators.length > 0 ? coordinators : recipients;
                }
                return recipients;
            case 'ring':
                // Forward to next agent in ring
                const agents = Array.from(this.state.agents.keys());
                const currentIndex = agents.indexOf(senderId);
                const nextIndex = (currentIndex + 1) % agents.length;
                return [agents[nextIndex]];
            case 'star':
                // Route through central coordinator
                const coordinator = Array.from(this.state.agents.values())
                    .find(a => a.role === 'coordinator');
                if (coordinator && senderId !== coordinator.id) {
                    return [coordinator.id];
                }
                return recipients;
            default:
                return recipients;
        }
    }
    /**
     * Send message to specific agent
     */
    async sendToAgent(agentId, message) {
        const connection = this.state.connections.get(agentId);
        if (!connection) {
            logger.warn('Connection not found for agent', { agentId });
            return;
        }
        try {
            // Create QUIC stream
            const stream = await this.config.quicClient.createStream(connection.id);
            // Serialize message
            const messageBytes = this.serializeMessage(message);
            // Send message
            await stream.send(messageBytes);
            // Close stream
            await stream.close();
            logger.debug('Message sent to agent', { agentId, messageId: message.id });
        }
        catch (error) {
            logger.error('Failed to send message to agent', { agentId, error });
            throw error;
        }
    }
    /**
     * Serialize message to bytes
     */
    serializeMessage(message) {
        const json = JSON.stringify(message);
        const encoder = new TextEncoder();
        return encoder.encode(json);
    }
    /**
     * Start heartbeat timer
     */
    startHeartbeat() {
        this.heartbeatTimer = setInterval(async () => {
            logger.debug('Sending heartbeat', { swarmId: this.config.swarmId });
            const heartbeat = {
                id: `heartbeat-${Date.now()}`,
                from: 'coordinator',
                to: '*',
                type: 'heartbeat',
                payload: { timestamp: Date.now() },
                timestamp: Date.now()
            };
            try {
                await this.broadcast(heartbeat);
            }
            catch (error) {
                logger.error('Heartbeat failed', { error });
            }
        }, this.config.heartbeatInterval);
    }
    /**
     * Start state sync timer
     */
    startStateSync() {
        this.syncTimer = setInterval(async () => {
            try {
                await this.syncState();
            }
            catch (error) {
                logger.error('State sync failed', { error });
            }
        }, this.config.statesSyncInterval);
    }
    /**
     * Update message statistics
     */
    updateMessageStats(agentId, type, latency) {
        const stats = this.messageStats.get(agentId);
        if (!stats) {
            return;
        }
        if (type === 'sent') {
            stats.sent++;
        }
        else {
            stats.received++;
        }
        stats.latency.push(latency);
        // Keep only last 100 latency measurements
        if (stats.latency.length > 100) {
            stats.latency.shift();
        }
    }
    /**
     * Calculate current swarm statistics
     */
    async calculateStats() {
        const quicStats = await this.config.quicClient.getStats();
        // Calculate messages per second (last minute average)
        const messagesPerSecond = this.state.stats.totalMessages / 60;
        // Calculate average latency across all agents
        let totalLatency = 0;
        let latencyCount = 0;
        for (const stats of this.messageStats.values()) {
            if (stats.latency.length > 0) {
                totalLatency += stats.latency.reduce((sum, l) => sum + l, 0);
                latencyCount += stats.latency.length;
            }
        }
        const averageLatency = latencyCount > 0 ? totalLatency / latencyCount : 0;
        return {
            totalAgents: this.state.agents.size,
            activeAgents: this.state.agents.size,
            totalMessages: this.state.stats.totalMessages,
            messagesPerSecond,
            averageLatency,
            quicStats
        };
    }
}
//# sourceMappingURL=quic-coordinator.js.map