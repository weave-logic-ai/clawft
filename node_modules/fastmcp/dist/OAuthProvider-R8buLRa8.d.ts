import { IncomingMessage } from 'node:http';

/**
 * OAuth Proxy Types
 * Type definitions for the OAuth 2.1 Proxy implementation
 */
/**
 * Default TTL values for token expiration (in seconds)
 */
declare const DEFAULT_ACCESS_TOKEN_TTL = 3600;
declare const DEFAULT_ACCESS_TOKEN_TTL_NO_REFRESH = 31536000;
declare const DEFAULT_REFRESH_TOKEN_TTL = 2592000;
declare const DEFAULT_AUTHORIZATION_CODE_TTL = 300;
declare const DEFAULT_TRANSACTION_TTL = 600;
/**
 * OAuth authorization request parameters
 */
interface AuthorizationParams {
    [key: string]: unknown;
    client_id: string;
    code_challenge?: string;
    code_challenge_method?: string;
    redirect_uri: string;
    response_type: string;
    scope?: string;
    state?: string;
}
/**
 * Authorization code storage with PKCE validation
 */
interface ClientCode {
    /** Client ID that owns this code */
    clientId: string;
    /** Authorization code */
    code: string;
    /** PKCE code challenge for validation */
    codeChallenge: string;
    /** PKCE code challenge method */
    codeChallengeMethod: string;
    /** Code creation timestamp */
    createdAt: Date;
    /** Code expiration timestamp */
    expiresAt: Date;
    /** Associated transaction ID */
    transactionId: string;
    /** Upstream tokens obtained from provider */
    upstreamTokens: UpstreamTokenSet;
    /** Whether code has been used */
    used?: boolean;
}
/**
 * Consent data for user approval
 */
interface ConsentData {
    clientName: string;
    provider: string;
    scope: string[];
    timestamp: number;
    transactionId: string;
}
/**
 * Custom claims passthrough configuration
 */
interface CustomClaimsPassthroughConfig {
    /** Allow nested objects/arrays in claims. Default: false (only primitives) */
    allowComplexClaims?: boolean;
    /** Only passthrough these specific claims (allowlist). Default: undefined (allow all non-protected) */
    allowedClaims?: string[];
    /** Never passthrough these claims (blocklist, in addition to protected claims). Default: [] */
    blockedClaims?: string[];
    /** Prefix upstream claims to prevent collisions. Default: false (no prefix) */
    claimPrefix?: false | string;
    /** Enable passthrough from upstream access token (if JWT format). Default: true */
    fromAccessToken?: boolean;
    /** Enable passthrough from upstream ID token. Default: true */
    fromIdToken?: boolean;
    /** Maximum length for claim values. Default: 2000 */
    maxClaimValueSize?: number;
}
/**
 * Client metadata for storage
 */
interface DCRClientMetadata {
    client_name?: string;
    client_uri?: string;
    contacts?: string[];
    jwks?: Record<string, unknown>;
    jwks_uri?: string;
    logo_uri?: string;
    policy_uri?: string;
    scope?: string;
    software_id?: string;
    software_version?: string;
    tos_uri?: string;
}
/**
 * RFC 7591 Dynamic Client Registration Request
 */
interface DCRRequest {
    /** Client name */
    client_name?: string;
    /** Client homepage URL */
    client_uri?: string;
    /** Contact email addresses */
    contacts?: string[];
    /** Allowed grant types */
    grant_types?: string[];
    /** JWKS object */
    jwks?: Record<string, unknown>;
    /** JWKS URI */
    jwks_uri?: string;
    /** Client logo URL */
    logo_uri?: string;
    /** Privacy policy URL */
    policy_uri?: string;
    /** REQUIRED: Array of redirect URIs */
    redirect_uris: string[];
    /** Allowed response types */
    response_types?: string[];
    /** Requested scope */
    scope?: string;
    /** Software identifier */
    software_id?: string;
    /** Software version */
    software_version?: string;
    /** Token endpoint authentication method */
    token_endpoint_auth_method?: string;
    /** Terms of service URL */
    tos_uri?: string;
}
/**
 * RFC 7591 Dynamic Client Registration Response
 */
interface DCRResponse {
    /** REQUIRED: Client identifier */
    client_id: string;
    /** Client ID issued timestamp */
    client_id_issued_at?: number;
    client_name?: string;
    /** Client secret */
    client_secret?: string;
    /** Client secret expiration (0 = never) */
    client_secret_expires_at?: number;
    client_uri?: string;
    contacts?: string[];
    grant_types?: string[];
    jwks?: Record<string, unknown>;
    jwks_uri?: string;
    logo_uri?: string;
    policy_uri?: string;
    /** Echo back all registered metadata */
    redirect_uris: string[];
    /** Registration access token */
    registration_access_token?: string;
    /** Registration client URI */
    registration_client_uri?: string;
    response_types?: string[];
    scope?: string;
    software_id?: string;
    software_version?: string;
    token_endpoint_auth_method?: string;
    tos_uri?: string;
}
/**
 * OAuth error response
 */
interface OAuthError {
    error: string;
    error_description?: string;
    error_uri?: string;
}
/**
 * OAuth Proxy provider for pre-configured providers
 */
interface OAuthProviderConfig {
    baseUrl: string;
    clientId: string;
    clientSecret: string;
    consentRequired?: boolean;
    scopes?: string[];
}
/**
 * Configuration for the OAuth Proxy
 */
interface OAuthProxyConfig {
    /** Access token TTL in seconds (default: 3600) */
    accessTokenTtl?: number;
    /** Allowed redirect URI patterns for client registration */
    allowedRedirectUriPatterns?: string[];
    /** Authorization code TTL in seconds (default: 300) */
    authorizationCodeTtl?: number;
    /** Base URL of this proxy server */
    baseUrl: string;
    /** Require user consent (default: true) */
    consentRequired?: boolean;
    /** Secret key for signing consent cookies */
    consentSigningKey?: string;
    /**
     * Custom claims passthrough configuration.
     * When enabled (default), extracts custom claims from upstream access token and ID token
     * and includes them in the proxy's issued JWT tokens.
     * This enables authorization based on upstream roles, permissions, etc.
     * Set to false to disable claims passthrough entirely.
     * Default: true (enabled with default settings)
     */
    customClaimsPassthrough?: boolean | CustomClaimsPassthroughConfig;
    /** Enable token swap pattern (default: true) - issues short-lived JWTs instead of passing through upstream tokens */
    enableTokenSwap?: boolean;
    /** Encryption key for token storage (default: auto-generated). Set to false to disable encryption. */
    encryptionKey?: false | string;
    /** Forward client's PKCE to upstream (default: false) */
    forwardPkce?: boolean;
    /** Secret key for signing JWTs when token swap is enabled */
    jwtSigningKey?: string;
    /** OAuth callback path (default: /oauth/callback) */
    redirectPath?: string;
    /** Refresh token TTL in seconds (default: 2592000) */
    refreshTokenTtl?: number;
    /** Scopes to request from upstream provider */
    scopes?: string[];
    /** Custom token storage backend */
    tokenStorage?: TokenStorage;
    /** Custom token verifier for validating upstream tokens */
    tokenVerifier?: TokenVerifier;
    /** Transaction TTL in seconds (default: 600) */
    transactionTtl?: number;
    /** Upstream provider's authorization endpoint URL */
    upstreamAuthorizationEndpoint: string;
    /** Pre-registered client ID with upstream provider */
    upstreamClientId: string;
    /** Pre-registered client secret with upstream provider */
    upstreamClientSecret: string;
    /** Upstream provider's token endpoint URL */
    upstreamTokenEndpoint: string;
    /** Upstream token endpoint authentication method (default: "client_secret_basic") */
    upstreamTokenEndpointAuthMethod?: "client_secret_basic" | "client_secret_post";
}
/**
 * OAuth transaction tracking active authorization flows
 */
interface OAuthTransaction {
    /** Client's callback URL */
    clientCallbackUrl: string;
    /** Client's PKCE code challenge */
    clientCodeChallenge: string;
    /** Client's PKCE code challenge method (S256 or plain) */
    clientCodeChallengeMethod: string;
    /** Client ID from registration */
    clientId: string;
    /** Whether user consent was given */
    consentGiven?: boolean;
    /** Transaction creation timestamp */
    createdAt: Date;
    /** Transaction expiration timestamp */
    expiresAt: Date;
    /** Unique transaction ID */
    id: string;
    /** Additional state data */
    metadata?: Record<string, unknown>;
    /** Proxy-generated PKCE challenge for upstream */
    proxyCodeChallenge: string;
    /** Proxy-generated PKCE verifier for upstream */
    proxyCodeVerifier: string;
    /** Requested scopes */
    scope: string[];
    /** OAuth state parameter */
    state: string;
}
/**
 * PKCE pair
 */
interface PKCEPair {
    challenge: string;
    verifier: string;
}
/**
 * Dynamic client registration data
 */
interface ProxyDCRClient {
    /** Registered callback URL */
    callbackUrl: string;
    /** Generated or assigned client ID */
    clientId: string;
    /** Client secret (optional) */
    clientSecret?: string;
    /** Client metadata from registration request */
    metadata?: DCRClientMetadata;
    /** Client registration timestamp */
    registeredAt: Date;
}
/**
 * OAuth refresh token request
 */
interface RefreshRequest {
    client_id: string;
    client_secret?: string;
    grant_type: "refresh_token";
    refresh_token: string;
    scope?: string;
}
/**
 * Token mapping for JWT swap pattern
 * Maps JTI to upstream token reference
 */
interface TokenMapping {
    /** Client ID */
    clientId: string;
    /** Creation timestamp */
    createdAt: Date;
    /** Expiration timestamp */
    expiresAt: Date;
    /** JTI from FastMCP JWT */
    jti: string;
    /** Scopes */
    scope: string[];
    /** Reference to upstream token set */
    upstreamTokenKey: string;
}
/**
 * OAuth token request
 */
interface TokenRequest {
    client_id: string;
    client_secret?: string;
    code: string;
    code_verifier?: string;
    grant_type: "authorization_code";
    redirect_uri: string;
}
/**
 * OAuth token response
 */
interface TokenResponse {
    access_token: string;
    expires_in: number;
    id_token?: string;
    refresh_token?: string;
    scope?: string;
    token_type: string;
}
/**
 * Token storage interface
 */
interface TokenStorage {
    /** Clean up expired entries */
    cleanup(): Promise<void>;
    /** Delete a value */
    delete(key: string): Promise<void>;
    /** Retrieve a value */
    get(key: string): Promise<null | unknown>;
    /** Save a value with optional TTL */
    save(key: string, value: unknown, ttl?: number): Promise<void>;
}
/**
 * Token verification result
 */
interface TokenVerificationResult {
    claims?: Record<string, unknown>;
    error?: string;
    valid: boolean;
}
/**
 * Token verifier for validating upstream tokens
 */
interface TokenVerifier {
    verify(token: string): Promise<TokenVerificationResult>;
}
/**
 * Token set from upstream OAuth provider
 */
interface UpstreamTokenSet {
    /** Access token */
    accessToken: string;
    /** Token expiration in seconds */
    expiresIn: number;
    /** ID token (for OIDC) */
    idToken?: string;
    /** Token issuance timestamp */
    issuedAt: Date;
    /** Refresh token expiration in seconds (if provided by upstream) */
    refreshExpiresIn?: number;
    /** Refresh token (if provided) */
    refreshToken?: string;
    /** Granted scopes */
    scope: string[];
    /** Token type (usually "Bearer") */
    tokenType: string;
}

/**
 * OAuth 2.1 Proxy Implementation
 * Provides DCR-compatible interface for non-DCR OAuth providers
 */

/**
 * OAuth 2.1 Proxy
 * Acts as transparent intermediary between MCP clients and upstream OAuth providers
 */
declare class OAuthProxy {
    private claimsExtractor;
    private cleanupInterval;
    private clientCodes;
    private config;
    private consentManager;
    private jwtIssuer?;
    private registeredClients;
    private tokenStorage;
    private transactions;
    constructor(config: OAuthProxyConfig);
    /**
     * OAuth authorization endpoint
     */
    authorize(params: AuthorizationParams): Promise<Response>;
    /**
     * Stop cleanup interval and destroy resources
     */
    destroy(): void;
    /**
     * Token endpoint - exchange authorization code for tokens
     */
    exchangeAuthorizationCode(request: TokenRequest): Promise<TokenResponse>;
    /**
     * Token endpoint - refresh access token
     */
    exchangeRefreshToken(request: RefreshRequest): Promise<TokenResponse>;
    /**
     * Get OAuth discovery metadata
     */
    getAuthorizationServerMetadata(): {
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
     * Handle OAuth callback from upstream provider
     */
    handleCallback(request: Request): Promise<Response>;
    /**
     * Handle consent form submission
     */
    handleConsent(request: Request): Promise<Response>;
    /**
     * Load upstream tokens from a FastMCP JWT
     */
    loadUpstreamTokens(fastmcpToken: string): Promise<null | UpstreamTokenSet>;
    /**
     * RFC 7591 Dynamic Client Registration
     */
    registerClient(request: DCRRequest): Promise<DCRResponse>;
    /**
     * Calculate access token TTL from upstream tokens
     */
    private calculateAccessTokenTtl;
    /**
     * Clean up expired transactions and codes
     */
    private cleanup;
    /**
     * Create a new OAuth transaction
     */
    private createTransaction;
    /**
     * Exchange authorization code with upstream provider
     */
    private exchangeUpstreamCode;
    /**
     * Extract JTI from a JWT token
     */
    private extractJti;
    /**
     * Extract custom claims from upstream tokens
     * Combines claims from access token and ID token (if present)
     */
    private extractUpstreamClaims;
    /**
     * Generate authorization code for client
     */
    private generateAuthorizationCode;
    /**
     * Generate secure random ID
     */
    private generateId;
    /**
     * Generate signing key for consent cookies
     */
    private generateSigningKey;
    /**
     * Generate Basic auth header value for upstream token endpoint
     * Per RFC 6749 Section 2.3.1, credentials must be URL-encoded before base64 encoding
     */
    private getBasicAuthHeader;
    /**
     * Get provider name for display
     */
    private getProviderName;
    /**
     * Handle passthrough mode refresh - forward refresh token directly to upstream
     */
    private handlePassthroughRefresh;
    /**
     * Handle swap mode refresh - verify FastMCP JWT and issue new tokens
     */
    private handleSwapModeRefresh;
    /**
     * Issue swapped tokens (JWT pattern)
     * Issues short-lived FastMCP JWTs and stores upstream tokens securely
     */
    private issueSwappedTokens;
    /**
     * Issue swapped tokens for refresh flow
     */
    private issueSwappedTokensForRefresh;
    /**
     * Match URI against pattern (supports wildcards)
     */
    private matchesPattern;
    /**
     * Parse token response that can be either JSON or URL-encoded
     * GitHub Apps return URL-encoded format, most providers return JSON
     */
    private parseTokenResponse;
    /**
     * Redirect to upstream OAuth provider
     */
    private redirectToUpstream;
    /**
     * Refresh upstream tokens with provider
     */
    private refreshUpstreamTokens;
    /**
     * Start periodic cleanup of expired transactions and codes
     */
    private startCleanup;
    /**
     * Validate redirect URI against allowed patterns
     */
    private validateRedirectUri;
}
/**
 * OAuth Proxy Error
 */
declare class OAuthProxyError extends Error {
    code: string;
    description?: string | undefined;
    statusCode: number;
    constructor(code: string, description?: string | undefined, statusCode?: number);
    toJSON(): OAuthError;
    toResponse(): Response;
}

/**
 * AuthProvider Base Class
 * High-level abstraction for OAuth authentication that simplifies configuration
 */

/**
 * Configuration common to all OAuth providers.
 */
interface AuthProviderConfig {
    /** Allowed redirect URI patterns (default: ["http://localhost:*", "https://*"]) */
    allowedRedirectUriPatterns?: string[];
    /** Base URL where the MCP server is accessible */
    baseUrl: string;
    /** OAuth client ID */
    clientId: string;
    /** OAuth client secret */
    clientSecret: string;
    /** Require user consent screen (default: true) */
    consentRequired?: boolean;
    /** Encryption key for token storage (auto-generated if not provided, set to false to disable) */
    encryptionKey?: false | string;
    /** JWT signing key (auto-generated if not provided) */
    jwtSigningKey?: string;
    /** Scopes to request (defaults vary by provider) */
    scopes?: string[];
    /** Token storage backend (default: MemoryTokenStorage) */
    tokenStorage?: TokenStorage;
}
/**
 * Configuration for generic OAuth provider (user-specified endpoints).
 */
interface GenericOAuthProviderConfig extends AuthProviderConfig {
    /** OAuth authorization endpoint URL */
    authorizationEndpoint: string;
    /** OAuth token endpoint URL */
    tokenEndpoint: string;
    /** Token endpoint auth method (default: "client_secret_basic") */
    tokenEndpointAuthMethod?: "client_secret_basic" | "client_secret_post";
}
/**
 * Standard session type for OAuth providers.
 * Contains the upstream access token and optional metadata.
 */
interface OAuthSession {
    /** The upstream OAuth access token */
    accessToken: string;
    /** Additional claims extracted from the token (if customClaimsPassthrough enabled) */
    claims?: Record<string, unknown>;
    /** Token expiration time (Unix timestamp in seconds) */
    expiresAt?: number;
    /** ID token from OIDC providers */
    idToken?: string;
    /** Refresh token (if available) */
    refreshToken?: string;
    /** Scopes granted by the OAuth provider */
    scopes?: string[];
}
/**
 * Abstract base class for OAuth providers.
 * Encapsulates OAuthProxy creation, authenticate function, and oauth config.
 *
 * Subclasses only need to implement the endpoint and default scope methods.
 */
declare abstract class AuthProvider<TSession extends OAuthSession = OAuthSession> {
    protected config: AuthProviderConfig;
    /**
     * Get the proxy, creating it lazily if needed.
     */
    protected get proxy(): OAuthProxy;
    private _proxy;
    constructor(config: AuthProviderConfig);
    /**
     * Authenticate function to be used by FastMCP.
     * Extracts Bearer token, validates it, and returns session with upstream access token.
     */
    authenticate(request: IncomingMessage | undefined): Promise<TSession | undefined>;
    /**
     * Get the OAuth configuration object for FastMCP ServerOptions.
     */
    getOAuthConfig(): {
        authorizationServer: ReturnType<OAuthProxy["getAuthorizationServerMetadata"]>;
        enabled: true;
        protectedResource: {
            authorizationServers: string[];
            resource: string;
            scopesSupported: string[];
        };
        proxy: OAuthProxy;
    };
    /**
     * Get the OAuthProxy instance (for advanced use cases).
     */
    getProxy(): OAuthProxy;
    /** Create the underlying OAuthProxy with provider-specific configuration */
    protected abstract createProxy(): OAuthProxy;
    /**
     * Create a session object from upstream tokens.
     * Override in subclasses to add provider-specific session data.
     */
    protected createSession(upstreamTokens: UpstreamTokenSet): TSession;
    /** Get the authorization endpoint for this provider */
    protected abstract getAuthorizationEndpoint(): string;
    /** Default scopes for this provider */
    protected abstract getDefaultScopes(): string[];
    /** Get the token endpoint for this provider */
    protected abstract getTokenEndpoint(): string;
}

/**
 * Authentication Helper Functions
 * Utility functions for use with canAccess on tools, resources, and prompts
 */

type SessionAuth = Record<string, unknown> | undefined;
/**
 * Extract and type-cast OAuth session from canAccess context.
 * Throws if session is undefined (use with canAccess: requireAuth).
 */
declare function getAuthSession<T extends OAuthSession = OAuthSession>(session: SessionAuth): T;
/**
 * Combines multiple canAccess checks with AND logic.
 */
declare function requireAll<T extends SessionAuth>(...checks: Array<((auth: T) => boolean) | boolean>): (auth: T) => boolean;
/**
 * Combines multiple canAccess checks with OR logic.
 */
declare function requireAny<T extends SessionAuth>(...checks: Array<((auth: T) => boolean) | boolean>): (auth: T) => boolean;
/**
 * Requires any authenticated session.
 */
declare function requireAuth<T extends SessionAuth>(auth: T): boolean;
/**
 * Requires session to have a specific role (OR logic for multiple roles).
 */
declare function requireRole<T extends SessionAuth>(...allowedRoles: string[]): (auth: T) => boolean;
/**
 * Requires session to have specific scopes.
 */
declare function requireScopes<T extends SessionAuth>(...requiredScopes: string[]): (auth: T) => boolean;

/**
 * Microsoft Azure/Entra ID OAuth Provider
 * Pre-configured OAuth provider for Microsoft Identity Platform
 */

/**
 * Azure-specific configuration
 */
interface AzureProviderConfig extends AuthProviderConfig {
    /** Tenant ID or 'common', 'organizations', 'consumers' (default: 'common') */
    tenantId?: string;
}
/**
 * Azure-specific session with additional user info
 */
interface AzureSession extends OAuthSession {
    upn?: string;
}
/**
 * Microsoft Azure AD / Entra ID OAuth 2.0 Provider
 * Callback URL: {baseUrl}/oauth/callback
 */
declare class AzureProvider extends AuthProvider<AzureSession> {
    private tenantId;
    constructor(config: AzureProviderConfig);
    protected createProxy(): OAuthProxy;
    protected getAuthorizationEndpoint(): string;
    protected getDefaultScopes(): string[];
    protected getTokenEndpoint(): string;
}

/**
 * GitHub OAuth Provider
 * Pre-configured OAuth provider for GitHub OAuth Apps
 */

/**
 * GitHub-specific session with additional user info
 */
interface GitHubSession extends OAuthSession {
    username?: string;
}
/**
 * GitHub OAuth 2.0 Provider
 * Callback URL: {baseUrl}/oauth/callback
 */
declare class GitHubProvider extends AuthProvider<GitHubSession> {
    constructor(config: AuthProviderConfig);
    protected createProxy(): OAuthProxy;
    protected getAuthorizationEndpoint(): string;
    protected getDefaultScopes(): string[];
    protected getTokenEndpoint(): string;
}

/**
 * Google OAuth Provider
 * Pre-configured OAuth provider for Google Identity Platform
 */

/**
 * Google-specific session with additional user info
 */
interface GoogleSession extends OAuthSession {
    email?: string;
}
/**
 * Google OAuth 2.0 Provider
 * Callback URL: {baseUrl}/oauth/callback
 */
declare class GoogleProvider extends AuthProvider<GoogleSession> {
    constructor(config: AuthProviderConfig);
    protected createProxy(): OAuthProxy;
    protected getAuthorizationEndpoint(): string;
    protected getDefaultScopes(): string[];
    protected getTokenEndpoint(): string;
}

/**
 * Generic OAuth Provider
 * For any OAuth 2.0 compliant authorization server
 */

/**
 * Generic OAuth provider for any OAuth 2.0 compliant authorization server.
 * Use when there's no built-in provider for your identity provider.
 */
declare class OAuthProvider<TSession extends OAuthSession = OAuthSession> extends AuthProvider<TSession> {
    protected genericConfig: GenericOAuthProviderConfig;
    constructor(config: GenericOAuthProviderConfig);
    protected createProxy(): OAuthProxy;
    protected getAuthorizationEndpoint(): string;
    protected getDefaultScopes(): string[];
    protected getTokenEndpoint(): string;
}

export { AuthProvider as A, type DCRClientMetadata as B, type ConsentData as C, DEFAULT_ACCESS_TOKEN_TTL as D, type DCRRequest as E, type DCRResponse as F, GitHubProvider as G, type OAuthError as H, type OAuthProviderConfig as I, type OAuthProxyConfig as J, type ProxyDCRClient as K, type TokenMapping as L, type TokenRequest as M, type TokenResponse as N, type OAuthSession as O, type PKCEPair as P, type RefreshRequest as R, type TokenStorage as T, type UpstreamTokenSet as U, OAuthProxy as a, AzureProvider as b, GoogleProvider as c, OAuthProvider as d, requireAny as e, requireAuth as f, getAuthSession as g, requireRole as h, requireScopes as i, type AuthProviderConfig as j, type AzureProviderConfig as k, type AzureSession as l, type GenericOAuthProviderConfig as m, type GitHubSession as n, type GoogleSession as o, type OAuthTransaction as p, type TokenVerifier as q, requireAll as r, type TokenVerificationResult as s, OAuthProxyError as t, DEFAULT_ACCESS_TOKEN_TTL_NO_REFRESH as u, DEFAULT_AUTHORIZATION_CODE_TTL as v, DEFAULT_REFRESH_TOKEN_TTL as w, DEFAULT_TRANSACTION_TTL as x, type AuthorizationParams as y, type ClientCode as z };
