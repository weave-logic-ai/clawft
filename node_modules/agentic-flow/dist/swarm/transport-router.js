// Transport Router - Protocol selection and routing with transparent fallback
// Routes agent messages through QUIC or HTTP/2 based on availability
import { QuicClient, QuicConnectionPool } from '../transport/quic.js';
import { QuicCoordinator } from './quic-coordinator.js';
import { logger } from '../utils/logger.js';
import http2 from 'http2';
/**
 * TransportRouter - Intelligent transport layer with automatic protocol selection
 *
 * Features:
 * - Automatic QUIC/HTTP2 protocol selection
 * - Transparent fallback on failure
 * - Connection pooling for both protocols
 * - Per-protocol statistics tracking
 * - Health checking and availability detection
 */
export class TransportRouter {
    config;
    quicClient;
    quicPool;
    quicCoordinator;
    http2Sessions;
    currentProtocol;
    stats;
    healthCheckTimer;
    quicAvailable;
    constructor(config) {
        this.config = {
            protocol: config.protocol || 'auto',
            enableFallback: config.enableFallback ?? true,
            quicConfig: config.quicConfig || {
                host: 'localhost',
                port: 4433,
                maxConnections: 100
            },
            http2Config: config.http2Config || {
                host: 'localhost',
                port: 8443,
                maxConnections: 100,
                secure: true
            }
        };
        this.http2Sessions = new Map();
        this.currentProtocol = 'http2'; // Default to HTTP2
        this.quicAvailable = false;
        // Initialize stats
        this.stats = new Map([
            ['quic', {
                    protocol: 'quic',
                    messagesSent: 0,
                    messagesReceived: 0,
                    bytesTransferred: 0,
                    averageLatency: 0,
                    errorRate: 0
                }],
            ['http2', {
                    protocol: 'http2',
                    messagesSent: 0,
                    messagesReceived: 0,
                    bytesTransferred: 0,
                    averageLatency: 0,
                    errorRate: 0
                }]
        ]);
        logger.info('Transport Router initialized', {
            protocol: config.protocol,
            enableFallback: config.enableFallback
        });
    }
    /**
     * Initialize transport router
     */
    async initialize() {
        logger.info('Initializing transport router...');
        // Try to initialize QUIC
        if (this.config.protocol === 'quic' || this.config.protocol === 'auto') {
            try {
                await this.initializeQuic();
                this.quicAvailable = true;
                this.currentProtocol = 'quic';
                logger.info('QUIC transport initialized successfully');
            }
            catch (error) {
                logger.warn('QUIC initialization failed, using HTTP/2', { error });
                this.quicAvailable = false;
                this.currentProtocol = 'http2';
            }
        }
        // Always initialize HTTP/2 as fallback
        if (!this.quicAvailable || this.config.enableFallback) {
            await this.initializeHttp2();
            logger.info('HTTP/2 transport initialized successfully');
        }
        // Start health checks if auto mode
        if (this.config.protocol === 'auto') {
            this.startHealthChecks();
        }
        logger.info('Transport router initialized', {
            currentProtocol: this.currentProtocol,
            quicAvailable: this.quicAvailable
        });
    }
    /**
     * Initialize QUIC transport
     */
    async initializeQuic() {
        this.quicClient = new QuicClient({
            serverHost: this.config.quicConfig.host,
            serverPort: this.config.quicConfig.port,
            maxConnections: this.config.quicConfig.maxConnections,
            certPath: this.config.quicConfig.certPath,
            keyPath: this.config.quicConfig.keyPath
        });
        await this.quicClient.initialize();
        this.quicPool = new QuicConnectionPool(this.quicClient, this.config.quicConfig.maxConnections);
        logger.debug('QUIC client and pool initialized');
    }
    /**
     * Initialize HTTP/2 transport
     */
    async initializeHttp2() {
        // HTTP/2 sessions are created on-demand in sendViaHttp2
        logger.debug('HTTP/2 transport configured');
    }
    /**
     * Initialize QUIC coordinator for swarm
     */
    async initializeSwarm(swarmId, topology, maxAgents = 10) {
        if (!this.quicClient || !this.quicPool) {
            throw new Error('QUIC not initialized. Cannot create swarm with QUIC transport.');
        }
        this.quicCoordinator = new QuicCoordinator({
            swarmId,
            topology,
            maxAgents,
            quicClient: this.quicClient,
            connectionPool: this.quicPool
        });
        await this.quicCoordinator.start();
        logger.info('QUIC swarm coordinator initialized', { swarmId, topology, maxAgents });
        return this.quicCoordinator;
    }
    /**
     * Route message through appropriate transport
     */
    async route(message, target) {
        const startTime = Date.now();
        try {
            // Try primary protocol
            if (this.currentProtocol === 'quic' && this.quicAvailable) {
                try {
                    await this.sendViaQuic(message, target);
                    const latency = Date.now() - startTime;
                    this.updateStats('quic', true, latency, this.estimateMessageSize(message));
                    return { success: true, protocol: 'quic', latency };
                }
                catch (error) {
                    logger.warn('QUIC send failed, attempting fallback', { error });
                    if (!this.config.enableFallback) {
                        throw error;
                    }
                    // Fallback to HTTP/2
                    await this.sendViaHttp2(message, target);
                    const latency = Date.now() - startTime;
                    this.updateStats('http2', true, latency, this.estimateMessageSize(message));
                    return { success: true, protocol: 'http2', latency };
                }
            }
            else {
                // Use HTTP/2
                await this.sendViaHttp2(message, target);
                const latency = Date.now() - startTime;
                this.updateStats('http2', true, latency, this.estimateMessageSize(message));
                return { success: true, protocol: 'http2', latency };
            }
        }
        catch (error) {
            const latency = Date.now() - startTime;
            this.updateStats(this.currentProtocol, false, latency, 0);
            return {
                success: false,
                protocol: this.currentProtocol,
                latency,
                error: error.message
            };
        }
    }
    /**
     * Send message via QUIC
     */
    async sendViaQuic(message, target) {
        if (!this.quicClient || !this.quicPool) {
            throw new Error('QUIC not initialized');
        }
        // Get or create connection
        const connection = await this.quicPool.getConnection(target.host, target.port);
        // Create stream and send
        const stream = await this.quicClient.createStream(connection.id);
        try {
            const messageBytes = this.serializeMessage(message);
            await stream.send(messageBytes);
            logger.debug('Message sent via QUIC', {
                messageId: message.id,
                target: target.id,
                bytes: messageBytes.length
            });
        }
        finally {
            await stream.close();
        }
    }
    /**
     * Send message via HTTP/2
     */
    async sendViaHttp2(message, target) {
        const sessionKey = `${target.host}:${target.port}`;
        // Get or create HTTP/2 session
        let session = this.http2Sessions.get(sessionKey);
        if (!session || session.destroyed) {
            const protocol = this.config.http2Config.secure ? 'https' : 'http';
            session = http2.connect(`${protocol}://${target.host}:${target.port}`);
            this.http2Sessions.set(sessionKey, session);
        }
        return new Promise((resolve, reject) => {
            const req = session.request({
                ':method': 'POST',
                ':path': '/message',
                'content-type': 'application/json'
            });
            const messageJson = JSON.stringify(message);
            req.on('response', (headers) => {
                const status = headers[':status'];
                if (status === 200) {
                    logger.debug('Message sent via HTTP/2', {
                        messageId: message.id,
                        target: target.id,
                        bytes: messageJson.length
                    });
                    resolve();
                }
                else {
                    reject(new Error(`HTTP/2 request failed with status ${status}`));
                }
            });
            req.on('error', (error) => {
                logger.error('HTTP/2 request error', { error });
                reject(error);
            });
            req.write(messageJson);
            req.end();
        });
    }
    /**
     * Get current transport protocol
     */
    getCurrentProtocol() {
        return this.currentProtocol;
    }
    /**
     * Check if QUIC is available
     */
    isQuicAvailable() {
        return this.quicAvailable;
    }
    /**
     * Get transport statistics
     */
    getStats(protocol) {
        if (protocol) {
            return this.stats.get(protocol);
        }
        return this.stats;
    }
    /**
     * Get QUIC coordinator (if initialized)
     */
    getCoordinator() {
        return this.quicCoordinator;
    }
    /**
     * Shutdown transport router
     */
    async shutdown() {
        logger.info('Shutting down transport router');
        // Stop health checks
        if (this.healthCheckTimer) {
            clearInterval(this.healthCheckTimer);
        }
        // Shutdown QUIC coordinator
        if (this.quicCoordinator) {
            await this.quicCoordinator.stop();
        }
        // Close QUIC pool
        if (this.quicPool) {
            await this.quicPool.clear();
        }
        // Shutdown QUIC client
        if (this.quicClient) {
            await this.quicClient.shutdown();
        }
        // Close HTTP/2 sessions
        for (const session of this.http2Sessions.values()) {
            session.close();
        }
        this.http2Sessions.clear();
        logger.info('Transport router shutdown complete');
    }
    // ========== Private Methods ==========
    /**
     * Start health checks for protocol availability
     */
    startHealthChecks() {
        this.healthCheckTimer = setInterval(async () => {
            await this.checkQuicHealth();
        }, 30000); // Check every 30 seconds
    }
    /**
     * Check QUIC health
     */
    async checkQuicHealth() {
        if (!this.quicClient) {
            return;
        }
        try {
            // Try to get stats as health check
            const stats = this.quicClient.getStats();
            if (!this.quicAvailable) {
                this.quicAvailable = true;
                this.currentProtocol = 'quic';
                logger.info('QUIC became available, switching protocol');
            }
        }
        catch (error) {
            if (this.quicAvailable) {
                this.quicAvailable = false;
                this.currentProtocol = 'http2';
                logger.warn('QUIC became unavailable, switching to HTTP/2', { error });
            }
        }
    }
    /**
     * Update transport statistics
     */
    updateStats(protocol, success, latency, bytes) {
        const stats = this.stats.get(protocol);
        if (success) {
            stats.messagesSent++;
            stats.bytesTransferred += bytes;
            // Update average latency (exponential moving average)
            const alpha = 0.1;
            stats.averageLatency = stats.averageLatency * (1 - alpha) + latency * alpha;
        }
        else {
            // Update error rate (exponential moving average)
            const alpha = 0.1;
            const errorCount = stats.messagesSent * stats.errorRate + 1;
            stats.errorRate = errorCount / (stats.messagesSent + 1);
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
     * Estimate message size in bytes
     */
    estimateMessageSize(message) {
        return JSON.stringify(message).length;
    }
}
//# sourceMappingURL=transport-router.js.map