/**
 * Cryptographic Utilities for AgentDB Authentication
 *
 * Provides secure cryptographic operations:
 * - Password hashing with Argon2id (OWASP recommended)
 * - Secure random token generation
 * - API key generation and verification
 * - Constant-time comparison
 *
 * Security Features:
 * - Argon2id algorithm (resistant to GPU/ASIC attacks)
 * - Cryptographically secure random generation
 * - Timing attack prevention
 * - Configurable security parameters
 */
/**
 * Argon2 configuration (OWASP recommended)
 */
export declare const ARGON2_CONFIG: {
    readonly type: 2;
    readonly memoryCost: 65536;
    readonly timeCost: 3;
    readonly parallelism: 4;
    readonly hashLength: 32;
};
/**
 * Generate cryptographically secure random bytes
 */
export declare function generateSecureRandom(length?: number): Buffer;
/**
 * Generate secure random hex string
 */
export declare function generateSecureRandomHex(length?: number): string;
/**
 * Generate URL-safe random string (base64url)
 */
export declare function generateSecureRandomBase64(length?: number): string;
/**
 * Hash password using Argon2id
 * OWASP recommended algorithm - resistant to GPU/ASIC attacks
 */
export declare function hashPassword(password: string): Promise<string>;
/**
 * Verify password against Argon2 hash
 * Uses constant-time comparison internally
 */
export declare function verifyPassword(password: string, hash: string): Promise<boolean>;
/**
 * Generate API key with cryptographically secure random data
 * Format: agdb_live_<64 hex chars> or agdb_test_<64 hex chars>
 */
export declare function generateApiKey(environment?: 'live' | 'test'): {
    key: string;
    hash: string;
};
/**
 * Hash API key for storage
 */
export declare function hashApiKey(apiKey: string): string;
/**
 * Verify API key against stored hash
 * Uses constant-time comparison to prevent timing attacks
 */
export declare function verifyApiKey(apiKey: string, storedHash: string): boolean;
/**
 * Generate secure session ID
 */
export declare function generateSessionId(): string;
/**
 * Generate JWT secret key (for initial setup)
 */
export declare function generateJWTSecret(): string;
/**
 * Constant-time string comparison
 * Prevents timing attacks when comparing secrets
 */
export declare function constantTimeCompare(a: string, b: string): boolean;
/**
 * Generate HMAC signature for data integrity
 */
export declare function generateHMAC(data: string, secret: string): string;
/**
 * Verify HMAC signature
 */
export declare function verifyHMAC(data: string, signature: string, secret: string): boolean;
/**
 * Hash data with SHA-256
 */
export declare function sha256(data: string): string;
/**
 * Generate cryptographically secure UUID v4
 */
export declare function generateUUID(): string;
/**
 * Derive key from password using PBKDF2 (for encryption keys)
 */
export declare function deriveKey(password: string, salt: Buffer, iterations?: number, keyLength?: number): Promise<Buffer>;
/**
 * Generate salt for key derivation
 */
export declare function generateSalt(length?: number): Buffer;
/**
 * Encrypt data using AES-256-GCM
 */
export declare function encrypt(plaintext: string, key: Buffer): {
    ciphertext: string;
    iv: string;
    authTag: string;
};
/**
 * Decrypt data using AES-256-GCM
 */
export declare function decrypt(ciphertext: string, key: Buffer, iv: string, authTag: string): string;
/**
 * Validate password strength
 */
export declare function validatePasswordStrength(password: string): {
    valid: boolean;
    errors: string[];
    score: number;
};
//# sourceMappingURL=crypto.utils.d.ts.map