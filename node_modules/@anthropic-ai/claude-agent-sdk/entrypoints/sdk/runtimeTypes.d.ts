import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import type { CallToolResult } from '@modelcontextprotocol/sdk/types.js';
import type { ZodRawShape } from 'zod';
import type { ZodRawShape as Zod4RawShape } from 'zod/v4';
export type AnyZodRawShape = ZodRawShape | Zod4RawShape;
export type InferShape<T extends AnyZodRawShape> = {
    [K in keyof T]: T[K] extends {
        _output: infer O;
    } ? O : never;
} & {};
import type { SpawnedProcess, SpawnOptions } from '../../transport/processTransportTypes.js';
import type { Transport } from '../../transport/transport.js';
import type { PermissionResult, PermissionUpdate, PermissionMode, HookInput, HookJSONOutput, HookEvent, McpStdioServerConfig, McpSSEServerConfig, McpHttpServerConfig, McpSdkServerConfig, SlashCommand, ModelInfo, McpServerStatus, AccountInfo, McpSetServersResult, RewindFilesResult, SDKMessage, SDKUserMessage, OutputFormat, SdkBeta, AgentDefinition, SettingSource, SdkPluginConfig, SandboxSettings } from './coreTypes.js';
export type { SpawnOptions, SpawnedProcess, Transport };
/**
 * Permission callback function for controlling tool usage.
 * Called before each tool execution to determine if it should be allowed.
 */
export type CanUseTool = (toolName: string, input: Record<string, unknown>, options: {
    /** Signaled if the operation should be aborted. */
    signal: AbortSignal;
    /**
     * Suggestions for updating permissions so that the user will not be
     * prompted again for this tool during this session.
     *
     * Typically if presenting the user an option 'always allow' or similar,
     * then this full set of suggestions should be returned as the
     * `updatedPermissions` in the PermissionResult.
     */
    suggestions?: PermissionUpdate[];
    /**
     * The file path that triggered the permission request, if applicable.
     * For example, when a Bash command tries to access a path outside allowed directories.
     */
    blockedPath?: string;
    /** Explains why this permission request was triggered. */
    decisionReason?: string;
    /**
     * Unique identifier for this specific tool call within the assistant message.
     * Multiple tool calls in the same assistant message will have different toolUseIDs.
     */
    toolUseID: string;
    /** If running within the context of a sub-agent, the sub-agent's ID. */
    agentID?: string;
}) => Promise<PermissionResult>;
/**
 * Hook callback function for responding to events during execution.
 */
export type HookCallback = (input: HookInput, toolUseID: string | undefined, options: {
    signal: AbortSignal;
}) => Promise<HookJSONOutput>;
/**
 * Hook callback matcher containing hook callbacks and optional pattern matching.
 */
export interface HookCallbackMatcher {
    matcher?: string;
    hooks: HookCallback[];
    /** Timeout in seconds for all hooks in this matcher */
    timeout?: number;
}
/**
 * MCP SDK server config with an actual McpServer instance.
 * Not serializable - contains a live McpServer object.
 */
export type McpSdkServerConfigWithInstance = McpSdkServerConfig & {
    instance: McpServer;
};
/**
 * Union of all MCP server config types, including those with non-serializable instances.
 */
export type McpServerConfig = McpStdioServerConfig | McpSSEServerConfig | McpHttpServerConfig | McpSdkServerConfigWithInstance;
/**
 * MCP tool definition for SDK servers.
 * Contains a handler function, so not serializable.
 * Supports both Zod 3 and Zod 4 schemas.
 */
export type SdkMcpToolDefinition<Schema extends AnyZodRawShape = AnyZodRawShape> = {
    name: string;
    description: string;
    inputSchema: Schema;
    handler: (args: InferShape<Schema>, extra: unknown) => Promise<CallToolResult>;
};
/**
 * Query interface with methods for controlling query execution.
 * Extends AsyncGenerator and has methods, so not serializable.
 */
export interface Query extends AsyncGenerator<SDKMessage, void> {
    /**
     * Control Requests
     * The following methods are control requests, and are only supported when
     * streaming input/output is used.
     */
    /**
     * Interrupt the current query execution. The query will stop processing
     * and return control to the caller.
     */
    interrupt(): Promise<void>;
    /**
     * Change the permission mode for the current session.
     * Only available in streaming input mode.
     *
     * @param mode - The new permission mode to set
     */
    setPermissionMode(mode: PermissionMode): Promise<void>;
    /**
     * Change the model used for subsequent responses.
     * Only available in streaming input mode.
     *
     * @param model - The model identifier to use, or undefined to use the default
     */
    setModel(model?: string): Promise<void>;
    /**
     * Set the maximum number of thinking tokens the model is allowed to use
     * when generating its response. This can be used to limit the amount of
     * tokens the model uses for its response, which can help control cost and
     * latency.
     *
     * Use `null` to clear any previously set limit and allow the model to
     * use the default maximum thinking tokens.
     *
     * @param maxThinkingTokens - Maximum tokens for thinking, or null to clear the limit
     */
    setMaxThinkingTokens(maxThinkingTokens: number | null): Promise<void>;
    /**
     * Get the list of available skills for the current session.
     *
     * @returns Array of available skills with their names and descriptions
     */
    supportedCommands(): Promise<SlashCommand[]>;
    /**
     * Get the list of available models.
     *
     * @returns Array of model information including display names and descriptions
     */
    supportedModels(): Promise<ModelInfo[]>;
    /**
     * Get the current status of all configured MCP servers.
     *
     * @returns Array of MCP server statuses (connected, failed, needs-auth, pending)
     */
    mcpServerStatus(): Promise<McpServerStatus[]>;
    /**
     * Get information about the authenticated account.
     *
     * @returns Account information including email, organization, and subscription type
     */
    accountInfo(): Promise<AccountInfo>;
    /**
     * Rewind tracked files to their state at a specific user message.
     * Requires file checkpointing to be enabled via the `enableFileCheckpointing` option.
     *
     * @param userMessageId - UUID of the user message to rewind to
     * @param options - Options object with optional `dryRun` boolean to preview changes without modifying files
     * @returns Object with canRewind boolean, optional error message, and file change statistics
     */
    rewindFiles(userMessageId: string, options?: {
        dryRun?: boolean;
    }): Promise<RewindFilesResult>;
    /**
     * Dynamically set the MCP servers for this session.
     * This replaces the current set of dynamically-added MCP servers with the provided set.
     * Servers that are removed will be disconnected, and new servers will be connected.
     *
     * Supports both process-based servers (stdio, sse, http) and SDK servers (in-process).
     * SDK servers are handled locally in the SDK process, while process-based servers
     * are managed by the CLI subprocess.
     *
     * Note: This only affects servers added dynamically via this method or the SDK.
     * Servers configured via settings files are not affected.
     *
     * @param servers - Record of server name to configuration. Pass an empty object to remove all dynamic servers.
     * @returns Information about which servers were added, removed, and any connection errors
     */
    setMcpServers(servers: Record<string, McpServerConfig>): Promise<McpSetServersResult>;
    /**
     * Stream input messages to the query.
     * Used internally for multi-turn conversations.
     *
     * @param stream - Async iterable of user messages to send
     */
    streamInput(stream: AsyncIterable<SDKUserMessage>): Promise<void>;
}
/**
 * V2 API - UNSTABLE
 * Options for creating a session.
 */
export type SDKSessionOptions = {
    /** Model to use */
    model: string;
    /** Path to Claude Code executable */
    pathToClaudeCodeExecutable?: string;
    /** Executable to use (node, bun) */
    executable?: 'node' | 'bun';
    /** Arguments to pass to executable */
    executableArgs?: string[];
    /**
     * Environment variables to pass to the Claude Code process.
     * Defaults to `process.env`.
     */
    env?: {
        [envVar: string]: string | undefined;
    };
};
/**
 * V2 API - UNSTABLE
 * Session interface for multi-turn conversations.
 * Has methods, so not serializable.
 */
export interface SDKSession {
    /**
     * The session ID. Available after receiving the first message.
     * For resumed sessions, available immediately.
     * Throws if accessed before the session is initialized.
     */
    readonly sessionId: string;
    /** Send a message to the agent */
    send(message: string | SDKUserMessage): Promise<void>;
    /** Stream messages from the agent */
    stream(): AsyncGenerator<SDKMessage, void>;
    /** Close the session */
    close(): void;
    /** Async disposal support (calls close if not already closed) */
    [Symbol.asyncDispose](): Promise<void>;
}
/**
 * Options for the query function.
 * Contains callbacks and other non-serializable fields.
 */
export type Options = {
    /**
     * Controller for cancelling the query. When aborted, the query will stop
     * and clean up resources.
     */
    abortController?: AbortController;
    /**
     * Additional directories Claude can access beyond the current working directory.
     * Paths should be absolute.
     */
    additionalDirectories?: string[];
    /**
     * Programmatically define custom subagents that can be invoked via the Task tool.
     * Keys are agent names, values are agent definitions.
     *
     * @example
     * ```typescript
     * agents: {
     *   'test-runner': {
     *     description: 'Runs tests and reports results',
     *     prompt: 'You are a test runner...',
     *     tools: ['Read', 'Grep', 'Glob', 'Bash']
     *   }
     * }
     * ```
     */
    agents?: Record<string, AgentDefinition>;
    /**
     * List of tool names that are auto-allowed without prompting for permission.
     * These tools will execute automatically without asking the user for approval.
     * To restrict which tools are available, use the `tools` option instead.
     */
    allowedTools?: string[];
    /**
     * Custom permission handler for controlling tool usage. Called before each
     * tool execution to determine if it should be allowed, denied, or prompt the user.
     */
    canUseTool?: CanUseTool;
    /**
     * Continue the most recent conversation instead of starting a new one.
     * Mutually exclusive with `resume`.
     */
    continue?: boolean;
    /**
     * Current working directory for the session. Defaults to `process.cwd()`.
     */
    cwd?: string;
    /**
     * List of tool names that are disallowed. These tools will be removed
     * from the model's context and cannot be used, even if they would
     * otherwise be allowed.
     */
    disallowedTools?: string[];
    /**
     * Specify the base set of available built-in tools.
     * - `string[]` - Array of specific tool names (e.g., `['Bash', 'Read', 'Edit']`)
     * - `[]` (empty array) - Disable all built-in tools
     * - `{ type: 'preset'; preset: 'claude_code' }` - Use all default Claude Code tools
     */
    tools?: string[] | {
        type: 'preset';
        preset: 'claude_code';
    };
    /**
     * Environment variables to pass to the Claude Code process.
     * Defaults to `process.env`.
     */
    env?: {
        [envVar: string]: string | undefined;
    };
    /**
     * JavaScript runtime to use for executing Claude Code.
     * Auto-detected if not specified.
     */
    executable?: 'bun' | 'deno' | 'node';
    /**
     * Additional arguments to pass to the JavaScript runtime executable.
     */
    executableArgs?: string[];
    /**
     * Additional CLI arguments to pass to Claude Code.
     * Keys are argument names (without --), values are argument values.
     * Use `null` for boolean flags.
     */
    extraArgs?: Record<string, string | null>;
    /**
     * Fallback model to use if the primary model fails or is unavailable.
     */
    fallbackModel?: string;
    /**
     * Enable file checkpointing to track file changes during the session.
     * When enabled, files can be rewound to their state at any user message
     * using `Query.rewindFiles()`.
     *
     * File checkpointing creates backups of files before they are modified,
     * allowing you to restore them to previous states.
     */
    enableFileCheckpointing?: boolean;
    /**
     * When true, resumed sessions will fork to a new session ID rather than
     * continuing the previous session. Use with `resume`.
     */
    forkSession?: boolean;
    /**
     * Enable beta features. Currently supported:
     * - `'context-1m-2025-08-07'` - Enable 1M token context window (Sonnet 4/4.5 only)
     *
     * @see https://docs.anthropic.com/en/api/beta-headers
     */
    betas?: SdkBeta[];
    /**
     * Hook callbacks for responding to various events during execution.
     * Hooks can modify behavior, add context, or implement custom logic.
     *
     * @example
     * ```typescript
     * hooks: {
     *   PreToolUse: [{
     *     hooks: [async (input) => ({ continue: true })]
     *   }]
     * }
     * ```
     */
    hooks?: Partial<Record<HookEvent, HookCallbackMatcher[]>>;
    /**
     * When false, disables session persistence to disk. Sessions will not be
     * saved to ~/.claude/projects/ and cannot be resumed later. Useful for
     * ephemeral or automated workflows where session history is not needed.
     *
     * @default true
     */
    persistSession?: boolean;
    /**
     * Include partial/streaming message events in the output.
     * When true, `SDKPartialAssistantMessage` events will be emitted during streaming.
     */
    includePartialMessages?: boolean;
    /**
     * Maximum number of tokens the model can use for its thinking/reasoning process.
     * Helps control cost and latency for complex tasks.
     */
    maxThinkingTokens?: number;
    /**
     * Maximum number of conversation turns before the query stops.
     * A turn consists of a user message and assistant response.
     */
    maxTurns?: number;
    /**
     * Maximum budget in USD for the query. The query will stop if this
     * budget is exceeded, returning an `error_max_budget_usd` result.
     */
    maxBudgetUsd?: number;
    /**
     * MCP (Model Context Protocol) server configurations.
     * Keys are server names, values are server configurations.
     *
     * @example
     * ```typescript
     * mcpServers: {
     *   'my-server': {
     *     command: 'node',
     *     args: ['./my-mcp-server.js']
     *   }
     * }
     * ```
     */
    mcpServers?: Record<string, McpServerConfig>;
    /**
     * Claude model to use. Defaults to the CLI default model.
     * Examples: 'claude-sonnet-4-5-20250929', 'claude-opus-4-20250514'
     */
    model?: string;
    /**
     * Output format configuration for structured responses.
     * When specified, the agent will return structured data matching the schema.
     *
     * @example
     * ```typescript
     * outputFormat: {
     *   type: 'json_schema',
     *   schema: { type: 'object', properties: { result: { type: 'string' } } }
     * }
     * ```
     */
    outputFormat?: OutputFormat;
    /**
     * Path to the Claude Code executable. Uses the built-in executable if not specified.
     */
    pathToClaudeCodeExecutable?: string;
    /**
     * Permission mode for the session.
     * - `'default'` - Standard permission behavior, prompts for dangerous operations
     * - `'acceptEdits'` - Auto-accept file edit operations
     * - `'bypassPermissions'` - Bypass all permission checks (requires `allowDangerouslySkipPermissions`)
     * - `'plan'` - Planning mode, no execution of tools
     * - `'dontAsk'` - Don't prompt for permissions, deny if not pre-approved
     */
    permissionMode?: PermissionMode;
    /**
     * Must be set to `true` when using `permissionMode: 'bypassPermissions'`.
     * This is a safety measure to ensure intentional bypassing of permissions.
     */
    allowDangerouslySkipPermissions?: boolean;
    /**
     * MCP tool name to use for permission prompts. When set, permission requests
     * will be routed through this MCP tool instead of the default handler.
     */
    permissionPromptToolName?: string;
    /**
     * Load plugins for this session. Plugins provide custom commands, agents,
     * skills, and hooks that extend Claude Code's capabilities.
     *
     * Currently only local plugins are supported via the 'local' type.
     *
     * @example
     * ```typescript
     * plugins: [
     *   { type: 'local', path: './my-plugin' },
     *   { type: 'local', path: '/absolute/path/to/plugin' }
     * ]
     * ```
     */
    plugins?: SdkPluginConfig[];
    /**
     * Session ID to resume. Loads the conversation history from the specified session.
     */
    resume?: string;
    /**
     * When resuming, only resume messages up to and including the message with this UUID.
     * Use with `resume`. This allows you to resume from a specific point in the conversation.
     * The message ID should be from `SDKAssistantMessage.uuid`.
     */
    resumeSessionAt?: string;
    /**
     * Sandbox settings for command execution isolation.
     *
     * When enabled, commands are executed in a sandboxed environment that restricts
     * filesystem and network access. This provides an additional security layer.
     *
     * **Important:** Filesystem and network restrictions are configured via permission
     * rules, not via these sandbox settings:
     * - Filesystem access: Use `Read` and `Edit` permission rules
     * - Network access: Use `WebFetch` permission rules
     *
     * These sandbox settings control sandbox behavior (enabled, auto-allow, etc.),
     * while the actual access restrictions come from your permission configuration.
     *
     * @example Enable sandboxing with auto-allow
     * ```typescript
     * sandbox: {
     *   enabled: true,
     *   autoAllowBashIfSandboxed: true
     * }
     * ```
     *
     * @example Configure network options (not restrictions)
     * ```typescript
     * sandbox: {
     *   enabled: true,
     *   network: {
     *     allowLocalBinding: true,
     *     allowUnixSockets: ['/var/run/docker.sock']
     *   }
     * }
     * ```
     *
     * @see https://docs.anthropic.com/en/docs/claude-code/settings#sandbox-settings
     */
    sandbox?: SandboxSettings;
    /**
     * Control which filesystem settings to load.
     * - `'user'` - Global user settings (`~/.claude/settings.json`)
     * - `'project'` - Project settings (`.claude/settings.json`)
     * - `'local'` - Local settings (`.claude/settings.local.json`)
     *
     * When omitted or empty, no filesystem settings are loaded (SDK isolation mode).
     * Must include `'project'` to load CLAUDE.md files.
     */
    settingSources?: SettingSource[];
    /**
     * Callback for stderr output from the Claude Code process.
     * Useful for debugging and logging.
     */
    stderr?: (data: string) => void;
    /**
     * Enforce strict validation of MCP server configurations.
     * When true, invalid configurations will cause errors instead of warnings.
     */
    strictMcpConfig?: boolean;
    /**
     * System prompt configuration.
     * - `string` - Use a custom system prompt
     * - `{ type: 'preset', preset: 'claude_code' }` - Use Claude Code's default system prompt
     * - `{ type: 'preset', preset: 'claude_code', append: '...' }` - Use default prompt with appended instructions
     *
     * @example Custom prompt
     * ```typescript
     * systemPrompt: 'You are a helpful coding assistant.'
     * ```
     *
     * @example Default with additions
     * ```typescript
     * systemPrompt: {
     *   type: 'preset',
     *   preset: 'claude_code',
     *   append: 'Always explain your reasoning.'
     * }
     * ```
     */
    systemPrompt?: string | {
        type: 'preset';
        preset: 'claude_code';
        append?: string;
    };
    /**
     * Custom function to spawn the Claude Code process.
     * Use this to run Claude Code in VMs, containers, or remote environments.
     *
     * When provided, this function is called instead of the default local spawn.
     * The default behavior checks if the executable exists before spawning.
     *
     * @example
     * ```typescript
     * spawnClaudeCodeProcess: (options) => {
     *   // Custom spawn logic for VM execution
     *   // options contains: command, args, cwd, env, signal
     *   return myVMProcess; // Must satisfy SpawnedProcess interface
     * }
     * ```
     */
    spawnClaudeCodeProcess?: (options: SpawnOptions) => SpawnedProcess;
};
