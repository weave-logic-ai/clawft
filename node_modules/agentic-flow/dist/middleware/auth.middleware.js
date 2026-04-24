/**
 * Authentication Middleware
 *
 * Provides authentication for RuVector integration endpoints:
 * - API key validation
 * - JWT token verification (HMAC-SHA256)
 * - Role-based access control (RBAC)
 * - Audit logging
 */
import { createHmac, timingSafeEqual } from 'crypto';
/**
 * Authentication Error
 */
export class AuthenticationError extends Error {
    code;
    constructor(message, code = 'MISSING_AUTH') {
        super(message);
        this.code = code;
        this.name = 'AuthenticationError';
    }
}
/**
 * Authentication Middleware
 *
 * Validates requests using:
 * - API key (X-API-Key header)
 * - JWT token (Authorization: Bearer <token>)
 * - Role-based permissions
 */
export class AuthMiddleware {
    config;
    constructor(config) {
        this.config = {
            enableApiKey: config?.enableApiKey ?? true,
            enableJWT: config?.enableJWT ?? false,
            apiKeys: config?.apiKeys ?? new Map(),
            jwtSecret: config?.jwtSecret ?? (process.env.JWT_SECRET || ''),
            requiredRoles: config?.requiredRoles ?? [],
        };
    }
    /**
     * Authenticate request
     *
     * @param req - Express request
     * @returns Authenticated user
     * @throws AuthenticationError if authentication fails
     */
    async authenticate(req) {
        // Try API key authentication
        if (this.config.enableApiKey) {
            const apiKey = req.headers['x-api-key'];
            if (apiKey) {
                return this.authenticateApiKey(apiKey);
            }
        }
        // Try JWT authentication
        if (this.config.enableJWT) {
            const authHeader = req.headers['authorization'];
            if (authHeader?.startsWith('Bearer ')) {
                const token = authHeader.substring(7);
                return this.authenticateJWT(token);
            }
        }
        throw new AuthenticationError('No authentication provided', 'MISSING_AUTH');
    }
    /**
     * Authenticate using API key
     *
     * @param apiKey - API key from header
     * @returns Authenticated user
     * @throws AuthenticationError if invalid
     */
    authenticateApiKey(apiKey) {
        const userInfo = this.config.apiKeys.get(apiKey);
        if (!userInfo) {
            throw new AuthenticationError('Invalid API key', 'INVALID_KEY');
        }
        return {
            id: userInfo.userId,
            tier: userInfo.tier,
            roles: ['user'],
            apiKey: apiKey,
        };
    }
    /**
     * Authenticate using JWT with HMAC-SHA256 signature verification
     *
     * @param token - JWT token
     * @returns Authenticated user
     * @throws AuthenticationError if invalid or signature mismatch
     */
    authenticateJWT(token) {
        try {
            const parts = token.split('.');
            if (parts.length !== 3) {
                throw new Error('Invalid token format');
            }
            const [header, payload, signature] = parts;
            // CRITICAL: Verify JWT secret is configured
            if (!this.config.jwtSecret || this.config.jwtSecret.length < 32) {
                throw new AuthenticationError('JWT authentication not properly configured', 'INVALID_KEY');
            }
            // CRITICAL: Verify signature using HMAC-SHA256
            const signatureInput = `${header}.${payload}`;
            const expectedSignature = this.base64UrlEncode(createHmac('sha256', this.config.jwtSecret)
                .update(signatureInput)
                .digest());
            // Use timing-safe comparison to prevent timing attacks
            const providedSigBuffer = Buffer.from(signature);
            const expectedSigBuffer = Buffer.from(expectedSignature);
            if (providedSigBuffer.length !== expectedSigBuffer.length ||
                !timingSafeEqual(providedSigBuffer, expectedSigBuffer)) {
                throw new AuthenticationError('Invalid token signature', 'INVALID_KEY');
            }
            // Decode and parse payload after signature verification
            const decodedPayload = JSON.parse(Buffer.from(this.base64UrlDecode(payload), 'base64').toString('utf-8'));
            // Check expiration
            if (decodedPayload.exp && decodedPayload.exp < Date.now() / 1000) {
                throw new AuthenticationError('Token expired', 'EXPIRED_TOKEN');
            }
            // Check not-before time
            if (decodedPayload.nbf && decodedPayload.nbf > Date.now() / 1000) {
                throw new AuthenticationError('Token not yet valid', 'INVALID_KEY');
            }
            return {
                id: decodedPayload.sub || decodedPayload.userId,
                tier: decodedPayload.tier || 'free',
                roles: decodedPayload.roles || ['user'],
            };
        }
        catch (error) {
            if (error instanceof AuthenticationError) {
                throw error;
            }
            throw new AuthenticationError('Invalid token', 'INVALID_KEY');
        }
    }
    /**
     * Convert base64url to standard base64
     */
    base64UrlDecode(input) {
        return input.replace(/-/g, '+').replace(/_/g, '/');
    }
    /**
     * Convert buffer to base64url encoding
     */
    base64UrlEncode(buffer) {
        return buffer
            .toString('base64')
            .replace(/\+/g, '-')
            .replace(/\//g, '_')
            .replace(/=+$/, '');
    }
    /**
     * Check if user has required roles
     *
     * @param user - Authenticated user
     * @param requiredRoles - Required roles
     * @returns True if user has all required roles
     */
    hasRequiredRoles(user, requiredRoles) {
        if (requiredRoles.length === 0) {
            return true;
        }
        return requiredRoles.every(role => user.roles.includes(role));
    }
    /**
     * Create Express middleware
     *
     * @param requiredRoles - Optional required roles
     * @returns Express middleware function
     */
    createMiddleware(requiredRoles) {
        return async (req, res, next) => {
            try {
                const user = await this.authenticate(req);
                // Check roles if specified
                const roles = requiredRoles || this.config.requiredRoles;
                if (roles.length > 0 && !this.hasRequiredRoles(user, roles)) {
                    throw new AuthenticationError('Insufficient permissions', 'INSUFFICIENT_PERMISSIONS');
                }
                // Attach user to request
                req.user = user;
                next();
            }
            catch (error) {
                if (error instanceof AuthenticationError) {
                    const statusCode = error.code === 'INSUFFICIENT_PERMISSIONS' ? 403 : 401;
                    res.status(statusCode).json({
                        error: error.name,
                        message: error.message,
                        code: error.code,
                    });
                }
                else {
                    res.status(500).json({
                        error: 'Internal Server Error',
                        message: 'Authentication failed',
                    });
                }
            }
        };
    }
    /**
     * Register API key
     *
     * @param apiKey - API key
     * @param userId - User ID
     * @param tier - User tier
     */
    registerApiKey(apiKey, userId, tier = 'free') {
        this.config.apiKeys.set(apiKey, { userId, tier });
    }
    /**
     * Revoke API key
     *
     * @param apiKey - API key to revoke
     */
    revokeApiKey(apiKey) {
        this.config.apiKeys.delete(apiKey);
    }
}
/**
 * Create authentication middleware (factory function)
 *
 * @param config - Authentication configuration
 * @returns Middleware function
 */
export function createAuthMiddleware(config) {
    const auth = new AuthMiddleware(config);
    return auth.createMiddleware();
}
/**
 * Require API key (simple helper)
 */
export function requireApiKey(req, res, next) {
    const apiKey = req.headers['x-api-key'];
    if (!apiKey) {
        res.status(401).json({
            error: 'Unauthorized',
            message: 'API key required',
        });
        return;
    }
    // In production, validate against database
    if (!apiKey.startsWith('ak_')) {
        res.status(401).json({
            error: 'Unauthorized',
            message: 'Invalid API key format',
        });
        return;
    }
    req.user = {
        id: 'api-user',
        tier: 'free',
        roles: ['user'],
    };
    next();
}
//# sourceMappingURL=auth.middleware.js.map