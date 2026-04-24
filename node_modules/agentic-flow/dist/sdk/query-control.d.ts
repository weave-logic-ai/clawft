/**
 * Query Control - Runtime control of active queries
 *
 * Provides methods to control running queries:
 * - Change model mid-execution
 * - Switch permission modes
 * - Interrupt/abort queries
 * - Get query status and introspection
 */
/**
 * Query state for tracking active queries
 */
export interface QueryState {
    id: string;
    startTime: number;
    model: string;
    permissionMode: string;
    status: 'running' | 'paused' | 'completed' | 'aborted' | 'error';
    turnCount: number;
    tokenCount: number;
    costUsd: number;
    abortController: AbortController;
}
/**
 * Model options for runtime switching
 */
export type ModelOption = 'claude-opus-4-5-20251101' | 'claude-sonnet-4-5-20250929' | 'claude-haiku-3-5-20241022' | 'inherit';
/**
 * Permission mode options
 */
export type PermissionModeOption = 'default' | 'acceptEdits' | 'bypassPermissions' | 'plan';
/**
 * Create a new query controller
 */
export declare function createQueryController(options: {
    model: string;
    permissionMode?: string;
}): QueryController;
/**
 * Query Controller - controls a single active query
 */
export declare class QueryController {
    private state;
    private modelChangeCallbacks;
    private permissionChangeCallbacks;
    constructor(state: QueryState);
    /**
     * Get query ID
     */
    get id(): string;
    /**
     * Get abort signal for SDK integration
     */
    get signal(): AbortSignal;
    /**
     * Get current status
     */
    getStatus(): QueryState;
    /**
     * Change model at runtime
     * Note: SDK must support this via setModel() method
     */
    setModel(model: ModelOption): Promise<boolean>;
    /**
     * Change permission mode at runtime
     */
    setPermissionMode(mode: PermissionModeOption): Promise<boolean>;
    /**
     * Set max thinking tokens
     */
    setMaxThinkingTokens(tokens: number): Promise<boolean>;
    /**
     * Interrupt the query (for streaming mode)
     */
    interrupt(): void;
    /**
     * Resume interrupted query
     */
    resume(): void;
    /**
     * Abort the query completely
     */
    abort(): void;
    /**
     * Mark query as completed
     */
    complete(result?: {
        tokenCount?: number;
        costUsd?: number;
    }): void;
    /**
     * Mark query as errored
     */
    error(message: string): void;
    /**
     * Increment turn count
     */
    incrementTurn(): void;
    /**
     * Add model change callback
     */
    onModelChange(callback: (model: string) => void): void;
    /**
     * Add permission change callback
     */
    onPermissionChange(callback: (mode: string) => void): void;
    /**
     * Get supported commands (introspection)
     */
    supportedCommands(): Promise<string[]>;
    /**
     * Get supported models (introspection)
     */
    supportedModels(): Promise<string[]>;
    /**
     * Get MCP server status
     */
    mcpServerStatus(): Promise<Record<string, {
        connected: boolean;
        tools: number;
    }>>;
    /**
     * Get account info
     */
    accountInfo(): Promise<{
        tier: string;
        usage: {
            tokens: number;
            cost: number;
        };
        limits: {
            maxTokens: number;
            maxCost: number;
        };
    }>;
}
/**
 * Get all active queries
 */
export declare function getActiveQueries(): QueryState[];
/**
 * Get query by ID
 */
export declare function getQuery(id: string): QueryController | null;
/**
 * Abort all active queries
 */
export declare function abortAllQueries(): void;
/**
 * Get query statistics
 */
export declare function getQueryStats(): {
    active: number;
    total: number;
    totalTokens: number;
    totalCost: number;
};
//# sourceMappingURL=query-control.d.ts.map