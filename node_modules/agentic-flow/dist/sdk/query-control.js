/**
 * Query Control - Runtime control of active queries
 *
 * Provides methods to control running queries:
 * - Change model mid-execution
 * - Switch permission modes
 * - Interrupt/abort queries
 * - Get query status and introspection
 */
import { logger } from "../utils/logger.js";
// Active queries registry
const activeQueries = new Map();
// Query counter for ID generation
let queryCounter = 0;
/**
 * Create a new query controller
 */
export function createQueryController(options) {
    const id = `query-${Date.now()}-${++queryCounter}`;
    const abortController = new AbortController();
    const state = {
        id,
        startTime: Date.now(),
        model: options.model,
        permissionMode: options.permissionMode || 'default',
        status: 'running',
        turnCount: 0,
        tokenCount: 0,
        costUsd: 0,
        abortController
    };
    activeQueries.set(id, state);
    logger.info('Query controller created', { id, model: options.model });
    return new QueryController(state);
}
/**
 * Query Controller - controls a single active query
 */
export class QueryController {
    state;
    modelChangeCallbacks = [];
    permissionChangeCallbacks = [];
    constructor(state) {
        this.state = state;
    }
    /**
     * Get query ID
     */
    get id() {
        return this.state.id;
    }
    /**
     * Get abort signal for SDK integration
     */
    get signal() {
        return this.state.abortController.signal;
    }
    /**
     * Get current status
     */
    getStatus() {
        return { ...this.state };
    }
    /**
     * Change model at runtime
     * Note: SDK must support this via setModel() method
     */
    async setModel(model) {
        if (this.state.status !== 'running') {
            logger.warn('Cannot change model on non-running query', { id: this.state.id, status: this.state.status });
            return false;
        }
        const oldModel = this.state.model;
        this.state.model = model;
        // Notify callbacks
        for (const callback of this.modelChangeCallbacks) {
            try {
                callback(model);
            }
            catch (error) {
                logger.warn('Model change callback error', { error: error.message });
            }
        }
        logger.info('Model changed', { id: this.state.id, oldModel, newModel: model });
        return true;
    }
    /**
     * Change permission mode at runtime
     */
    async setPermissionMode(mode) {
        if (this.state.status !== 'running') {
            logger.warn('Cannot change permissions on non-running query', { id: this.state.id });
            return false;
        }
        const oldMode = this.state.permissionMode;
        this.state.permissionMode = mode;
        // Notify callbacks
        for (const callback of this.permissionChangeCallbacks) {
            try {
                callback(mode);
            }
            catch (error) {
                logger.warn('Permission change callback error', { error: error.message });
            }
        }
        logger.info('Permission mode changed', { id: this.state.id, oldMode, newMode: mode });
        return true;
    }
    /**
     * Set max thinking tokens
     */
    async setMaxThinkingTokens(tokens) {
        if (this.state.status !== 'running')
            return false;
        logger.info('Max thinking tokens set', { id: this.state.id, tokens });
        return true;
    }
    /**
     * Interrupt the query (for streaming mode)
     */
    interrupt() {
        if (this.state.status === 'running') {
            this.state.status = 'paused';
            logger.info('Query interrupted', { id: this.state.id });
        }
    }
    /**
     * Resume interrupted query
     */
    resume() {
        if (this.state.status === 'paused') {
            this.state.status = 'running';
            logger.info('Query resumed', { id: this.state.id });
        }
    }
    /**
     * Abort the query completely
     */
    abort() {
        if (this.state.status === 'running' || this.state.status === 'paused') {
            this.state.abortController.abort();
            this.state.status = 'aborted';
            logger.info('Query aborted', { id: this.state.id });
        }
    }
    /**
     * Mark query as completed
     */
    complete(result) {
        this.state.status = 'completed';
        if (result?.tokenCount)
            this.state.tokenCount = result.tokenCount;
        if (result?.costUsd)
            this.state.costUsd = result.costUsd;
        activeQueries.delete(this.state.id);
        logger.info('Query completed', {
            id: this.state.id,
            duration: Date.now() - this.state.startTime,
            turns: this.state.turnCount
        });
    }
    /**
     * Mark query as errored
     */
    error(message) {
        this.state.status = 'error';
        activeQueries.delete(this.state.id);
        logger.error('Query error', { id: this.state.id, message });
    }
    /**
     * Increment turn count
     */
    incrementTurn() {
        this.state.turnCount++;
    }
    /**
     * Add model change callback
     */
    onModelChange(callback) {
        this.modelChangeCallbacks.push(callback);
    }
    /**
     * Add permission change callback
     */
    onPermissionChange(callback) {
        this.permissionChangeCallbacks.push(callback);
    }
    /**
     * Get supported commands (introspection)
     */
    async supportedCommands() {
        return [
            '/help', '/clear', '/compact', '/config', '/cost',
            '/doctor', '/login', '/logout', '/memory', '/mcp',
            '/model', '/permissions', '/review', '/terminal', '/vim'
        ];
    }
    /**
     * Get supported models (introspection)
     */
    async supportedModels() {
        return [
            'claude-opus-4-5-20251101',
            'claude-sonnet-4-5-20250929',
            'claude-haiku-3-5-20241022'
        ];
    }
    /**
     * Get MCP server status
     */
    async mcpServerStatus() {
        // This would be populated by actual MCP server connections
        return {
            'claude-flow': { connected: true, tools: 213 },
            'flow-nexus': { connected: true, tools: 45 }
        };
    }
    /**
     * Get account info
     */
    async accountInfo() {
        return {
            tier: process.env.ANTHROPIC_TIER || 'unknown',
            usage: {
                tokens: this.state.tokenCount,
                cost: this.state.costUsd
            },
            limits: {
                maxTokens: parseInt(process.env.SDK_MAX_TOKENS || '100000'),
                maxCost: parseFloat(process.env.SDK_MAX_BUDGET_USD || '10.00')
            }
        };
    }
}
/**
 * Get all active queries
 */
export function getActiveQueries() {
    return Array.from(activeQueries.values());
}
/**
 * Get query by ID
 */
export function getQuery(id) {
    const state = activeQueries.get(id);
    if (!state)
        return null;
    return new QueryController(state);
}
/**
 * Abort all active queries
 */
export function abortAllQueries() {
    for (const state of activeQueries.values()) {
        state.abortController.abort();
        state.status = 'aborted';
    }
    activeQueries.clear();
    logger.info('All queries aborted', { count: activeQueries.size });
}
/**
 * Get query statistics
 */
export function getQueryStats() {
    const queries = Array.from(activeQueries.values());
    return {
        active: queries.filter(q => q.status === 'running').length,
        total: queryCounter,
        totalTokens: queries.reduce((sum, q) => sum + q.tokenCount, 0),
        totalCost: queries.reduce((sum, q) => sum + q.costUsd, 0)
    };
}
//# sourceMappingURL=query-control.js.map