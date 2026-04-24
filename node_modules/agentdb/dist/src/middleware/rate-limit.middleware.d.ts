/**
 * Rate Limiting Middleware for AgentDB
 *
 * Provides comprehensive rate limiting:
 * - Per-IP rate limiting
 * - Per-user rate limiting
 * - Per-endpoint rate limiting
 * - Sliding window algorithm
 * - Configurable limits and windows
 * - Rate limit headers (X-RateLimit-*)
 *
 * Security Features:
 * - Prevents brute force attacks
 * - Prevents DoS attacks
 * - Configurable strict mode for auth endpoints
 * - IP whitelist/blacklist support
 * - Automatic cleanup of expired records
 */
import { Request, Response, NextFunction } from 'express';
/**
 * Rate limit configuration
 */
export interface RateLimitConfig {
    windowMs?: number;
    max?: number;
    message?: string;
    statusCode?: number;
    skipSuccessfulRequests?: boolean;
    skipFailedRequests?: boolean;
    keyGenerator?: (req: Request) => string;
}
/**
 * Default rate limit configuration
 */
export declare const RATE_LIMIT_CONFIG: {
    readonly GENERAL: {
        readonly windowMs: number;
        readonly max: 100;
    };
    readonly AUTH: {
        readonly windowMs: number;
        readonly max: 5;
    };
    readonly REGISTRATION: {
        readonly windowMs: number;
        readonly max: 3;
    };
    readonly API_KEY: {
        readonly windowMs: number;
        readonly max: 10;
    };
    readonly PASSWORD_RESET: {
        readonly windowMs: number;
        readonly max: 3;
    };
    readonly READ: {
        readonly windowMs: number;
        readonly max: 200;
    };
    readonly WRITE: {
        readonly windowMs: number;
        readonly max: 50;
    };
};
/**
 * General API rate limiter
 */
export declare const generalRateLimiter: import("express-rate-limit").RateLimitRequestHandler;
/**
 * Authentication rate limiter (strict)
 */
export declare const authRateLimiter: import("express-rate-limit").RateLimitRequestHandler;
/**
 * Registration rate limiter (very strict)
 */
export declare const registrationRateLimiter: import("express-rate-limit").RateLimitRequestHandler;
/**
 * API key operations rate limiter
 */
export declare const apiKeyRateLimiter: import("express-rate-limit").RateLimitRequestHandler;
/**
 * Password reset rate limiter
 */
export declare const passwordResetRateLimiter: import("express-rate-limit").RateLimitRequestHandler;
/**
 * Read operations rate limiter
 */
export declare const readRateLimiter: import("express-rate-limit").RateLimitRequestHandler;
/**
 * Write operations rate limiter
 */
export declare const writeRateLimiter: import("express-rate-limit").RateLimitRequestHandler;
/**
 * Create custom rate limiter
 */
export declare function createRateLimiter(config: RateLimitConfig, name?: string): import("express-rate-limit").RateLimitRequestHandler;
/**
 * IP-based rate limiter (ignores authentication)
 */
export declare function createIPRateLimiter(windowMs: number, max: number): import("express-rate-limit").RateLimitRequestHandler;
/**
 * Create sliding window rate limiter
 */
export declare function createSlidingWindowRateLimiter(windowMs: number, maxRequests: number): (req: Request, res: Response, next: NextFunction) => void;
/**
 * Dynamic rate limiter based on user tier
 */
export declare function createTieredRateLimiter(tiers: Record<string, {
    windowMs: number;
    max: number;
}>): (req: Request, res: Response, next: NextFunction) => void;
//# sourceMappingURL=rate-limit.middleware.d.ts.map