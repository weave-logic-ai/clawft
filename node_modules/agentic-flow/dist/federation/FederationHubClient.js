/**
 * Federation Hub Client - WebSocket client for agent-to-hub communication
 */
import WebSocket from 'ws';
import { logger } from '../utils/logger.js';
export class FederationHubClient {
    config;
    ws;
    connected = false;
    vectorClock = {};
    lastSyncTime = 0;
    messageHandlers = new Map();
    constructor(config) {
        this.config = config;
    }
    /**
     * Connect to hub with WebSocket
     */
    async connect() {
        return new Promise((resolve, reject) => {
            try {
                // Convert quic:// to ws:// for WebSocket connection
                const wsEndpoint = this.config.endpoint
                    .replace('quic://', 'ws://')
                    .replace(':4433', ':8443'); // Map QUIC port to WebSocket port
                logger.info('Connecting to federation hub', {
                    endpoint: wsEndpoint,
                    agentId: this.config.agentId
                });
                this.ws = new WebSocket(wsEndpoint);
                this.ws.on('open', async () => {
                    logger.info('WebSocket connected, authenticating...');
                    // Send authentication
                    await this.send({
                        type: 'auth',
                        agentId: this.config.agentId,
                        tenantId: this.config.tenantId,
                        token: this.config.token,
                        vectorClock: this.vectorClock,
                        timestamp: Date.now()
                    });
                    // Wait for auth acknowledgment
                    const authTimeout = setTimeout(() => {
                        reject(new Error('Authentication timeout'));
                    }, 5000);
                    const authHandler = (msg) => {
                        if (msg.type === 'ack') {
                            clearTimeout(authTimeout);
                            this.connected = true;
                            this.lastSyncTime = Date.now();
                            logger.info('Authenticated with hub');
                            resolve();
                        }
                        else if (msg.type === 'error') {
                            clearTimeout(authTimeout);
                            reject(new Error(msg.error || 'Authentication failed'));
                        }
                    };
                    this.messageHandlers.set('auth', authHandler);
                });
                this.ws.on('message', (data) => {
                    try {
                        const message = JSON.parse(data.toString());
                        this.handleMessage(message);
                    }
                    catch (error) {
                        logger.error('Failed to parse message', { error: error.message });
                    }
                });
                this.ws.on('close', () => {
                    this.connected = false;
                    logger.info('Disconnected from hub');
                });
                this.ws.on('error', (error) => {
                    logger.error('WebSocket error', { error: error.message });
                    reject(error);
                });
            }
            catch (error) {
                logger.error('Failed to connect to hub', { error: error.message });
                reject(error);
            }
        });
    }
    /**
     * Handle incoming message
     */
    handleMessage(message) {
        // Check for specific handlers first
        const handler = this.messageHandlers.get('auth');
        if (handler) {
            handler(message);
            this.messageHandlers.delete('auth');
            return;
        }
        // Handle sync responses
        if (message.type === 'ack' && message.data) {
            logger.debug('Received sync data', { count: message.data.length });
        }
        else if (message.type === 'error') {
            logger.error('Hub error', { error: message.error });
        }
        // Update vector clock if provided
        if (message.vectorClock) {
            this.updateVectorClock(message.vectorClock);
        }
    }
    /**
     * Sync with hub
     */
    async sync(db) {
        if (!this.connected) {
            throw new Error('Not connected to hub');
        }
        const startTime = Date.now();
        try {
            // Increment vector clock
            this.vectorClock[this.config.agentId] =
                (this.vectorClock[this.config.agentId] || 0) + 1;
            // PULL: Get updates from hub
            await this.send({
                type: 'pull',
                agentId: this.config.agentId,
                tenantId: this.config.tenantId,
                vectorClock: this.vectorClock,
                timestamp: Date.now()
            });
            // Wait for response (simplified for now)
            await new Promise(resolve => setTimeout(resolve, 100));
            // PUSH: Send local changes to hub
            const localChanges = await this.getLocalChanges(db);
            if (localChanges.length > 0) {
                await this.send({
                    type: 'push',
                    agentId: this.config.agentId,
                    tenantId: this.config.tenantId,
                    vectorClock: this.vectorClock,
                    data: localChanges,
                    timestamp: Date.now()
                });
                logger.info('Sync completed', {
                    agentId: this.config.agentId,
                    pushCount: localChanges.length,
                    duration: Date.now() - startTime
                });
            }
            this.lastSyncTime = Date.now();
        }
        catch (error) {
            logger.error('Sync failed', { error: error.message });
            throw error;
        }
    }
    /**
     * Get local changes from database
     */
    async getLocalChanges(db) {
        // Query recent episodes from local database
        // This is a simplified version - in production, track changes since last sync
        try {
            // Get recent patterns from AgentDB
            // For now, return empty array as placeholder
            return [];
        }
        catch (error) {
            logger.error('Failed to get local changes', { error });
            return [];
        }
    }
    /**
     * Update vector clock
     */
    updateVectorClock(remoteVectorClock) {
        for (const [agentId, ts] of Object.entries(remoteVectorClock)) {
            this.vectorClock[agentId] = Math.max(this.vectorClock[agentId] || 0, ts);
        }
    }
    /**
     * Send message to hub
     */
    async send(message) {
        if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
            throw new Error('WebSocket not connected');
        }
        this.ws.send(JSON.stringify(message));
    }
    /**
     * Disconnect from hub
     */
    async disconnect() {
        if (this.ws) {
            this.ws.close();
            this.ws = undefined;
        }
        this.connected = false;
    }
    /**
     * Check connection status
     */
    isConnected() {
        return this.connected;
    }
    /**
     * Get sync stats
     */
    getSyncStats() {
        return {
            lastSyncTime: this.lastSyncTime,
            vectorClock: { ...this.vectorClock }
        };
    }
}
//# sourceMappingURL=FederationHubClient.js.map