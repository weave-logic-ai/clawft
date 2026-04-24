/**
 * AgentDB Runtime Patch
 *
 * Automatically fixes AgentDB v1.3.9 import resolution issues at runtime.
 * This patch works in all contexts: npm install, npm install -g, and npx.
 *
 * Issue: agentdb v1.3.9 missing .js extensions in ESM exports
 * Solution: Patch the controller index.js file at runtime before first import
 */
/**
 * Apply AgentDB import fix at runtime
 * Safe to call multiple times - only patches once
 */
export declare function applyAgentDBPatch(): boolean;
/**
 * Check if AgentDB patch is needed
 */
export declare function isAgentDBPatchNeeded(): boolean;
/**
 * Get patch status information
 */
export declare function getAgentDBPatchStatus(): {
    needed: boolean;
    applied: boolean;
    attempted: boolean;
    location: string | null;
};
//# sourceMappingURL=agentdb-runtime-patch.d.ts.map