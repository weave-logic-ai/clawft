/**
 * SDK Session Manager - Manages Claude Agent SDK session lifecycle
 *
 * Provides session ID capture, resume capability, and session forking
 * for maintaining context across multiple queries.
 */
interface SessionInfo {
    sessionId: string;
    startTime: number;
    messageCount: number;
    lastActivity: number;
    agentName?: string;
    resumed: boolean;
}
/**
 * SDK Message types (from documentation)
 */
interface SDKSystemMessage {
    type: 'system';
    subtype: 'init' | 'compact_boundary';
    uuid: string;
    session_id: string;
    apiKeySource?: string;
    cwd?: string;
    tools?: string[];
    model?: string;
    permissionMode?: string;
}
interface SDKResultMessage {
    type: 'result';
    subtype: 'success' | 'error_max_turns' | 'error_during_execution' | 'error_max_budget_usd';
    uuid: string;
    session_id: string;
    duration_ms: number;
    total_cost_usd: number;
    result?: string;
    errors?: string[];
}
type SDKMessage = SDKSystemMessage | SDKResultMessage | {
    type: string;
    session_id?: string;
};
/**
 * Capture session ID from SDK init message
 * Call this for every message received from query()
 */
export declare function captureSessionId(message: SDKMessage): string | null;
/**
 * Get the current active session ID
 */
export declare function getCurrentSessionId(): string | null;
/**
 * Get session info by ID
 */
export declare function getSessionInfo(sessionId: string): SessionInfo | undefined;
/**
 * Get all active sessions
 */
export declare function getActiveSessions(): SessionInfo[];
/**
 * Get resume options for continuing a session
 * @param sessionId - Session ID to resume, or uses current session if not provided
 */
export declare function getResumeOptions(sessionId?: string): {
    resume?: string;
};
/**
 * Get fork options for creating a new session branch from an existing one
 * @param sessionId - Session ID to fork from
 */
export declare function getForkOptions(sessionId: string): {
    resume: string;
    forkSession: boolean;
};
/**
 * Mark a session as ended
 */
export declare function endSession(sessionId: string): void;
/**
 * Clear all sessions (for testing/reset)
 */
export declare function clearAllSessions(): void;
/**
 * Get session statistics
 */
export declare function getSessionStats(): {
    totalSessions: number;
    activeSessions: number;
    currentSessionId: string | null;
    totalMessages: number;
};
/**
 * Process result message and extract session info
 */
export declare function processResultMessage(message: SDKMessage): {
    success: boolean;
    sessionId?: string;
    duration?: number;
    cost?: number;
    result?: string;
    errors?: string[];
} | null;
/**
 * Build query options with session support
 */
export declare function buildQueryOptionsWithSession(baseOptions: Record<string, any>, options?: {
    resumeSession?: boolean;
    sessionId?: string;
    forkSession?: boolean;
}): Record<string, any>;
export {};
//# sourceMappingURL=session-manager.d.ts.map