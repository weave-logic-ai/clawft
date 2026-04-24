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

import crypto from 'crypto';
import argon2 from 'argon2';

/**
 * Argon2 configuration (OWASP recommended)
 */
export const ARGON2_CONFIG = {
  type: argon2.argon2id,
  memoryCost: 65536, // 64 MB
  timeCost: 3,       // 3 iterations
  parallelism: 4,    // 4 threads
  hashLength: 32,    // 32 bytes output
} as const;

/**
 * Generate cryptographically secure random bytes
 */
export function generateSecureRandom(length: number = 32): Buffer {
  if (length < 16) {
    throw new Error('Random length must be at least 16 bytes for security');
  }
  if (length > 1024) {
    throw new Error('Random length too large (max 1024 bytes)');
  }

  return crypto.randomBytes(length);
}

/**
 * Generate secure random hex string
 */
export function generateSecureRandomHex(length: number = 32): string {
  return generateSecureRandom(length).toString('hex');
}

/**
 * Generate URL-safe random string (base64url)
 */
export function generateSecureRandomBase64(length: number = 32): string {
  return generateSecureRandom(length)
    .toString('base64')
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=/g, '');
}

/**
 * Hash password using Argon2id
 * OWASP recommended algorithm - resistant to GPU/ASIC attacks
 */
export async function hashPassword(password: string): Promise<string> {
  if (!password || typeof password !== 'string') {
    throw new Error('Password must be a non-empty string');
  }

  if (password.length < 8) {
    throw new Error('Password must be at least 8 characters');
  }

  if (password.length > 128) {
    throw new Error('Password exceeds maximum length (128 characters)');
  }

  try {
    const hash = await argon2.hash(password, ARGON2_CONFIG);
    return hash;
  } catch (error) {
    throw new Error(`Password hashing failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
  }
}

/**
 * Verify password against Argon2 hash
 * Uses constant-time comparison internally
 */
export async function verifyPassword(
  password: string,
  hash: string
): Promise<boolean> {
  if (!password || !hash) {
    return false;
  }

  try {
    return await argon2.verify(hash, password);
  } catch (error) {
    // Invalid hash format or verification error
    console.error('Password verification error:', error);
    return false;
  }
}

/**
 * Generate API key with cryptographically secure random data
 * Format: agdb_live_<64 hex chars> or agdb_test_<64 hex chars>
 */
export function generateApiKey(environment: 'live' | 'test' = 'live'): {
  key: string;
  hash: string;
} {
  const prefix = environment === 'live' ? 'agdb_live_' : 'agdb_test_';
  const randomPart = generateSecureRandomHex(32); // 64 hex chars
  const key = prefix + randomPart;

  // Create hash for storage (we only store the hash, never the raw key)
  const hash = crypto
    .createHash('sha256')
    .update(key)
    .digest('hex');

  return { key, hash };
}

/**
 * Hash API key for storage
 */
export function hashApiKey(apiKey: string): string {
  if (!apiKey || typeof apiKey !== 'string') {
    throw new Error('API key must be a non-empty string');
  }

  // Validate API key format
  const validPrefixes = ['agdb_live_', 'agdb_test_'];
  const hasValidPrefix = validPrefixes.some(prefix => apiKey.startsWith(prefix));

  if (!hasValidPrefix) {
    throw new Error('Invalid API key format');
  }

  return crypto
    .createHash('sha256')
    .update(apiKey)
    .digest('hex');
}

/**
 * Verify API key against stored hash
 * Uses constant-time comparison to prevent timing attacks
 */
export function verifyApiKey(apiKey: string, storedHash: string): boolean {
  if (!apiKey || !storedHash) {
    return false;
  }

  try {
    const computedHash = hashApiKey(apiKey);

    // Constant-time comparison
    return crypto.timingSafeEqual(
      Buffer.from(computedHash, 'hex'),
      Buffer.from(storedHash, 'hex')
    );
  } catch (error) {
    return false;
  }
}

/**
 * Generate secure session ID
 */
export function generateSessionId(): string {
  return generateSecureRandomBase64(32);
}

/**
 * Generate JWT secret key (for initial setup)
 */
export function generateJWTSecret(): string {
  return generateSecureRandomBase64(64);
}

/**
 * Constant-time string comparison
 * Prevents timing attacks when comparing secrets
 */
export function constantTimeCompare(a: string, b: string): boolean {
  if (!a || !b) {
    return false;
  }

  try {
    const bufferA = Buffer.from(a);
    const bufferB = Buffer.from(b);

    if (bufferA.length !== bufferB.length) {
      return false;
    }

    return crypto.timingSafeEqual(bufferA, bufferB);
  } catch (error) {
    return false;
  }
}

/**
 * Generate HMAC signature for data integrity
 */
export function generateHMAC(data: string, secret: string): string {
  return crypto
    .createHmac('sha256', secret)
    .update(data)
    .digest('hex');
}

/**
 * Verify HMAC signature
 */
export function verifyHMAC(
  data: string,
  signature: string,
  secret: string
): boolean {
  const expectedSignature = generateHMAC(data, secret);
  return constantTimeCompare(signature, expectedSignature);
}

/**
 * Hash data with SHA-256
 */
export function sha256(data: string): string {
  return crypto.createHash('sha256').update(data).digest('hex');
}

/**
 * Generate cryptographically secure UUID v4
 */
export function generateUUID(): string {
  return crypto.randomUUID();
}

/**
 * Derive key from password using PBKDF2 (for encryption keys)
 */
export async function deriveKey(
  password: string,
  salt: Buffer,
  iterations: number = 100000,
  keyLength: number = 32
): Promise<Buffer> {
  return new Promise((resolve, reject) => {
    crypto.pbkdf2(password, salt, iterations, keyLength, 'sha256', (err, derivedKey) => {
      if (err) reject(err);
      else resolve(derivedKey);
    });
  });
}

/**
 * Generate salt for key derivation
 */
export function generateSalt(length: number = 16): Buffer {
  return crypto.randomBytes(length);
}

/**
 * Encrypt data using AES-256-GCM
 */
export function encrypt(plaintext: string, key: Buffer): {
  ciphertext: string;
  iv: string;
  authTag: string;
} {
  const iv = crypto.randomBytes(12); // 96-bit IV for GCM
  const cipher = crypto.createCipheriv('aes-256-gcm', key, iv);

  let ciphertext = cipher.update(plaintext, 'utf8', 'hex');
  ciphertext += cipher.final('hex');

  const authTag = cipher.getAuthTag();

  return {
    ciphertext,
    iv: iv.toString('hex'),
    authTag: authTag.toString('hex'),
  };
}

/**
 * Decrypt data using AES-256-GCM
 */
export function decrypt(
  ciphertext: string,
  key: Buffer,
  iv: string,
  authTag: string
): string {
  const decipher = crypto.createDecipheriv(
    'aes-256-gcm',
    key,
    Buffer.from(iv, 'hex')
  );

  decipher.setAuthTag(Buffer.from(authTag, 'hex'));

  let plaintext = decipher.update(ciphertext, 'hex', 'utf8');
  plaintext += decipher.final('utf8');

  return plaintext;
}

/**
 * Validate password strength
 */
export function validatePasswordStrength(password: string): {
  valid: boolean;
  errors: string[];
  score: number;
} {
  const errors: string[] = [];
  let score = 0;

  if (!password) {
    return { valid: false, errors: ['Password is required'], score: 0 };
  }

  // Length check
  if (password.length < 8) {
    errors.push('Password must be at least 8 characters');
  } else {
    score += 1;
  }

  if (password.length >= 12) {
    score += 1;
  }

  // Complexity checks
  if (/[a-z]/.test(password)) score += 1;
  if (/[A-Z]/.test(password)) score += 1;
  if (/[0-9]/.test(password)) score += 1;
  if (/[^a-zA-Z0-9]/.test(password)) score += 1;

  // Common patterns
  if (/^[a-zA-Z]+$/.test(password)) {
    errors.push('Password should contain numbers or special characters');
  }

  if (/^[0-9]+$/.test(password)) {
    errors.push('Password should contain letters');
  }

  // Sequential or repeated characters
  if (/(.)\1{2,}/.test(password)) {
    errors.push('Password contains repeated characters');
    score -= 1;
  }

  if (/012|123|234|345|456|567|678|789|abc|bcd|cde|def/.test(password.toLowerCase())) {
    errors.push('Password contains sequential characters');
    score -= 1;
  }

  const valid = errors.length === 0 && score >= 4;

  return { valid, errors, score: Math.max(0, score) };
}
