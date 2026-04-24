/**
 * Authentication Middleware for AgentDB
 *
 * Provides comprehensive request authentication:
 * - JWT token validation (Bearer tokens)
 * - API key validation
 * - Role-based access control (RBAC)
 * - Permission checking
 * - Request context enrichment
 *
 * Security Features:
 * - Multi-method authentication (JWT + API keys)
 * - Automatic token expiration handling
 * - Secure header parsing
 * - Audit logging integration
 * - Development mode bypass (configurable)
 */
import { Request, Response, NextFunction } from 'express';
/**
 * Extended Express Request with auth context
 */
export interface AuthenticatedRequest extends Request {
    user?: {
        userId: string;
        email?: string;
        role?: string;
        permissions?: string[];
        sessionId?: string;
    };
    apiKey?: {
        keyId: string;
        userId: string;
        environment: 'live' | 'test';
    };
    authMethod?: 'jwt' | 'api_key' | 'none';
}
/**
 * Authentication options
 */
export interface AuthOptions {
    required?: boolean;
    allowApiKey?: boolean;
    allowJWT?: boolean;
    requiredRole?: string | string[];
    requiredPermissions?: string[];
    allowDevelopmentBypass?: boolean;
}
/**
 * Main authentication middleware
 */
export declare function authenticate(options?: AuthOptions): (req: AuthenticatedRequest, res: Response, next: NextFunction) => Promise<void>;
/**
 * Require JWT authentication only
 */
export declare function requireJWT(options?: Omit<AuthOptions, 'allowApiKey'>): (req: AuthenticatedRequest, res: Response, next: NextFunction) => Promise<void>;
/**
 * Require API key authentication only
 */
export declare function requireApiKey(options?: Omit<AuthOptions, 'allowJWT'>): (req: AuthenticatedRequest, res: Response, next: NextFunction) => Promise<void>;
/**
 * Optional authentication (allows unauthenticated requests)
 */
export declare function optionalAuth(options?: Omit<AuthOptions, 'required'>): (req: AuthenticatedRequest, res: Response, next: NextFunction) => Promise<void>;
/**
 * Require specific role
 */
export declare function requireRole(role: string | string[], options?: AuthOptions): (req: AuthenticatedRequest, res: Response, next: NextFunction) => Promise<void>;
/**
 * Require specific permissions
 */
export declare function requirePermissions(permissions: string[], options?: AuthOptions): (req: AuthenticatedRequest, res: Response, next: NextFunction) => Promise<void>;
/**
 * Require admin role
 */
export declare function requireAdmin(options?: AuthOptions): (req: AuthenticatedRequest, res: Response, next: NextFunction) => Promise<void>;
/**
 * Check if user has permission
 */
export declare function hasPermission(req: AuthenticatedRequest, permission: string): boolean;
/**
 * Check if user has role
 */
export declare function hasRole(req: AuthenticatedRequest, role: string): boolean;
/**
 * Get current user ID
 */
export declare function getCurrentUserId(req: AuthenticatedRequest): string | undefined;
/**
 * Get current user
 */
export declare function getCurrentUser(req: AuthenticatedRequest): AuthenticatedRequest['user'];
//# sourceMappingURL=auth.middleware.d.ts.map