/**
 * Safe Command Execution Utility
 *
 * Prevents command injection by:
 * 1. Using execFileSync instead of execSync (no shell interpretation)
 * 2. Validating all inputs against strict patterns
 * 3. Never passing user input through shell
 */
import { ExecFileSyncOptions } from 'child_process';
/**
 * Validation patterns for common input types
 */
export declare const VALIDATION_PATTERNS: {
    key: RegExp;
    namespace: RegExp;
    pattern: RegExp;
    swarmId: RegExp;
    agentName: RegExp;
    taskId: RegExp;
    category: RegExp;
    topology: RegExp;
    agentType: RegExp;
    strategy: RegExp;
    priority: RegExp;
};
/**
 * Validate a string against a pattern
 */
export declare function validateInput(value: string, pattern: RegExp, fieldName: string): string;
/**
 * Validate and sanitize a key parameter
 */
export declare function validateKey(key: string): string;
/**
 * Validate and sanitize a namespace parameter
 */
export declare function validateNamespace(namespace: string): string;
/**
 * Validate and sanitize a pattern parameter
 */
export declare function validatePattern(pattern: string): string;
/**
 * Validate a value - allow any content but limit size
 */
export declare function validateValue(value: string, maxLength?: number): string;
/**
 * Safe execution of npx commands
 * Uses execFileSync with array arguments to prevent shell injection
 */
export declare function safeExecNpx(pkg: string, args: string[], options?: ExecFileSyncOptions): string;
/**
 * Execute claude-flow memory commands safely
 */
export declare function execMemoryStore(key: string, value: string, namespace?: string, ttl?: number): string;
/**
 * Execute claude-flow memory retrieve safely
 */
export declare function execMemoryRetrieve(key: string, namespace?: string): string;
/**
 * Execute claude-flow memory search safely
 */
export declare function execMemorySearch(pattern: string, namespace?: string, limit?: number): string;
/**
 * Execute claude-flow swarm init safely
 */
export declare function execSwarmInit(swarmId: string, topology: string, maxAgents?: number): string;
/**
 * Execute claude-flow agent spawn safely
 */
export declare function execAgentSpawn(name: string, type: string, swarmId?: string, capabilities?: string[]): string;
/**
 * Execute claude-flow task orchestrate safely
 */
export declare function execTaskOrchestrate(task: string, strategy?: string, priority?: string): string;
declare const _default: {
    validateInput: typeof validateInput;
    validateKey: typeof validateKey;
    validateNamespace: typeof validateNamespace;
    validatePattern: typeof validatePattern;
    validateValue: typeof validateValue;
    safeExecNpx: typeof safeExecNpx;
    execMemoryStore: typeof execMemoryStore;
    execMemoryRetrieve: typeof execMemoryRetrieve;
    execMemorySearch: typeof execMemorySearch;
    execSwarmInit: typeof execSwarmInit;
    execAgentSpawn: typeof execAgentSpawn;
    execTaskOrchestrate: typeof execTaskOrchestrate;
    VALIDATION_PATTERNS: {
        key: RegExp;
        namespace: RegExp;
        pattern: RegExp;
        swarmId: RegExp;
        agentName: RegExp;
        taskId: RegExp;
        category: RegExp;
        topology: RegExp;
        agentType: RegExp;
        strategy: RegExp;
        priority: RegExp;
    };
};
export default _default;
//# sourceMappingURL=safe-exec.d.ts.map