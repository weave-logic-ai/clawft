/**
 * Security Module - Comprehensive security hardening for SDK operations
 *
 * Provides:
 * - Input validation and sanitization
 * - Rate limiting
 * - Audit logging
 * - Secret detection
 * - Path traversal protection
 * - Command injection prevention
 */
/**
 * Validate and sanitize file paths
 */
export declare function sanitizePath(inputPath: string, allowedBases?: string[]): string | null;
/**
 * Validate command for injection attacks
 */
export declare function validateCommand(command: string): {
    valid: boolean;
    reason?: string;
};
/**
 * Sanitize user input for logging
 */
export declare function sanitizeForLog(input: any, maxLength?: number): string;
/**
 * Patterns that indicate potential secrets
 */
declare const SECRET_PATTERNS: {
    pattern: RegExp;
    replacement: string;
}[];
/**
 * Redact secrets from a string
 */
export declare function redactSecrets(text: string): string;
/**
 * Check if text contains potential secrets
 */
export declare function containsSecrets(text: string): boolean;
/**
 * Rate limit configuration
 */
export interface RateLimitConfig {
    maxRequests: number;
    windowMs: number;
}
/**
 * Check rate limit
 */
export declare function checkRateLimit(key: string, config: RateLimitConfig): {
    allowed: boolean;
    remaining: number;
    resetIn: number;
};
/**
 * Create a rate limiter function
 */
export declare function createRateLimiter(config: RateLimitConfig): (key: string) => boolean;
/**
 * Audit log entry
 */
export interface AuditLogEntry {
    timestamp: string;
    event: string;
    actor: string;
    resource: string;
    action: string;
    outcome: 'success' | 'failure' | 'blocked';
    details?: Record<string, any>;
    ip?: string;
    sessionId?: string;
}
declare const AUDIT_LOG_DIR: string;
declare const AUDIT_LOG_FILE: string;
/**
 * Write audit log entry
 */
export declare function auditLog(entry: Omit<AuditLogEntry, 'timestamp'>): void;
/**
 * Audit tool usage
 */
export declare function auditToolUsage(toolName: string, input: any, outcome: 'success' | 'failure' | 'blocked', sessionId?: string): void;
/**
 * Audit permission decision
 */
export declare function auditPermissionDecision(toolName: string, decision: 'allow' | 'deny', reason: string, sessionId?: string): void;
/**
 * Security context for a session
 */
export interface SecurityContext {
    sessionId: string;
    allowedPaths: string[];
    allowedCommands: string[];
    blockedPatterns: RegExp[];
    rateLimit: RateLimitConfig;
    auditEnabled: boolean;
}
/**
 * Default security context
 */
export declare function getDefaultSecurityContext(): SecurityContext;
/**
 * Validate operation against security context
 */
export declare function validateOperation(operation: 'read' | 'write' | 'execute', target: string, context: SecurityContext): {
    allowed: boolean;
    reason?: string;
};
/**
 * Create secure hash of content
 */
export declare function secureHash(content: string, algorithm?: 'sha256' | 'sha512'): string;
/**
 * Generate secure random token
 */
export declare function generateSecureToken(length?: number): string;
export { SECRET_PATTERNS, AUDIT_LOG_DIR, AUDIT_LOG_FILE };
//# sourceMappingURL=security.d.ts.map