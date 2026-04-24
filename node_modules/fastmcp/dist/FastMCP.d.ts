import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { EventStore } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { RequestOptions } from '@modelcontextprotocol/sdk/shared/protocol.js';
import { Transport } from '@modelcontextprotocol/sdk/shared/transport.js';
import { ResourceLink, Root, ClientCapabilities, GetPromptResult, CreateMessageRequestSchema } from '@modelcontextprotocol/sdk/types.js';
export { ResourceLink } from '@modelcontextprotocol/sdk/types.js';
import { StandardSchemaV1 } from '@standard-schema/spec';
import { EventEmitter } from 'events';
import { Hono } from 'hono';
import http from 'http';
import { StrictEventEmitter } from 'strict-event-emitter-types';
import { z } from 'zod';
import { A as AuthProvider, O as OAuthSession, a as OAuthProxy } from './OAuthProvider-R8buLRa8.js';
export { j as AuthProviderConfig, b as AzureProvider, k as AzureProviderConfig, l as AzureSession, m as GenericOAuthProviderConfig, G as GitHubProvider, n as GitHubSession, c as GoogleProvider, o as GoogleSession, d as OAuthProvider, g as getAuthSession, r as requireAll, e as requireAny, f as requireAuth, h as requireRole, i as requireScopes } from './OAuthProvider-R8buLRa8.js';
import 'node:http';

declare class DiscoveryDocumentCache {
    #private;
    get size(): number;
    /**
     * @param options - configuration options
     * @param options.ttl - time-to-live in miliseconds
     */
    constructor(options?: {
        ttl?: number;
    });
    /**
     * @param url - optional URL to clear. if omitted, clears all cached documents.
     */
    clear(url?: string): void;
    /**
     * fetches a discovery document from the given URL.
     * uses cached value if available and not expired.
     * coalesces concurrent requests for the same URL to prevent duplicate fetches.
     *
     * @param url - the discovery document URL (e.g., /.well-known/openid-configuration)
     * @returns the discovery document as a JSON object
     * @throws Error if the fetch fails or returns non-OK status
     */
    get(url: string): Promise<unknown>;
    /**
     * @param url - the URL to check
     * @returns true if the URL is cached and nott expired
     */
    has(url: string): boolean;
}

interface Logger {
    debug(...args: unknown[]): void;
    error(...args: unknown[]): void;
    info(...args: unknown[]): void;
    log(...args: unknown[]): void;
    warn(...args: unknown[]): void;
}
type SSEServer = {
    close: () => Promise<void>;
};
type FastMCPEvents<T extends FastMCPSessionAuth> = {
    connect: (event: {
        session: FastMCPSession<T>;
    }) => void;
    disconnect: (event: {
        session: FastMCPSession<T>;
    }) => void;
};
type FastMCPSessionEvents = {
    error: (event: {
        error: Error;
    }) => void;
    ready: () => void;
    rootsChanged: (event: {
        roots: Root[];
    }) => void;
};
declare const imageContent: (input: {
    buffer: Buffer;
} | {
    path: string;
} | {
    url: string;
}) => Promise<ImageContent>;
declare const audioContent: (input: {
    buffer: Buffer;
} | {
    path: string;
} | {
    url: string;
}) => Promise<AudioContent>;
type Context<T extends FastMCPSessionAuth> = {
    client: {
        version: ReturnType<Server["getClientVersion"]>;
    };
    log: {
        debug: (message: string, data?: SerializableValue) => void;
        error: (message: string, data?: SerializableValue) => void;
        info: (message: string, data?: SerializableValue) => void;
        warn: (message: string, data?: SerializableValue) => void;
    };
    reportProgress: (progress: Progress) => Promise<void>;
    /**
     * Request ID from the current MCP request.
     * Available for all transports when the client provides it.
     */
    requestId?: string;
    session: T | undefined;
    /**
     * Session ID from the Mcp-Session-Id header.
     * Only available for HTTP-based transports (SSE, HTTP Stream).
     * Can be used to track per-session state, implement session-specific
     * counters, or maintain user-specific data across multiple requests.
     */
    sessionId?: string;
    streamContent: (content: Content | Content[]) => Promise<void>;
};
type Extra = unknown;
type Extras = Record<string, Extra>;
type Literal = boolean | null | number | string | undefined;
type Progress = {
    /**
     * The progress thus far. This should increase every time progress is made, even if the total is unknown.
     */
    progress: number;
    /**
     * Total number of items to process (or total progress required), if known.
     */
    total?: number;
};
type SerializableValue = {
    [key: string]: SerializableValue;
} | Literal | SerializableValue[];
type TextContent = {
    text: string;
    type: "text";
};
type ToolParameters = StandardSchemaV1;
declare abstract class FastMCPError extends Error {
    constructor(message?: string);
}
declare class UnexpectedStateError extends FastMCPError {
    extras?: Extras;
    constructor(message: string, extras?: Extras);
}
/**
 * An error that is meant to be surfaced to the user.
 */
declare class UserError extends UnexpectedStateError {
}
type ImageContent = {
    data: string;
    mimeType: string;
    type: "image";
};
type AudioContent = {
    data: string;
    mimeType: string;
    type: "audio";
};
type ResourceContent = {
    resource: {
        blob?: string;
        mimeType?: string;
        text?: string;
        uri: string;
    };
    type: "resource";
};
type Content = AudioContent | ImageContent | ResourceContent | ResourceLink | TextContent;
type ContentResult = {
    content: Content[];
    isError?: boolean;
};
type Completion = {
    hasMore?: boolean;
    total?: number;
    values: string[];
};
type ArgumentValueCompleter<T extends FastMCPSessionAuth = FastMCPSessionAuth> = (value: string, auth?: T) => Promise<Completion>;
type InputPrompt<T extends FastMCPSessionAuth = FastMCPSessionAuth, Arguments extends InputPromptArgument<T>[] = InputPromptArgument<T>[], Args = PromptArgumentsToObject<Arguments>> = {
    arguments?: InputPromptArgument<T>[];
    complete?: (name: string, value: string, auth?: T) => Promise<Completion>;
    description?: string;
    load: (args: Args, auth?: T) => Promise<PromptResult>;
    name: string;
};
type InputPromptArgument<T extends FastMCPSessionAuth = FastMCPSessionAuth> = Readonly<{
    complete?: ArgumentValueCompleter<T>;
    description?: string;
    enum?: string[];
    name: string;
    required?: boolean;
}>;
type InputResourceTemplate<T extends FastMCPSessionAuth, Arguments extends InputResourceTemplateArgument<T>[] = InputResourceTemplateArgument<T>[]> = {
    arguments: Arguments;
    complete?: (name: string, value: string, auth?: T) => Promise<Completion>;
    description?: string;
    load: (args: ResourceTemplateArgumentsToObject<Arguments>, auth?: T) => Promise<ResourceResult | ResourceResult[]>;
    mimeType?: string;
    name: string;
    uriTemplate: string;
};
type InputResourceTemplateArgument<T extends FastMCPSessionAuth = FastMCPSessionAuth> = Readonly<{
    complete?: ArgumentValueCompleter<T>;
    description?: string;
    name: string;
    required?: boolean;
}>;
type LoggingLevel = "alert" | "critical" | "debug" | "emergency" | "error" | "info" | "notice" | "warning";
type Prompt<T extends FastMCPSessionAuth = FastMCPSessionAuth, Arguments extends PromptArgument<T>[] = PromptArgument<T>[], Args = PromptArgumentsToObject<Arguments>> = {
    arguments?: PromptArgument<T>[];
    complete?: (name: string, value: string, auth?: T) => Promise<Completion>;
    description?: string;
    load: (args: Args, auth?: T) => Promise<PromptResult>;
    name: string;
};
type PromptArgument<T extends FastMCPSessionAuth = FastMCPSessionAuth> = Readonly<{
    complete?: ArgumentValueCompleter<T>;
    description?: string;
    enum?: string[];
    name: string;
    required?: boolean;
}>;
type PromptArgumentsToObject<T extends {
    name: string;
    required?: boolean;
}[]> = {
    [K in T[number]["name"]]: Extract<T[number], {
        name: K;
    }>["required"] extends true ? string : string | undefined;
};
type PromptResult = Pick<GetPromptResult, "messages"> | string;
type Resource<T extends FastMCPSessionAuth> = {
    complete?: (name: string, value: string, auth?: T) => Promise<Completion>;
    description?: string;
    load: (auth?: T) => Promise<ResourceResult | ResourceResult[]>;
    mimeType?: string;
    name: string;
    uri: string;
};
type ResourceResult = {
    blob: string;
    mimeType?: string;
    uri?: string;
} | {
    mimeType?: string;
    text: string;
    uri?: string;
};
type ResourceTemplate<T extends FastMCPSessionAuth, Arguments extends ResourceTemplateArgument<T>[] = ResourceTemplateArgument<T>[]> = {
    arguments: Arguments;
    complete?: (name: string, value: string, auth?: T) => Promise<Completion>;
    description?: string;
    load: (args: ResourceTemplateArgumentsToObject<Arguments>, auth?: T) => Promise<ResourceResult | ResourceResult[]>;
    mimeType?: string;
    name: string;
    uriTemplate: string;
};
type ResourceTemplateArgument<T extends FastMCPSessionAuth = FastMCPSessionAuth> = Readonly<{
    complete?: ArgumentValueCompleter<T>;
    description?: string;
    name: string;
    required?: boolean;
}>;
type ResourceTemplateArgumentsToObject<T extends {
    name: string;
}[]> = {
    [K in T[number]["name"]]: string;
};
type SamplingResponse = {
    content: AudioContent | ImageContent | TextContent;
    model: string;
    role: "assistant" | "user";
    stopReason?: "endTurn" | "maxTokens" | "stopSequence" | string;
};
type ServerOptions<T extends FastMCPSessionAuth> = {
    /**
     * Authentication provider for OAuth flows.
     * When provided, automatically configures the `authenticate` function
     * and `oauth` settings.
     *
     * For custom authentication logic, use the `authenticate` option instead.
     * If both are provided, `authenticate` takes precedence.
     *
     * @example
     * ```typescript
     * import { FastMCP, GitHubProvider } from "fastmcp";
     *
     * const server = new FastMCP({
     *   auth: new GitHubProvider({
     *     baseUrl: "http://localhost:8000",
     *     clientId: process.env.GITHUB_CLIENT_ID!,
     *     clientSecret: process.env.GITHUB_CLIENT_SECRET!,
     *   }),
     *   name: "My Server",
     *   version: "1.0.0",
     * });
     * ```
     */
    auth?: AuthProvider<T extends OAuthSession ? T : OAuthSession>;
    authenticate?: Authenticate<T>;
    /**
     * Configuration for the health-check endpoint that can be exposed when the
     * server is running using the HTTP Stream transport. When enabled, the
     * server will respond to an HTTP GET request with the configured path (by
     * default "/health") rendering a plain-text response (by default "ok") and
     * the configured status code (by default 200).
     *
     * The endpoint is only added when the server is started with
     * `transportType: "httpStream"` – it is ignored for the stdio transport.
     */
    health?: {
        /**
         * When set to `false` the health-check endpoint is disabled.
         * @default true
         */
        enabled?: boolean;
        /**
         * Plain-text body returned by the endpoint.
         * @default "ok"
         */
        message?: string;
        /**
         * HTTP path that should be handled.
         * @default "/health"
         */
        path?: string;
        /**
         * HTTP response status that will be returned.
         * @default 200
         */
        status?: number;
    };
    instructions?: string;
    /**
     * Custom logger instance. If not provided, defaults to console.
     * Use this to integrate with your own logging system.
     */
    logger?: Logger;
    name: string;
    /**
     * Configuration for OAuth well-known discovery endpoints that can be exposed
     * when the server is running using HTTP-based transports (SSE or HTTP Stream).
     * When enabled, the server will respond to requests for OAuth discovery endpoints
     * with the configured metadata.
     *
     * The endpoints are only added when the server is started with
     * `transportType: "httpStream"` – they are ignored for the stdio transport.
     * Both SSE and HTTP Stream transports support OAuth endpoints.
     */
    oauth?: {
        /**
         * OAuth Authorization Server metadata for /.well-known/oauth-authorization-server
         *
         * This endpoint follows RFC 8414 (OAuth 2.0 Authorization Server Metadata)
         * and provides metadata about the OAuth 2.0 authorization server.
         *
         * Required by MCP Specification 2025-03-26
         */
        authorizationServer?: {
            authorizationEndpoint: string;
            codeChallengeMethodsSupported?: string[];
            dpopSigningAlgValuesSupported?: string[];
            grantTypesSupported?: string[];
            introspectionEndpoint?: string;
            issuer: string;
            jwksUri?: string;
            opPolicyUri?: string;
            opTosUri?: string;
            registrationEndpoint?: string;
            responseModesSupported?: string[];
            responseTypesSupported: string[];
            revocationEndpoint?: string;
            scopesSupported?: string[];
            serviceDocumentation?: string;
            tokenEndpoint: string;
            tokenEndpointAuthMethodsSupported?: string[];
            tokenEndpointAuthSigningAlgValuesSupported?: string[];
            uiLocalesSupported?: string[];
        };
        /**
         * Whether OAuth discovery endpoints should be enabled.
         */
        enabled: boolean;
        /**
         * OAuth Protected Resource metadata for `/.well-known/oauth-protected-resource`
         *
         * This endpoint follows {@link https://www.rfc-editor.org/rfc/rfc9728.html | RFC 9728}
         * and provides metadata describing how an OAuth 2.0 protected resource (in this case,
         * an MCP server) expects to be accessed.
         *
         * When configured, FastMCP will automatically serve this metadata at the
         * `/.well-known/oauth-protected-resource` endpoint. The `authorizationServers` and `resource`
         * fields are required. All others are optional and will be omitted from the published
         * metadata if not specified.
         *
         * This satisfies the requirements of the MCP Authorization specification's
         * {@link https://modelcontextprotocol.io/specification/2025-06-18/basic/authorization#authorization-server-location | Authorization Server Location section}.
         *
         * Clients consuming this metadata MUST validate that any presented values comply with
         * RFC 9728, including strict validation of the `resource` identifier and intended audience
         * when access tokens are issued and presented (per RFC 8707 §2).
         *
         * @remarks Required by MCP Specification version 2025-06-18
         */
        protectedResource?: {
            /**
             * Allows for additional metadata fields beyond those defined in RFC 9728.
             *
             * @remarks This supports vendor-specific or experimental extensions.
             * @see {@link https://www.rfc-editor.org/rfc/rfc9728.html#section-2.3 | RFC 9728 §2.3}
             */
            [key: string]: unknown;
            /**
             * Supported values for the `authorization_details` parameter (RFC 9396).
             *
             * @remarks Used when fine-grained access control is in play.
             * @see {@link https://www.rfc-editor.org/rfc/rfc9728.html#section-2-2.23 | RFC 9728 §2.2.23}
             */
            authorizationDetailsTypesSupported?: string[];
            /**
             * List of OAuth 2.0 authorization server issuer identifiers.
             *
             * These correspond to ASes that can issue access tokens for this protected resource.
             * MCP clients use these values to locate the relevant `/.well-known/oauth-authorization-server`
             * metadata for initiating the OAuth flow.
             *
             * @remarks Required by the MCP spec. MCP servers MUST provide at least one issuer.
             * Clients are responsible for choosing among them (see RFC 9728 §7.6).
             * @see {@link https://www.rfc-editor.org/rfc/rfc9728.html#section-2-2.3 | RFC 9728 §2.2.3}
             */
            authorizationServers: string[];
            /**
             * List of supported methods for presenting OAuth 2.0 bearer tokens.
             *
             * @remarks Valid values are `header`, `body`, and `query`.
             * If omitted, clients MAY assume only `header` is supported, per RFC 6750.
             * This is a client-side interpretation and not a serialization default.
             * @see {@link https://www.rfc-editor.org/rfc/rfc9728.html#section-2-2.9 | RFC 9728 §2.2.9}
             */
            bearerMethodsSupported?: string[];
            /**
             * Whether this resource requires all access tokens to be DPoP-bound.
             *
             * @remarks If omitted, clients SHOULD assume this is `false`.
             * @see {@link https://www.rfc-editor.org/rfc/rfc9728.html#section-2-2.27 | RFC 9728 §2.2.27}
             */
            dpopBoundAccessTokensRequired?: boolean;
            /**
             * Supported algorithms for verifying DPoP proofs (RFC 9449).
             *
             * @see {@link https://www.rfc-editor.org/rfc/rfc9728.html#section-2-2.25 | RFC 9728 §2.2.25}
             */
            dpopSigningAlgValuesSupported?: string[];
            /**
             * JWKS URI of this resource. Used to validate access tokens or sign responses.
             *
             * @remarks When present, this MUST be an `https:` URI pointing to a valid JWK Set (RFC 7517).
             * @see {@link https://www.rfc-editor.org/rfc/rfc9728.html#section-2-2.5 | RFC 9728 §2.2.5}
             */
            jwksUri?: string;
            /**
             * Canonical OAuth resource identifier for this protected resource (the MCP server).
             *
             * @remarks Typically the base URL of the MCP server. Clients MUST use this as the
             * `resource` parameter in authorization and token requests (per RFC 8707).
             * @see {@link https://www.rfc-editor.org/rfc/rfc9728.html#section-2-2.1 | RFC 9728 §2.2.1}
             */
            resource: string;
            /**
             * URL to developer-accessible documentation for this resource.
             *
             * @remarks This field MAY be localized.
             * @see {@link https://www.rfc-editor.org/rfc/rfc9728.html#section-2-2.15 | RFC 9728 §2.2.15}
             */
            resourceDocumentation?: string;
            /**
             * Human-readable name for display purposes (e.g., in UIs).
             *
             * @remarks This field MAY be localized using language tags (`resource_name#en`, etc.).
             * @see {@link https://www.rfc-editor.org/rfc/rfc9728.html#section-2-2.13 | RFC 9728 §2.2.13}
             */
            resourceName?: string;
            /**
             * URL to a human-readable policy page describing acceptable use.
             *
             * @remarks This field MAY be localized.
             * @see {@link https://www.rfc-editor.org/rfc/rfc9728.html#section-2-2.17 | RFC 9728 §2.2.17}
             */
            resourcePolicyUri?: string;
            /**
             * Supported JWS algorithms for signed responses from this resource (e.g., response signing).
             *
             * @remarks MUST NOT include `none`.
             * @see {@link https://www.rfc-editor.org/rfc/rfc9728.html#section-2-2.11 | RFC 9728 §2.2.11}
             */
            resourceSigningAlgValuesSupported?: string[];
            /**
             * URL to the protected resource’s Terms of Service.
             *
             * @remarks This field MAY be localized.
             * @see {@link https://www.rfc-editor.org/rfc/rfc9728.html#section-2-2.19 | RFC 9728 §2.2.19}
             */
            resourceTosUri?: string;
            /**
             * Supported OAuth scopes for requesting access to this resource.
             *
             * @remarks Useful for discovery, but clients SHOULD still request the minimal scope required.
             * @see {@link https://www.rfc-editor.org/rfc/rfc9728.html#section-2-2.7 | RFC 9728 §2.2.7}
             */
            scopesSupported?: string[];
            /**
             * Developer-accessible documentation for how to use the service (not end-user docs).
             *
             * @remarks Semantically equivalent to `resourceDocumentation`, but included under its
             * alternate name for compatibility with tools or schemas expecting either.
             * @see {@link https://www.rfc-editor.org/rfc/rfc9728.html#section-2-2.15 | RFC 9728 §2.2.15}
             */
            serviceDocumentation?: string;
            /**
             * Whether mutual-TLS-bound access tokens are required.
             *
             * @remarks If omitted, clients SHOULD assume this is `false` (client-side behavior).
             * @see {@link https://www.rfc-editor.org/rfc/rfc9728.html#section-2-2.21 | RFC 9728 §2.2.21}
             */
            tlsClientCertificateBoundAccessTokens?: boolean;
        };
        /**
         * OAuth Proxy instance for automatic OAuth flow handling.
         * When provided, FastMCP will automatically register OAuth endpoints:
         * - /oauth/register (DCR)
         * - /oauth/authorize
         * - /oauth/token
         * - /oauth/callback
         * - /oauth/consent
         */
        proxy?: OAuthProxy;
    };
    ping?: {
        /**
         * Whether ping should be enabled by default.
         * - true for SSE or HTTP Stream
         * - false for stdio
         */
        enabled?: boolean;
        /**
         * Interval
         * @default 5000 (5s)
         */
        intervalMs?: number;
        /**
         * Logging level for ping-related messages.
         * @default 'debug'
         */
        logLevel?: LoggingLevel;
    };
    /**
     * Configuration for roots capability
     */
    roots?: {
        /**
         * Whether roots capability should be enabled
         * Set to false to completely disable roots support
         * @default true
         */
        enabled?: boolean;
    };
    /**
     * General utilities
     */
    utils?: {
        formatInvalidParamsErrorMessage?: (issues: readonly StandardSchemaV1.Issue[]) => string;
    };
    version: `${number}.${number}.${number}`;
};
type Tool<T extends FastMCPSessionAuth, Params extends ToolParameters = ToolParameters, OutputParams extends ToolParameters = ToolParameters> = {
    /**
     * MCP ext-apps metadata for linking interactive UI components.
     * This field is passed through to the tool listing response.
     * @see https://modelcontextprotocol.github.io/ext-apps/
     */
    _meta?: {
        /** Additional metadata fields */
        [key: string]: unknown;
        /** UI component configuration */
        ui?: {
            /** URI of the resource serving the UI (e.g., "ui://my-tool/app.html") */
            resourceUri?: string;
        };
    };
    annotations?: {
        /**
         * When true, the tool leverages incremental content streaming
         * Return void for tools that handle all their output via streaming
         */
        streamingHint?: boolean;
    } & ToolAnnotations;
    canAccess?: (auth: T) => boolean;
    description?: string;
    execute: (args: StandardSchemaV1.InferOutput<Params>, context: Context<T>) => Promise<AudioContent | ContentResult | ImageContent | ResourceContent | ResourceLink | string | TextContent | void>;
    name: string;
    outputSchema?: OutputParams;
    parameters?: Params;
    timeoutMs?: number;
};
/**
 * Tool annotations as defined in MCP Specification (2025-03-26)
 * These provide hints about a tool's behavior.
 */
type ToolAnnotations = {
    /**
     * If true, the tool may perform destructive updates
     * Only meaningful when readOnlyHint is false
     * @default true
     */
    destructiveHint?: boolean;
    /**
     * If true, calling the tool repeatedly with the same arguments has no additional effect
     * Only meaningful when readOnlyHint is false
     * @default false
     */
    idempotentHint?: boolean;
    /**
     * If true, the tool may interact with an "open world" of external entities
     * @default true
     */
    openWorldHint?: boolean;
    /**
     * If true, indicates the tool does not modify its environment
     * @default false
     */
    readOnlyHint?: boolean;
    /**
     * A human-readable title for the tool, useful for UI display
     */
    title?: string;
};
declare const FastMCPSessionEventEmitterBase: {
    new (): StrictEventEmitter<EventEmitter, FastMCPSessionEvents>;
};
declare enum ServerState {
    Error = "error",
    Running = "running",
    Stopped = "stopped"
}
/**
 * Enhanced request object for custom routes
 */
interface FastMCPRequest<T extends FastMCPSessionAuth = FastMCPSessionAuth> {
    auth?: T;
    body?: unknown;
    headers: http.IncomingHttpHeaders;
    json(): Promise<unknown>;
    method: string;
    params: Record<string, string>;
    query: Record<string, string | string[]>;
    text(): Promise<string>;
    url: string;
}
/**
 * Enhanced response object for custom routes
 */
interface FastMCPResponse {
    end(data?: Buffer | string): void;
    json(data: unknown): void;
    send(data: Buffer | string): void;
    setHeader(name: string, value: number | string | string[]): FastMCPResponse;
    status(code: number): FastMCPResponse;
}
/**
 * HTTP method types for custom routes
 */
type HTTPMethod = "DELETE" | "GET" | "OPTIONS" | "PATCH" | "POST" | "PUT";
/**
 * Route handler function type
 */
type RouteHandler<T extends FastMCPSessionAuth = FastMCPSessionAuth> = (req: FastMCPRequest<T>, res: FastMCPResponse) => Promise<void> | void;
/**
 * Options for configuring custom routes
 */
interface RouteOptions {
    /**
     * Whether this route should bypass authentication.
     * When true, the route handler will be called without authentication,
     * and req.auth will be undefined.
     * @default false
     */
    public?: boolean;
}
type Authenticate<T> = (request: http.IncomingMessage) => Promise<T>;
type FastMCPSessionAuth = Record<string, unknown> | undefined;
declare class FastMCPSessionEventEmitter extends FastMCPSessionEventEmitterBase {
}
declare class FastMCPSession<T extends FastMCPSessionAuth = FastMCPSessionAuth> extends FastMCPSessionEventEmitter {
    #private;
    get clientCapabilities(): ClientCapabilities | null;
    get isReady(): boolean;
    get loggingLevel(): LoggingLevel;
    get roots(): Root[];
    get server(): Server;
    get sessionId(): string | undefined;
    set sessionId(value: string | undefined);
    constructor({ auth, instructions, logger, name, ping, prompts, resources, resourcesTemplates, roots, sessionId, tools, transportType, utils, version, }: {
        auth?: T;
        instructions?: string;
        logger: Logger;
        name: string;
        ping?: ServerOptions<T>["ping"];
        prompts: Prompt<T>[];
        resources: Resource<T>[];
        resourcesTemplates: InputResourceTemplate<T>[];
        roots?: ServerOptions<T>["roots"];
        sessionId?: string;
        tools: Tool<T>[];
        transportType?: "httpStream" | "stdio";
        utils?: ServerOptions<T>["utils"];
        version: string;
    });
    close(): Promise<void>;
    connect(transport: Transport): Promise<void>;
    promptsListChanged(prompts: Prompt<T>[]): void;
    requestSampling(message: z.infer<typeof CreateMessageRequestSchema>["params"], options?: RequestOptions): Promise<SamplingResponse>;
    resourcesListChanged(resources: Resource<T>[]): void;
    resourceTemplatesListChanged(resourceTemplates: ResourceTemplate<T>[]): void;
    toolsListChanged(tools: Tool<T>[]): void;
    triggerListChangedNotification(method: string): Promise<void>;
    /**
     * Update the session's authentication context.
     * Called by mcp-proxy when a new token is validated on subsequent requests.
     */
    updateAuth(auth: T): void;
    waitForReady(): Promise<void>;
    private addPrompt;
    private addResource;
    private addResourceTemplate;
    private setupCompleteHandlers;
    private setupErrorHandling;
    private setupLoggingHandlers;
    private setupPromptHandlers;
    private setupResourceHandlers;
    private setupResourceTemplateHandlers;
    private setupRootsHandlers;
    private setupToolHandlers;
}
declare const FastMCPEventEmitterBase: {
    new (): StrictEventEmitter<EventEmitter, FastMCPEvents<FastMCPSessionAuth>>;
};
declare class FastMCPEventEmitter extends FastMCPEventEmitterBase {
}
declare class FastMCP<T extends FastMCPSessionAuth = FastMCPSessionAuth> extends FastMCPEventEmitter {
    #private;
    options: ServerOptions<T>;
    get serverState(): ServerState;
    get sessions(): FastMCPSession<T>[];
    constructor(options: ServerOptions<T>);
    /**
     * Adds a prompt to the server.
     */
    addPrompt<const Args extends InputPromptArgument<T>[]>(prompt: InputPrompt<T, Args>): void;
    /**
     * Adds prompts to the server.
     */
    addPrompts<const Args extends InputPromptArgument<T>[]>(prompts: InputPrompt<T, Args>[]): void;
    /**
     * Adds a resource to the server.
     */
    addResource(resource: Resource<T>): void;
    /**
     * Adds resources to the server.
     */
    addResources(resources: Resource<T>[]): void;
    /**
     * Adds a resource template to the server.
     */
    addResourceTemplate<const Args extends InputResourceTemplateArgument[]>(resource: InputResourceTemplate<T, Args>): void;
    /**
     * Adds resource templates to the server.
     */
    addResourceTemplates<const Args extends InputResourceTemplateArgument[]>(resources: InputResourceTemplate<T, Args>[]): void;
    /**
     * Adds a tool to the server.
     */
    addTool<Params extends ToolParameters>(tool: Tool<T, Params>): void;
    /**
     * Adds tools to the server.
     */
    addTools<Params extends ToolParameters>(tools: Tool<T, Params>[]): void;
    /**
     * Embeds a resource by URI, making it easy to include resources in tool responses.
     *
     * @param uri - The URI of the resource to embed
     * @returns Promise<ResourceContent> - The embedded resource content
     */
    embedded(uri: string): Promise<ResourceContent["resource"]>;
    /**
     * Returns the underlying Hono app instance for direct access to Hono's native API.
     * This allows you to add custom routes, middleware, and handlers using Hono's standard methods.
     *
     * @returns The Hono app instance
     *
     * @example
     * ```typescript
     * const app = server.getApp();
     *
     * // Add routes using native Hono API
     * app.get('/api/users', async (c) => {
     *   return c.json({ users: [] });
     * });
     *
     * app.post('/api/users/:id', async (c) => {
     *   const id = c.req.param('id');
     *   return c.json({ id });
     * });
     * ```
     */
    getApp(): Hono;
    /**
     * Removes a prompt from the server.
     */
    removePrompt(name: string): void;
    /**
     * Removes prompts from the server.
     */
    removePrompts(names: string[]): void;
    /**
     * Removes a resource from the server.
     */
    removeResource(name: string): void;
    /**
     * Removes resources from the server.
     */
    removeResources(names: string[]): void;
    /**
     * Removes a resource template from the server.
     */
    removeResourceTemplate(name: string): void;
    /**
     * Removes resource templates from the server.
     */
    removeResourceTemplates(names: string[]): void;
    /**
     * Removes a tool from the server.
     */
    removeTool(name: string): void;
    /**
     * Removes tools from the server.
     */
    removeTools(names: string[]): void;
    /**
     * Starts the server.
     */
    start(options?: Partial<{
        httpStream: {
            enableJsonResponse?: boolean;
            endpoint?: `/${string}`;
            eventStore?: EventStore;
            host?: string;
            port: number;
            sslCa?: string;
            sslCert?: string;
            sslKey?: string;
            stateless?: boolean;
        };
        transportType: "httpStream" | "stdio";
    }>): Promise<void>;
    /**
     * Stops the server.
     */
    stop(): Promise<void>;
}

export { type AudioContent, AuthProvider, type Content, type ContentResult, type Context, DiscoveryDocumentCache, FastMCP, type FastMCPEvents, type FastMCPRequest, type FastMCPResponse, FastMCPSession, type FastMCPSessionAuth, type FastMCPSessionEvents, type HTTPMethod, type ImageContent, type InputPrompt, type InputPromptArgument, type Logger, type LoggingLevel, OAuthSession, type Progress, type Prompt, type PromptArgument, type Resource, type ResourceContent, type ResourceResult, type ResourceTemplate, type ResourceTemplateArgument, type RouteHandler, type RouteOptions, type SSEServer, type SerializableValue, type ServerOptions, ServerState, type TextContent, type Tool, type ToolParameters, UnexpectedStateError, UserError, audioContent, imageContent };
