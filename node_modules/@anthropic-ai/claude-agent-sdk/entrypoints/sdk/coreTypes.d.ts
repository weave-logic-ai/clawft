import type { MessageParam as APIUserMessage } from '@anthropic-ai/sdk/resources';
import type { BetaMessage as APIAssistantMessage, BetaUsage as Usage, BetaRawMessageStreamEvent as RawMessageStreamEvent } from '@anthropic-ai/sdk/resources/beta/messages/messages.mjs';
import type { UUID } from 'crypto';
export type { SandboxSettings, SandboxNetworkConfig, SandboxIgnoreViolations, } from '../sandboxTypes.js';
export type NonNullableUsage = {
    [K in keyof Usage]: NonNullable<Usage[K]>;
};
export type ModelUsage = {
    inputTokens: number;
    outputTokens: number;
    cacheReadInputTokens: number;
    cacheCreationInputTokens: number;
    webSearchRequests: number;
    costUSD: number;
    contextWindow: number;
};
export type OutputFormatType = 'json_schema';
export type BaseOutputFormat = {
    type: OutputFormatType;
};
export type JsonSchemaOutputFormat = BaseOutputFormat & {
    type: 'json_schema';
    schema: Record<string, unknown>;
};
export type OutputFormat = JsonSchemaOutputFormat;
export type ApiKeySource = 'user' | 'project' | 'org' | 'temporary';
export type ConfigScope = 'local' | 'user' | 'project';
/**
 * Allowed beta headers that can be passed via SDK options.
 */
export type SdkBeta = 'context-1m-2025-08-07';
export type McpStdioServerConfig = {
    type?: 'stdio';
    command: string;
    args?: string[];
    env?: Record<string, string>;
};
export type McpSSEServerConfig = {
    type: 'sse';
    url: string;
    headers?: Record<string, string>;
};
export type McpHttpServerConfig = {
    type: 'http';
    url: string;
    headers?: Record<string, string>;
};
export type McpSdkServerConfig = {
    type: 'sdk';
    name: string;
};
/**
 * MCP server config types that can be serialized for process transport.
 * Does not include McpSdkServerConfigWithInstance which contains a non-serializable McpServer instance.
 */
export type McpServerConfigForProcessTransport = McpStdioServerConfig | McpSSEServerConfig | McpHttpServerConfig | McpSdkServerConfig;
/**
 * Status information for an MCP server connection.
 */
export type McpServerStatus = {
    /** Server name as configured */
    name: string;
    /** Current connection status */
    status: 'connected' | 'failed' | 'needs-auth' | 'pending';
    /** Server information (available when connected) */
    serverInfo?: {
        name: string;
        version: string;
    };
};
/**
 * Result of a setMcpServers operation.
 */
export type McpSetServersResult = {
    /** Names of servers that were added */
    added: string[];
    /** Names of servers that were removed */
    removed: string[];
    /** Map of server names to error messages for servers that failed to connect */
    errors: Record<string, string>;
};
type PermissionUpdateDestination = 'userSettings' | 'projectSettings' | 'localSettings' | 'session' | 'cliArg';
export type PermissionBehavior = 'allow' | 'deny' | 'ask';
export type PermissionUpdate = {
    type: 'addRules';
    rules: PermissionRuleValue[];
    behavior: PermissionBehavior;
    destination: PermissionUpdateDestination;
} | {
    type: 'replaceRules';
    rules: PermissionRuleValue[];
    behavior: PermissionBehavior;
    destination: PermissionUpdateDestination;
} | {
    type: 'removeRules';
    rules: PermissionRuleValue[];
    behavior: PermissionBehavior;
    destination: PermissionUpdateDestination;
} | {
    type: 'setMode';
    mode: PermissionMode;
    destination: PermissionUpdateDestination;
} | {
    type: 'addDirectories';
    directories: string[];
    destination: PermissionUpdateDestination;
} | {
    type: 'removeDirectories';
    directories: string[];
    destination: PermissionUpdateDestination;
};
export type PermissionResult = {
    behavior: 'allow';
    /**
     * Updated tool input to use, if any changes are needed.
     *
     * For example if the user was given the option to update the tool use
     * input before approving, then this would be the updated input which
     * would be executed by the tool.
     */
    updatedInput: Record<string, unknown>;
    /**
     * Permissions updates to be applied as part of accepting this tool use.
     *
     * Typically this is used as part of the 'always allow' flow and these
     * permission updates are from the `suggestions` field from the
     * CanUseTool callback.
     *
     * It is recommended that you use these suggestions rather than
     * attempting to re-derive them from the tool use input, as the
     * suggestions may include other permission changes such as adding
     * directories or incorporate complex tool-use logic such as bash
     * commands.
     */
    updatedPermissions?: PermissionUpdate[];
    /**
     * The tool use ID. Supplied and used internally.
     */
    toolUseID?: string;
} | {
    behavior: 'deny';
    /**
     * Message indicating the reason for denial, or guidance of what the
     * model should do instead.
     */
    message: string;
    /**
     * If true, interrupt execution and do not continue.
     *
     * Typically this should be set to true when the user says 'no' with no
     * further guidance. Leave unset or false if the user provides guidance
     * which the model should incorporate and continue.
     */
    interrupt?: boolean;
    /**
     * The tool use ID. Supplied and used internally.
     */
    toolUseID?: string;
};
export type PermissionRuleValue = {
    toolName: string;
    ruleContent?: string;
};
/**
 * Permission mode for controlling how tool executions are handled.
 * - `'default'` - Standard behavior, prompts for dangerous operations
 * - `'acceptEdits'` - Auto-accept file edit operations
 * - `'bypassPermissions'` - Bypass all permission checks (requires `allowDangerouslySkipPermissions`)
 * - `'plan'` - Planning mode, no actual tool execution
 * - `'delegate'` - Delegate mode, restricts team leader to only Teammate and Task tools
 * - `'dontAsk'` - Don't prompt for permissions, deny if not pre-approved
 */
export type PermissionMode = 'default' | 'acceptEdits' | 'bypassPermissions' | 'plan' | 'delegate' | 'dontAsk';
export declare const HOOK_EVENTS: readonly ["PreToolUse", "PostToolUse", "PostToolUseFailure", "Notification", "UserPromptSubmit", "SessionStart", "SessionEnd", "Stop", "SubagentStart", "SubagentStop", "PreCompact", "PermissionRequest"];
export type HookEvent = (typeof HOOK_EVENTS)[number];
export type BaseHookInput = {
    session_id: string;
    transcript_path: string;
    cwd: string;
    permission_mode?: string;
};
export type PreToolUseHookInput = BaseHookInput & {
    hook_event_name: 'PreToolUse';
    tool_name: string;
    tool_input: unknown;
    tool_use_id: string;
};
export type PermissionRequestHookInput = BaseHookInput & {
    hook_event_name: 'PermissionRequest';
    tool_name: string;
    tool_input: unknown;
    permission_suggestions?: PermissionUpdate[];
};
export type PostToolUseHookInput = BaseHookInput & {
    hook_event_name: 'PostToolUse';
    tool_name: string;
    tool_input: unknown;
    tool_response: unknown;
    tool_use_id: string;
};
export type PostToolUseFailureHookInput = BaseHookInput & {
    hook_event_name: 'PostToolUseFailure';
    tool_name: string;
    tool_input: unknown;
    tool_use_id: string;
    error: string;
    is_interrupt?: boolean;
};
export type NotificationHookInput = BaseHookInput & {
    hook_event_name: 'Notification';
    message: string;
    title?: string;
    notification_type: string;
};
export type UserPromptSubmitHookInput = BaseHookInput & {
    hook_event_name: 'UserPromptSubmit';
    prompt: string;
};
export type SessionStartHookInput = BaseHookInput & {
    hook_event_name: 'SessionStart';
    source: 'startup' | 'resume' | 'clear' | 'compact';
};
export type StopHookInput = BaseHookInput & {
    hook_event_name: 'Stop';
    stop_hook_active: boolean;
};
export type SubagentStartHookInput = BaseHookInput & {
    hook_event_name: 'SubagentStart';
    agent_id: string;
    agent_type: string;
};
export type SubagentStopHookInput = BaseHookInput & {
    hook_event_name: 'SubagentStop';
    stop_hook_active: boolean;
    agent_id: string;
    agent_transcript_path: string;
};
export type PreCompactHookInput = BaseHookInput & {
    hook_event_name: 'PreCompact';
    trigger: 'manual' | 'auto';
    custom_instructions: string | null;
};
export declare const EXIT_REASONS: string[];
export type ExitReason = (typeof EXIT_REASONS)[number];
export type SessionEndHookInput = BaseHookInput & {
    hook_event_name: 'SessionEnd';
    reason: ExitReason;
};
export type HookInput = PreToolUseHookInput | PostToolUseHookInput | PostToolUseFailureHookInput | NotificationHookInput | UserPromptSubmitHookInput | SessionStartHookInput | SessionEndHookInput | StopHookInput | SubagentStartHookInput | SubagentStopHookInput | PreCompactHookInput | PermissionRequestHookInput;
export type AsyncHookJSONOutput = {
    async: true;
    asyncTimeout?: number;
};
export type SyncHookJSONOutput = {
    continue?: boolean;
    suppressOutput?: boolean;
    stopReason?: string;
    decision?: 'approve' | 'block';
    systemMessage?: string;
    reason?: string;
    hookSpecificOutput?: {
        hookEventName: 'PreToolUse';
        permissionDecision?: 'allow' | 'deny' | 'ask';
        permissionDecisionReason?: string;
        updatedInput?: Record<string, unknown>;
    } | {
        hookEventName: 'UserPromptSubmit';
        additionalContext?: string;
    } | {
        hookEventName: 'SessionStart';
        additionalContext?: string;
    } | {
        hookEventName: 'SubagentStart';
        additionalContext?: string;
    } | {
        hookEventName: 'PostToolUse';
        additionalContext?: string;
        updatedMCPToolOutput?: unknown;
    } | {
        hookEventName: 'PostToolUseFailure';
        additionalContext?: string;
    } | {
        hookEventName: 'PermissionRequest';
        decision: {
            behavior: 'allow';
            updatedInput?: Record<string, unknown>;
            updatedPermissions?: PermissionUpdate[];
        } | {
            behavior: 'deny';
            message?: string;
            interrupt?: boolean;
        };
    };
};
export type HookJSONOutput = AsyncHookJSONOutput | SyncHookJSONOutput;
/**
 * Information about an available skill (invoked via /command syntax).
 */
export type SlashCommand = {
    /** Skill name (without the leading slash) */
    name: string;
    /** Description of what the skill does */
    description: string;
    /** Hint for skill arguments (e.g., "<file>") */
    argumentHint: string;
};
/**
 * Information about an available model.
 */
export type ModelInfo = {
    /** Model identifier to use in API calls */
    value: string;
    /** Human-readable display name */
    displayName: string;
    /** Description of the model's capabilities */
    description: string;
};
/** Information about the logged in user's account. */
export type AccountInfo = {
    email?: string;
    organization?: string;
    subscriptionType?: string;
    tokenSource?: string;
    apiKeySource?: string;
};
/**
 * MCP server specification for agents. Can be either:
 * - A string reference to an existing MCP server by name
 * - An inline MCP server definition as \{ [name]: config \}
 */
export type AgentMcpServerSpec = string | {
    [name: string]: McpServerConfigForProcessTransport;
};
/**
 * Definition for a custom subagent that can be invoked via the Task tool.
 */
export type AgentDefinition = {
    /** Natural language description of when to use this agent */
    description: string;
    /** Array of allowed tool names. If omitted, inherits all tools from parent */
    tools?: string[];
    /** Array of tool names to explicitly disallow for this agent */
    disallowedTools?: string[];
    /** The agent's system prompt */
    prompt: string;
    /** Model to use for this agent. If omitted or 'inherit', uses the main model */
    model?: 'sonnet' | 'opus' | 'haiku' | 'inherit';
    /**
     * MCP servers specific to this agent. These are additive to any parent MCP servers.
     * Servers are connected when the agent starts and cleaned up when it finishes.
     *
     * @example
     * ```typescript
     * mcpServers: [
     *   'slack',  // Reference existing server by name
     *   {         // Inline server definition
     *     'custom-server': {
     *       type: 'stdio',
     *       command: 'node',
     *       args: ['./my-server.js']
     *     }
     *   }
     * ]
     * ```
     */
    mcpServers?: AgentMcpServerSpec[];
    /** Experimental: Critical reminder added to system prompt */
    criticalSystemReminder_EXPERIMENTAL?: string;
};
/**
 * Source for loading filesystem-based settings.
 * - `'user'` - Global user settings (`~/.claude/settings.json`)
 * - `'project'` - Project settings (`.claude/settings.json`)
 * - `'local'` - Local settings (`.claude/settings.local.json`)
 */
export type SettingSource = 'user' | 'project' | 'local';
/**
 * Configuration for loading a plugin.
 */
export type SdkPluginConfig = {
    /** Plugin type. Currently only 'local' is supported */
    type: 'local';
    /** Absolute or relative path to the plugin directory */
    path: string;
};
/**
 * Result of a rewindFiles operation.
 */
export type RewindFilesResult = {
    canRewind: boolean;
    error?: string;
    filesChanged?: string[];
    insertions?: number;
    deletions?: number;
};
type SDKUserMessageContent = {
    type: 'user';
    message: APIUserMessage;
    parent_tool_use_id: string | null;
    /**
     * True if this is a 'synthetic' user message which did not originate from
     * the user directly, but instead was generated by the system.
     */
    isSynthetic?: boolean;
    /**
     * If present, the JSON result of a tool use that this user message is
     * responding to. This is provided to make it easier for applications to
     * present the tool result in a formatted way. The model only receives
     * the content within the user message.
     * The specific format is tool-dependent.
     */
    tool_use_result?: unknown;
};
export type SDKUserMessage = SDKUserMessageContent & {
    uuid?: UUID;
    session_id: string;
};
export type SDKUserMessageReplay = SDKUserMessageContent & {
    uuid: UUID;
    session_id: string;
    /**
     * True if this is a replay/acknowledgment of a user message that was already
     * added to the messages array. Used internally to prevent duplicate messages.
     */
    isReplay: true;
};
export type SDKAssistantMessageError = 'authentication_failed' | 'billing_error' | 'rate_limit' | 'invalid_request' | 'server_error' | 'unknown';
export type SDKAssistantMessage = {
    type: 'assistant';
    message: APIAssistantMessage;
    parent_tool_use_id: string | null;
    error?: SDKAssistantMessageError;
    uuid: UUID;
    session_id: string;
};
export type SDKPermissionDenial = {
    tool_name: string;
    tool_use_id: string;
    tool_input: Record<string, unknown>;
};
export type SDKResultMessage = {
    type: 'result';
    subtype: 'success';
    duration_ms: number;
    duration_api_ms: number;
    is_error: boolean;
    num_turns: number;
    result: string;
    total_cost_usd: number;
    usage: NonNullableUsage;
    modelUsage: {
        [modelName: string]: ModelUsage;
    };
    permission_denials: SDKPermissionDenial[];
    structured_output?: unknown;
    uuid: UUID;
    session_id: string;
} | {
    type: 'result';
    subtype: 'error_during_execution' | 'error_max_turns' | 'error_max_budget_usd' | 'error_max_structured_output_retries';
    duration_ms: number;
    duration_api_ms: number;
    is_error: boolean;
    num_turns: number;
    total_cost_usd: number;
    usage: NonNullableUsage;
    modelUsage: {
        [modelName: string]: ModelUsage;
    };
    permission_denials: SDKPermissionDenial[];
    errors: string[];
    uuid: UUID;
    session_id: string;
};
export type SDKSystemMessage = {
    type: 'system';
    subtype: 'init';
    agents?: string[];
    apiKeySource: ApiKeySource;
    betas?: string[];
    claude_code_version: string;
    cwd: string;
    tools: string[];
    mcp_servers: {
        name: string;
        status: string;
    }[];
    model: string;
    permissionMode: PermissionMode;
    slash_commands: string[];
    output_style: string;
    skills: string[];
    plugins: {
        name: string;
        path: string;
    }[];
    uuid: UUID;
    session_id: string;
};
export type SDKPartialAssistantMessage = {
    type: 'stream_event';
    event: RawMessageStreamEvent;
    parent_tool_use_id: string | null;
    uuid: UUID;
    session_id: string;
};
export type SDKCompactBoundaryMessage = {
    type: 'system';
    subtype: 'compact_boundary';
    compact_metadata: {
        trigger: 'manual' | 'auto';
        pre_tokens: number;
    };
    uuid: UUID;
    session_id: string;
};
export type SDKStatus = 'compacting' | null;
export type SDKStatusMessage = {
    type: 'system';
    subtype: 'status';
    status: SDKStatus;
    uuid: UUID;
    session_id: string;
};
export type SDKHookResponseMessage = {
    type: 'system';
    subtype: 'hook_response';
    hook_name: string;
    hook_event: string;
    stdout: string;
    stderr: string;
    exit_code?: number;
    uuid: UUID;
    session_id: string;
};
export type SDKToolProgressMessage = {
    type: 'tool_progress';
    tool_use_id: string;
    tool_name: string;
    parent_tool_use_id: string | null;
    elapsed_time_seconds: number;
    uuid: UUID;
    session_id: string;
};
export type SDKAuthStatusMessage = {
    type: 'auth_status';
    isAuthenticating: boolean;
    output: string[];
    error?: string;
    uuid: UUID;
    session_id: string;
};
export type SDKMessage = SDKAssistantMessage | SDKUserMessage | SDKUserMessageReplay | SDKResultMessage | SDKSystemMessage | SDKPartialAssistantMessage | SDKCompactBoundaryMessage | SDKStatusMessage | SDKHookResponseMessage | SDKToolProgressMessage | SDKAuthStatusMessage;
