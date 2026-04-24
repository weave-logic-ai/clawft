import { p as OAuthTransaction, C as ConsentData, T as TokenStorage, q as TokenVerifier, s as TokenVerificationResult, P as PKCEPair } from '../OAuthProvider-R8buLRa8.cjs';
export { A as AuthProvider, j as AuthProviderConfig, y as AuthorizationParams, b as AzureProvider, k as AzureProviderConfig, l as AzureSession, z as ClientCode, B as DCRClientMetadata, E as DCRRequest, F as DCRResponse, D as DEFAULT_ACCESS_TOKEN_TTL, u as DEFAULT_ACCESS_TOKEN_TTL_NO_REFRESH, v as DEFAULT_AUTHORIZATION_CODE_TTL, w as DEFAULT_REFRESH_TOKEN_TTL, x as DEFAULT_TRANSACTION_TTL, m as GenericOAuthProviderConfig, G as GitHubProvider, n as GitHubSession, c as GoogleProvider, o as GoogleSession, H as OAuthError, d as OAuthProvider, I as OAuthProviderConfig, a as OAuthProxy, J as OAuthProxyConfig, t as OAuthProxyError, O as OAuthSession, K as ProxyDCRClient, R as RefreshRequest, L as TokenMapping, M as TokenRequest, N as TokenResponse, U as UpstreamTokenSet, g as getAuthSession, r as requireAll, e as requireAny, f as requireAuth, h as requireRole, i as requireScopes } from '../OAuthProvider-R8buLRa8.cjs';
import 'node:http';

/**
 * Consent Management
 * Handles user consent flow for OAuth authorization
 */

/**
 * Manages consent screens and cookie signing
 */
declare class ConsentManager {
    private signingKey;
    constructor(signingKey: string);
    /**
     * Create HTTP response with consent screen
     */
    createConsentResponse(transaction: OAuthTransaction, provider: string): Response;
    /**
     * Generate HTML for consent screen
     */
    generateConsentScreen(data: ConsentData): string;
    /**
     * Sign consent data for cookie
     */
    signConsentCookie(data: ConsentData): string;
    /**
     * Validate and parse consent cookie
     */
    validateConsentCookie(cookie: string): ConsentData | null;
    /**
     * Escape HTML to prevent XSS
     */
    private escapeHtml;
    /**
     * Format scope for display
     */
    private formatScope;
    /**
     * Generate default signing key if none provided
     */
    private generateDefaultKey;
    /**
     * Sign a payload using HMAC-SHA256
     */
    private sign;
}

/**
 * Disk-based Token Storage Implementation
 * Provides persistent file-based storage for OAuth tokens and transaction state
 */

interface DiskStoreOptions {
    /**
     * How often to run cleanup (in milliseconds)
     * @default 60000 (1 minute)
     */
    cleanupIntervalMs?: number;
    /**
     * Directory path for storing data
     */
    directory: string;
    /**
     * File extension for stored files
     * @default ".json"
     */
    fileExtension?: string;
}
/**
 * Disk-based token storage with TTL support
 * Persists tokens to filesystem for survival across server restarts
 */
declare class DiskStore implements TokenStorage {
    private cleanupInterval;
    private directory;
    private fileExtension;
    constructor(options: DiskStoreOptions);
    /**
     * Clean up expired entries
     */
    cleanup(): Promise<void>;
    /**
     * Delete a value
     */
    delete(key: string): Promise<void>;
    /**
     * Destroy the storage and clear cleanup interval
     */
    destroy(): void;
    /**
     * Retrieve a value
     */
    get(key: string): Promise<null | unknown>;
    /**
     * Save a value with optional TTL
     */
    save(key: string, value: unknown, ttl?: number): Promise<void>;
    /**
     * Get the number of stored items
     */
    size(): Promise<number>;
    /**
     * Ensure storage directory exists
     */
    private ensureDirectory;
    /**
     * Get file path for a key
     */
    private getFilePath;
}

/**
 * JWT Issuer for OAuth Proxy
 * Issues and validates short-lived JWTs that reference upstream provider tokens
 */
/**
 * JWT Claims for FastMCP tokens
 */
interface JWTClaims {
    /** Additional custom claims from upstream tokens */
    [key: string]: unknown;
    /** Audience */
    aud: string;
    /** Client ID */
    client_id: string;
    /** Expiration time (seconds since epoch) */
    exp: number;
    /** Issued at time (seconds since epoch) */
    iat: number;
    /** Issuer */
    iss: string;
    /** JWT ID (unique identifier) */
    jti: string;
    /** Scopes */
    scope: string[];
}
/**
 * JWT Issuer configuration
 */
interface JWTIssuerConfig {
    /** Token expiration in seconds (default: 3600 = 1 hour) */
    accessTokenTtl?: number;
    /** Audience for issued tokens */
    audience: string;
    /** Issuer identifier */
    issuer: string;
    /** Refresh token expiration in seconds (default: 2592000 = 30 days) */
    refreshTokenTtl?: number;
    /** Secret key for signing tokens */
    signingKey: string;
}
/**
 * Token validation result
 */
interface TokenValidationResult {
    /** Decoded claims if valid */
    claims?: JWTClaims;
    /** Error message if invalid */
    error?: string;
    /** Whether token is valid */
    valid: boolean;
}
/**
 * JWT Issuer
 * Issues and validates HS256-signed JWTs for the OAuth proxy
 */
declare class JWTIssuer {
    private accessTokenTtl;
    private audience;
    private issuer;
    private refreshTokenTtl;
    private signingKey;
    constructor(config: JWTIssuerConfig);
    /**
     * Derive a signing key from a secret
     * Uses PBKDF2 for key derivation
     */
    static deriveKey(secret: string, iterations?: number): Promise<string>;
    /**
     * Issue an access token
     */
    issueAccessToken(clientId: string, scope: string[], additionalClaims?: Record<string, unknown>, expiresIn?: number): string;
    /**
     * Issue a refresh token
     */
    issueRefreshToken(clientId: string, scope: string[], additionalClaims?: Record<string, unknown>, expiresIn?: number): string;
    /**
     * Validate a JWT token
     */
    verify(token: string): Promise<TokenValidationResult>;
    /**
     * Generate unique JWT ID
     */
    private generateJti;
    /**
     * Sign data with HMAC-SHA256
     */
    private sign;
    /**
     * Sign a JWT token
     */
    private signToken;
}

/**
 * JWKS (JSON Web Key Set) Verifier
 * Provides JWT verification using public keys from JWKS endpoints
 *
 * Requires the 'jose' package as an optional peer dependency.
 * Install with: npm install jose
 */

/**
 * Token verification result
 */
interface JWKSVerificationResult {
    claims?: JWTClaims;
    error?: string;
    valid: boolean;
}
/**
 * JWKS configuration options
 */
interface JWKSVerifierConfig {
    /**
     * Expected token audience
     */
    audience?: string;
    /**
     * Cache duration for JWKS keys in milliseconds
     * @default 3600000 (1 hour)
     */
    cacheDuration?: number;
    /**
     * Cooldown duration between JWKS refetches in milliseconds
     * @default 30000 (30 seconds)
     */
    cooldownDuration?: number;
    /**
     * Expected token issuer
     */
    issuer?: string;
    /**
     * JWKS endpoint URL (e.g., https://provider.com/.well-known/jwks.json)
     */
    jwksUri: string;
}
/**
 * JWKS Verifier
 * Verifies JWTs using public keys from a JWKS endpoint
 *
 * This class requires the 'jose' package to be installed:
 * ```bash
 * npm install jose
 * ```
 *
 * @example
 * ```typescript
 * const verifier = new JWKSVerifier({
 *   jwksUri: 'https://accounts.google.com/.well-known/jwks.json',
 *   audience: 'your-client-id',
 *   issuer: 'https://accounts.google.com'
 * });
 *
 * const result = await verifier.verify(token);
 * if (result.valid) {
 *   console.log('Token claims:', result.claims);
 * }
 * ```
 */
declare class JWKSVerifier implements TokenVerifier {
    private config;
    private jose;
    private joseLoaded;
    private jwksCache;
    constructor(config: JWKSVerifierConfig);
    /**
     * Get the JWKS URI being used
     */
    getJwksUri(): string;
    /**
     * Refresh the JWKS cache
     * Useful if you need to force a key refresh
     */
    refreshKeys(): Promise<void>;
    /**
     * Verify a JWT token using JWKS
     *
     * @param token - The JWT token to verify
     * @returns Verification result with claims if valid
     *
     * @example
     * ```typescript
     * const result = await verifier.verify(token);
     * if (result.valid) {
     *   console.log('User:', result.claims?.client_id);
     * } else {
     *   console.error('Invalid token:', result.error);
     * }
     * ```
     */
    verify(token: string): Promise<TokenVerificationResult>;
    /**
     * Lazy load the jose library
     * Only loads when verification is first attempted
     */
    private loadJose;
    /**
     * Parse scope from token payload
     * Handles both string (space-separated) and array formats
     */
    private parseScope;
}

/**
 * PKCE (Proof Key for Code Exchange) Utilities
 * Implements RFC 7636 for OAuth 2.0 public clients
 */

/**
 * PKCE utility class for generating and validating code challenges
 */
declare class PKCEUtils {
    /**
     * Generate a code challenge from a verifier
     * @param verifier The code verifier
     * @param method Challenge method: 'S256' or 'plain' (default: 'S256')
     * @returns Base64URL-encoded challenge string
     */
    static generateChallenge(verifier: string, method?: "plain" | "S256"): string;
    /**
     * Generate a complete PKCE pair (verifier + challenge)
     * @param method Challenge method: 'S256' or 'plain' (default: 'S256')
     * @returns Object containing verifier and challenge
     */
    static generatePair(method?: "plain" | "S256"): PKCEPair;
    /**
     * Generate a cryptographically secure code verifier
     * @param length Length of verifier (43-128 characters, default: 128)
     * @returns Base64URL-encoded verifier string
     */
    static generateVerifier(length?: number): string;
    /**
     * Validate a code verifier against a challenge
     * @param verifier The code verifier to validate
     * @param challenge The expected challenge
     * @param method The challenge method used
     * @returns True if verifier matches challenge
     */
    static validateChallenge(verifier: string, challenge: string, method: string): boolean;
    /**
     * Encode a buffer as base64url (RFC 4648)
     * @param buffer Buffer to encode
     * @returns Base64URL-encoded string
     */
    private static base64URLEncode;
}

/**
 * Token Storage Implementations
 * Secure storage for OAuth tokens and transaction state
 */

/**
 * Encrypted token storage wrapper
 * Encrypts values using AES-256-GCM before storing
 */
declare class EncryptedTokenStorage implements TokenStorage {
    private algorithm;
    private backend;
    private encryptionKey;
    constructor(backend: TokenStorage, encryptionKey: string);
    cleanup(): Promise<void>;
    delete(key: string): Promise<void>;
    get(key: string): Promise<null | unknown>;
    save(key: string, value: unknown, ttl?: number): Promise<void>;
    private decrypt;
    private encrypt;
}
/**
 * In-memory token storage with TTL support
 */
declare class MemoryTokenStorage implements TokenStorage {
    private cleanupInterval;
    private store;
    constructor(cleanupIntervalMs?: number);
    cleanup(): Promise<void>;
    delete(key: string): Promise<void>;
    /**
     * Destroy the storage and clear cleanup interval
     */
    destroy(): void;
    get(key: string): Promise<null | unknown>;
    save(key: string, value: unknown, ttl?: number): Promise<void>;
    /**
     * Get the number of stored items
     */
    size(): number;
}

export { ConsentData, ConsentManager, DiskStore, EncryptedTokenStorage, type JWKSVerificationResult, JWKSVerifier, type JWKSVerifierConfig, type JWTClaims, JWTIssuer, MemoryTokenStorage, OAuthTransaction, PKCEPair, PKCEUtils, TokenStorage, TokenVerificationResult, TokenVerifier };
