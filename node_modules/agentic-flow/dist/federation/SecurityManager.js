/**
 * Security Manager - Authentication, encryption, and access control
 *
 * Features:
 * - JWT token generation and validation
 * - AES-256 encryption for data at rest
 * - Tenant isolation
 * - mTLS certificate management
 */
import crypto from 'crypto';
import { logger } from '../utils/logger.js';
export class SecurityManager {
    algorithm = 'aes-256-gcm';
    jwtSecret;
    encryptionCache = new Map();
    constructor() {
        // In production, load from secure vault (AWS Secrets Manager, HashiCorp Vault, etc.)
        this.jwtSecret = process.env.JWT_SECRET || crypto.randomBytes(32).toString('hex');
    }
    /**
     * Create JWT token for agent authentication
     */
    async createAgentToken(payload) {
        const header = {
            alg: 'HS256',
            typ: 'JWT'
        };
        const now = Date.now();
        const tokenPayload = {
            ...payload,
            iat: now,
            exp: payload.expiresAt,
            iss: 'agentic-flow-federation'
        };
        // Encode header and payload
        const encodedHeader = this.base64UrlEncode(JSON.stringify(header));
        const encodedPayload = this.base64UrlEncode(JSON.stringify(tokenPayload));
        // Create signature
        const signature = crypto
            .createHmac('sha256', this.jwtSecret)
            .update(`${encodedHeader}.${encodedPayload}`)
            .digest('base64url');
        const token = `${encodedHeader}.${encodedPayload}.${signature}`;
        logger.info('Created agent token', {
            agentId: payload.agentId,
            tenantId: payload.tenantId,
            expiresAt: new Date(payload.expiresAt).toISOString()
        });
        return token;
    }
    /**
     * Verify JWT token
     */
    async verifyAgentToken(token) {
        const parts = token.split('.');
        if (parts.length !== 3) {
            throw new Error('Invalid token format');
        }
        const [encodedHeader, encodedPayload, signature] = parts;
        // Verify signature
        const expectedSignature = crypto
            .createHmac('sha256', this.jwtSecret)
            .update(`${encodedHeader}.${encodedPayload}`)
            .digest('base64url');
        if (signature !== expectedSignature) {
            throw new Error('Invalid token signature');
        }
        // Decode payload
        const payload = JSON.parse(this.base64UrlDecode(encodedPayload));
        // Check expiration
        if (Date.now() >= payload.exp) {
            throw new Error('Token expired');
        }
        logger.debug('Token verified', {
            agentId: payload.agentId,
            tenantId: payload.tenantId
        });
        return payload;
    }
    /**
     * Get or create encryption keys for a tenant
     */
    async getEncryptionKeys(tenantId) {
        // Check cache
        if (this.encryptionCache.has(tenantId)) {
            return this.encryptionCache.get(tenantId);
        }
        // Generate new keys for tenant
        // In production, these would be stored in a secure key management service
        const encryptionKey = crypto.randomBytes(32); // 256-bit key
        const iv = crypto.randomBytes(16); // 128-bit IV
        const keys = { encryptionKey, iv };
        // Cache keys
        this.encryptionCache.set(tenantId, keys);
        logger.info('Generated encryption keys for tenant', { tenantId });
        return keys;
    }
    /**
     * Encrypt data with AES-256-GCM
     */
    async encrypt(data, tenantId) {
        const keys = await this.getEncryptionKeys(tenantId);
        const cipher = crypto.createCipheriv(this.algorithm, keys.encryptionKey, keys.iv);
        let encrypted = cipher.update(data, 'utf8', 'base64');
        encrypted += cipher.final('base64');
        const authTag = cipher.getAuthTag().toString('base64');
        logger.debug('Data encrypted', {
            tenantId,
            originalLength: data.length,
            encryptedLength: encrypted.length
        });
        return { encrypted, authTag };
    }
    /**
     * Decrypt data with AES-256-GCM
     */
    async decrypt(encrypted, authTag, tenantId) {
        const keys = await this.getEncryptionKeys(tenantId);
        const decipher = crypto.createDecipheriv(this.algorithm, keys.encryptionKey, keys.iv);
        decipher.setAuthTag(Buffer.from(authTag, 'base64'));
        let decrypted = decipher.update(encrypted, 'base64', 'utf8');
        decrypted += decipher.final('utf8');
        logger.debug('Data decrypted', {
            tenantId,
            decryptedLength: decrypted.length
        });
        return decrypted;
    }
    /**
     * Generate mTLS certificates for agent-to-hub communication
     */
    async generateMTLSCertificates(agentId) {
        // Placeholder: Actual implementation would use OpenSSL or similar
        // to generate X.509 certificates with proper CA chain
        logger.info('Generating mTLS certificates', { agentId });
        return {
            cert: 'PLACEHOLDER_CERT',
            key: 'PLACEHOLDER_KEY',
            ca: 'PLACEHOLDER_CA'
        };
    }
    /**
     * Validate tenant access to data
     */
    validateTenantAccess(requestTenantId, dataTenantId) {
        if (requestTenantId !== dataTenantId) {
            logger.warn('Tenant access violation detected', {
                requestTenantId,
                dataTenantId
            });
            return false;
        }
        return true;
    }
    /**
     * Hash sensitive data for storage (one-way)
     */
    hashData(data) {
        return crypto.createHash('sha256').update(data).digest('hex');
    }
    /**
     * Generate secure random ID
     */
    generateSecureId() {
        return crypto.randomBytes(16).toString('hex');
    }
    /**
     * Base64 URL-safe encoding
     */
    base64UrlEncode(str) {
        return Buffer.from(str)
            .toString('base64')
            .replace(/\+/g, '-')
            .replace(/\//g, '_')
            .replace(/=/g, '');
    }
    /**
     * Base64 URL-safe decoding
     */
    base64UrlDecode(str) {
        const base64 = str.replace(/-/g, '+').replace(/_/g, '/');
        return Buffer.from(base64, 'base64').toString('utf8');
    }
    /**
     * Clear cached keys (for testing or security refresh)
     */
    clearCache() {
        this.encryptionCache.clear();
        logger.info('Encryption cache cleared');
    }
}
//# sourceMappingURL=SecurityManager.js.map