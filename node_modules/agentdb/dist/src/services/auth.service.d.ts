/**
 * Authentication Service for AgentDB
 *
 * Comprehensive authentication management:
 * - User registration and login
 * - Password-based authentication
 * - API key management (generation, validation, rotation)
 * - Session management
 * - Token lifecycle management
 * - Rate limiting and brute force protection
 *
 * Security Features:
 * - Argon2id password hashing
 * - Secure API key generation and storage
 * - Automatic key rotation
 * - Login attempt tracking
 * - Account lockout protection
 * - Audit logging
 */
import { TokenPair } from './token.service.js';
/**
 * User interface
 */
export interface User {
    id: string;
    email: string;
    passwordHash: string;
    role: string;
    permissions: string[];
    createdAt: Date;
    updatedAt: Date;
    lastLoginAt?: Date;
    failedLoginAttempts: number;
    lockedUntil?: Date;
    active: boolean;
}
/**
 * API Key interface
 */
export interface ApiKey {
    id: string;
    userId: string;
    keyHash: string;
    name: string;
    environment: 'live' | 'test';
    createdAt: Date;
    expiresAt?: Date;
    lastUsedAt?: Date;
    active: boolean;
    revokedAt?: Date;
    revokedReason?: string;
}
/**
 * Login result
 */
export interface LoginResult {
    success: boolean;
    user?: Partial<User>;
    tokens?: TokenPair;
    error?: string;
    accountLocked?: boolean;
    attemptsRemaining?: number;
}
/**
 * Registration result
 */
export interface RegistrationResult {
    success: boolean;
    user?: Partial<User>;
    error?: string;
}
/**
 * API Key creation result
 */
export interface ApiKeyCreationResult {
    success: boolean;
    apiKey?: string;
    keyInfo?: Omit<ApiKey, 'keyHash'>;
    error?: string;
}
/**
 * Register a new user
 */
export declare function registerUser(email: string, password: string, role?: string, permissions?: string[]): Promise<RegistrationResult>;
/**
 * Authenticate user with email and password
 */
export declare function login(email: string, password: string): Promise<LoginResult>;
/**
 * Logout user (revoke tokens and clear session)
 */
export declare function logout(sessionId: string, refreshToken?: string): void;
/**
 * Validate session
 */
export declare function validateSession(sessionId: string): boolean;
/**
 * Refresh session (extend expiration)
 */
export declare function refreshSession(sessionId: string): boolean;
/**
 * Create API key for user
 */
export declare function createUserApiKey(userId: string, name: string, environment?: 'live' | 'test', expiresInDays?: number): Promise<ApiKeyCreationResult>;
/**
 * Validate API key
 */
export declare function validateApiKey(apiKey: string): {
    valid: boolean;
    userId?: string;
    keyInfo?: Omit<ApiKey, 'keyHash'>;
    error?: string;
};
/**
 * Revoke API key
 */
export declare function revokeApiKey(keyId: string, reason?: string): {
    success: boolean;
    error?: string;
};
/**
 * List API keys for user
 */
export declare function listUserApiKeys(userId: string): Array<Omit<ApiKey, 'keyHash'>>;
/**
 * Rotate API key (create new key and revoke old one)
 */
export declare function rotateApiKey(keyId: string, reason?: string): Promise<ApiKeyCreationResult>;
/**
 * Get user by ID
 */
export declare function getUserById(userId: string): Partial<User> | null;
/**
 * Update user password
 */
export declare function updateUserPassword(userId: string, oldPassword: string, newPassword: string): Promise<{
    success: boolean;
    error?: string;
}>;
/**
 * Clean up expired sessions (call periodically)
 */
export declare function cleanupExpiredSessions(): number;
/**
 * Get active session count
 */
export declare function getActiveSessionCount(): number;
/**
 * Export for testing
 */
export declare const __testing__: {
    users: Map<string, User>;
    usersByEmail: Map<string, User>;
    apiKeys: Map<string, ApiKey>;
    activeSessions: Map<string, {
        userId: string;
        expiresAt: Date;
    }>;
    AUTH_CONFIG: {
        readonly MAX_LOGIN_ATTEMPTS: 5;
        readonly LOCKOUT_DURATION_MS: number;
        readonly API_KEY_DEFAULT_EXPIRY_DAYS: 365;
        readonly SESSION_TIMEOUT_MS: number;
    };
};
//# sourceMappingURL=auth.service.d.ts.map