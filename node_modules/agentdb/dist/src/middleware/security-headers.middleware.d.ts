/**
 * Security Headers Middleware for AgentDB
 *
 * Implements comprehensive security headers using Helmet.js:
 * - Content Security Policy (CSP)
 * - HTTP Strict Transport Security (HSTS)
 * - X-Frame-Options
 * - X-Content-Type-Options
 * - X-XSS-Protection
 * - Referrer-Policy
 * - Permissions-Policy
 *
 * Security Features:
 * - Prevents XSS attacks
 * - Prevents clickjacking
 * - Enforces HTTPS
 * - Prevents MIME sniffing
 * - Controls browser features
 */
import { Request, Response, NextFunction } from 'express';
/**
 * Apply all security headers
 */
export declare const securityHeaders: (req: import("http").IncomingMessage, res: import("http").ServerResponse, next: (err?: unknown) => void) => void;
/**
 * CORS configuration middleware
 */
export declare function configureCORS(options?: {
    allowedOrigins?: string[];
    allowCredentials?: boolean;
    maxAge?: number;
}): (req: Request, res: Response, next: NextFunction) => void;
/**
 * Add custom security headers
 */
export declare function customSecurityHeaders(req: Request, res: Response, next: NextFunction): void;
/**
 * Remove sensitive headers
 */
export declare function removeSensitiveHeaders(req: Request, res: Response, next: NextFunction): void;
/**
 * HSTS preload header (for production)
 */
export declare function hstsPreload(req: Request, res: Response, next: NextFunction): void;
/**
 * Security headers for API responses
 */
export declare function apiSecurityHeaders(req: Request, res: Response, next: NextFunction): void;
/**
 * Combined security middleware
 */
export declare function applySecurity(options?: {
    cors?: boolean;
    corsOrigins?: string[];
    hsts?: boolean;
    csp?: boolean;
}): (req: Request, res: Response, next: NextFunction) => void;
/**
 * Development-only CORS (allow all)
 */
export declare function developmentCORS(req: Request, res: Response, next: NextFunction): void;
//# sourceMappingURL=security-headers.middleware.d.ts.map