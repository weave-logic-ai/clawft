/**
 * SDK Session Manager - Manages Claude Agent SDK session lifecycle
 *
 * Provides session ID capture, resume capability, and session forking
 * for maintaining context across multiple queries.
 */
import { logger } from "../utils/logger.js";
// Session TTL: 30 minutes of inactivity
const SESSION_TTL_MS = 30 * 60 * 1000;
// In-memory session storage
const activeSessions = new Map();
// Current session for quick access
let currentSessionId = null;
// Cleanup stale sessions periodically
function cleanupStaleSessions() {
    const now = Date.now();
    for (const [id, session] of activeSessions.entries()) {
        if (now - session.lastActivity > SESSION_TTL_MS) {
            activeSessions.delete(id);
            if (currentSessionId === id) {
                currentSessionId = null;
            }
        }
    }
}
// Run cleanup every 5 minutes
setInterval(cleanupStaleSessions, 5 * 60 * 1000).unref();
/**
 * Capture session ID from SDK init message
 * Call this for every message received from query()
 */
export function captureSessionId(message) {
    if (message.type === 'system' &&
        message.subtype === 'init' &&
        message.session_id) {
        const sessionId = message.session_id;
        // Store session info
        activeSessions.set(sessionId, {
            sessionId,
            startTime: Date.now(),
            messageCount: 1,
            lastActivity: Date.now(),
            resumed: false
        });
        currentSessionId = sessionId;
        logger.info('Session captured', {
            sessionId,
            model: message.model,
            tools: message.tools?.length
        });
        return sessionId;
    }
    // Update message count for existing session
    if (message.session_id && activeSessions.has(message.session_id)) {
        const session = activeSessions.get(message.session_id);
        session.messageCount++;
        session.lastActivity = Date.now();
    }
    return null;
}
/**
 * Get the current active session ID
 */
export function getCurrentSessionId() {
    return currentSessionId;
}
/**
 * Get session info by ID
 */
export function getSessionInfo(sessionId) {
    return activeSessions.get(sessionId);
}
/**
 * Get all active sessions
 */
export function getActiveSessions() {
    return Array.from(activeSessions.values());
}
/**
 * Get resume options for continuing a session
 * @param sessionId - Session ID to resume, or uses current session if not provided
 */
export function getResumeOptions(sessionId) {
    const id = sessionId || currentSessionId;
    if (!id) {
        return {};
    }
    // Mark session as resumed
    const session = activeSessions.get(id);
    if (session) {
        session.resumed = true;
    }
    logger.info('Preparing session resume', { sessionId: id });
    return {
        resume: id
    };
}
/**
 * Get fork options for creating a new session branch from an existing one
 * @param sessionId - Session ID to fork from
 */
export function getForkOptions(sessionId) {
    logger.info('Forking session', { sourceSessionId: sessionId });
    return {
        resume: sessionId,
        forkSession: true
    };
}
/**
 * Mark a session as ended
 */
export function endSession(sessionId) {
    if (activeSessions.has(sessionId)) {
        const session = activeSessions.get(sessionId);
        logger.info('Session ended', {
            sessionId,
            duration: Date.now() - session.startTime,
            messageCount: session.messageCount
        });
        activeSessions.delete(sessionId);
        if (currentSessionId === sessionId) {
            currentSessionId = null;
        }
    }
}
/**
 * Clear all sessions (for testing/reset)
 */
export function clearAllSessions() {
    activeSessions.clear();
    currentSessionId = null;
    logger.info('All sessions cleared');
}
/**
 * Get session statistics
 */
export function getSessionStats() {
    const sessions = Array.from(activeSessions.values());
    return {
        totalSessions: sessions.length,
        activeSessions: sessions.filter(s => !s.resumed).length,
        currentSessionId,
        totalMessages: sessions.reduce((sum, s) => sum + s.messageCount, 0)
    };
}
/**
 * Process result message and extract session info
 */
export function processResultMessage(message) {
    if (message.type !== 'result')
        return null;
    const resultMsg = message;
    return {
        success: resultMsg.subtype === 'success',
        sessionId: resultMsg.session_id,
        duration: resultMsg.duration_ms,
        cost: resultMsg.total_cost_usd,
        result: resultMsg.result,
        errors: resultMsg.errors
    };
}
/**
 * Build query options with session support
 */
export function buildQueryOptionsWithSession(baseOptions, options = {}) {
    const { resumeSession, sessionId, forkSession } = options;
    if (forkSession && sessionId) {
        return {
            ...baseOptions,
            ...getForkOptions(sessionId)
        };
    }
    if (resumeSession) {
        return {
            ...baseOptions,
            ...getResumeOptions(sessionId)
        };
    }
    return baseOptions;
}
//# sourceMappingURL=session-manager.js.map