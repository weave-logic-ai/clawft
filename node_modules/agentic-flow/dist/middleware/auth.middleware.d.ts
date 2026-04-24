/**
 * Authentication Middleware
 *
 * Provides authentication for RuVector integration endpoints:
 * - API key validation
 * - JWT token verification (HMAC-SHA256)
 * - Role-based access control (RBAC)
 * - Audit logging
 */
export interface AuthConfig {
    /** Enable API key authentication */
    enableApiKey?: boolean;
    /** Enable JWT authentication */
    enableJWT?: boolean;
    /** API keys (key -> user mapping) */
    apiKeys?: Map<string, {
        userId: string;
        tier: string;
    }>;
    /** JWT secret */
    jwtSecret?: string;
    /** Required roles */
    requiredRoles?: string[];
}
export interface AuthUser {
    id: string;
    tier: string;
    roles: string[];
    apiKey?: string;
}
/**
 * Authentication Error
 */
export declare class AuthenticationError extends Error {
    code: 'MISSING_AUTH' | 'INVALID_KEY' | 'EXPIRED_TOKEN' | 'INSUFFICIENT_PERMISSIONS';
    constructor(message: string, code?: 'MISSING_AUTH' | 'INVALID_KEY' | 'EXPIRED_TOKEN' | 'INSUFFICIENT_PERMISSIONS');
}
/**
 * Authentication Middleware
 *
 * Validates requests using:
 * - API key (X-API-Key header)
 * - JWT token (Authorization: Bearer <token>)
 * - Role-based permissions
 */
export declare class AuthMiddleware {
    private config;
    constructor(config?: AuthConfig);
    /**
     * Authenticate request
     *
     * @param req - Express request
     * @returns Authenticated user
     * @throws AuthenticationError if authentication fails
     */
    authenticate(req: any): Promise<AuthUser>;
    /**
     * Authenticate using API key
     *
     * @param apiKey - API key from header
     * @returns Authenticated user
     * @throws AuthenticationError if invalid
     */
    private authenticateApiKey;
    /**
     * Authenticate using JWT with HMAC-SHA256 signature verification
     *
     * @param token - JWT token
     * @returns Authenticated user
     * @throws AuthenticationError if invalid or signature mismatch
     */
    private authenticateJWT;
    /**
     * Convert base64url to standard base64
     */
    private base64UrlDecode;
    /**
     * Convert buffer to base64url encoding
     */
    private base64UrlEncode;
    /**
     * Check if user has required roles
     *
     * @param user - Authenticated user
     * @param requiredRoles - Required roles
     * @returns True if user has all required roles
     */
    hasRequiredRoles(user: AuthUser, requiredRoles: string[]): boolean;
    /**
     * Create Express middleware
     *
     * @param requiredRoles - Optional required roles
     * @returns Express middleware function
     */
    createMiddleware(requiredRoles?: string[]): (req: any, res: any, next: any) => Promise<void>;
    /**
     * Register API key
     *
     * @param apiKey - API key
     * @param userId - User ID
     * @param tier - User tier
     */
    registerApiKey(apiKey: string, userId: string, tier?: string): void;
    /**
     * Revoke API key
     *
     * @param apiKey - API key to revoke
     */
    revokeApiKey(apiKey: string): void;
}
/**
 * Create authentication middleware (factory function)
 *
 * @param config - Authentication configuration
 * @returns Middleware function
 */
export declare function createAuthMiddleware(config?: AuthConfig): (req: any, res: any, next: any) => Promise<void>;
/**
 * Require API key (simple helper)
 */
export declare function requireApiKey(req: any, res: any, next: any): void;
//# sourceMappingURL=auth.middleware.d.ts.map