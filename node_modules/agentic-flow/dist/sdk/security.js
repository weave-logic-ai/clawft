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
import { logger } from "../utils/logger.js";
import { existsSync, appendFileSync, mkdirSync } from "fs";
import { join, resolve, normalize } from "path";
import { homedir } from "os";
import { createHash, randomBytes } from "crypto";
// =============================================================================
// Input Validation
// =============================================================================
/**
 * Validate and sanitize file paths
 */
export function sanitizePath(inputPath, allowedBases = [process.cwd()]) {
    if (!inputPath || typeof inputPath !== 'string') {
        return null;
    }
    // Normalize the path
    const normalizedPath = normalize(resolve(inputPath));
    // Check for null bytes (path traversal attack)
    if (inputPath.includes('\0')) {
        logger.warn('Null byte detected in path', { path: inputPath });
        return null;
    }
    // Check for path traversal attempts
    if (inputPath.includes('..')) {
        const resolved = resolve(inputPath);
        const isWithinAllowed = allowedBases.some(base => resolved.startsWith(resolve(base)));
        if (!isWithinAllowed) {
            logger.warn('Path traversal attempt blocked', { path: inputPath });
            return null;
        }
    }
    // Verify the path is within allowed bases
    const isAllowed = allowedBases.some(base => normalizedPath.startsWith(resolve(base)));
    if (!isAllowed) {
        logger.warn('Path outside allowed directories', { path: normalizedPath, allowed: allowedBases });
        return null;
    }
    return normalizedPath;
}
/**
 * Validate command for injection attacks
 */
export function validateCommand(command) {
    if (!command || typeof command !== 'string') {
        return { valid: false, reason: 'Command must be a non-empty string' };
    }
    // Check for command chaining/injection patterns
    const dangerousPatterns = [
        /;\s*rm\s/i, // Command chaining with rm
        /\|\s*sh\b/i, // Piping to shell
        /\|\s*bash\b/i, // Piping to bash
        /`[^`]+`/, // Backtick command substitution
        /\$\([^)]*\beval\b/, // Eval in command substitution
        />\s*\/dev\/sd[a-z]/i, // Writing to block devices
        /&&\s*rm\s+-rf/i, // Chained destructive rm
        /\|\|\s*rm\s+-rf/i, // OR chained destructive rm
        /\bnc\s+-[elp]/i, // Netcat reverse shell flags
        /\bcurl\b.*\|\s*\w+sh/i, // Curl pipe to shell
        /\bwget\b.*\|\s*\w+sh/i, // Wget pipe to shell
        />\s*~\/.bashrc/i, // Writing to shell config
        />\s*~\/.profile/i, // Writing to profile
        />\s*\/etc\//i, // Writing to /etc
        /\bsudo\s/i, // Sudo escalation
        /\bchmod\s+[0-7]*7[0-7]*/i, // World-writable permissions
    ];
    for (const pattern of dangerousPatterns) {
        if (pattern.test(command)) {
            return { valid: false, reason: `Dangerous pattern detected: ${pattern.source}` };
        }
    }
    // Check command length
    if (command.length > 10000) {
        return { valid: false, reason: 'Command too long (max 10000 chars)' };
    }
    return { valid: true };
}
/**
 * Sanitize user input for logging
 */
export function sanitizeForLog(input, maxLength = 1000) {
    if (input === null || input === undefined) {
        return 'null';
    }
    let str = typeof input === 'string' ? input : JSON.stringify(input);
    // Truncate
    if (str.length > maxLength) {
        str = str.substring(0, maxLength) + '...[truncated]';
    }
    // Remove potential secrets
    str = redactSecrets(str);
    return str;
}
// =============================================================================
// Secret Detection
// =============================================================================
/**
 * Patterns that indicate potential secrets
 */
const SECRET_PATTERNS = [
    // API Keys
    { pattern: /sk-[a-zA-Z0-9]{20,}/g, replacement: 'sk-***REDACTED***' },
    { pattern: /api[_-]?key['":\s]*[a-zA-Z0-9]{20,}/gi, replacement: 'api_key=***REDACTED***' },
    { pattern: /bearer\s+[a-zA-Z0-9._-]{20,}/gi, replacement: 'bearer ***REDACTED***' },
    // Anthropic/OpenAI
    { pattern: /sk-ant-[a-zA-Z0-9-]{20,}/g, replacement: 'sk-ant-***REDACTED***' },
    { pattern: /sk-proj-[a-zA-Z0-9-]{20,}/g, replacement: 'sk-proj-***REDACTED***' },
    // AWS
    { pattern: /AKIA[0-9A-Z]{16}/g, replacement: 'AKIA***REDACTED***' },
    { pattern: /aws[_-]?secret['":\s]*[a-zA-Z0-9/+=]{40}/gi, replacement: 'aws_secret=***REDACTED***' },
    // GitHub
    { pattern: /ghp_[a-zA-Z0-9]{36}/g, replacement: 'ghp_***REDACTED***' },
    { pattern: /github_pat_[a-zA-Z0-9_]{22,}/g, replacement: 'github_pat_***REDACTED***' },
    // Generic tokens/passwords
    { pattern: /password['":\s]*[^\s'"]{8,}/gi, replacement: 'password=***REDACTED***' },
    { pattern: /token['":\s]*[a-zA-Z0-9._-]{20,}/gi, replacement: 'token=***REDACTED***' },
    { pattern: /secret['":\s]*[a-zA-Z0-9._-]{20,}/gi, replacement: 'secret=***REDACTED***' },
    // Private keys
    { pattern: /-----BEGIN [A-Z]+ PRIVATE KEY-----[\s\S]*?-----END [A-Z]+ PRIVATE KEY-----/g, replacement: '***PRIVATE_KEY_REDACTED***' },
    // Database URLs
    { pattern: /postgres:\/\/[^@]+@[^\s]+/gi, replacement: 'postgres://***REDACTED***' },
    { pattern: /mysql:\/\/[^@]+@[^\s]+/gi, replacement: 'mysql://***REDACTED***' },
    { pattern: /mongodb(\+srv)?:\/\/[^@]+@[^\s]+/gi, replacement: 'mongodb://***REDACTED***' },
];
/**
 * Redact secrets from a string
 */
export function redactSecrets(text) {
    let result = text;
    for (const { pattern, replacement } of SECRET_PATTERNS) {
        result = result.replace(pattern, replacement);
    }
    return result;
}
/**
 * Check if text contains potential secrets
 */
export function containsSecrets(text) {
    for (const { pattern } of SECRET_PATTERNS) {
        if (pattern.test(text)) {
            // Reset lastIndex for global patterns
            pattern.lastIndex = 0;
            return true;
        }
        pattern.lastIndex = 0;
    }
    return false;
}
const rateLimitStore = new Map();
/**
 * Check rate limit
 */
export function checkRateLimit(key, config) {
    const now = Date.now();
    const entry = rateLimitStore.get(key);
    if (!entry || now > entry.resetTime) {
        // New window
        rateLimitStore.set(key, {
            count: 1,
            resetTime: now + config.windowMs
        });
        return { allowed: true, remaining: config.maxRequests - 1, resetIn: config.windowMs };
    }
    if (entry.count >= config.maxRequests) {
        return { allowed: false, remaining: 0, resetIn: entry.resetTime - now };
    }
    entry.count++;
    return { allowed: true, remaining: config.maxRequests - entry.count, resetIn: entry.resetTime - now };
}
/**
 * Create a rate limiter function
 */
export function createRateLimiter(config) {
    return (key) => checkRateLimit(key, config).allowed;
}
// Audit log path
const AUDIT_LOG_DIR = join(homedir(), '.agentic-flow', 'audit');
const AUDIT_LOG_FILE = join(AUDIT_LOG_DIR, 'security.log');
/**
 * Write audit log entry
 */
export function auditLog(entry) {
    const fullEntry = {
        ...entry,
        timestamp: new Date().toISOString()
    };
    // Log to application logger
    logger.info('Security audit', fullEntry);
    // Write to audit file
    try {
        if (!existsSync(AUDIT_LOG_DIR)) {
            mkdirSync(AUDIT_LOG_DIR, { recursive: true });
        }
        appendFileSync(AUDIT_LOG_FILE, JSON.stringify(fullEntry) + '\n');
    }
    catch (error) {
        logger.warn('Failed to write audit log', { error: error.message });
    }
}
/**
 * Audit tool usage
 */
export function auditToolUsage(toolName, input, outcome, sessionId) {
    auditLog({
        event: 'tool_usage',
        actor: 'agent',
        resource: toolName,
        action: 'execute',
        outcome,
        details: { input: sanitizeForLog(input, 500) },
        sessionId
    });
}
/**
 * Audit permission decision
 */
export function auditPermissionDecision(toolName, decision, reason, sessionId) {
    auditLog({
        event: 'permission_decision',
        actor: 'permission_handler',
        resource: toolName,
        action: decision,
        outcome: decision === 'allow' ? 'success' : 'blocked',
        details: { reason },
        sessionId
    });
}
/**
 * Default security context
 */
export function getDefaultSecurityContext() {
    return {
        sessionId: randomBytes(16).toString('hex'),
        allowedPaths: [
            process.cwd(),
            '/tmp',
            join(homedir(), '.agentic-flow')
        ],
        allowedCommands: [
            'ls', 'cat', 'head', 'tail', 'grep', 'find', 'wc',
            'git', 'npm', 'node', 'python', 'python3',
            'echo', 'pwd', 'date', 'which', 'env'
        ],
        blockedPatterns: [
            /rm\s+-rf\s+[\/~]/,
            /chmod\s+777/,
            /curl.*\|\s*bash/,
            /wget.*\|\s*sh/,
            />\s*\/etc\//
        ],
        rateLimit: {
            maxRequests: 100,
            windowMs: 60000 // 1 minute
        },
        auditEnabled: true
    };
}
/**
 * Validate operation against security context
 */
export function validateOperation(operation, target, context) {
    // Check rate limit
    const rateCheck = checkRateLimit(`${context.sessionId}:${operation}`, context.rateLimit);
    if (!rateCheck.allowed) {
        return { allowed: false, reason: `Rate limit exceeded. Reset in ${Math.ceil(rateCheck.resetIn / 1000)}s` };
    }
    // Check path for read/write
    if (operation === 'read' || operation === 'write') {
        const sanitized = sanitizePath(target, context.allowedPaths);
        if (!sanitized) {
            return { allowed: false, reason: 'Path not allowed or invalid' };
        }
    }
    // Check command for execute
    if (operation === 'execute') {
        const validation = validateCommand(target);
        if (!validation.valid) {
            return { allowed: false, reason: validation.reason };
        }
        // Check against blocked patterns
        for (const pattern of context.blockedPatterns) {
            if (pattern.test(target)) {
                return { allowed: false, reason: `Blocked pattern: ${pattern.source}` };
            }
        }
    }
    // Audit if enabled
    if (context.auditEnabled) {
        auditLog({
            event: 'operation_validated',
            actor: 'security_context',
            resource: target,
            action: operation,
            outcome: 'success',
            sessionId: context.sessionId
        });
    }
    return { allowed: true };
}
// =============================================================================
// Secure Hash
// =============================================================================
/**
 * Create secure hash of content
 */
export function secureHash(content, algorithm = 'sha256') {
    return createHash(algorithm).update(content).digest('hex');
}
/**
 * Generate secure random token
 */
export function generateSecureToken(length = 32) {
    return randomBytes(length).toString('hex');
}
// =============================================================================
// Exports
// =============================================================================
export { SECRET_PATTERNS, AUDIT_LOG_DIR, AUDIT_LOG_FILE };
//# sourceMappingURL=security.js.map