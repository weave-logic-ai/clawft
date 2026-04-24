/**
 * Audit Logger
 *
 * Comprehensive audit logging for security and compliance:
 * - All API requests
 * - Authentication events
 * - Security violations
 * - Performance metrics
 */
export interface AuditEvent {
    timestamp: number;
    eventType: 'REQUEST' | 'AUTH' | 'ERROR' | 'SECURITY' | 'PERFORMANCE';
    severity: 'INFO' | 'WARNING' | 'ERROR' | 'CRITICAL';
    userId?: string;
    ip?: string;
    method?: string;
    path?: string;
    statusCode?: number;
    latencyMs?: number;
    message: string;
    metadata?: Record<string, any>;
}
export interface AuditLoggerConfig {
    /** Enable console logging */
    enableConsole?: boolean;
    /** Enable file logging */
    enableFile?: boolean;
    /** Log file path */
    logFilePath?: string;
    /** Minimum severity to log */
    minSeverity?: 'INFO' | 'WARNING' | 'ERROR' | 'CRITICAL';
    /** Maximum log entries in memory */
    maxMemoryEntries?: number;
}
/**
 * Audit Logger
 *
 * Logs all security-relevant events:
 * - API requests and responses
 * - Authentication attempts
 * - Rate limit violations
 * - Input validation failures
 * - Circuit breaker state changes
 */
export declare class AuditLogger {
    private config;
    private memoryLog;
    private static instance;
    constructor(config?: AuditLoggerConfig);
    /**
     * Get singleton instance
     */
    static getInstance(config?: AuditLoggerConfig): AuditLogger;
    /**
     * Log event
     *
     * @param event - Audit event to log
     */
    log(event: AuditEvent): void;
    /**
     * Log API request
     */
    logRequest(req: any, res: any, latencyMs: number): void;
    /**
     * Log authentication event
     */
    logAuth(success: boolean, userId?: string, ip?: string, reason?: string): void;
    /**
     * Log security violation
     */
    logSecurityViolation(type: 'RATE_LIMIT' | 'INPUT_VALIDATION' | 'AUTH_FAILURE' | 'SUSPICIOUS_ACTIVITY', userId?: string, ip?: string, details?: string): void;
    /**
     * Log error
     */
    logError(error: Error, userId?: string, ip?: string, context?: Record<string, any>): void;
    /**
     * Get recent logs
     */
    getRecentLogs(limit?: number): AuditEvent[];
    /**
     * Get logs by user
     */
    getLogsByUser(userId: string, limit?: number): AuditEvent[];
    /**
     * Get logs by type
     */
    getLogsByType(eventType: AuditEvent['eventType'], limit?: number): AuditEvent[];
    /**
     * Log to console
     */
    private logToConsole;
    /**
     * Log to file (placeholder - use Winston/Pino in production)
     */
    private logToFile;
    /**
     * Clear memory logs
     */
    clear(): void;
    /**
     * Get statistics
     */
    getStats(): {
        totalEvents: number;
        byType: Record<string, number>;
        bySeverity: Record<string, number>;
        errorRate: number;
    };
}
/**
 * Create audit logging middleware
 */
export declare function createAuditMiddleware(logger?: AuditLogger): (req: any, res: any, next: any) => void;
export declare const auditLogger: AuditLogger;
//# sourceMappingURL=audit-logger.d.ts.map