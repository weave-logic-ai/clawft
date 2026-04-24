/**
 * SDK Hooks Bridge - Connects agentic-flow intelligence layer to Claude Agent SDK hooks
 *
 * Bridges our custom hooks (intelligence-bridge.ts) with the native SDK hook system
 * enabling seamless integration with Claude Code's event loop.
 */
export type HookEvent = 'PreToolUse' | 'PostToolUse' | 'PostToolUseFailure' | 'Notification' | 'UserPromptSubmit' | 'SessionStart' | 'SessionEnd' | 'Stop' | 'SubagentStart' | 'SubagentStop' | 'PreCompact' | 'PermissionRequest';
export interface BaseHookInput {
    session_id: string;
    transcript_path: string;
    cwd: string;
    permission_mode?: string;
}
export interface PreToolUseHookInput extends BaseHookInput {
    hook_event_name: 'PreToolUse';
    tool_name: string;
    tool_input: unknown;
}
export interface PostToolUseHookInput extends BaseHookInput {
    hook_event_name: 'PostToolUse';
    tool_name: string;
    tool_input: unknown;
    tool_response: unknown;
}
export interface SessionStartHookInput extends BaseHookInput {
    hook_event_name: 'SessionStart';
    source: 'startup' | 'resume' | 'clear' | 'compact';
}
export interface SessionEndHookInput extends BaseHookInput {
    hook_event_name: 'SessionEnd';
    reason: string;
}
export interface SubagentStartHookInput extends BaseHookInput {
    hook_event_name: 'SubagentStart';
    agent_id: string;
    agent_type: string;
}
export interface SubagentStopHookInput extends BaseHookInput {
    hook_event_name: 'SubagentStop';
    stop_hook_active: boolean;
}
export type HookInput = PreToolUseHookInput | PostToolUseHookInput | SessionStartHookInput | SessionEndHookInput | SubagentStartHookInput | SubagentStopHookInput | BaseHookInput & {
    hook_event_name: string;
};
export interface HookJSONOutput {
    continue?: boolean;
    suppressOutput?: boolean;
    stopReason?: string;
    decision?: 'approve' | 'block';
    systemMessage?: string;
    reason?: string;
    hookSpecificOutput?: {
        hookEventName: string;
        additionalContext?: string;
        permissionDecision?: 'allow' | 'deny' | 'ask';
        permissionDecisionReason?: string;
        updatedInput?: Record<string, unknown>;
    };
}
export type HookCallback = (input: HookInput, toolUseId: string | undefined, options: {
    signal: AbortSignal;
}) => Promise<HookJSONOutput>;
export interface HookCallbackMatcher {
    matcher?: string;
    hooks: HookCallback[];
}
/**
 * PreToolUse hook - Called before tool execution
 * Routes to best agent and starts trajectory tracking
 */
export declare const preToolUseHook: HookCallback;
/**
 * PostToolUse hook - Called after successful tool execution
 * Records patterns and ends trajectories
 */
export declare const postToolUseHook: HookCallback;
/**
 * PostToolUseFailure hook - Called when tool execution fails
 * Ends trajectories as failures
 */
export declare const postToolUseFailureHook: HookCallback;
/**
 * SessionStart hook - Called when session begins
 * Initializes intelligence layer
 */
export declare const sessionStartHook: HookCallback;
/**
 * SessionEnd hook - Called when session ends
 * Persists learning data
 */
export declare const sessionEndHook: HookCallback;
/**
 * SubagentStart hook - Called when a subagent is spawned
 */
export declare const subagentStartHook: HookCallback;
/**
 * SubagentStop hook - Called when a subagent completes
 */
export declare const subagentStopHook: HookCallback;
/**
 * Get SDK hooks configuration
 * Returns hooks in the format expected by Claude Agent SDK query() options
 */
export declare function getSdkHooks(): Partial<Record<HookEvent, HookCallbackMatcher[]>>;
/**
 * Get filtered hooks for specific tools
 */
export declare function getToolSpecificHooks(toolMatcher: string): Partial<Record<HookEvent, HookCallbackMatcher[]>>;
//# sourceMappingURL=hooks-bridge.d.ts.map