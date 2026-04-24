/**
 * SDK Permission Handler - Custom permission control for Claude Agent SDK
 *
 * Provides fine-grained permission control beyond simple bypass mode,
 * including dangerous command blocking, directory restrictions, and audit logging.
 */
import { logger } from "../utils/logger.js";
import { existsSync, appendFileSync } from "fs";
import { join, resolve, dirname } from "path";
import { homedir } from "os";
// Dangerous command patterns to block
const DANGEROUS_PATTERNS = [
    // Destructive file operations
    /rm\s+-rf\s+[\/~]/,
    /rm\s+-rf\s+\*/,
    /rm\s+--no-preserve-root/,
    // Permission changes
    /chmod\s+777\s+\//,
    /chown\s+-R.*\s+\//,
    // Remote code execution
    /curl.*\|\s*(bash|sh|zsh)/,
    /wget.*\|\s*(bash|sh|zsh)/,
    /curl.*-o\s*\/tmp.*&&.*bash/,
    // Dangerous evals (only match explicit eval with strings, not general command substitution)
    /eval\s+['"`]/,
    /\$\([^)]*rm\s/, // Only block command substitution containing rm
    /\$\([^)]*curl.*\|/, // Block curl piped inside substitution
    // SQL injection patterns
    /DROP\s+TABLE/i,
    /DELETE\s+FROM.*WHERE\s+1\s*=\s*1/i,
    /TRUNCATE\s+TABLE/i,
    // System damage
    /mkfs\./,
    /dd\s+if=.*of=\/dev/,
    /shutdown\s/,
    /reboot\s/,
    // Dangerous git operations
    /git\s+push\s+.*--force/,
    /git\s+push\s+-f\s/,
    /git\s+reset\s+--hard\s+origin/,
    // Package publishing (require explicit confirmation)
    /npm\s+publish/,
    // Credential exposure
    /cat.*\.env\b/,
    /cat.*credentials/i,
    /cat.*\.ssh\/id_/,
];
// File path patterns to block
const BLOCKED_PATHS = [
    /^\/etc\/passwd$/,
    /^\/etc\/shadow$/,
    /^\/etc\/sudoers/,
    /\.ssh\/id_/,
    /\.aws\/credentials/,
    /\.env$/,
    /\.env\.local$/,
    /\.env\.production$/,
];
// Allowed directories (relative to cwd by default)
let allowedDirectories = [];
/**
 * Initialize permission handler with allowed directories
 */
export function initPermissionHandler(dirs) {
    allowedDirectories = dirs || [
        process.cwd(),
        '/tmp',
        '/var/tmp',
        join(homedir(), '.agentic-flow')
    ];
    logger.info('Permission handler initialized', { allowedDirs: allowedDirectories.length });
}
// Initialize with defaults
initPermissionHandler();
/**
 * Check if a path is in allowed directories
 */
function isPathAllowed(filePath) {
    if (!filePath)
        return true;
    const resolvedPath = resolve(filePath);
    // Check against blocked patterns
    for (const pattern of BLOCKED_PATHS) {
        if (pattern.test(resolvedPath)) {
            return false;
        }
    }
    // Check if in allowed directories
    return allowedDirectories.some(dir => resolvedPath.startsWith(resolve(dir)));
}
/**
 * Check if a command contains dangerous patterns
 */
function isDangerousCommand(command) {
    if (!command)
        return { dangerous: false };
    for (const pattern of DANGEROUS_PATTERNS) {
        if (pattern.test(command)) {
            return { dangerous: true, pattern: pattern.source };
        }
    }
    return { dangerous: false };
}
/**
 * Log permission decision for audit
 */
function logPermissionDecision(toolName, input, decision, reason) {
    const logEntry = {
        timestamp: new Date().toISOString(),
        tool: toolName,
        decision,
        reason,
        input: JSON.stringify(input).substring(0, 500) // Truncate for safety
    };
    logger.info('Permission decision', logEntry);
    // Optionally write to audit log file
    const auditLogPath = join(process.cwd(), '.agentic-flow', 'audit.log');
    try {
        const dir = dirname(auditLogPath);
        if (existsSync(dir)) {
            appendFileSync(auditLogPath, JSON.stringify(logEntry) + '\n');
        }
    }
    catch (e) {
        // Silently fail if can't write audit log
    }
}
/**
 * Custom permission handler for Claude Agent SDK
 *
 * This replaces the simple 'bypassPermissions' mode with intelligent permission control
 */
export async function customPermissionHandler(toolName, input, options) {
    // Always allow read-only tools
    const readOnlyTools = ['Read', 'Glob', 'Grep', 'WebSearch', 'WebFetch', 'TodoWrite'];
    if (readOnlyTools.includes(toolName)) {
        logPermissionDecision(toolName, input, 'allow', 'read-only tool');
        return { behavior: 'allow', updatedInput: input };
    }
    // Check Bash commands
    if (toolName === 'Bash') {
        const command = input.command || '';
        const { dangerous, pattern } = isDangerousCommand(command);
        if (dangerous) {
            logPermissionDecision(toolName, input, 'deny', `dangerous pattern: ${pattern}`);
            return {
                behavior: 'deny',
                message: `Dangerous command blocked: matches pattern "${pattern}"`,
                interrupt: false
            };
        }
        logPermissionDecision(toolName, input, 'allow', 'command safe');
        return { behavior: 'allow', updatedInput: input };
    }
    // Check file operations
    if (['Write', 'Edit', 'NotebookEdit'].includes(toolName)) {
        const filePath = input.file_path || input.notebook_path || '';
        if (!isPathAllowed(filePath)) {
            logPermissionDecision(toolName, input, 'deny', `path not allowed: ${filePath}`);
            return {
                behavior: 'deny',
                message: `File access not allowed: ${filePath}`,
                interrupt: false
            };
        }
        logPermissionDecision(toolName, input, 'allow', 'path allowed');
        return { behavior: 'allow', updatedInput: input };
    }
    // Check MCP resource operations
    if (['ListMcpResources', 'ReadMcpResource'].includes(toolName)) {
        logPermissionDecision(toolName, input, 'allow', 'MCP resource access');
        return { behavior: 'allow', updatedInput: input };
    }
    // Check background shell operations
    if (['KillBash', 'BashOutput'].includes(toolName)) {
        logPermissionDecision(toolName, input, 'allow', 'shell control');
        return { behavior: 'allow', updatedInput: input };
    }
    // Task tool for subagents - always allow
    if (toolName === 'Task') {
        logPermissionDecision(toolName, input, 'allow', 'subagent task');
        return { behavior: 'allow', updatedInput: input };
    }
    // AskUserQuestion - always allow (interactive)
    if (toolName === 'AskUserQuestion') {
        logPermissionDecision(toolName, input, 'allow', 'user interaction');
        return { behavior: 'allow', updatedInput: input };
    }
    // ExitPlanMode - always allow
    if (toolName === 'ExitPlanMode') {
        logPermissionDecision(toolName, input, 'allow', 'plan mode control');
        return { behavior: 'allow', updatedInput: input };
    }
    // Default: allow with logging
    logPermissionDecision(toolName, input, 'allow', 'default allow');
    return { behavior: 'allow', updatedInput: input };
}
/**
 * Strict permission handler - more restrictive, blocks more operations
 */
export async function strictPermissionHandler(toolName, input, options) {
    // Only allow read operations by default
    const allowedTools = ['Read', 'Glob', 'Grep', 'WebSearch', 'TodoWrite'];
    if (!allowedTools.includes(toolName)) {
        logPermissionDecision(toolName, input, 'deny', 'strict mode - tool not allowed');
        return {
            behavior: 'deny',
            message: `Tool ${toolName} not allowed in strict mode`,
            interrupt: false
        };
    }
    return customPermissionHandler(toolName, input, options);
}
/**
 * Get permission handler by mode
 */
export function getPermissionHandler(mode) {
    switch (mode) {
        case 'default':
            return customPermissionHandler;
        case 'strict':
            return strictPermissionHandler;
        case 'bypass':
            return undefined; // No handler = bypass all permissions
        default:
            return customPermissionHandler;
    }
}
//# sourceMappingURL=permission-handler.js.map