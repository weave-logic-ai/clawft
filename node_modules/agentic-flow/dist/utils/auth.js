/**
 * Simple API key authentication for proxy
 */
export class AuthManager {
    validKeys;
    constructor(apiKeys) {
        // Load from environment or provided keys
        const envKeys = process.env.PROXY_API_KEYS?.split(',').map(k => k.trim()).filter(Boolean) || [];
        const allKeys = [...(apiKeys || []), ...envKeys];
        this.validKeys = new Set(allKeys);
        if (this.validKeys.size === 0) {
            console.warn('⚠️  Warning: No API keys configured for authentication');
            console.warn('   Set PROXY_API_KEYS environment variable or pass keys to constructor');
        }
    }
    authenticate(headers) {
        // If no keys configured, allow all (development mode)
        if (this.validKeys.size === 0) {
            return true;
        }
        // Check x-api-key header
        const apiKey = this.extractApiKey(headers);
        if (!apiKey) {
            return false;
        }
        return this.validKeys.has(apiKey);
    }
    extractApiKey(headers) {
        // Try x-api-key header
        let key = headers['x-api-key'];
        // Try authorization header (Bearer token)
        if (!key) {
            const auth = headers['authorization'];
            if (typeof auth === 'string' && auth.startsWith('Bearer ')) {
                key = auth.substring(7);
            }
        }
        if (Array.isArray(key)) {
            key = key[0];
        }
        return (typeof key === 'string' && key.length > 0) ? key : null;
    }
    addKey(key) {
        this.validKeys.add(key);
    }
    removeKey(key) {
        this.validKeys.delete(key);
    }
    hasKeys() {
        return this.validKeys.size > 0;
    }
}
//# sourceMappingURL=auth.js.map