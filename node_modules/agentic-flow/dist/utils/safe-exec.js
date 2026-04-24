/**
 * Safe Command Execution Utility
 *
 * Prevents command injection by:
 * 1. Using execFileSync instead of execSync (no shell interpretation)
 * 2. Validating all inputs against strict patterns
 * 3. Never passing user input through shell
 */
import { execFileSync } from 'child_process';
/**
 * Validation patterns for common input types
 */
export const VALIDATION_PATTERNS = {
    // Alphanumeric with dashes, underscores, dots, colons
    key: /^[a-zA-Z0-9_\-.:]+$/,
    // Namespace: alphanumeric with dashes and underscores only
    namespace: /^[a-zA-Z0-9_-]+$/,
    // Pattern: alphanumeric with wildcards
    pattern: /^[a-zA-Z0-9_\-.*?]+$/,
    // Swarm ID: alphanumeric with dashes
    swarmId: /^[a-zA-Z0-9_-]+$/,
    // Agent name: kebab-case
    agentName: /^[a-z0-9]+(?:-[a-z0-9]+)*$/,
    // Task ID: UUID or simple ID
    taskId: /^[a-zA-Z0-9_-]+$/,
    // Category: alphanumeric with dashes and underscores
    category: /^[a-zA-Z0-9_-]+$/,
    // Topology: specific allowed values
    topology: /^(hierarchical|mesh|ring|star)$/,
    // Agent type: specific allowed values
    agentType: /^(coordinator|analyst|optimizer|documenter|monitor|specialist|architect|task-orchestrator|code-analyzer|perf-analyzer|api-docs|performance-benchmarker|system-architect|researcher|coder|tester|reviewer)$/,
    // Strategy: specific allowed values
    strategy: /^(parallel|sequential|adaptive|balanced)$/,
    // Priority: specific allowed values
    priority: /^(low|medium|high|critical)$/,
};
/**
 * Validate a string against a pattern
 */
export function validateInput(value, pattern, fieldName) {
    if (!pattern.test(value)) {
        throw new Error(`Invalid ${fieldName}: contains disallowed characters. ` +
            `Only alphanumeric characters and limited special characters are allowed.`);
    }
    return value;
}
/**
 * Validate and sanitize a key parameter
 */
export function validateKey(key) {
    if (key.length > 256) {
        throw new Error('Key too long (max 256 characters)');
    }
    return validateInput(key, VALIDATION_PATTERNS.key, 'key');
}
/**
 * Validate and sanitize a namespace parameter
 */
export function validateNamespace(namespace) {
    if (namespace.length > 64) {
        throw new Error('Namespace too long (max 64 characters)');
    }
    return validateInput(namespace, VALIDATION_PATTERNS.namespace, 'namespace');
}
/**
 * Validate and sanitize a pattern parameter
 */
export function validatePattern(pattern) {
    if (pattern.length > 256) {
        throw new Error('Pattern too long (max 256 characters)');
    }
    return validateInput(pattern, VALIDATION_PATTERNS.pattern, 'pattern');
}
/**
 * Validate a value - allow any content but limit size
 */
export function validateValue(value, maxLength = 1048576) {
    if (value.length > maxLength) {
        throw new Error(`Value too long (max ${maxLength} characters)`);
    }
    // Values can contain any content - they're passed as array args, not shell-interpreted
    return value;
}
/**
 * Safe execution of npx commands
 * Uses execFileSync with array arguments to prevent shell injection
 */
export function safeExecNpx(pkg, args, options) {
    const defaultOptions = {
        encoding: 'utf-8',
        maxBuffer: 10 * 1024 * 1024,
        timeout: 30000,
        ...options,
    };
    try {
        // Use npx with --yes to auto-confirm
        const result = execFileSync('npx', ['--yes', pkg, ...args], defaultOptions);
        return typeof result === 'string' ? result : result.toString();
    }
    catch (error) {
        // Sanitize error message to not leak system info
        const message = error.message || 'Command execution failed';
        throw new Error(`Execution failed: ${message.substring(0, 200)}`);
    }
}
/**
 * Execute claude-flow memory commands safely
 */
export function execMemoryStore(key, value, namespace = 'default', ttl) {
    // Validate inputs
    const safeKey = validateKey(key);
    const safeValue = validateValue(value);
    const safeNamespace = validateNamespace(namespace);
    const args = ['memory', 'store', safeKey, safeValue, '--namespace', safeNamespace];
    if (ttl !== undefined && ttl > 0) {
        args.push('--ttl', String(ttl));
    }
    return safeExecNpx('claude-flow@alpha', args);
}
/**
 * Execute claude-flow memory retrieve safely
 */
export function execMemoryRetrieve(key, namespace = 'default') {
    const safeKey = validateKey(key);
    const safeNamespace = validateNamespace(namespace);
    return safeExecNpx('claude-flow@alpha', [
        'memory', 'retrieve', safeKey, '--namespace', safeNamespace
    ]);
}
/**
 * Execute claude-flow memory search safely
 */
export function execMemorySearch(pattern, namespace, limit = 10) {
    const safePattern = validatePattern(pattern);
    const safeLimit = Math.min(Math.max(1, limit), 100);
    const args = ['memory', 'search', safePattern, '--limit', String(safeLimit)];
    if (namespace) {
        args.push('--namespace', validateNamespace(namespace));
    }
    return safeExecNpx('claude-flow@alpha', args);
}
/**
 * Execute claude-flow swarm init safely
 */
export function execSwarmInit(swarmId, topology, maxAgents = 8) {
    const safeSwarmId = validateInput(swarmId, VALIDATION_PATTERNS.swarmId, 'swarmId');
    const safeTopology = validateInput(topology, VALIDATION_PATTERNS.topology, 'topology');
    const safeMaxAgents = Math.min(Math.max(1, maxAgents), 100);
    return safeExecNpx('claude-flow@alpha', [
        'swarm', 'init',
        '--id', safeSwarmId,
        '--topology', safeTopology,
        '--max-agents', String(safeMaxAgents)
    ]);
}
/**
 * Execute claude-flow agent spawn safely
 */
export function execAgentSpawn(name, type, swarmId, capabilities) {
    const safeName = validateInput(name, VALIDATION_PATTERNS.agentName, 'name');
    const safeType = validateInput(type, VALIDATION_PATTERNS.agentType, 'type');
    const args = ['agent', 'spawn', '--name', safeName, '--type', safeType];
    if (swarmId) {
        args.push('--swarm', validateInput(swarmId, VALIDATION_PATTERNS.swarmId, 'swarmId'));
    }
    if (capabilities && capabilities.length > 0) {
        // Validate each capability
        const safeCapabilities = capabilities.map(cap => validateInput(cap, VALIDATION_PATTERNS.key, 'capability'));
        args.push('--capabilities', safeCapabilities.join(','));
    }
    return safeExecNpx('claude-flow@alpha', args);
}
/**
 * Execute claude-flow task orchestrate safely
 */
export function execTaskOrchestrate(task, strategy = 'auto', priority = 'medium') {
    // Task description can contain most text, but validate length
    if (task.length > 10000) {
        throw new Error('Task description too long (max 10000 characters)');
    }
    const safeStrategy = validateInput(strategy, VALIDATION_PATTERNS.strategy, 'strategy');
    const safePriority = validateInput(priority, VALIDATION_PATTERNS.priority, 'priority');
    return safeExecNpx('claude-flow@alpha', [
        'task', 'orchestrate',
        '--task', task,
        '--strategy', safeStrategy,
        '--priority', safePriority
    ]);
}
export default {
    validateInput,
    validateKey,
    validateNamespace,
    validatePattern,
    validateValue,
    safeExecNpx,
    execMemoryStore,
    execMemoryRetrieve,
    execMemorySearch,
    execSwarmInit,
    execAgentSpawn,
    execTaskOrchestrate,
    VALIDATION_PATTERNS,
};
//# sourceMappingURL=safe-exec.js.map