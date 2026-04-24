/**
 * Ephemeral Agent - Short-lived agent with federated memory access
 *
 * Features:
 * - Automatic lifecycle management (spawn → execute → learn → destroy)
 * - Federated memory sync via QUIC
 * - Tenant isolation with JWT authentication
 * - Memory persistence after agent destruction
 */
import Database from 'better-sqlite3';
import { FederationHub } from './FederationHub.js';
import { SecurityManager } from './SecurityManager.js';
import { logger } from '../utils/logger.js';
export class EphemeralAgent {
    config;
    context;
    hub;
    security;
    cleanupTimer;
    syncTimer;
    constructor(config) {
        this.config = {
            lifetime: 300, // 5 minutes default
            syncInterval: 5000, // 5 seconds default
            enableEncryption: true,
            ...config
        };
        this.security = new SecurityManager();
    }
    /**
     * Spawn a new ephemeral agent with federated memory access
     */
    static async spawn(config) {
        const agent = new EphemeralAgent(config);
        await agent.initialize();
        return agent;
    }
    /**
     * Initialize agent: setup DB, connect to hub, start lifecycle timers
     */
    async initialize() {
        const agentId = `eph-${this.config.tenantId}-${Date.now()}`;
        const spawnTime = Date.now();
        const expiresAt = spawnTime + ((this.config.lifetime || 300) * 1000);
        logger.info('Spawning ephemeral agent', {
            agentId,
            tenantId: this.config.tenantId,
            lifetime: this.config.lifetime,
            expiresAt: new Date(expiresAt).toISOString()
        });
        // Initialize local database instance
        const memoryPath = this.config.memoryPath || `:memory:`;
        // Use better-sqlite3 for now (AgentDB integration planned)
        const db = new Database(memoryPath);
        // Create JWT token for authentication
        const token = await this.security.createAgentToken({
            agentId,
            tenantId: this.config.tenantId,
            expiresAt
        });
        // Connect to federation hub if endpoint provided
        if (this.config.hubEndpoint) {
            this.hub = new FederationHub({
                endpoint: this.config.hubEndpoint,
                agentId,
                tenantId: this.config.tenantId,
                token
            });
            await this.hub.connect();
            // Start periodic sync
            if (this.config.syncInterval) {
                this.syncTimer = setInterval(async () => {
                    await this.syncWithHub();
                }, this.config.syncInterval);
            }
        }
        // Store context
        this.context = {
            agentId,
            tenantId: this.config.tenantId,
            db,
            spawnTime,
            expiresAt
        };
        // Schedule automatic cleanup at expiration
        const timeUntilExpiry = expiresAt - Date.now();
        this.cleanupTimer = setTimeout(async () => {
            logger.warn('Agent lifetime expired, auto-destroying', { agentId });
            await this.destroy();
        }, timeUntilExpiry);
        logger.info('Ephemeral agent spawned successfully', {
            agentId,
            hubConnected: !!this.hub,
            timeUntilExpiry
        });
    }
    /**
     * Execute a task within the agent context
     * Automatically syncs memory before and after execution
     */
    async execute(task) {
        if (!this.context) {
            throw new Error('Agent not initialized. Call spawn() first.');
        }
        const { agentId, db } = this.context;
        // Check if agent has expired
        if (Date.now() >= this.context.expiresAt) {
            throw new Error(`Agent ${agentId} has expired and cannot execute tasks`);
        }
        logger.info('Executing task', { agentId });
        try {
            // Pre-execution sync: pull latest memories from hub
            if (this.hub) {
                await this.syncWithHub();
            }
            // Execute user task
            const result = await task(db, this.context);
            // Post-execution sync: push new memories to hub
            if (this.hub) {
                await this.syncWithHub();
            }
            logger.info('Task execution completed', { agentId });
            return result;
        }
        catch (error) {
            logger.error('Task execution failed', {
                agentId,
                error: error.message
            });
            throw error;
        }
    }
    /**
     * Query memories from federated database
     */
    async queryMemories(task, k = 5) {
        if (!this.context) {
            throw new Error('Agent not initialized');
        }
        const { db, tenantId } = this.context;
        // Query using ReasoningBank pattern search
        const patterns = await db.patternSearch({
            task,
            k,
            tenantId // Apply tenant isolation
        });
        return patterns || [];
    }
    /**
     * Store a learning episode to persistent memory
     */
    async storeEpisode(episode) {
        if (!this.context) {
            throw new Error('Agent not initialized');
        }
        const { db, agentId, tenantId } = this.context;
        // Store episode with tenant isolation
        await db.patternStore({
            sessionId: agentId,
            task: episode.task,
            input: episode.input,
            output: episode.output,
            reward: episode.reward,
            critique: episode.critique || '',
            success: episode.reward >= 0.7,
            tokensUsed: 0,
            latencyMs: 0,
            tenantId // Ensure tenant isolation
        });
        logger.info('Episode stored', {
            agentId,
            task: episode.task,
            reward: episode.reward
        });
    }
    /**
     * Sync local memory with federation hub
     */
    async syncWithHub() {
        if (!this.hub || !this.context) {
            return;
        }
        try {
            await this.hub.sync(this.context.db);
        }
        catch (error) {
            logger.error('Federation sync failed', {
                agentId: this.context.agentId,
                error: error.message
            });
        }
    }
    /**
     * Get remaining lifetime in seconds
     */
    getRemainingLifetime() {
        if (!this.context) {
            return 0;
        }
        return Math.max(0, Math.floor((this.context.expiresAt - Date.now()) / 1000));
    }
    /**
     * Destroy agent and cleanup resources
     * Memory persists in federation hub
     */
    async destroy() {
        if (!this.context) {
            return;
        }
        const { agentId, db } = this.context;
        logger.info('Destroying ephemeral agent', { agentId });
        // Clear timers
        if (this.cleanupTimer) {
            clearTimeout(this.cleanupTimer);
        }
        if (this.syncTimer) {
            clearInterval(this.syncTimer);
        }
        // Final sync to persist any pending changes
        if (this.hub) {
            try {
                await this.syncWithHub();
                await this.hub.disconnect();
            }
            catch (error) {
                logger.error('Final sync failed', {
                    agentId,
                    error: error.message
                });
            }
        }
        // Close local database
        try {
            await db.close?.();
        }
        catch (error) {
            // Ignore close errors for in-memory databases
        }
        // Clear context
        this.context = undefined;
        logger.info('Ephemeral agent destroyed', { agentId });
    }
    /**
     * Check if agent is still alive
     */
    isAlive() {
        if (!this.context) {
            return false;
        }
        return Date.now() < this.context.expiresAt;
    }
    /**
     * Get agent info
     */
    getInfo() {
        return this.context || null;
    }
}
//# sourceMappingURL=EphemeralAgent.js.map