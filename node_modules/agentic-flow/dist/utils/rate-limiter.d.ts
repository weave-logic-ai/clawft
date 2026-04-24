/**
 * Simple in-memory rate limiter for proxy protection
 */
export interface RateLimiterConfig {
    points: number;
    duration: number;
    blockDuration: number;
}
export declare class RateLimiter {
    private config;
    private clients;
    private cleanupInterval;
    constructor(config: RateLimiterConfig);
    consume(key: string): Promise<void>;
    destroy(): void;
}
//# sourceMappingURL=rate-limiter.d.ts.map