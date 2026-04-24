/**
 * QUICServer - QUIC Protocol Server for AgentDB Synchronization
 *
 * Implements a QUIC server for receiving and handling synchronization requests
 * from remote AgentDB instances. Supports episodes, skills, and edge synchronization.
 *
 * Features:
 * - Start/stop server lifecycle management
 * - Client connection handling
 * - Authentication and authorization
 * - Rate limiting per client
 * - Sync request processing (episodes, skills, edges)
 * - Comprehensive error handling and logging
 */
import chalk from 'chalk';
export class QUICServer {
    db;
    config;
    isRunning = false;
    connections = new Map();
    rateLimitState = new Map();
    server = null;
    cleanupInterval = null;
    constructor(db, config = {}) {
        this.db = db;
        this.config = {
            host: config.host || '0.0.0.0',
            port: config.port || 4433,
            maxConnections: config.maxConnections || 100,
            authToken: config.authToken || '',
            rateLimit: config.rateLimit || {
                maxRequestsPerMinute: 60,
                maxBytesPerMinute: 10 * 1024 * 1024, // 10MB
            },
            tlsConfig: config.tlsConfig || {},
        };
    }
    /**
     * Start the QUIC server
     */
    async start() {
        if (this.isRunning) {
            console.log(chalk.yellow('⚠️  QUIC server is already running'));
            return;
        }
        try {
            console.log(chalk.blue('🚀 Starting QUIC server...'));
            console.log(chalk.gray(`   Host: ${this.config.host}`));
            console.log(chalk.gray(`   Port: ${this.config.port}`));
            // Note: Actual QUIC implementation would use a library like @fails-components/webtransport
            // or node-quic. This is a reference implementation showing the interface.
            // Initialize server state
            this.isRunning = true;
            this.startCleanupInterval();
            console.log(chalk.green('✓ QUIC server started successfully'));
            console.log(chalk.gray(`  Max connections: ${this.config.maxConnections}`));
            console.log(chalk.gray(`  Rate limit: ${this.config.rateLimit.maxRequestsPerMinute} req/min`));
        }
        catch (error) {
            const err = error;
            console.error(chalk.red('✗ Failed to start QUIC server:'), err.message);
            throw new Error(`QUIC server start failed: ${err.message}`);
        }
    }
    /**
     * Stop the QUIC server
     */
    async stop() {
        if (!this.isRunning) {
            console.log(chalk.yellow('⚠️  QUIC server is not running'));
            return;
        }
        try {
            console.log(chalk.blue('🛑 Stopping QUIC server...'));
            // Close all connections
            for (const [clientId, connection] of this.connections.entries()) {
                console.log(chalk.gray(`  Closing connection: ${clientId}`));
                // Close connection logic here
            }
            this.connections.clear();
            this.rateLimitState.clear();
            // Stop cleanup interval
            if (this.cleanupInterval) {
                clearInterval(this.cleanupInterval);
                this.cleanupInterval = null;
            }
            // Close server
            if (this.server) {
                // await this.server.close();
                this.server = null;
            }
            this.isRunning = false;
            console.log(chalk.green('✓ QUIC server stopped successfully'));
        }
        catch (error) {
            const err = error;
            console.error(chalk.red('✗ Error stopping QUIC server:'), err.message);
            throw new Error(`QUIC server stop failed: ${err.message}`);
        }
    }
    /**
     * Handle incoming client connection
     */
    async handleConnection(clientId, address) {
        // Check max connections
        if (this.connections.size >= this.config.maxConnections) {
            console.log(chalk.yellow(`⚠️  Max connections reached, rejecting ${clientId}`));
            return false;
        }
        // Register connection
        const connection = {
            id: clientId,
            address,
            connectedAt: Date.now(),
            requestCount: 0,
            bytesReceived: 0,
            lastRequestAt: 0,
        };
        this.connections.set(clientId, connection);
        console.log(chalk.green(`✓ Client connected: ${clientId} from ${address}`));
        console.log(chalk.gray(`  Active connections: ${this.connections.size}`));
        return true;
    }
    /**
     * Authenticate client request
     */
    authenticate(clientId, authToken) {
        if (!this.config.authToken) {
            return true; // No auth required
        }
        const isValid = authToken === this.config.authToken;
        if (!isValid) {
            console.log(chalk.red(`✗ Authentication failed for client: ${clientId}`));
        }
        return isValid;
    }
    /**
     * Check rate limits for client
     */
    checkRateLimit(clientId, requestSize) {
        const now = Date.now();
        let state = this.rateLimitState.get(clientId);
        if (!state || now - state.windowStart > 60000) {
            // New window
            state = {
                requestCount: 0,
                bytesTransferred: 0,
                windowStart: now,
            };
            this.rateLimitState.set(clientId, state);
        }
        // Check limits
        if (state.requestCount >= this.config.rateLimit.maxRequestsPerMinute) {
            console.log(chalk.yellow(`⚠️  Rate limit exceeded (requests) for ${clientId}`));
            return false;
        }
        if (state.bytesTransferred + requestSize > this.config.rateLimit.maxBytesPerMinute) {
            console.log(chalk.yellow(`⚠️  Rate limit exceeded (bytes) for ${clientId}`));
            return false;
        }
        // Update state
        state.requestCount++;
        state.bytesTransferred += requestSize;
        return true;
    }
    /**
     * Process sync request from client
     */
    async processSyncRequest(clientId, request, authToken) {
        try {
            // Authenticate
            if (!this.authenticate(clientId, authToken)) {
                return {
                    success: false,
                    error: 'Authentication failed',
                };
            }
            // Check rate limit
            const requestSize = JSON.stringify(request).length;
            if (!this.checkRateLimit(clientId, requestSize)) {
                return {
                    success: false,
                    error: 'Rate limit exceeded',
                };
            }
            // Update connection stats
            const connection = this.connections.get(clientId);
            if (connection) {
                connection.requestCount++;
                connection.bytesReceived += requestSize;
                connection.lastRequestAt = Date.now();
            }
            console.log(chalk.blue(`📥 Processing sync request from ${clientId}`));
            console.log(chalk.gray(`   Type: ${request.type}`));
            console.log(chalk.gray(`   Since: ${request.since || 'full sync'}`));
            // Process based on type
            let data;
            let count = 0;
            switch (request.type) {
                case 'episodes':
                    data = await this.syncEpisodes(request);
                    count = data.length;
                    break;
                case 'skills':
                    data = await this.syncSkills(request);
                    count = data.length;
                    break;
                case 'edges':
                    data = await this.syncEdges(request);
                    count = data.length;
                    break;
                case 'full':
                    data = await this.syncFull(request);
                    count = data.episodes?.length + data.skills?.length + data.edges?.length || 0;
                    break;
                default:
                    return {
                        success: false,
                        error: `Unknown sync type: ${request.type}`,
                    };
            }
            console.log(chalk.green(`✓ Sync completed: ${count} items sent`));
            return {
                success: true,
                data,
                count,
                hasMore: false, // Could implement pagination here
            };
        }
        catch (error) {
            const err = error;
            console.error(chalk.red('✗ Sync request failed:'), err.message);
            return {
                success: false,
                error: err.message,
            };
        }
    }
    /**
     * Sync episodes data
     */
    async syncEpisodes(request) {
        const { since, filters, batchSize = 1000 } = request;
        let query = 'SELECT * FROM episodes WHERE 1=1';
        const params = [];
        if (since) {
            query += ' AND ts > ?';
            params.push(since);
        }
        // Apply filters
        if (filters) {
            if (filters.sessionId) {
                query += ' AND session_id = ?';
                params.push(filters.sessionId);
            }
            if (filters.success !== undefined) {
                query += ' AND success = ?';
                params.push(filters.success ? 1 : 0);
            }
        }
        query += ` ORDER BY ts DESC LIMIT ${batchSize}`;
        const stmt = this.db.prepare(query);
        const rows = stmt.all(...params);
        return rows.map((row) => ({
            id: row.id,
            ts: row.ts,
            sessionId: row.session_id,
            task: row.task,
            input: row.input,
            output: row.output,
            critique: row.critique,
            reward: row.reward,
            success: row.success === 1,
            latencyMs: row.latency_ms,
            tokensUsed: row.tokens_used,
            tags: row.tags ? JSON.parse(row.tags) : [],
            metadata: row.metadata ? JSON.parse(row.metadata) : {},
        }));
    }
    /**
     * Sync skills data
     */
    async syncSkills(request) {
        const { since, batchSize = 1000 } = request;
        let query = 'SELECT * FROM skills WHERE 1=1';
        const params = [];
        if (since) {
            query += ' AND ts > ?';
            params.push(since);
        }
        query += ` ORDER BY ts DESC LIMIT ${batchSize}`;
        const stmt = this.db.prepare(query);
        const rows = stmt.all(...params);
        return rows.map((row) => ({
            id: row.id,
            ts: row.ts,
            name: row.name,
            description: row.description,
            code: row.code,
            successRate: row.success_rate,
            usageCount: row.usage_count,
            avgReward: row.avg_reward,
            tags: row.tags ? JSON.parse(row.tags) : [],
            metadata: row.metadata ? JSON.parse(row.metadata) : {},
        }));
    }
    /**
     * Sync edges (skill relationships)
     */
    async syncEdges(request) {
        const { since, batchSize = 1000 } = request;
        let query = 'SELECT * FROM skill_edges WHERE 1=1';
        const params = [];
        if (since) {
            query += ' AND ts > ?';
            params.push(since);
        }
        query += ` ORDER BY ts DESC LIMIT ${batchSize}`;
        const stmt = this.db.prepare(query);
        const rows = stmt.all(...params);
        return rows.map((row) => ({
            id: row.id,
            ts: row.ts,
            fromSkillId: row.from_skill_id,
            toSkillId: row.to_skill_id,
            weight: row.weight,
            coOccurrences: row.co_occurrences,
        }));
    }
    /**
     * Full sync of all data
     */
    async syncFull(request) {
        const [episodes, skills, edges] = await Promise.all([
            this.syncEpisodes(request),
            this.syncSkills(request),
            this.syncEdges(request),
        ]);
        return {
            episodes,
            skills,
            edges,
        };
    }
    /**
     * Start cleanup interval for stale connections
     */
    startCleanupInterval() {
        this.cleanupInterval = setInterval(() => {
            const now = Date.now();
            const staleThreshold = 5 * 60 * 1000; // 5 minutes
            for (const [clientId, connection] of this.connections.entries()) {
                if (now - connection.lastRequestAt > staleThreshold && connection.requestCount > 0) {
                    console.log(chalk.gray(`🧹 Removing stale connection: ${clientId}`));
                    this.connections.delete(clientId);
                    this.rateLimitState.delete(clientId);
                }
            }
        }, 60000); // Run every minute
    }
    /**
     * Get server status
     */
    getStatus() {
        let totalRequests = 0;
        for (const connection of this.connections.values()) {
            totalRequests += connection.requestCount;
        }
        return {
            isRunning: this.isRunning,
            activeConnections: this.connections.size,
            totalRequests,
            config: this.config,
        };
    }
    /**
     * Get connection info
     */
    getConnections() {
        return Array.from(this.connections.values());
    }
}
//# sourceMappingURL=QUICServer.js.map