/**
 * Audit Logger
 *
 * Comprehensive audit logging for security and compliance:
 * - All API requests
 * - Authentication events
 * - Security violations
 * - Performance metrics
 */
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
export class AuditLogger {
    config;
    memoryLog;
    static instance;
    constructor(config) {
        this.config = {
            enableConsole: config?.enableConsole ?? true,
            enableFile: config?.enableFile ?? false,
            logFilePath: config?.logFilePath ?? './logs/audit.log',
            minSeverity: config?.minSeverity ?? 'INFO',
            maxMemoryEntries: config?.maxMemoryEntries ?? 1000,
        };
        this.memoryLog = [];
    }
    /**
     * Get singleton instance
     */
    static getInstance(config) {
        if (!AuditLogger.instance) {
            AuditLogger.instance = new AuditLogger(config);
        }
        return AuditLogger.instance;
    }
    /**
     * Log event
     *
     * @param event - Audit event to log
     */
    log(event) {
        // Check severity filter
        const severities = ['INFO', 'WARNING', 'ERROR', 'CRITICAL'];
        const minIndex = severities.indexOf(this.config.minSeverity);
        const eventIndex = severities.indexOf(event.severity);
        if (eventIndex < minIndex) {
            return;
        }
        // Add to memory log
        this.memoryLog.push(event);
        if (this.memoryLog.length > this.config.maxMemoryEntries) {
            this.memoryLog.shift();
        }
        // Console logging
        if (this.config.enableConsole) {
            this.logToConsole(event);
        }
        // File logging (in production, use Winston or Pino)
        if (this.config.enableFile) {
            this.logToFile(event);
        }
    }
    /**
     * Log API request
     */
    logRequest(req, res, latencyMs) {
        this.log({
            timestamp: Date.now(),
            eventType: 'REQUEST',
            severity: res.statusCode >= 500 ? 'ERROR' : res.statusCode >= 400 ? 'WARNING' : 'INFO',
            userId: req.user?.id,
            ip: req.ip || req.connection.remoteAddress,
            method: req.method,
            path: req.path,
            statusCode: res.statusCode,
            latencyMs,
            message: `${req.method} ${req.path} - ${res.statusCode} (${latencyMs}ms)`,
            metadata: {
                userAgent: req.headers['user-agent'],
                query: req.query,
            },
        });
    }
    /**
     * Log authentication event
     */
    logAuth(success, userId, ip, reason) {
        this.log({
            timestamp: Date.now(),
            eventType: 'AUTH',
            severity: success ? 'INFO' : 'WARNING',
            userId,
            ip,
            message: success
                ? `Authentication successful for user ${userId}`
                : `Authentication failed: ${reason}`,
            metadata: { success, reason },
        });
    }
    /**
     * Log security violation
     */
    logSecurityViolation(type, userId, ip, details) {
        this.log({
            timestamp: Date.now(),
            eventType: 'SECURITY',
            severity: 'WARNING',
            userId,
            ip,
            message: `Security violation: ${type}`,
            metadata: { type, details },
        });
    }
    /**
     * Log error
     */
    logError(error, userId, ip, context) {
        this.log({
            timestamp: Date.now(),
            eventType: 'ERROR',
            severity: 'ERROR',
            userId,
            ip,
            message: error.message,
            metadata: {
                error: error.name,
                stack: error.stack,
                ...context,
            },
        });
    }
    /**
     * Get recent logs
     */
    getRecentLogs(limit = 100) {
        return this.memoryLog.slice(-limit);
    }
    /**
     * Get logs by user
     */
    getLogsByUser(userId, limit = 100) {
        return this.memoryLog
            .filter(event => event.userId === userId)
            .slice(-limit);
    }
    /**
     * Get logs by type
     */
    getLogsByType(eventType, limit = 100) {
        return this.memoryLog
            .filter(event => event.eventType === eventType)
            .slice(-limit);
    }
    /**
     * Log to console
     */
    logToConsole(event) {
        const timestamp = new Date(event.timestamp).toISOString();
        const prefix = `[${timestamp}] [${event.severity}] [${event.eventType}]`;
        const logFn = event.severity === 'ERROR' || event.severity === 'CRITICAL'
            ? console.error
            : event.severity === 'WARNING'
                ? console.warn
                : console.log;
        logFn(`${prefix} ${event.message}`);
        if (event.metadata) {
            logFn('  Metadata:', event.metadata);
        }
    }
    /**
     * Log to file (placeholder - use Winston/Pino in production)
     */
    logToFile(event) {
        // In production, use Winston or Pino for file logging
        // This is a placeholder
        const line = JSON.stringify(event) + '\n';
        // fs.appendFileSync(this.config.logFilePath, line);
    }
    /**
     * Clear memory logs
     */
    clear() {
        this.memoryLog = [];
    }
    /**
     * Get statistics
     */
    getStats() {
        const stats = {
            totalEvents: this.memoryLog.length,
            byType: {},
            bySeverity: {},
            errorRate: 0,
        };
        for (const event of this.memoryLog) {
            stats.byType[event.eventType] = (stats.byType[event.eventType] || 0) + 1;
            stats.bySeverity[event.severity] = (stats.bySeverity[event.severity] || 0) + 1;
        }
        const errorCount = (stats.bySeverity['ERROR'] || 0) + (stats.bySeverity['CRITICAL'] || 0);
        stats.errorRate = stats.totalEvents > 0 ? errorCount / stats.totalEvents : 0;
        return stats;
    }
}
/**
 * Create audit logging middleware
 */
export function createAuditMiddleware(logger) {
    const auditLogger = logger || AuditLogger.getInstance();
    return (req, res, next) => {
        const startTime = Date.now();
        // Log after response finishes
        res.on('finish', () => {
            const latencyMs = Date.now() - startTime;
            auditLogger.logRequest(req, res, latencyMs);
        });
        next();
    };
}
// Export singleton
export const auditLogger = AuditLogger.getInstance();
//# sourceMappingURL=audit-logger.js.map