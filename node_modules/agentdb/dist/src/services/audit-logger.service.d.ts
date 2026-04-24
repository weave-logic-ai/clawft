/**
 * Audit Logger Service for AgentDB
 *
 * Comprehensive audit logging for security events:
 * - Authentication events (login, logout, failed attempts)
 * - Authorization failures
 * - API key operations
 * - Rate limiting events
 * - Sensitive data access
 * - Configuration changes
 *
 * Features:
 * - Structured logging with timestamps
 * - Automatic log rotation
 * - Configurable log levels
 * - Performance metrics
 * - Compliance support (SOC2, GDPR, HIPAA)
 */
/**
 * Audit event types
 */
export type AuditEventType = 'jwt_auth_success' | 'jwt_auth_failed' | 'api_key_auth_success' | 'api_key_auth_failed' | 'login_success' | 'login_failed' | 'logout' | 'registration' | 'password_change' | 'api_key_created' | 'api_key_revoked' | 'api_key_rotated' | 'authorization_failed' | 'rate_limit_exceeded' | 'sensitive_data_access' | 'config_change' | 'session_expired' | 'auth_error';
/**
 * Audit event interface
 */
export interface AuditEvent {
    type: AuditEventType;
    timestamp: Date;
    userId?: string;
    ip?: string;
    userAgent?: string;
    path?: string;
    method?: string;
    statusCode?: number;
    reason?: string;
    metadata?: Record<string, any>;
}
/**
 * Audit log entry
 */
interface AuditLogEntry {
    id: string;
    event: AuditEvent;
    severity: 'low' | 'medium' | 'high' | 'critical';
    message: string;
}
/**
 * Audit logger configuration
 */
interface AuditLoggerConfig {
    enabled: boolean;
    logDirectory: string;
    maxFileSize: number;
    maxFiles: number;
    logToConsole: boolean;
    logToFile: boolean;
}
/**
 * Audit Logger Service
 */
export declare class AuditLogger {
    private config;
    private currentLogFile?;
    private currentFileSize;
    private eventCount;
    constructor(config?: Partial<AuditLoggerConfig>);
    /**
     * Initialize log directory
     */
    private initializeLogDirectory;
    /**
     * Get current log file path
     */
    private getCurrentLogFilePath;
    /**
     * Rotate log files if necessary
     */
    private rotateLogsIfNeeded;
    /**
     * Clean up old log files
     */
    private cleanupOldLogs;
    /**
     * Get log file stream
     */
    private getLogStream;
    /**
     * Determine event severity
     */
    private getSeverity;
    /**
     * Format audit event message
     */
    private formatMessage;
    /**
     * Log audit event
     */
    logEvent(event: AuditEvent): Promise<void>;
    /**
     * Log authentication event
     */
    logAuthEvent(event: Omit<AuditEvent, 'timestamp'> & {
        timestamp?: Date;
    }): Promise<void>;
    /**
     * Log rate limit event
     */
    logRateLimitEvent(event: {
        type: 'rate_limit_exceeded';
        limitType: string;
        userId?: string;
        ip?: string;
        userAgent?: string;
        path?: string;
        method?: string;
        timestamp: Date;
    }): Promise<void>;
    /**
     * Log API key event
     */
    logApiKeyEvent(event: {
        type: 'api_key_created' | 'api_key_revoked' | 'api_key_rotated';
        userId: string;
        keyId: string;
        environment: 'live' | 'test';
        reason?: string;
    }): Promise<void>;
    /**
     * Log configuration change
     */
    logConfigChange(event: {
        userId: string;
        setting: string;
        oldValue?: any;
        newValue?: any;
        reason?: string;
    }): Promise<void>;
    /**
     * Log sensitive data access
     */
    logSensitiveDataAccess(event: {
        userId: string;
        dataType: string;
        recordId?: string;
        ip?: string;
        userAgent?: string;
    }): Promise<void>;
    /**
     * Query audit logs
     */
    queryLogs(filter: {
        startDate?: Date;
        endDate?: Date;
        userId?: string;
        eventType?: AuditEventType;
        severity?: 'low' | 'medium' | 'high' | 'critical';
    }): Promise<AuditLogEntry[]>;
    /**
     * Close logger
     */
    close(): void;
}
/**
 * Global audit logger instance
 */
export declare const auditLogger: AuditLogger;
export {};
//# sourceMappingURL=audit-logger.service.d.ts.map