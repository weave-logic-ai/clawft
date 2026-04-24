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
import fs from 'fs/promises';
import path from 'path';
import { createWriteStream } from 'fs';
/**
 * Default configuration
 */
const DEFAULT_CONFIG = {
    enabled: true,
    logDirectory: './logs/audit',
    maxFileSize: 10 * 1024 * 1024, // 10 MB
    maxFiles: 10,
    logToConsole: process.env.NODE_ENV !== 'production',
    logToFile: true,
};
/**
 * Audit Logger Service
 */
export class AuditLogger {
    config;
    currentLogFile;
    currentFileSize = 0;
    eventCount = 0;
    constructor(config = {}) {
        this.config = { ...DEFAULT_CONFIG, ...config };
        if (this.config.enabled && this.config.logToFile) {
            this.initializeLogDirectory();
        }
    }
    /**
     * Initialize log directory
     */
    async initializeLogDirectory() {
        try {
            await fs.mkdir(this.config.logDirectory, { recursive: true });
        }
        catch (error) {
            console.error('[Audit Logger] Failed to create log directory:', error);
        }
    }
    /**
     * Get current log file path
     */
    getCurrentLogFilePath() {
        const timestamp = new Date().toISOString().split('T')[0];
        return path.join(this.config.logDirectory, `audit-${timestamp}.log`);
    }
    /**
     * Rotate log files if necessary
     */
    async rotateLogsIfNeeded() {
        if (this.currentFileSize < this.config.maxFileSize) {
            return;
        }
        // Close current log file
        if (this.currentLogFile) {
            this.currentLogFile.end();
            this.currentLogFile = undefined;
        }
        // Rename current file with timestamp
        const currentPath = this.getCurrentLogFilePath();
        const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
        const rotatedPath = currentPath.replace('.log', `.${timestamp}.log`);
        try {
            await fs.rename(currentPath, rotatedPath);
            this.currentFileSize = 0;
            // Clean up old files
            await this.cleanupOldLogs();
        }
        catch (error) {
            console.error('[Audit Logger] Failed to rotate logs:', error);
        }
    }
    /**
     * Clean up old log files
     */
    async cleanupOldLogs() {
        try {
            const files = await fs.readdir(this.config.logDirectory);
            const auditFiles = files
                .filter(f => f.startsWith('audit-') && f.endsWith('.log'))
                .map(f => ({
                name: f,
                path: path.join(this.config.logDirectory, f),
            }));
            // Sort by modification time (newest first)
            const fileStats = await Promise.all(auditFiles.map(async (f) => ({
                ...f,
                stats: await fs.stat(f.path),
            })));
            fileStats.sort((a, b) => b.stats.mtimeMs - a.stats.mtimeMs);
            // Delete files beyond maxFiles limit
            const filesToDelete = fileStats.slice(this.config.maxFiles);
            await Promise.all(filesToDelete.map(f => fs.unlink(f.path).catch(err => console.error(`[Audit Logger] Failed to delete ${f.name}:`, err))));
        }
        catch (error) {
            console.error('[Audit Logger] Failed to cleanup old logs:', error);
        }
    }
    /**
     * Get log file stream
     */
    getLogStream() {
        if (!this.currentLogFile) {
            const logPath = this.getCurrentLogFilePath();
            this.currentLogFile = createWriteStream(logPath, { flags: 'a' });
        }
        return this.currentLogFile;
    }
    /**
     * Determine event severity
     */
    getSeverity(event) {
        switch (event.type) {
            case 'login_failed':
            case 'jwt_auth_failed':
            case 'api_key_auth_failed':
                return 'medium';
            case 'authorization_failed':
            case 'rate_limit_exceeded':
                return 'medium';
            case 'api_key_revoked':
            case 'password_change':
                return 'high';
            case 'auth_error':
                return 'critical';
            case 'sensitive_data_access':
            case 'config_change':
                return 'high';
            default:
                return 'low';
        }
    }
    /**
     * Format audit event message
     */
    formatMessage(event) {
        const parts = [event.type.toUpperCase()];
        if (event.userId)
            parts.push(`user=${event.userId}`);
        if (event.ip)
            parts.push(`ip=${event.ip}`);
        if (event.path)
            parts.push(`path=${event.path}`);
        if (event.method)
            parts.push(`method=${event.method}`);
        if (event.reason)
            parts.push(`reason=${event.reason}`);
        return parts.join(' ');
    }
    /**
     * Log audit event
     */
    async logEvent(event) {
        if (!this.config.enabled) {
            return;
        }
        const severity = this.getSeverity(event);
        const message = this.formatMessage(event);
        const logEntry = {
            id: `${Date.now()}-${++this.eventCount}`,
            event,
            severity,
            message,
        };
        // Log to console
        if (this.config.logToConsole) {
            const color = {
                low: '\x1b[32m', // Green
                medium: '\x1b[33m', // Yellow
                high: '\x1b[35m', // Magenta
                critical: '\x1b[31m', // Red
            }[severity];
            console.log(`${color}[AUDIT ${severity.toUpperCase()}]\x1b[0m ${message}`, event.metadata || '');
        }
        // Log to file
        if (this.config.logToFile) {
            try {
                await this.rotateLogsIfNeeded();
                const logStream = this.getLogStream();
                const logLine = JSON.stringify(logEntry) + '\n';
                logStream.write(logLine);
                this.currentFileSize += Buffer.byteLength(logLine);
            }
            catch (error) {
                console.error('[Audit Logger] Failed to write to log file:', error);
            }
        }
    }
    /**
     * Log authentication event
     */
    async logAuthEvent(event) {
        await this.logEvent({
            ...event,
            timestamp: event.timestamp || new Date(),
        });
    }
    /**
     * Log rate limit event
     */
    async logRateLimitEvent(event) {
        await this.logEvent({
            ...event,
            metadata: { limitType: event.limitType },
        });
    }
    /**
     * Log API key event
     */
    async logApiKeyEvent(event) {
        await this.logEvent({
            type: event.type,
            userId: event.userId,
            reason: event.reason,
            timestamp: new Date(),
            metadata: {
                keyId: event.keyId,
                environment: event.environment,
            },
        });
    }
    /**
     * Log configuration change
     */
    async logConfigChange(event) {
        await this.logEvent({
            type: 'config_change',
            userId: event.userId,
            reason: event.reason,
            timestamp: new Date(),
            metadata: {
                setting: event.setting,
                oldValue: event.oldValue,
                newValue: event.newValue,
            },
        });
    }
    /**
     * Log sensitive data access
     */
    async logSensitiveDataAccess(event) {
        await this.logEvent({
            type: 'sensitive_data_access',
            userId: event.userId,
            ip: event.ip,
            userAgent: event.userAgent,
            timestamp: new Date(),
            metadata: {
                dataType: event.dataType,
                recordId: event.recordId,
            },
        });
    }
    /**
     * Query audit logs
     */
    async queryLogs(filter) {
        // In production, this would query a database
        // For now, read from log files
        const logs = [];
        try {
            const files = await fs.readdir(this.config.logDirectory);
            const auditFiles = files.filter(f => f.startsWith('audit-') && f.endsWith('.log'));
            for (const file of auditFiles) {
                const content = await fs.readFile(path.join(this.config.logDirectory, file), 'utf-8');
                const lines = content.split('\n').filter(Boolean);
                for (const line of lines) {
                    try {
                        const entry = JSON.parse(line);
                        // Apply filters
                        if (filter.startDate && new Date(entry.event.timestamp) < filter.startDate) {
                            continue;
                        }
                        if (filter.endDate && new Date(entry.event.timestamp) > filter.endDate) {
                            continue;
                        }
                        if (filter.userId && entry.event.userId !== filter.userId) {
                            continue;
                        }
                        if (filter.eventType && entry.event.type !== filter.eventType) {
                            continue;
                        }
                        if (filter.severity && entry.severity !== filter.severity) {
                            continue;
                        }
                        logs.push(entry);
                    }
                    catch (error) {
                        // Skip malformed log lines
                    }
                }
            }
        }
        catch (error) {
            console.error('[Audit Logger] Failed to query logs:', error);
        }
        return logs.sort((a, b) => new Date(b.event.timestamp).getTime() - new Date(a.event.timestamp).getTime());
    }
    /**
     * Close logger
     */
    close() {
        if (this.currentLogFile) {
            this.currentLogFile.end();
            this.currentLogFile = undefined;
        }
    }
}
/**
 * Global audit logger instance
 */
export const auditLogger = new AuditLogger();
//# sourceMappingURL=audit-logger.service.js.map