/**
 * Main entrypoint for Claude Code Agent SDK types.
 *
 * This file re-exports the public SDK API from:
 * - sdk/coreTypes.ts - Common serializable types (messages, configs)
 * - sdk/runtimeTypes.ts - Non-serializable types (callbacks, interfaces)
 *
 * SDK builders who need control protocol types should import from
 * sdk/controlTypes.ts directly.
 */
import type { CallToolResult } from '@modelcontextprotocol/sdk/types.js';
export * from './sdk/coreTypes.js';
export * from './sdk/runtimeTypes.js';
import type { Query, Options, SDKSessionOptions, SDKSession, SdkMcpToolDefinition, McpSdkServerConfigWithInstance, AnyZodRawShape, InferShape } from './sdk/runtimeTypes.js';
import type { SDKUserMessage, SDKResultMessage } from './sdk/coreTypes.js';
export declare function tool<Schema extends AnyZodRawShape>(_name: string, _description: string, _inputSchema: Schema, _handler: (args: InferShape<Schema>, extra: unknown) => Promise<CallToolResult>): SdkMcpToolDefinition<Schema>;
type CreateSdkMcpServerOptions = {
    name: string;
    version?: string;
    tools?: Array<SdkMcpToolDefinition<any>>;
};
/**
 * Creates an MCP server instance that can be used with the SDK transport.
 * This allows SDK users to define custom tools that run in the same process.
 *
 * If your SDK MCP calls will run longer than 60s, override CLAUDE_CODE_STREAM_CLOSE_TIMEOUT
 */
export declare function createSdkMcpServer(_options: CreateSdkMcpServerOptions): McpSdkServerConfigWithInstance;
export declare class AbortError extends Error {
}
export declare function query(_params: {
    prompt: string | AsyncIterable<SDKUserMessage>;
    options?: Options;
}): Query;
/**
 * V2 API - UNSTABLE
 * Create a persistent session for multi-turn conversations.
 */
export declare function unstable_v2_createSession(_options: SDKSessionOptions): SDKSession;
/**
 * V2 API - UNSTABLE
 * Resume an existing session by ID.
 */
export declare function unstable_v2_resumeSession(_sessionId: string, _options: SDKSessionOptions): SDKSession;
/**
 * V2 API - UNSTABLE
 * One-shot convenience function for single prompts.
 *
 * @example
 * ```typescript
 * const result = await unstable_v2_prompt("What files are here?", {
 *   model: 'claude-sonnet-4-5-20250929'
 * })
 * ```
 */
export declare function unstable_v2_prompt(_message: string, _options: SDKSessionOptions): Promise<SDKResultMessage>;
