/**
 * JWT Token Service for AgentDB Authentication
 *
 * Manages JWT access and refresh tokens:
 * - Access tokens: Short-lived (15 minutes), for API requests
 * - Refresh tokens: Long-lived (7 days), for renewing access tokens
 * - Token verification and validation
 * - Token revocation support
 * - Secure secret management
 *
 * Security Features:
 * - RS256 or HS256 signing algorithms
 * - Automatic expiration handling
 * - Token rotation on refresh
 * - Revocation list support
 * - Audience and issuer validation
 */
import jwt from 'jsonwebtoken';
/**
 * Token configuration
 */
export declare const TOKEN_CONFIG: {
    readonly ACCESS_TOKEN_EXPIRES_IN: "15m";
    readonly REFRESH_TOKEN_EXPIRES_IN: "7d";
    readonly ALGORITHM: "HS256";
    readonly ISSUER: "agentdb";
    readonly AUDIENCE: "agentdb-api";
};
/**
 * Token payload interface
 */
export interface TokenPayload {
    userId: string;
    email?: string;
    role?: string;
    permissions?: string[];
    type: 'access' | 'refresh';
    sessionId?: string;
}
/**
 * Token verification result
 */
export interface TokenVerificationResult {
    valid: boolean;
    payload?: TokenPayload;
    error?: string;
    expired?: boolean;
}
/**
 * Token pair (access + refresh)
 */
export interface TokenPair {
    accessToken: string;
    refreshToken: string;
    accessTokenExpiresAt: Date;
    refreshTokenExpiresAt: Date;
}
/**
 * Create JWT access token
 */
export declare function createAccessToken(payload: Omit<TokenPayload, 'type'>): string;
/**
 * Create JWT refresh token
 */
export declare function createRefreshToken(payload: Omit<TokenPayload, 'type'>): string;
/**
 * Create both access and refresh tokens
 */
export declare function createTokenPair(payload: Omit<TokenPayload, 'type'>): TokenPair;
/**
 * Verify JWT token
 */
export declare function verifyToken(token: string, type?: 'access' | 'refresh'): TokenVerificationResult;
/**
 * Verify access token
 */
export declare function verifyAccessToken(token: string): TokenVerificationResult;
/**
 * Verify refresh token
 */
export declare function verifyRefreshToken(token: string): TokenVerificationResult;
/**
 * Refresh access token using refresh token
 */
export declare function refreshAccessToken(refreshToken: string): {
    success: boolean;
    accessToken?: string;
    accessTokenExpiresAt?: Date;
    error?: string;
};
/**
 * Revoke token (add to revocation list)
 */
export declare function revokeToken(token: string): void;
/**
 * Register a token with its session for bulk revocation support
 */
export declare function registerTokenWithSession(token: string, sessionId: string): void;
/**
 * Revoke all tokens for a user (by session ID)
 */
export declare function revokeUserSession(sessionId: string): {
    revokedCount: number;
};
/**
 * Get active session count
 */
export declare function getActiveSessionCount(): number;
/**
 * Get tokens count for a session
 */
export declare function getSessionTokenCount(sessionId: string): number;
/**
 * Decode token without verification (use cautiously)
 */
export declare function decodeToken(token: string): jwt.JwtPayload | null;
/**
 * Get token expiration time
 */
export declare function getTokenExpiration(token: string): Date | null;
/**
 * Check if token is expired
 */
export declare function isTokenExpired(token: string): boolean;
/**
 * Get time until token expires (in seconds)
 */
export declare function getTokenTimeRemaining(token: string): number;
/**
 * Rotate token pair (create new tokens and revoke old refresh token)
 */
export declare function rotateTokenPair(refreshToken: string): {
    success: boolean;
    tokens?: TokenPair;
    error?: string;
};
/**
 * Extract token from Authorization header
 */
export declare function extractTokenFromHeader(authHeader: string | undefined): string | null;
/**
 * Create token for service account (long-lived, no expiration)
 */
export declare function createServiceAccountToken(payload: Omit<TokenPayload, 'type'>): string;
//# sourceMappingURL=token.service.d.ts.map