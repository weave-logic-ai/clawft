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
import { verifyAccessToken, extractTokenFromHeader } from '../services/token.service.js';
import { validateApiKey, validateSession, refreshSession } from '../services/auth.service.js';
import { AuditLogger } from '../services/audit-logger.service.js';

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
 * Default authentication options
 */
const DEFAULT_AUTH_OPTIONS: AuthOptions = {
  required: true,
  allowApiKey: true,
  allowJWT: true,
  allowDevelopmentBypass: false,
};

/**
 * Main authentication middleware
 */
export function authenticate(options: AuthOptions = {}) {
  const opts = { ...DEFAULT_AUTH_OPTIONS, ...options };
  const auditLogger = new AuditLogger();

  return async (
    req: AuthenticatedRequest,
    res: Response,
    next: NextFunction
  ): Promise<void> => {
    try {
      // Development bypass - SECURITY HARDENED
      // Requires explicit opt-in via environment variable AND development mode
      // The bypass header value must match the secret, not just be "development"
      if (opts.allowDevelopmentBypass) {
        const devBypassSecret = process.env.AGENTDB_DEV_BYPASS_SECRET;
        const isDevMode = process.env.NODE_ENV === 'development';
        const bypassHeader = req.headers['x-bypass-auth'] as string;

        // SECURITY: Require BOTH explicit secret AND development mode
        // The secret should be unique per developer, not a well-known value
        if (devBypassSecret && isDevMode && bypassHeader === devBypassSecret) {
          console.warn('[SECURITY WARNING] Authentication bypassed in development mode');
          console.warn('[SECURITY WARNING] Never use AGENTDB_DEV_BYPASS_SECRET in production!');

          // Log bypass for audit trail (use auth_error type with descriptive reason)
          await auditLogger.logAuthEvent({
            type: 'auth_error',
            ip: req.ip,
            userAgent: req.headers['user-agent'],
            reason: 'DEV_BYPASS: Development authentication bypass used',
            timestamp: new Date(),
          });

          req.authMethod = 'none';
          req.user = {
            userId: 'dev_user',
            email: 'dev@localhost',
            role: 'developer', // Reduced from 'admin' - dev users shouldn't auto-get admin
            permissions: ['read', 'write'], // Explicit permissions instead of wildcard
          };
          return next();
        }
      }

      let authenticated = false;

      // Try JWT authentication first
      if (opts.allowJWT) {
        const authHeader = req.headers.authorization;
        const token = extractTokenFromHeader(authHeader);

        if (token) {
          const verification = verifyAccessToken(token);

          if (verification.valid && verification.payload) {
            // Validate session if sessionId is present
            if (verification.payload.sessionId) {
              const sessionValid = validateSession(verification.payload.sessionId);

              if (!sessionValid) {
                await auditLogger.logAuthEvent({
                  type: 'jwt_auth_failed',
                  userId: verification.payload.userId,
                  ip: req.ip,
                  userAgent: req.headers['user-agent'],
                  reason: 'Session expired',
                  timestamp: new Date(),
                });

                res.status(401).json({
                  error: 'Session expired',
                  code: 'SESSION_EXPIRED',
                });
                return;
              }

              // Refresh session on successful request
              refreshSession(verification.payload.sessionId);
            }

            req.user = {
              userId: verification.payload.userId,
              email: verification.payload.email,
              role: verification.payload.role,
              permissions: verification.payload.permissions || [],
              sessionId: verification.payload.sessionId,
            };
            req.authMethod = 'jwt';
            authenticated = true;

            await auditLogger.logAuthEvent({
              type: 'jwt_auth_success',
              userId: verification.payload.userId,
              ip: req.ip,
              userAgent: req.headers['user-agent'],
              timestamp: new Date(),
            });
          } else {
            await auditLogger.logAuthEvent({
              type: 'jwt_auth_failed',
              ip: req.ip,
              userAgent: req.headers['user-agent'],
              reason: verification.error || 'Invalid token',
              timestamp: new Date(),
            });
          }
        }
      }

      // Try API key authentication if JWT failed
      if (!authenticated && opts.allowApiKey) {
        const apiKey = req.headers['x-api-key'] as string;

        if (apiKey) {
          const validation = validateApiKey(apiKey);

          if (validation.valid && validation.userId && validation.keyInfo) {
            req.apiKey = {
              keyId: validation.keyInfo.id,
              userId: validation.userId,
              environment: validation.keyInfo.environment,
            };
            req.user = {
              userId: validation.userId,
              role: 'api_user',
              permissions: [],
            };
            req.authMethod = 'api_key';
            authenticated = true;

            await auditLogger.logAuthEvent({
              type: 'api_key_auth_success',
              userId: validation.userId,
              ip: req.ip,
              userAgent: req.headers['user-agent'],
              metadata: {
                keyId: validation.keyInfo.id,
                environment: validation.keyInfo.environment,
              },
              timestamp: new Date(),
            });
          } else {
            await auditLogger.logAuthEvent({
              type: 'api_key_auth_failed',
              ip: req.ip,
              userAgent: req.headers['user-agent'],
              reason: validation.error || 'Invalid API key',
              timestamp: new Date(),
            });
          }
        }
      }

      // Check if authentication is required
      if (opts.required && !authenticated) {
        res.status(401).json({
          error: 'Authentication required',
          code: 'UNAUTHENTICATED',
          message: 'Valid JWT token (Bearer) or API key (X-API-Key) is required',
        });
        return;
      }

      // Check role requirements
      if (authenticated && opts.requiredRole) {
        const requiredRoles = Array.isArray(opts.requiredRole)
          ? opts.requiredRole
          : [opts.requiredRole];

        const userRole = req.user?.role;

        if (!userRole || !requiredRoles.includes(userRole)) {
          await auditLogger.logAuthEvent({
            type: 'authorization_failed',
            userId: req.user?.userId,
            ip: req.ip,
            userAgent: req.headers['user-agent'],
            reason: `Insufficient role: required ${requiredRoles.join(' or ')}, got ${userRole}`,
            timestamp: new Date(),
          });

          res.status(403).json({
            error: 'Insufficient permissions',
            code: 'FORBIDDEN',
            message: `Required role: ${requiredRoles.join(' or ')}`,
          });
          return;
        }
      }

      // Check permission requirements
      if (authenticated && opts.requiredPermissions && opts.requiredPermissions.length > 0) {
        const userPermissions = req.user?.permissions || [];

        // Check if user has all required permissions or wildcard permission
        const hasWildcard = userPermissions.includes('*');
        const hasAllPermissions = opts.requiredPermissions.every(
          perm => userPermissions.includes(perm)
        );

        if (!hasWildcard && !hasAllPermissions) {
          await auditLogger.logAuthEvent({
            type: 'authorization_failed',
            userId: req.user?.userId,
            ip: req.ip,
            userAgent: req.headers['user-agent'],
            reason: `Missing permissions: ${opts.requiredPermissions.join(', ')}`,
            timestamp: new Date(),
          });

          res.status(403).json({
            error: 'Insufficient permissions',
            code: 'FORBIDDEN',
            message: `Required permissions: ${opts.requiredPermissions.join(', ')}`,
          });
          return;
        }
      }

      next();
    } catch (error) {
      console.error('[Auth Middleware] Error:', error);

      await auditLogger.logAuthEvent({
        type: 'auth_error',
        ip: req.ip,
        userAgent: req.headers['user-agent'],
        reason: error instanceof Error ? error.message : 'Unknown error',
        timestamp: new Date(),
      });

      res.status(500).json({
        error: 'Authentication error',
        code: 'AUTH_ERROR',
      });
    }
  };
}

/**
 * Require JWT authentication only
 */
export function requireJWT(options: Omit<AuthOptions, 'allowApiKey'> = {}) {
  return authenticate({
    ...options,
    allowApiKey: false,
    allowJWT: true,
    required: true,
  });
}

/**
 * Require API key authentication only
 */
export function requireApiKey(options: Omit<AuthOptions, 'allowJWT'> = {}) {
  return authenticate({
    ...options,
    allowJWT: false,
    allowApiKey: true,
    required: true,
  });
}

/**
 * Optional authentication (allows unauthenticated requests)
 */
export function optionalAuth(options: Omit<AuthOptions, 'required'> = {}) {
  return authenticate({
    ...options,
    required: false,
  });
}

/**
 * Require specific role
 */
export function requireRole(role: string | string[], options: AuthOptions = {}) {
  return authenticate({
    ...options,
    requiredRole: role,
  });
}

/**
 * Require specific permissions
 */
export function requirePermissions(permissions: string[], options: AuthOptions = {}) {
  return authenticate({
    ...options,
    requiredPermissions: permissions,
  });
}

/**
 * Require admin role
 */
export function requireAdmin(options: AuthOptions = {}) {
  return requireRole('admin', options);
}

/**
 * Check if user has permission
 */
export function hasPermission(req: AuthenticatedRequest, permission: string): boolean {
  const permissions = req.user?.permissions || [];
  return permissions.includes('*') || permissions.includes(permission);
}

/**
 * Check if user has role
 */
export function hasRole(req: AuthenticatedRequest, role: string): boolean {
  return req.user?.role === role;
}

/**
 * Get current user ID
 */
export function getCurrentUserId(req: AuthenticatedRequest): string | undefined {
  return req.user?.userId;
}

/**
 * Get current user
 */
export function getCurrentUser(req: AuthenticatedRequest): AuthenticatedRequest['user'] {
  return req.user;
}
