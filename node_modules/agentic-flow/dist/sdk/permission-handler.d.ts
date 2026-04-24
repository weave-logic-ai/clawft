/**
 * SDK Permission Handler - Custom permission control for Claude Agent SDK
 *
 * Provides fine-grained permission control beyond simple bypass mode,
 * including dangerous command blocking, directory restrictions, and audit logging.
 */
/**
 * Permission result from canUseTool
 */
export interface PermissionResult {
    behavior: 'allow' | 'deny';
    updatedInput?: any;
    message?: string;
    interrupt?: boolean;
}
/**
 * Tool input type (generic)
 */
export type ToolInput = Record<string, unknown>;
/**
 * Permission handler options
 */
export interface PermissionHandlerOptions {
    signal: AbortSignal;
    suggestions?: Array<{
        type: string;
        rules?: Array<{
            toolName: string;
            ruleContent?: string;
        }>;
        behavior?: string;
        destination?: string;
    }>;
}
/**
 * Initialize permission handler with allowed directories
 */
export declare function initPermissionHandler(dirs?: string[]): void;
/**
 * Custom permission handler for Claude Agent SDK
 *
 * This replaces the simple 'bypassPermissions' mode with intelligent permission control
 */
export declare function customPermissionHandler(toolName: string, input: ToolInput, options: PermissionHandlerOptions): Promise<PermissionResult>;
/**
 * Strict permission handler - more restrictive, blocks more operations
 */
export declare function strictPermissionHandler(toolName: string, input: ToolInput, options: PermissionHandlerOptions): Promise<PermissionResult>;
/**
 * Get permission handler by mode
 */
export declare function getPermissionHandler(mode: 'default' | 'strict' | 'bypass'): ((toolName: string, input: ToolInput, options: PermissionHandlerOptions) => Promise<PermissionResult>) | undefined;
//# sourceMappingURL=permission-handler.d.ts.map