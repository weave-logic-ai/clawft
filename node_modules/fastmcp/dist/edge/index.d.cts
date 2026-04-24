import { StandardSchemaV1 } from '@standard-schema/spec';
import { Hono } from 'hono';
import { z } from 'zod';
import { Transport } from '@modelcontextprotocol/sdk/shared/transport.js';
import { JSONRPCMessage, MessageExtraInfo, RequestId } from '@modelcontextprotocol/sdk/types.js';

/**
 * Web-standard Streamable HTTP Server Transport for MCP
 *
 * This transport implements the MCP Streamable HTTP specification using
 * web standard APIs (Request, Response, TransformStream) for compatibility
 * with edge runtimes like Cloudflare Workers, Deno, and Bun.
 */

type EventId = string;
/**
 * Interface for resumability support via event storage
 */
interface EventStore {
    getStreamIdForEventId?(eventId: EventId): Promise<StreamId | undefined>;
    replayEventsAfter(lastEventId: EventId, options: {
        send: (eventId: EventId, message: JSONRPCMessage) => Promise<void>;
    }): Promise<StreamId>;
    storeEvent(streamId: StreamId, message: JSONRPCMessage): Promise<EventId>;
}
type StreamId = string;
/**
 * Configuration options for WebStreamableHTTPServerTransport
 */
interface WebStreamableHTTPServerTransportOptions {
    /**
     * If true, return JSON responses instead of SSE streams
     */
    enableJsonResponse?: boolean;
    /**
     * Event store for resumability support
     */
    eventStore?: EventStore;
    /**
     * Callback for session close events
     */
    onsessionclosed?: (sessionId: string) => Promise<void> | void;
    /**
     * Callback for session initialization events
     */
    onsessioninitialized?: (sessionId: string) => Promise<void> | void;
    /**
     * Function that generates a session ID for the transport.
     * Return undefined to disable session management (stateless mode).
     */
    sessionIdGenerator: (() => string) | undefined;
}
/**
 * Web-standard Server transport for Streamable HTTP.
 * Uses web APIs (Request, Response, TransformStream) for edge runtime compatibility.
 */
declare class WebStreamableHTTPServerTransport implements Transport {
    onclose?: () => void;
    onerror?: (error: Error) => void;
    onmessage?: (message: JSONRPCMessage, extra?: MessageExtraInfo) => void;
    sessionId?: string;
    private _enableJsonResponse;
    private _encoder;
    private _eventStore?;
    private _onsessionclosed?;
    private _onsessioninitialized?;
    private _pendingResponses;
    private _requestToStreamMapping;
    private _standaloneSseStreamId;
    private _started;
    private _streamMapping;
    private sessionIdGenerator;
    constructor(options: WebStreamableHTTPServerTransportOptions);
    /**
     * Close the transport
     */
    close(): Promise<void>;
    /**
     * Handles an incoming web Request and returns a Response
     */
    handleRequest(request: Request, parsedBody?: unknown): Promise<Response>;
    /**
     * Send a message to connected clients
     */
    send(message: JSONRPCMessage, options?: {
        relatedRequestId?: RequestId;
    }): Promise<void>;
    start(): Promise<void>;
    /**
     * Create an error response
     */
    private createErrorResponse;
    /**
     * Get common response headers
     */
    private getResponseHeaders;
    /**
     * Handles DELETE requests to terminate sessions
     */
    private handleDeleteRequest;
    /**
     * Handles GET requests for SSE stream
     */
    private handleGetRequest;
    /**
     * Handles POST requests containing JSON-RPC messages
     */
    private handlePostRequest;
    /**
     * Replay events for resumability
     */
    private handleReplayEvents;
    /**
     * Handles unsupported HTTP methods
     */
    private handleUnsupportedRequest;
    /**
     * Write an SSE event to the stream
     */
    private writeSSEEvent;
    /**
     * Write an SSE event with ID to the stream
     */
    private writeSSEEventWithId;
}

/**
 * FastMCP Edge Runtime Support
 *
 * This module provides edge runtime compatibility for FastMCP, enabling
 * deployment to Cloudflare Workers, Deno Deploy, and other edge platforms.
 *
 * @example
 * ```typescript
 * // Cloudflare Workers
 * import { EdgeFastMCP } from "fastmcp/edge";
 * import { z } from "zod";
 *
 * const server = new EdgeFastMCP({ name: "MyMCP", version: "1.0.0" });
 *
 * server.addTool({
 *   name: "hello",
 *   description: "Say hello",
 *   parameters: z.object({ name: z.string() }),
 *   execute: async ({ name }) => `Hello, ${name}!`,
 * });
 *
 * export default server;
 * ```
 */

/**
 * Options for EdgeFastMCP
 */
interface EdgeFastMCPOptions {
    description?: string;
    logger?: EdgeLogger;
    /**
     * Base path for MCP endpoints (default: "/mcp")
     */
    mcpPath?: string;
    name: string;
    version: string;
}
/**
 * Type for edge runtime fetch handler
 */
type EdgeFetchHandler = (request: Request) => Promise<Response>;
/**
 * Logger interface for edge environments
 */
interface EdgeLogger {
    debug(...args: unknown[]): void;
    error(...args: unknown[]): void;
    info(...args: unknown[]): void;
    log(...args: unknown[]): void;
    warn(...args: unknown[]): void;
}
/**
 * Prompt definition for EdgeFastMCP
 */
interface EdgePrompt {
    arguments?: Array<{
        description?: string;
        name: string;
        required?: boolean;
    }>;
    description?: string;
    load: (args: Record<string, string>) => Promise<{
        messages: Array<{
            content: {
                text: string;
                type: string;
            };
            role: string;
        }>;
    } | string>;
    name: string;
}
/**
 * Resource definition for EdgeFastMCP
 */
interface EdgeResource {
    description?: string;
    load: () => Promise<{
        blob?: string;
        mimeType?: string;
        text?: string;
    } | string>;
    mimeType?: string;
    name: string;
    uri: string;
}
/**
 * Tool definition for EdgeFastMCP
 */
interface EdgeTool<TParams = unknown> {
    description: string;
    execute: (params: TParams) => Promise<{
        content: Array<{
            data?: string;
            mimeType?: string;
            text?: string;
            type: string;
        }>;
    } | string>;
    name: string;
    parameters?: StandardSchemaV1<TParams> | z.ZodType<TParams>;
}
/**
 * Edge-compatible FastMCP server for Cloudflare Workers, Deno, and Bun
 *
 * This is a simplified implementation optimized for stateless edge environments.
 * It uses web-standard APIs only (no Node.js dependencies).
 */
declare class EdgeFastMCP {
    #private;
    constructor(options: EdgeFastMCPOptions);
    /**
     * Add a prompt to the server
     */
    addPrompt(prompt: EdgePrompt): this;
    /**
     * Add a resource to the server
     */
    addResource(resource: EdgeResource): this;
    /**
     * Add a tool to the server
     */
    addTool<TParams>(tool: EdgeTool<TParams>): this;
    /**
     * Handle an incoming request (main entry point for edge runtimes)
     */
    fetch(request: Request): Promise<Response>;
    /**
     * Get the Hono app for adding custom routes
     */
    getApp(): Hono;
}

export { EdgeFastMCP, type EdgeFastMCPOptions, type EdgeFetchHandler, type EdgeLogger, type EdgePrompt, type EdgeResource, type EdgeTool, type EventStore, WebStreamableHTTPServerTransport, type WebStreamableHTTPServerTransportOptions };
