"use strict";Object.defineProperty(exports, "__esModule", {value: true}); function _interopRequireWildcard(obj) { if (obj && obj.__esModule) { return obj; } else { var newObj = {}; if (obj != null) { for (var key in obj) { if (Object.prototype.hasOwnProperty.call(obj, key)) { newObj[key] = obj[key]; } } } newObj.default = obj; return newObj; } } function _nullishCoalesce(lhs, rhsFn) { if (lhs != null) { return lhs; } else { return rhsFn(); } } function _optionalChain(ops) { let lastAccessLHS = undefined; let value = ops[0]; let i = 1; while (i < ops.length) { const op = ops[i]; const fn = ops[i + 1]; i += 2; if ((op === 'optionalAccess' || op === 'optionalCall') && value == null) { return undefined; } if (op === 'access' || op === 'optionalAccess') { lastAccessLHS = value; value = fn(value); } else if (op === 'call' || op === 'optionalCall') { value = fn((...args) => value.call(lastAccessLHS, ...args)); lastAccessLHS = undefined; } } return value; } var _class; var _class2; var _class3; var _class4; var _class5; var _class6;// src/auth/helpers.ts
function getAuthSession(session) {
  if (!session) {
    throw new Error("Session is not authenticated");
  }
  return session;
}
function requireAll(...checks) {
  return (auth) => checks.every(
    (check) => typeof check === "function" ? check(auth) : check
  );
}
function requireAny(...checks) {
  return (auth) => checks.some((check) => typeof check === "function" ? check(auth) : check);
}
function requireAuth(auth) {
  return auth !== void 0 && auth !== null;
}
function requireRole(...allowedRoles) {
  return (auth) => {
    if (!auth) return false;
    const role = auth.role;
    return typeof role === "string" && allowedRoles.includes(role);
  };
}
function requireScopes(...requiredScopes) {
  return (auth) => {
    if (!auth) return false;
    const authScopes = auth.scopes;
    if (!authScopes) return false;
    const scopeSet = Array.isArray(authScopes) ? new Set(authScopes) : authScopes instanceof Set ? authScopes : /* @__PURE__ */ new Set();
    return requiredScopes.every((scope) => scopeSet.has(scope));
  };
}

// src/auth/OAuthProxy.ts
var _crypto = require('crypto');
var _zod = require('zod');

// src/auth/types.ts
var DEFAULT_ACCESS_TOKEN_TTL = 3600;
var DEFAULT_ACCESS_TOKEN_TTL_NO_REFRESH = 31536e3;
var DEFAULT_REFRESH_TOKEN_TTL = 2592e3;
var DEFAULT_AUTHORIZATION_CODE_TTL = 300;
var DEFAULT_TRANSACTION_TTL = 600;

// src/auth/utils/claimsExtractor.ts
var ClaimsExtractor = (_class = class {
  
  // Claims that MUST NOT be copied from upstream (protect proxy's JWT integrity)
  __init() {this.PROTECTED_CLAIMS = /* @__PURE__ */ new Set([
    "aud",
    "client_id",
    "exp",
    "iat",
    "iss",
    "jti",
    "nbf"
  ])}
  constructor(config) {;_class.prototype.__init.call(this);
    if (typeof config === "boolean") {
      config = config ? {} : { fromAccessToken: false, fromIdToken: false };
    }
    this.config = {
      allowComplexClaims: config.allowComplexClaims || false,
      allowedClaims: config.allowedClaims,
      blockedClaims: config.blockedClaims || [],
      claimPrefix: config.claimPrefix !== void 0 ? config.claimPrefix : false,
      // Default: no prefix
      fromAccessToken: config.fromAccessToken !== false,
      // Default: true
      fromIdToken: config.fromIdToken !== false,
      // Default: true
      maxClaimValueSize: config.maxClaimValueSize || 2e3
    };
  }
  /**
   * Extract claims from a token (access token or ID token)
   */
  async extract(token, tokenType) {
    if (tokenType === "access" && !this.config.fromAccessToken) {
      return null;
    }
    if (tokenType === "id" && !this.config.fromIdToken) {
      return null;
    }
    if (!this.isJWT(token)) {
      return null;
    }
    const payload = this.decodeJWTPayload(token);
    if (!payload) {
      return null;
    }
    const filtered = this.filterClaims(payload);
    return this.applyPrefix(filtered);
  }
  /**
   * Apply prefix to claim names (if configured)
   */
  applyPrefix(claims) {
    const prefix = this.config.claimPrefix;
    if (prefix === false || prefix === "" || prefix === void 0) {
      return claims;
    }
    const result = {};
    for (const [key, value] of Object.entries(claims)) {
      result[`${prefix}${key}`] = value;
    }
    return result;
  }
  /**
   * Decode JWT payload without signature verification
   * Safe because token came from trusted upstream via server-to-server exchange
   */
  decodeJWTPayload(token) {
    try {
      const parts = token.split(".");
      if (parts.length !== 3) {
        return null;
      }
      const payload = Buffer.from(parts[1], "base64url").toString("utf-8");
      return JSON.parse(payload);
    } catch (error) {
      console.warn(`Failed to decode JWT payload: ${error}`);
      return null;
    }
  }
  /**
   * Filter claims based on security rules
   */
  filterClaims(claims) {
    const result = {};
    for (const [key, value] of Object.entries(claims)) {
      if (this.PROTECTED_CLAIMS.has(key)) {
        continue;
      }
      if (_optionalChain([this, 'access', _ => _.config, 'access', _2 => _2.blockedClaims, 'optionalAccess', _3 => _3.includes, 'call', _4 => _4(key)])) {
        continue;
      }
      if (this.config.allowedClaims && !this.config.allowedClaims.includes(key)) {
        continue;
      }
      if (!this.isValidClaimValue(value)) {
        console.warn(`Skipping claim '${key}' due to invalid value`);
        continue;
      }
      result[key] = value;
    }
    return result;
  }
  /**
   * Check if a token is in JWT format
   */
  isJWT(token) {
    return token.split(".").length === 3;
  }
  /**
   * Validate a claim value (type and size checks)
   */
  isValidClaimValue(value) {
    if (value === null || value === void 0) {
      return false;
    }
    const type = typeof value;
    if (type === "string") {
      const maxSize = _nullishCoalesce(this.config.maxClaimValueSize, () => ( 2e3));
      return value.length <= maxSize;
    }
    if (type === "number" || type === "boolean") {
      return true;
    }
    if (Array.isArray(value) || type === "object") {
      if (!this.config.allowComplexClaims) {
        return false;
      }
      try {
        const stringified = JSON.stringify(value);
        const maxSize = _nullishCoalesce(this.config.maxClaimValueSize, () => ( 2e3));
        return stringified.length <= maxSize;
      } catch (e) {
        return false;
      }
    }
    return false;
  }
}, _class);

// src/auth/utils/consent.ts

var ConsentManager = class {
  
  constructor(signingKey) {
    this.signingKey = signingKey || this.generateDefaultKey();
  }
  /**
   * Create HTTP response with consent screen
   */
  createConsentResponse(transaction, provider) {
    const consentData = {
      clientName: "MCP Client",
      provider,
      scope: transaction.scope,
      timestamp: Date.now(),
      transactionId: transaction.id
    };
    const html = this.generateConsentScreen(consentData);
    return new Response(html, {
      headers: {
        "Content-Type": "text/html; charset=utf-8"
      },
      status: 200
    });
  }
  /**
   * Generate HTML for consent screen
   */
  generateConsentScreen(data) {
    const { clientName, provider, scope, transactionId } = data;
    return `
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Authorization Request</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            justify-content: center;
            align-items: center;
            padding: 20px;
        }

        .consent-container {
            background: white;
            border-radius: 12px;
            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
            max-width: 480px;
            width: 100%;
            padding: 40px;
        }

        .header {
            text-align: center;
            margin-bottom: 30px;
        }

        .header h1 {
            color: #1a202c;
            font-size: 24px;
            margin-bottom: 8px;
        }

        .header p {
            color: #718096;
            font-size: 14px;
        }

        .app-info {
            background: #f7fafc;
            border-radius: 8px;
            padding: 20px;
            margin-bottom: 24px;
        }

        .app-info h2 {
            color: #2d3748;
            font-size: 18px;
            margin-bottom: 12px;
        }

        .app-name {
            color: #667eea;
            font-weight: 600;
        }

        .permissions {
            margin-top: 16px;
        }

        .permissions h3 {
            color: #4a5568;
            font-size: 14px;
            margin-bottom: 8px;
            font-weight: 600;
        }

        .permissions ul {
            list-style: none;
        }

        .permissions li {
            color: #718096;
            font-size: 14px;
            padding: 6px 0;
            padding-left: 24px;
            position: relative;
        }

        .permissions li:before {
            content: "\u2713";
            position: absolute;
            left: 0;
            color: #48bb78;
            font-weight: bold;
        }

        .warning {
            background: #fffaf0;
            border-left: 4px solid #ed8936;
            padding: 12px 16px;
            margin-bottom: 24px;
            border-radius: 4px;
        }

        .warning p {
            color: #744210;
            font-size: 13px;
            line-height: 1.5;
        }

        .actions {
            display: flex;
            gap: 12px;
        }

        button {
            flex: 1;
            padding: 14px 24px;
            border: none;
            border-radius: 6px;
            font-size: 16px;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.2s;
        }

        .approve {
            background: #667eea;
            color: white;
        }

        .approve:hover {
            background: #5a67d8;
            transform: translateY(-1px);
            box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
        }

        .deny {
            background: #e2e8f0;
            color: #4a5568;
        }

        .deny:hover {
            background: #cbd5e0;
        }

        .footer {
            margin-top: 24px;
            text-align: center;
            color: #a0aec0;
            font-size: 12px;
        }
    </style>
</head>
<body>
    <div class="consent-container">
        <div class="header">
            <h1>\u{1F510} Authorization Request</h1>
            <p>via ${this.escapeHtml(provider)}</p>
        </div>

        <div class="app-info">
            <h2>
                <span class="app-name">${this.escapeHtml(clientName || "An application")}</span>
                requests access
            </h2>

            <div class="permissions">
                <h3>This will allow the app to:</h3>
                <ul>
                    ${scope.map((s) => `<li>${this.escapeHtml(this.formatScope(s))}</li>`).join("")}
                </ul>
            </div>
        </div>

        <div class="warning">
            <p>
                <strong>\u26A0\uFE0F Important:</strong> Only approve if you trust this application.
                By approving, you authorize it to access your account information.
            </p>
        </div>

        <form method="POST" action="/oauth/consent">
            <input type="hidden" name="transaction_id" value="${this.escapeHtml(transactionId)}">
            <div class="actions">
                <button type="submit" name="action" value="deny" class="deny">
                    Deny
                </button>
                <button type="submit" name="action" value="approve" class="approve">
                    Approve
                </button>
            </div>
        </form>

        <div class="footer">
            <p>This consent is required to prevent unauthorized access.</p>
        </div>
    </div>
</body>
</html>
    `.trim();
  }
  /**
   * Sign consent data for cookie
   */
  signConsentCookie(data) {
    const payload = JSON.stringify(data);
    const signature = this.sign(payload);
    return `${Buffer.from(payload).toString("base64")}.${signature}`;
  }
  /**
   * Validate and parse consent cookie
   */
  validateConsentCookie(cookie) {
    try {
      const [payloadB64, signature] = cookie.split(".");
      if (!payloadB64 || !signature) {
        return null;
      }
      const payload = Buffer.from(payloadB64, "base64").toString("utf8");
      const expectedSignature = this.sign(payload);
      if (signature !== expectedSignature) {
        return null;
      }
      const data = JSON.parse(payload);
      const age = Date.now() - data.timestamp;
      if (age > 5 * 60 * 1e3) {
        return null;
      }
      return data;
    } catch (e2) {
      return null;
    }
  }
  /**
   * Escape HTML to prevent XSS
   */
  escapeHtml(text) {
    const map = {
      "'": "&#x27;",
      '"': "&quot;",
      "/": "&#x2F;",
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;"
    };
    return text.replace(/[&<>"'/]/g, (char) => map[char] || char);
  }
  /**
   * Format scope for display
   */
  formatScope(scope) {
    const scopeMap = {
      email: "Access your email address",
      openid: "Verify your identity",
      profile: "View your basic profile information",
      "read:user": "Read your user information",
      "write:user": "Modify your user information"
    };
    return scopeMap[scope] || scope.replace(/_/g, " ").replace(/:/g, " - ");
  }
  /**
   * Generate default signing key if none provided
   */
  generateDefaultKey() {
    return `fastmcp-consent-${Date.now()}-${Math.random()}`;
  }
  /**
   * Sign a payload using HMAC-SHA256
   */
  sign(payload) {
    return _crypto.createHmac.call(void 0, "sha256", this.signingKey).update(payload).digest("hex");
  }
};

// src/auth/utils/jwtIssuer.ts

var _util = require('util');
var pbkdf2Async = _util.promisify.call(void 0, _crypto.pbkdf2);
var JWTIssuer = class {
  
  
  
  
  
  constructor(config) {
    this.issuer = config.issuer;
    this.audience = config.audience;
    this.accessTokenTtl = config.accessTokenTtl || DEFAULT_ACCESS_TOKEN_TTL;
    this.refreshTokenTtl = config.refreshTokenTtl || DEFAULT_REFRESH_TOKEN_TTL;
    this.signingKey = Buffer.from(config.signingKey);
  }
  /**
   * Derive a signing key from a secret
   * Uses PBKDF2 for key derivation
   */
  static async deriveKey(secret, iterations = 1e5) {
    const salt = Buffer.from("fastmcp-oauth-proxy");
    const key = await pbkdf2Async(secret, salt, iterations, 32, "sha256");
    return key.toString("base64");
  }
  /**
   * Issue an access token
   */
  issueAccessToken(clientId, scope, additionalClaims, expiresIn) {
    const now = Math.floor(Date.now() / 1e3);
    const jti = this.generateJti();
    const claims = {
      aud: this.audience,
      client_id: clientId,
      exp: now + (_nullishCoalesce(expiresIn, () => ( this.accessTokenTtl))),
      iat: now,
      iss: this.issuer,
      jti,
      scope,
      // Merge additional claims (custom claims from upstream)
      ...additionalClaims || {}
    };
    return this.signToken(claims);
  }
  /**
   * Issue a refresh token
   */
  issueRefreshToken(clientId, scope, additionalClaims, expiresIn) {
    const now = Math.floor(Date.now() / 1e3);
    const jti = this.generateJti();
    const claims = {
      aud: this.audience,
      client_id: clientId,
      exp: now + (_nullishCoalesce(expiresIn, () => ( this.refreshTokenTtl))),
      iat: now,
      iss: this.issuer,
      jti,
      scope,
      // Merge additional claims (custom claims from upstream)
      ...additionalClaims || {}
    };
    return this.signToken(claims);
  }
  /**
   * Validate a JWT token
   */
  async verify(token) {
    try {
      const parts = token.split(".");
      if (parts.length !== 3) {
        return {
          error: "Invalid token format",
          valid: false
        };
      }
      const [headerB64, payloadB64, signatureB64] = parts;
      const expectedSignature = this.sign(`${headerB64}.${payloadB64}`);
      if (signatureB64 !== expectedSignature) {
        return {
          error: "Invalid signature",
          valid: false
        };
      }
      const claims = JSON.parse(
        Buffer.from(payloadB64, "base64url").toString("utf-8")
      );
      const now = Math.floor(Date.now() / 1e3);
      if (claims.exp <= now) {
        return {
          claims,
          error: "Token expired",
          valid: false
        };
      }
      if (claims.iss !== this.issuer) {
        return {
          claims,
          error: "Invalid issuer",
          valid: false
        };
      }
      if (claims.aud !== this.audience) {
        return {
          claims,
          error: "Invalid audience",
          valid: false
        };
      }
      return {
        claims,
        valid: true
      };
    } catch (error) {
      return {
        error: error instanceof Error ? error.message : "Validation failed",
        valid: false
      };
    }
  }
  /**
   * Generate unique JWT ID
   */
  generateJti() {
    return _crypto.randomBytes.call(void 0, 16).toString("base64url");
  }
  /**
   * Sign data with HMAC-SHA256
   */
  sign(data) {
    const hmac = _crypto.createHmac.call(void 0, "sha256", this.signingKey);
    hmac.update(data);
    return hmac.digest("base64url");
  }
  /**
   * Sign a JWT token
   */
  signToken(claims) {
    const header = {
      alg: "HS256",
      typ: "JWT"
    };
    const headerB64 = Buffer.from(JSON.stringify(header)).toString("base64url");
    const payloadB64 = Buffer.from(JSON.stringify(claims)).toString(
      "base64url"
    );
    const signature = this.sign(`${headerB64}.${payloadB64}`);
    return `${headerB64}.${payloadB64}.${signature}`;
  }
};

// src/auth/utils/pkce.ts

var PKCEUtils = class _PKCEUtils {
  /**
   * Generate a code challenge from a verifier
   * @param verifier The code verifier
   * @param method Challenge method: 'S256' or 'plain' (default: 'S256')
   * @returns Base64URL-encoded challenge string
   */
  static generateChallenge(verifier, method = "S256") {
    if (method === "plain") {
      return verifier;
    }
    if (method === "S256") {
      const hash = _crypto.createHash.call(void 0, "sha256");
      hash.update(verifier);
      return _PKCEUtils.base64URLEncode(hash.digest());
    }
    throw new Error(`Unsupported challenge method: ${method}`);
  }
  /**
   * Generate a complete PKCE pair (verifier + challenge)
   * @param method Challenge method: 'S256' or 'plain' (default: 'S256')
   * @returns Object containing verifier and challenge
   */
  static generatePair(method = "S256") {
    const verifier = _PKCEUtils.generateVerifier();
    const challenge = _PKCEUtils.generateChallenge(verifier, method);
    return {
      challenge,
      verifier
    };
  }
  /**
   * Generate a cryptographically secure code verifier
   * @param length Length of verifier (43-128 characters, default: 128)
   * @returns Base64URL-encoded verifier string
   */
  static generateVerifier(length = 128) {
    if (length < 43 || length > 128) {
      throw new Error("PKCE verifier length must be between 43 and 128");
    }
    const byteLength = Math.ceil(length * 3 / 4);
    const randomBytesBuffer = _crypto.randomBytes.call(void 0, byteLength);
    return _PKCEUtils.base64URLEncode(randomBytesBuffer).slice(0, length);
  }
  /**
   * Validate a code verifier against a challenge
   * @param verifier The code verifier to validate
   * @param challenge The expected challenge
   * @param method The challenge method used
   * @returns True if verifier matches challenge
   */
  static validateChallenge(verifier, challenge, method) {
    if (!verifier || !challenge) {
      return false;
    }
    if (method === "plain") {
      return verifier === challenge;
    }
    if (method === "S256") {
      const computedChallenge = _PKCEUtils.generateChallenge(verifier, "S256");
      return computedChallenge === challenge;
    }
    return false;
  }
  /**
   * Encode a buffer as base64url (RFC 4648)
   * @param buffer Buffer to encode
   * @returns Base64URL-encoded string
   */
  static base64URLEncode(buffer) {
    return buffer.toString("base64").replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
  }
};

// src/auth/utils/tokenStore.ts






var EncryptedTokenStorage = (_class2 = class {
  __init2() {this.algorithm = "aes-256-gcm"}
  
  
  constructor(backend, encryptionKey) {;_class2.prototype.__init2.call(this);
    this.backend = backend;
    const salt = Buffer.from("fastmcp-oauth-proxy-salt");
    this.encryptionKey = _crypto.scryptSync.call(void 0, encryptionKey, salt, 32);
  }
  async cleanup() {
    await this.backend.cleanup();
  }
  async delete(key) {
    await this.backend.delete(key);
  }
  async get(key) {
    const encrypted = await this.backend.get(key);
    if (!encrypted) {
      return null;
    }
    try {
      const decrypted = await this.decrypt(
        encrypted,
        this.encryptionKey
      );
      return JSON.parse(decrypted);
    } catch (error) {
      console.error("Failed to decrypt value:", error);
      return null;
    }
  }
  async save(key, value, ttl) {
    const encrypted = await this.encrypt(
      JSON.stringify(value),
      this.encryptionKey
    );
    await this.backend.save(key, encrypted, ttl);
  }
  async decrypt(ciphertext, key) {
    const parts = ciphertext.split(":");
    if (parts.length !== 3) {
      throw new Error("Invalid encrypted data format");
    }
    const [ivHex, authTagHex, encrypted] = parts;
    const iv = Buffer.from(ivHex, "hex");
    const authTag = Buffer.from(authTagHex, "hex");
    const decipher = _crypto.createDecipheriv.call(void 0, this.algorithm, key, iv);
    decipher.setAuthTag(
      authTag
    );
    let decrypted = decipher.update(encrypted, "hex", "utf8");
    decrypted += decipher.final("utf8");
    return decrypted;
  }
  async encrypt(plaintext, key) {
    const iv = _crypto.randomBytes.call(void 0, 16);
    const cipher = _crypto.createCipheriv.call(void 0, this.algorithm, key, iv);
    let encrypted = cipher.update(plaintext, "utf8", "hex");
    encrypted += cipher.final("hex");
    const authTag = cipher.getAuthTag();
    return `${iv.toString("hex")}:${authTag.toString("hex")}:${encrypted}`;
  }
}, _class2);
var MemoryTokenStorage = (_class3 = class {
  __init3() {this.cleanupInterval = null}
  __init4() {this.store = /* @__PURE__ */ new Map()}
  constructor(cleanupIntervalMs = 6e4) {;_class3.prototype.__init3.call(this);_class3.prototype.__init4.call(this);
    this.cleanupInterval = setInterval(
      () => void this.cleanup(),
      cleanupIntervalMs
    );
  }
  async cleanup() {
    const now = Date.now();
    const keysToDelete = [];
    for (const [key, entry] of this.store.entries()) {
      if (entry.expiresAt < now) {
        keysToDelete.push(key);
      }
    }
    for (const key of keysToDelete) {
      this.store.delete(key);
    }
  }
  async delete(key) {
    this.store.delete(key);
  }
  /**
   * Destroy the storage and clear cleanup interval
   */
  destroy() {
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
      this.cleanupInterval = null;
    }
    this.store.clear();
  }
  async get(key) {
    const entry = this.store.get(key);
    if (!entry) {
      return null;
    }
    if (entry.expiresAt < Date.now()) {
      this.store.delete(key);
      return null;
    }
    return entry.value;
  }
  async save(key, value, ttl) {
    const expiresAt = ttl ? Date.now() + ttl * 1e3 : Number.MAX_SAFE_INTEGER;
    this.store.set(key, {
      expiresAt,
      value
    });
  }
  /**
   * Get the number of stored items
   */
  size() {
    return this.store.size;
  }
}, _class3);

// src/auth/OAuthProxy.ts
var OAuthProxy = (_class4 = class {
  __init5() {this.claimsExtractor = null}
  __init6() {this.cleanupInterval = null}
  __init7() {this.clientCodes = /* @__PURE__ */ new Map()}
  
  
  
  __init8() {this.registeredClients = /* @__PURE__ */ new Map()}
  
  __init9() {this.transactions = /* @__PURE__ */ new Map()}
  constructor(config) {;_class4.prototype.__init5.call(this);_class4.prototype.__init6.call(this);_class4.prototype.__init7.call(this);_class4.prototype.__init8.call(this);_class4.prototype.__init9.call(this);
    this.config = {
      allowedRedirectUriPatterns: ["https://*", "http://localhost:*"],
      authorizationCodeTtl: DEFAULT_AUTHORIZATION_CODE_TTL,
      consentRequired: true,
      enableTokenSwap: true,
      // Enabled by default for security
      redirectPath: "/oauth/callback",
      transactionTtl: DEFAULT_TRANSACTION_TTL,
      upstreamTokenEndpointAuthMethod: "client_secret_basic",
      ...config
    };
    let storage = config.tokenStorage || new MemoryTokenStorage();
    const isAlreadyEncrypted = storage.constructor.name === "EncryptedTokenStorage";
    if (!isAlreadyEncrypted && config.encryptionKey !== false) {
      const encryptionKey = typeof config.encryptionKey === "string" ? config.encryptionKey : this.generateSigningKey();
      storage = new EncryptedTokenStorage(storage, encryptionKey);
    }
    this.tokenStorage = storage;
    this.consentManager = new ConsentManager(
      config.consentSigningKey || this.generateSigningKey()
    );
    if (this.config.enableTokenSwap) {
      const signingKey = this.config.jwtSigningKey || this.generateSigningKey();
      this.jwtIssuer = new JWTIssuer({
        audience: this.config.baseUrl,
        issuer: this.config.baseUrl,
        signingKey
      });
    }
    const claimsConfig = config.customClaimsPassthrough !== void 0 ? config.customClaimsPassthrough : true;
    if (claimsConfig !== false) {
      this.claimsExtractor = new ClaimsExtractor(claimsConfig);
    }
    this.startCleanup();
  }
  /**
   * OAuth authorization endpoint
   */
  async authorize(params) {
    if (!params.client_id || !params.redirect_uri || !params.response_type) {
      throw new OAuthProxyError(
        "invalid_request",
        "Missing required parameters"
      );
    }
    if (params.response_type !== "code") {
      throw new OAuthProxyError(
        "unsupported_response_type",
        "Only 'code' response type is supported"
      );
    }
    if (params.code_challenge && !params.code_challenge_method) {
      throw new OAuthProxyError(
        "invalid_request",
        "code_challenge_method required when code_challenge is present"
      );
    }
    const transaction = await this.createTransaction(params);
    if (this.config.consentRequired && !transaction.consentGiven) {
      return this.consentManager.createConsentResponse(
        transaction,
        this.getProviderName()
      );
    }
    return this.redirectToUpstream(transaction);
  }
  /**
   * Stop cleanup interval and destroy resources
   */
  destroy() {
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
      this.cleanupInterval = null;
    }
    this.transactions.clear();
    this.clientCodes.clear();
    this.registeredClients.clear();
  }
  /**
   * Token endpoint - exchange authorization code for tokens
   */
  async exchangeAuthorizationCode(request) {
    if (request.grant_type !== "authorization_code") {
      throw new OAuthProxyError(
        "unsupported_grant_type",
        "Only authorization_code grant type is supported"
      );
    }
    const clientCode = this.clientCodes.get(request.code);
    if (!clientCode) {
      throw new OAuthProxyError(
        "invalid_grant",
        "Invalid or expired authorization code"
      );
    }
    if (clientCode.clientId !== request.client_id) {
      throw new OAuthProxyError("invalid_client", "Client ID mismatch");
    }
    if (clientCode.codeChallenge) {
      if (!request.code_verifier) {
        throw new OAuthProxyError(
          "invalid_request",
          "code_verifier required for PKCE"
        );
      }
      const valid = PKCEUtils.validateChallenge(
        request.code_verifier,
        clientCode.codeChallenge,
        clientCode.codeChallengeMethod
      );
      if (!valid) {
        throw new OAuthProxyError("invalid_grant", "Invalid PKCE verifier");
      }
    }
    if (clientCode.used) {
      throw new OAuthProxyError(
        "invalid_grant",
        "Authorization code already used"
      );
    }
    clientCode.used = true;
    this.clientCodes.set(request.code, clientCode);
    if (this.config.enableTokenSwap && this.jwtIssuer) {
      return await this.issueSwappedTokens(
        clientCode.clientId,
        clientCode.upstreamTokens
      );
    } else {
      const response = {
        access_token: clientCode.upstreamTokens.accessToken,
        expires_in: clientCode.upstreamTokens.expiresIn,
        token_type: clientCode.upstreamTokens.tokenType
      };
      if (clientCode.upstreamTokens.refreshToken) {
        response.refresh_token = clientCode.upstreamTokens.refreshToken;
      }
      if (clientCode.upstreamTokens.idToken) {
        response.id_token = clientCode.upstreamTokens.idToken;
      }
      if (clientCode.upstreamTokens.scope.length > 0) {
        response.scope = clientCode.upstreamTokens.scope.join(" ");
      }
      return response;
    }
  }
  /**
   * Token endpoint - refresh access token
   */
  async exchangeRefreshToken(request) {
    if (request.grant_type !== "refresh_token") {
      throw new OAuthProxyError(
        "unsupported_grant_type",
        "Only refresh_token grant type is supported"
      );
    }
    if (this.config.enableTokenSwap && this.jwtIssuer) {
      return await this.handleSwapModeRefresh(request);
    }
    return await this.handlePassthroughRefresh(request);
  }
  /**
   * Get OAuth discovery metadata
   */
  getAuthorizationServerMetadata() {
    return {
      authorizationEndpoint: `${this.config.baseUrl}/oauth/authorize`,
      codeChallengeMethodsSupported: ["S256", "plain"],
      grantTypesSupported: ["authorization_code", "refresh_token"],
      issuer: this.config.baseUrl,
      registrationEndpoint: `${this.config.baseUrl}/oauth/register`,
      responseTypesSupported: ["code"],
      scopesSupported: this.config.scopes || [],
      tokenEndpoint: `${this.config.baseUrl}/oauth/token`,
      tokenEndpointAuthMethodsSupported: [
        "client_secret_basic",
        "client_secret_post"
      ]
    };
  }
  /**
   * Handle OAuth callback from upstream provider
   */
  async handleCallback(request) {
    const url = new URL(request.url);
    const code = url.searchParams.get("code");
    const state = url.searchParams.get("state");
    const error = url.searchParams.get("error");
    if (error) {
      const errorDescription = url.searchParams.get("error_description");
      throw new OAuthProxyError(error, errorDescription || void 0);
    }
    if (!code || !state) {
      throw new OAuthProxyError(
        "invalid_request",
        "Missing code or state parameter"
      );
    }
    const transaction = this.transactions.get(state);
    if (!transaction) {
      throw new OAuthProxyError("invalid_request", "Invalid or expired state");
    }
    const upstreamTokens = await this.exchangeUpstreamCode(code, transaction);
    const clientCode = this.generateAuthorizationCode(
      transaction,
      upstreamTokens
    );
    this.transactions.delete(state);
    const redirectUrl = new URL(transaction.clientCallbackUrl);
    redirectUrl.searchParams.set("code", clientCode);
    redirectUrl.searchParams.set("state", transaction.state);
    return new Response(null, {
      headers: {
        Location: redirectUrl.toString()
      },
      status: 302
    });
  }
  /**
   * Handle consent form submission
   */
  async handleConsent(request) {
    const formData = await request.formData();
    const transactionId = formData.get("transaction_id");
    const action = formData.get("action");
    if (!transactionId) {
      throw new OAuthProxyError("invalid_request", "Missing transaction_id");
    }
    const transaction = this.transactions.get(transactionId);
    if (!transaction) {
      throw new OAuthProxyError(
        "invalid_request",
        "Invalid or expired transaction"
      );
    }
    if (action === "deny") {
      this.transactions.delete(transactionId);
      const redirectUrl = new URL(transaction.clientCallbackUrl);
      redirectUrl.searchParams.set("error", "access_denied");
      redirectUrl.searchParams.set(
        "error_description",
        "User denied authorization"
      );
      redirectUrl.searchParams.set("state", transaction.state);
      return new Response(null, {
        headers: {
          Location: redirectUrl.toString()
        },
        status: 302
      });
    }
    transaction.consentGiven = true;
    this.transactions.set(transactionId, transaction);
    return this.redirectToUpstream(transaction);
  }
  /**
   * Load upstream tokens from a FastMCP JWT
   */
  async loadUpstreamTokens(fastmcpToken) {
    if (!this.jwtIssuer) {
      return null;
    }
    const result = await this.jwtIssuer.verify(fastmcpToken);
    if (!result.valid || !_optionalChain([result, 'access', _5 => _5.claims, 'optionalAccess', _6 => _6.jti])) {
      return null;
    }
    const mapping = await this.tokenStorage.get(
      `mapping:${result.claims.jti}`
    );
    if (!mapping) {
      return null;
    }
    const upstreamTokens = await this.tokenStorage.get(
      `upstream:${mapping.upstreamTokenKey}`
    );
    return upstreamTokens;
  }
  /**
   * RFC 7591 Dynamic Client Registration
   */
  async registerClient(request) {
    if (!request.redirect_uris || request.redirect_uris.length === 0) {
      throw new OAuthProxyError(
        "invalid_client_metadata",
        "redirect_uris is required"
      );
    }
    for (const uri of request.redirect_uris) {
      if (!this.validateRedirectUri(uri)) {
        throw new OAuthProxyError(
          "invalid_redirect_uri",
          `Invalid redirect URI: ${uri}`
        );
      }
    }
    const clientId = this.config.upstreamClientId;
    const client = {
      callbackUrl: request.redirect_uris[0],
      clientId,
      clientSecret: this.config.upstreamClientSecret,
      metadata: {
        client_name: request.client_name,
        client_uri: request.client_uri,
        contacts: request.contacts,
        jwks: request.jwks,
        jwks_uri: request.jwks_uri,
        logo_uri: request.logo_uri,
        policy_uri: request.policy_uri,
        scope: request.scope,
        software_id: request.software_id,
        software_version: request.software_version,
        tos_uri: request.tos_uri
      },
      registeredAt: /* @__PURE__ */ new Date()
    };
    this.registeredClients.set(request.redirect_uris[0], client);
    const response = {
      client_id: clientId,
      client_id_issued_at: Math.floor(Date.now() / 1e3),
      // Echo back optional metadata
      client_name: request.client_name,
      client_secret: this.config.upstreamClientSecret,
      client_secret_expires_at: 0,
      // Never expires
      client_uri: request.client_uri,
      contacts: request.contacts,
      grant_types: request.grant_types || [
        "authorization_code",
        "refresh_token"
      ],
      jwks: request.jwks,
      jwks_uri: request.jwks_uri,
      logo_uri: request.logo_uri,
      policy_uri: request.policy_uri,
      redirect_uris: request.redirect_uris,
      response_types: request.response_types || ["code"],
      scope: request.scope,
      software_id: request.software_id,
      software_version: request.software_version,
      token_endpoint_auth_method: request.token_endpoint_auth_method || "client_secret_basic",
      tos_uri: request.tos_uri
    };
    return response;
  }
  /**
   * Calculate access token TTL from upstream tokens
   */
  calculateAccessTokenTtl(upstreamTokens) {
    if (upstreamTokens.expiresIn > 0) {
      return upstreamTokens.expiresIn;
    } else if (this.config.accessTokenTtl) {
      return this.config.accessTokenTtl;
    } else if (upstreamTokens.refreshToken) {
      return DEFAULT_ACCESS_TOKEN_TTL;
    } else {
      return DEFAULT_ACCESS_TOKEN_TTL_NO_REFRESH;
    }
  }
  /**
   * Clean up expired transactions and codes
   */
  cleanup() {
    const now = Date.now();
    for (const [id, transaction] of this.transactions.entries()) {
      if (transaction.expiresAt.getTime() < now) {
        this.transactions.delete(id);
      }
    }
    for (const [code, clientCode] of this.clientCodes.entries()) {
      if (clientCode.expiresAt.getTime() < now) {
        this.clientCodes.delete(code);
      }
    }
    void this.tokenStorage.cleanup();
  }
  /**
   * Create a new OAuth transaction
   */
  async createTransaction(params) {
    const transactionId = this.generateId();
    const proxyPkce = PKCEUtils.generatePair("S256");
    const transaction = {
      clientCallbackUrl: params.redirect_uri,
      clientCodeChallenge: params.code_challenge || "",
      clientCodeChallengeMethod: params.code_challenge_method || "plain",
      clientId: params.client_id,
      createdAt: /* @__PURE__ */ new Date(),
      expiresAt: new Date(
        Date.now() + (this.config.transactionTtl || 600) * 1e3
      ),
      id: transactionId,
      proxyCodeChallenge: proxyPkce.challenge,
      proxyCodeVerifier: proxyPkce.verifier,
      scope: params.scope ? params.scope.split(" ") : this.config.scopes || [],
      state: params.state || this.generateId()
    };
    this.transactions.set(transactionId, transaction);
    return transaction;
  }
  /**
   * Exchange authorization code with upstream provider
   */
  async exchangeUpstreamCode(code, transaction) {
    const useBasicAuth = this.config.upstreamTokenEndpointAuthMethod === "client_secret_basic";
    const bodyParams = {
      code,
      code_verifier: transaction.proxyCodeVerifier,
      grant_type: "authorization_code",
      redirect_uri: `${this.config.baseUrl}${this.config.redirectPath}`
    };
    if (!useBasicAuth) {
      bodyParams.client_id = this.config.upstreamClientId;
      bodyParams.client_secret = this.config.upstreamClientSecret;
    }
    const headers = {
      "Content-Type": "application/x-www-form-urlencoded"
    };
    if (useBasicAuth) {
      headers["Authorization"] = this.getBasicAuthHeader();
    }
    const tokenResponse = await fetch(this.config.upstreamTokenEndpoint, {
      body: new URLSearchParams(bodyParams),
      headers,
      method: "POST"
    });
    if (!tokenResponse.ok) {
      const error = await tokenResponse.json();
      throw new OAuthProxyError(
        error.error || "server_error",
        error.error_description
      );
    }
    const tokens = await this.parseTokenResponse(tokenResponse);
    return {
      accessToken: tokens.access_token,
      expiresIn: tokens.expires_in || 3600,
      idToken: tokens.id_token,
      issuedAt: /* @__PURE__ */ new Date(),
      refreshExpiresIn: tokens.refresh_expires_in,
      refreshToken: tokens.refresh_token,
      scope: tokens.scope ? tokens.scope.split(" ") : transaction.scope,
      tokenType: tokens.token_type || "Bearer"
    };
  }
  /**
   * Extract JTI from a JWT token
   */
  async extractJti(token) {
    if (!this.jwtIssuer) {
      throw new Error("JWT issuer not initialized");
    }
    const result = await this.jwtIssuer.verify(token);
    if (!result.valid || !_optionalChain([result, 'access', _7 => _7.claims, 'optionalAccess', _8 => _8.jti])) {
      throw new Error("Failed to extract JTI from token");
    }
    return result.claims.jti;
  }
  /**
   * Extract custom claims from upstream tokens
   * Combines claims from access token and ID token (if present)
   */
  async extractUpstreamClaims(upstreamTokens) {
    if (!this.claimsExtractor) {
      return null;
    }
    const allClaims = {};
    const accessClaims = await this.claimsExtractor.extract(
      upstreamTokens.accessToken,
      "access"
    );
    if (accessClaims) {
      Object.assign(allClaims, accessClaims);
    }
    if (upstreamTokens.idToken) {
      const idClaims = await this.claimsExtractor.extract(
        upstreamTokens.idToken,
        "id"
      );
      if (idClaims) {
        for (const [key, value] of Object.entries(idClaims)) {
          if (!(key in allClaims)) {
            allClaims[key] = value;
          }
        }
      }
    }
    return Object.keys(allClaims).length > 0 ? allClaims : null;
  }
  /**
   * Generate authorization code for client
   */
  generateAuthorizationCode(transaction, upstreamTokens) {
    const code = this.generateId();
    const clientCode = {
      clientId: transaction.clientId,
      code,
      codeChallenge: transaction.clientCodeChallenge,
      codeChallengeMethod: transaction.clientCodeChallengeMethod,
      createdAt: /* @__PURE__ */ new Date(),
      expiresAt: new Date(
        Date.now() + (this.config.authorizationCodeTtl || 300) * 1e3
      ),
      transactionId: transaction.id,
      upstreamTokens
    };
    this.clientCodes.set(code, clientCode);
    return code;
  }
  /**
   * Generate secure random ID
   */
  generateId() {
    return _crypto.randomBytes.call(void 0, 32).toString("base64url");
  }
  /**
   * Generate signing key for consent cookies
   */
  generateSigningKey() {
    return _crypto.randomBytes.call(void 0, 32).toString("hex");
  }
  /**
   * Generate Basic auth header value for upstream token endpoint
   * Per RFC 6749 Section 2.3.1, credentials must be URL-encoded before base64 encoding
   */
  getBasicAuthHeader() {
    const encodedClientId = encodeURIComponent(this.config.upstreamClientId);
    const encodedClientSecret = encodeURIComponent(
      this.config.upstreamClientSecret
    );
    return `Basic ${Buffer.from(`${encodedClientId}:${encodedClientSecret}`).toString("base64")}`;
  }
  /**
   * Get provider name for display
   */
  getProviderName() {
    const url = new URL(this.config.upstreamAuthorizationEndpoint);
    return url.hostname;
  }
  /**
   * Handle passthrough mode refresh - forward refresh token directly to upstream
   */
  async handlePassthroughRefresh(request) {
    const useBasicAuth = this.config.upstreamTokenEndpointAuthMethod === "client_secret_basic";
    const bodyParams = {
      grant_type: "refresh_token",
      refresh_token: request.refresh_token,
      ...request.scope && { scope: request.scope }
    };
    if (!useBasicAuth) {
      bodyParams.client_id = this.config.upstreamClientId;
      bodyParams.client_secret = this.config.upstreamClientSecret;
    }
    const headers = {
      "Content-Type": "application/x-www-form-urlencoded"
    };
    if (useBasicAuth) {
      headers["Authorization"] = this.getBasicAuthHeader();
    }
    const tokenResponse = await fetch(this.config.upstreamTokenEndpoint, {
      body: new URLSearchParams(bodyParams),
      headers,
      method: "POST"
    });
    if (!tokenResponse.ok) {
      const error = await tokenResponse.json();
      throw new OAuthProxyError(
        error.error || "invalid_grant",
        error.error_description
      );
    }
    const tokens = await this.parseTokenResponse(tokenResponse);
    return {
      access_token: tokens.access_token,
      expires_in: tokens.expires_in || 3600,
      id_token: tokens.id_token,
      refresh_token: tokens.refresh_token,
      scope: tokens.scope,
      token_type: tokens.token_type || "Bearer"
    };
  }
  /**
   * Handle swap mode refresh - verify FastMCP JWT and issue new tokens
   */
  async handleSwapModeRefresh(request) {
    if (!this.jwtIssuer) {
      throw new Error("JWT issuer not initialized");
    }
    const verifyResult = await this.jwtIssuer.verify(request.refresh_token);
    if (!verifyResult.valid) {
      throw new OAuthProxyError(
        "invalid_grant",
        "Invalid or expired refresh token"
      );
    }
    const jti = _optionalChain([verifyResult, 'access', _9 => _9.claims, 'optionalAccess', _10 => _10.jti]);
    if (!jti) {
      throw new OAuthProxyError("invalid_grant", "Refresh token missing JTI");
    }
    const mapping = await this.tokenStorage.get(`mapping:${jti}`);
    if (!mapping) {
      throw new OAuthProxyError(
        "invalid_grant",
        "Refresh token already used or expired"
      );
    }
    const upstreamTokens = await this.tokenStorage.get(
      `upstream:${mapping.upstreamTokenKey}`
    );
    if (!upstreamTokens) {
      throw new OAuthProxyError(
        "invalid_grant",
        "Upstream tokens not found or expired"
      );
    }
    if (!upstreamTokens.refreshToken) {
      throw new OAuthProxyError(
        "invalid_grant",
        "No upstream refresh token available"
      );
    }
    const refreshedUpstreamTokens = await this.refreshUpstreamTokens(
      upstreamTokens.refreshToken,
      request.scope
    );
    if (refreshedUpstreamTokens.scope.length === 0) {
      refreshedUpstreamTokens.scope = upstreamTokens.scope;
    }
    const refreshTokenTtl = _nullishCoalesce(_nullishCoalesce(refreshedUpstreamTokens.refreshExpiresIn, () => ( this.config.refreshTokenTtl)), () => ( DEFAULT_REFRESH_TOKEN_TTL));
    const accessTokenTtl = this.calculateAccessTokenTtl(
      refreshedUpstreamTokens
    );
    const upstreamStorageTtl = Math.max(accessTokenTtl, refreshTokenTtl, 1);
    await this.tokenStorage.save(
      `upstream:${mapping.upstreamTokenKey}`,
      refreshedUpstreamTokens,
      upstreamStorageTtl
    );
    return await this.issueSwappedTokensForRefresh(
      mapping.clientId,
      refreshedUpstreamTokens,
      mapping.upstreamTokenKey,
      jti
    );
  }
  /**
   * Issue swapped tokens (JWT pattern)
   * Issues short-lived FastMCP JWTs and stores upstream tokens securely
   */
  async issueSwappedTokens(clientId, upstreamTokens) {
    if (!this.jwtIssuer) {
      throw new Error("JWT issuer not initialized");
    }
    const customClaims = await this.extractUpstreamClaims(upstreamTokens);
    let accessTokenTtl;
    if (upstreamTokens.expiresIn > 0) {
      accessTokenTtl = upstreamTokens.expiresIn;
    } else if (this.config.accessTokenTtl) {
      accessTokenTtl = this.config.accessTokenTtl;
    } else if (upstreamTokens.refreshToken) {
      accessTokenTtl = DEFAULT_ACCESS_TOKEN_TTL;
    } else {
      accessTokenTtl = DEFAULT_ACCESS_TOKEN_TTL_NO_REFRESH;
    }
    const refreshTokenTtl = upstreamTokens.refreshToken ? _nullishCoalesce(_nullishCoalesce(upstreamTokens.refreshExpiresIn, () => ( this.config.refreshTokenTtl)), () => ( DEFAULT_REFRESH_TOKEN_TTL)) : 0;
    const upstreamStorageTtl = Math.max(accessTokenTtl, refreshTokenTtl, 1);
    const upstreamTokenKey = this.generateId();
    await this.tokenStorage.save(
      `upstream:${upstreamTokenKey}`,
      upstreamTokens,
      upstreamStorageTtl
    );
    const accessToken = this.jwtIssuer.issueAccessToken(
      clientId,
      upstreamTokens.scope,
      customClaims || void 0,
      accessTokenTtl
    );
    const accessJti = await this.extractJti(accessToken);
    await this.tokenStorage.save(
      `mapping:${accessJti}`,
      {
        clientId,
        createdAt: /* @__PURE__ */ new Date(),
        expiresAt: new Date(Date.now() + accessTokenTtl * 1e3),
        jti: accessJti,
        scope: upstreamTokens.scope,
        upstreamTokenKey
      },
      accessTokenTtl
    );
    const response = {
      access_token: accessToken,
      expires_in: accessTokenTtl,
      scope: upstreamTokens.scope.join(" "),
      token_type: "Bearer"
    };
    if (upstreamTokens.refreshToken) {
      const refreshToken = this.jwtIssuer.issueRefreshToken(
        clientId,
        upstreamTokens.scope,
        customClaims || void 0,
        refreshTokenTtl
      );
      const refreshJti = await this.extractJti(refreshToken);
      await this.tokenStorage.save(
        `mapping:${refreshJti}`,
        {
          clientId,
          createdAt: /* @__PURE__ */ new Date(),
          expiresAt: new Date(Date.now() + refreshTokenTtl * 1e3),
          jti: refreshJti,
          scope: upstreamTokens.scope,
          upstreamTokenKey
        },
        refreshTokenTtl
      );
      response.refresh_token = refreshToken;
    }
    return response;
  }
  /**
   * Issue swapped tokens for refresh flow
   */
  async issueSwappedTokensForRefresh(clientId, upstreamTokens, upstreamTokenKey, oldJti) {
    if (!this.jwtIssuer) {
      throw new Error("JWT issuer not initialized");
    }
    await this.tokenStorage.delete(`mapping:${oldJti}`);
    const customClaims = await this.extractUpstreamClaims(upstreamTokens);
    const accessTokenTtl = this.calculateAccessTokenTtl(upstreamTokens);
    const refreshTokenTtl = upstreamTokens.refreshToken ? _nullishCoalesce(_nullishCoalesce(upstreamTokens.refreshExpiresIn, () => ( this.config.refreshTokenTtl)), () => ( DEFAULT_REFRESH_TOKEN_TTL)) : 0;
    const accessToken = this.jwtIssuer.issueAccessToken(
      clientId,
      upstreamTokens.scope,
      customClaims || void 0,
      accessTokenTtl
    );
    const accessJti = await this.extractJti(accessToken);
    await this.tokenStorage.save(
      `mapping:${accessJti}`,
      {
        clientId,
        createdAt: /* @__PURE__ */ new Date(),
        expiresAt: new Date(Date.now() + accessTokenTtl * 1e3),
        jti: accessJti,
        scope: upstreamTokens.scope,
        upstreamTokenKey
      },
      accessTokenTtl
    );
    const response = {
      access_token: accessToken,
      expires_in: accessTokenTtl,
      scope: upstreamTokens.scope.join(" "),
      token_type: "Bearer"
    };
    if (upstreamTokens.refreshToken) {
      const refreshToken = this.jwtIssuer.issueRefreshToken(
        clientId,
        upstreamTokens.scope,
        customClaims || void 0,
        refreshTokenTtl
      );
      const refreshJti = await this.extractJti(refreshToken);
      await this.tokenStorage.save(
        `mapping:${refreshJti}`,
        {
          clientId,
          createdAt: /* @__PURE__ */ new Date(),
          expiresAt: new Date(Date.now() + refreshTokenTtl * 1e3),
          jti: refreshJti,
          scope: upstreamTokens.scope,
          upstreamTokenKey
        },
        refreshTokenTtl
      );
      response.refresh_token = refreshToken;
    }
    return response;
  }
  /**
   * Match URI against pattern (supports wildcards)
   */
  matchesPattern(uri, pattern) {
    const regex = new RegExp(
      "^" + pattern.replace(/\*/g, ".*").replace(/\?/g, ".") + "$"
    );
    return regex.test(uri);
  }
  /**
   * Parse token response that can be either JSON or URL-encoded
   * GitHub Apps return URL-encoded format, most providers return JSON
   */
  async parseTokenResponse(response) {
    const contentType = (response.headers.get("content-type") || "").toLowerCase();
    const tokenResponseSchema = _zod.z.object({
      access_token: _zod.z.string().min(1, "access_token cannot be empty"),
      expires_in: _zod.z.coerce.number().int().positive().optional(),
      id_token: _zod.z.string().optional(),
      refresh_expires_in: _zod.z.coerce.number().int().positive().optional(),
      refresh_token: _zod.z.string().optional(),
      scope: _zod.z.string().optional(),
      token_type: _zod.z.string().optional()
    });
    if (contentType.includes("application/x-www-form-urlencoded")) {
      const text = await response.text();
      const params = new URLSearchParams(text);
      const rawData = {
        access_token: params.get("access_token") || "",
        expires_in: params.get("expires_in") ? parseInt(params.get("expires_in")) : void 0,
        id_token: params.get("id_token") || void 0,
        refresh_expires_in: params.get("refresh_expires_in") ? parseInt(params.get("refresh_expires_in")) : void 0,
        refresh_token: params.get("refresh_token") || void 0,
        scope: params.get("scope") || void 0,
        token_type: params.get("token_type") || void 0
      };
      return tokenResponseSchema.parse(rawData);
    }
    const rawJson = await response.json();
    return tokenResponseSchema.parse(rawJson);
  }
  /**
   * Redirect to upstream OAuth provider
   */
  redirectToUpstream(transaction) {
    const authUrl = new URL(this.config.upstreamAuthorizationEndpoint);
    authUrl.searchParams.set("client_id", this.config.upstreamClientId);
    authUrl.searchParams.set(
      "redirect_uri",
      `${this.config.baseUrl}${this.config.redirectPath}`
    );
    authUrl.searchParams.set("response_type", "code");
    authUrl.searchParams.set("state", transaction.id);
    if (transaction.scope.length > 0) {
      authUrl.searchParams.set("scope", transaction.scope.join(" "));
    }
    if (!this.config.forwardPkce) {
      authUrl.searchParams.set(
        "code_challenge",
        transaction.proxyCodeChallenge
      );
      authUrl.searchParams.set("code_challenge_method", "S256");
    }
    return new Response(null, {
      headers: {
        Location: authUrl.toString()
      },
      status: 302
    });
  }
  /**
   * Refresh upstream tokens with provider
   */
  async refreshUpstreamTokens(upstreamRefreshToken, requestedScope) {
    const useBasicAuth = this.config.upstreamTokenEndpointAuthMethod === "client_secret_basic";
    const bodyParams = {
      grant_type: "refresh_token",
      refresh_token: upstreamRefreshToken,
      ...requestedScope && { scope: requestedScope }
    };
    if (!useBasicAuth) {
      bodyParams.client_id = this.config.upstreamClientId;
      bodyParams.client_secret = this.config.upstreamClientSecret;
    }
    const headers = {
      "Content-Type": "application/x-www-form-urlencoded"
    };
    if (useBasicAuth) {
      headers["Authorization"] = this.getBasicAuthHeader();
    }
    const tokenResponse = await fetch(this.config.upstreamTokenEndpoint, {
      body: new URLSearchParams(bodyParams),
      headers,
      method: "POST"
    });
    if (!tokenResponse.ok) {
      const error = await tokenResponse.json();
      throw new OAuthProxyError(
        error.error || "invalid_grant",
        error.error_description || "Upstream refresh failed"
      );
    }
    const tokens = await this.parseTokenResponse(tokenResponse);
    return {
      accessToken: tokens.access_token,
      expiresIn: tokens.expires_in || 3600,
      idToken: tokens.id_token,
      issuedAt: /* @__PURE__ */ new Date(),
      refreshExpiresIn: tokens.refresh_expires_in,
      refreshToken: tokens.refresh_token || upstreamRefreshToken,
      scope: tokens.scope ? tokens.scope.split(" ") : [],
      tokenType: tokens.token_type || "Bearer"
    };
  }
  /**
   * Start periodic cleanup of expired transactions and codes
   */
  startCleanup() {
    this.cleanupInterval = setInterval(() => {
      this.cleanup();
    }, 6e4);
  }
  /**
   * Validate redirect URI against allowed patterns
   */
  validateRedirectUri(uri) {
    try {
      const url = new URL(uri);
      const patterns = this.config.allowedRedirectUriPatterns || [];
      for (const pattern of patterns) {
        if (this.matchesPattern(uri, pattern)) {
          return true;
        }
      }
      return url.protocol === "https:" || url.hostname === "localhost" || url.hostname === "127.0.0.1";
    } catch (e3) {
      return false;
    }
  }
}, _class4);
var OAuthProxyError = class extends Error {
  constructor(code, description, statusCode = 400) {
    super(code);
    this.code = code;
    this.description = description;
    this.statusCode = statusCode;
    this.name = "OAuthProxyError";
  }
  toJSON() {
    return {
      error: this.code,
      error_description: this.description
    };
  }
  toResponse() {
    return new Response(JSON.stringify(this.toJSON()), {
      headers: { "Content-Type": "application/json" },
      status: this.statusCode
    });
  }
};

// src/auth/providers/AuthProvider.ts
var AuthProvider = class {
  
  /**
   * Get the proxy, creating it lazily if needed.
   */
  get proxy() {
    if (!this._proxy) {
      this._proxy = this.createProxy();
    }
    return this._proxy;
  }
  
  constructor(config) {
    this.config = config;
  }
  /**
   * Authenticate function to be used by FastMCP.
   * Extracts Bearer token, validates it, and returns session with upstream access token.
   */
  async authenticate(request) {
    if (!request) {
      return void 0;
    }
    const authHeader = _optionalChain([request, 'access', _11 => _11.headers, 'optionalAccess', _12 => _12.authorization]);
    if (!authHeader || !authHeader.startsWith("Bearer ")) {
      return void 0;
    }
    const token = authHeader.slice(7);
    const upstreamTokens = await this.proxy.loadUpstreamTokens(token);
    if (!upstreamTokens) {
      return void 0;
    }
    return this.createSession(upstreamTokens);
  }
  /**
   * Get the OAuth configuration object for FastMCP ServerOptions.
   */
  getOAuthConfig() {
    return {
      authorizationServer: this.proxy.getAuthorizationServerMetadata(),
      enabled: true,
      protectedResource: {
        authorizationServers: [this.config.baseUrl],
        resource: this.config.baseUrl,
        scopesSupported: _nullishCoalesce(this.config.scopes, () => ( this.getDefaultScopes()))
      },
      proxy: this.proxy
    };
  }
  /**
   * Get the OAuthProxy instance (for advanced use cases).
   */
  getProxy() {
    return this.proxy;
  }
  /**
   * Create a session object from upstream tokens.
   * Override in subclasses to add provider-specific session data.
   */
  createSession(upstreamTokens) {
    return {
      accessToken: upstreamTokens.accessToken,
      expiresAt: upstreamTokens.expiresIn ? Math.floor(Date.now() / 1e3) + upstreamTokens.expiresIn : void 0,
      idToken: upstreamTokens.idToken,
      refreshToken: upstreamTokens.refreshToken,
      scopes: upstreamTokens.scope
    };
  }
};

// src/auth/providers/AzureProvider.ts
var AzureProvider = class extends AuthProvider {
  
  constructor(config) {
    super(config);
    this.tenantId = _nullishCoalesce(config.tenantId, () => ( "common"));
  }
  createProxy() {
    return new OAuthProxy({
      allowedRedirectUriPatterns: _nullishCoalesce(this.config.allowedRedirectUriPatterns, () => ( [
        "http://localhost:*",
        "https://*"
      ])),
      baseUrl: this.config.baseUrl,
      consentRequired: _nullishCoalesce(this.config.consentRequired, () => ( true)),
      encryptionKey: this.config.encryptionKey,
      jwtSigningKey: this.config.jwtSigningKey,
      scopes: _nullishCoalesce(this.config.scopes, () => ( this.getDefaultScopes())),
      tokenStorage: this.config.tokenStorage,
      upstreamAuthorizationEndpoint: this.getAuthorizationEndpoint(),
      upstreamClientId: this.config.clientId,
      upstreamClientSecret: this.config.clientSecret,
      upstreamTokenEndpoint: this.getTokenEndpoint()
    });
  }
  getAuthorizationEndpoint() {
    return `https://login.microsoftonline.com/${this.tenantId}/oauth2/v2.0/authorize`;
  }
  getDefaultScopes() {
    return ["openid", "profile", "email"];
  }
  getTokenEndpoint() {
    return `https://login.microsoftonline.com/${this.tenantId}/oauth2/v2.0/token`;
  }
};

// src/auth/providers/GitHubProvider.ts
var GitHubProvider = class extends AuthProvider {
  constructor(config) {
    super(config);
  }
  createProxy() {
    return new OAuthProxy({
      allowedRedirectUriPatterns: _nullishCoalesce(this.config.allowedRedirectUriPatterns, () => ( [
        "http://localhost:*",
        "https://*"
      ])),
      baseUrl: this.config.baseUrl,
      consentRequired: _nullishCoalesce(this.config.consentRequired, () => ( true)),
      encryptionKey: this.config.encryptionKey,
      jwtSigningKey: this.config.jwtSigningKey,
      scopes: _nullishCoalesce(this.config.scopes, () => ( this.getDefaultScopes())),
      tokenStorage: this.config.tokenStorage,
      upstreamAuthorizationEndpoint: this.getAuthorizationEndpoint(),
      upstreamClientId: this.config.clientId,
      upstreamClientSecret: this.config.clientSecret,
      upstreamTokenEndpoint: this.getTokenEndpoint()
    });
  }
  getAuthorizationEndpoint() {
    return "https://github.com/login/oauth/authorize";
  }
  getDefaultScopes() {
    return ["read:user", "user:email"];
  }
  getTokenEndpoint() {
    return "https://github.com/login/oauth/access_token";
  }
};

// src/auth/providers/GoogleProvider.ts
var GoogleProvider = class extends AuthProvider {
  constructor(config) {
    super(config);
  }
  createProxy() {
    return new OAuthProxy({
      allowedRedirectUriPatterns: _nullishCoalesce(this.config.allowedRedirectUriPatterns, () => ( [
        "http://localhost:*",
        "https://*"
      ])),
      baseUrl: this.config.baseUrl,
      consentRequired: _nullishCoalesce(this.config.consentRequired, () => ( true)),
      encryptionKey: this.config.encryptionKey,
      jwtSigningKey: this.config.jwtSigningKey,
      scopes: _nullishCoalesce(this.config.scopes, () => ( this.getDefaultScopes())),
      tokenStorage: this.config.tokenStorage,
      upstreamAuthorizationEndpoint: this.getAuthorizationEndpoint(),
      upstreamClientId: this.config.clientId,
      upstreamClientSecret: this.config.clientSecret,
      upstreamTokenEndpoint: this.getTokenEndpoint()
    });
  }
  getAuthorizationEndpoint() {
    return "https://accounts.google.com/o/oauth2/v2/auth";
  }
  getDefaultScopes() {
    return ["openid", "profile", "email"];
  }
  getTokenEndpoint() {
    return "https://oauth2.googleapis.com/token";
  }
};

// src/auth/providers/OAuthProvider.ts
var OAuthProvider = class extends AuthProvider {
  
  constructor(config) {
    super(config);
    this.genericConfig = config;
  }
  createProxy() {
    return new OAuthProxy({
      allowedRedirectUriPatterns: _nullishCoalesce(this.config.allowedRedirectUriPatterns, () => ( [
        "http://localhost:*",
        "https://*"
      ])),
      baseUrl: this.config.baseUrl,
      consentRequired: _nullishCoalesce(this.config.consentRequired, () => ( true)),
      encryptionKey: this.config.encryptionKey,
      jwtSigningKey: this.config.jwtSigningKey,
      scopes: _nullishCoalesce(this.config.scopes, () => ( this.getDefaultScopes())),
      tokenStorage: this.config.tokenStorage,
      upstreamAuthorizationEndpoint: this.getAuthorizationEndpoint(),
      upstreamClientId: this.config.clientId,
      upstreamClientSecret: this.config.clientSecret,
      upstreamTokenEndpoint: this.getTokenEndpoint(),
      upstreamTokenEndpointAuthMethod: _nullishCoalesce(this.genericConfig.tokenEndpointAuthMethod, () => ( "client_secret_basic"))
    });
  }
  getAuthorizationEndpoint() {
    return this.genericConfig.authorizationEndpoint;
  }
  getDefaultScopes() {
    return ["openid"];
  }
  getTokenEndpoint() {
    return this.genericConfig.tokenEndpoint;
  }
};

// src/auth/utils/diskStore.ts
var _promises = require('fs/promises');
var _path = require('path');
var DiskStore = (_class5 = class {
  __init10() {this.cleanupInterval = null}
  
  
  constructor(options) {;_class5.prototype.__init10.call(this);
    this.directory = options.directory;
    this.fileExtension = options.fileExtension || ".json";
    void this.ensureDirectory();
    const cleanupIntervalMs = options.cleanupIntervalMs || 6e4;
    this.cleanupInterval = setInterval(() => {
      void this.cleanup();
    }, cleanupIntervalMs);
  }
  /**
   * Clean up expired entries
   */
  async cleanup() {
    try {
      await this.ensureDirectory();
      const files = await _promises.readdir.call(void 0, this.directory);
      const now = Date.now();
      for (const file of files) {
        if (!file.endsWith(this.fileExtension)) {
          continue;
        }
        try {
          const filePath = _path.join.call(void 0, this.directory, file);
          const content = await _promises.readFile.call(void 0, filePath, "utf-8");
          const entry = JSON.parse(content);
          if (entry.expiresAt < now) {
            await _promises.rm.call(void 0, filePath);
          }
        } catch (error) {
          console.warn(`Failed to read/parse file ${file}, deleting:`, error);
          try {
            await _promises.rm.call(void 0, _path.join.call(void 0, this.directory, file));
          } catch (e4) {
          }
        }
      }
    } catch (error) {
      console.error("Cleanup failed:", error);
    }
  }
  /**
   * Delete a value
   */
  async delete(key) {
    const filePath = this.getFilePath(key);
    try {
      await _promises.rm.call(void 0, filePath);
    } catch (error) {
      if (error.code !== "ENOENT") {
        console.error(`Failed to delete key ${key}:`, error);
      }
    }
  }
  /**
   * Destroy the storage and clear cleanup interval
   */
  destroy() {
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
      this.cleanupInterval = null;
    }
  }
  /**
   * Retrieve a value
   */
  async get(key) {
    const filePath = this.getFilePath(key);
    try {
      const content = await _promises.readFile.call(void 0, filePath, "utf-8");
      const entry = JSON.parse(content);
      if (entry.expiresAt < Date.now()) {
        await _promises.rm.call(void 0, filePath);
        return null;
      }
      return entry.value;
    } catch (error) {
      if (error.code === "ENOENT") {
        return null;
      }
      console.error(`Failed to read key ${key}:`, error);
      return null;
    }
  }
  /**
   * Save a value with optional TTL
   */
  async save(key, value, ttl) {
    await this.ensureDirectory();
    const filePath = this.getFilePath(key);
    const expiresAt = ttl ? Date.now() + ttl * 1e3 : Number.MAX_SAFE_INTEGER;
    const entry = {
      expiresAt,
      value
    };
    try {
      await _promises.writeFile.call(void 0, filePath, JSON.stringify(entry, null, 2), "utf-8");
    } catch (error) {
      console.error(`Failed to save key ${key}:`, error);
      throw error;
    }
  }
  /**
   * Get the number of stored items
   */
  async size() {
    try {
      await this.ensureDirectory();
      const files = await _promises.readdir.call(void 0, this.directory);
      return files.filter((f) => f.endsWith(this.fileExtension)).length;
    } catch (e5) {
      return 0;
    }
  }
  /**
   * Ensure storage directory exists
   */
  async ensureDirectory() {
    try {
      const stats = await _promises.stat.call(void 0, this.directory);
      if (!stats.isDirectory()) {
        throw new Error(`Path ${this.directory} exists but is not a directory`);
      }
    } catch (error) {
      if (error.code === "ENOENT") {
        await _promises.mkdir.call(void 0, this.directory, { recursive: true });
      } else {
        throw error;
      }
    }
  }
  /**
   * Get file path for a key
   */
  getFilePath(key) {
    const sanitizedKey = key.replace(/[^a-zA-Z0-9_-]/g, "_");
    return _path.join.call(void 0, this.directory, `${sanitizedKey}${this.fileExtension}`);
  }
}, _class5);

// src/auth/utils/jwks.ts
var JWKSVerifier = (_class6 = class {
  
  
  __init11() {this.joseLoaded = false}
  
  constructor(config) {;_class6.prototype.__init11.call(this);
    this.config = {
      cacheDuration: 36e5,
      // 1 hour
      cooldownDuration: 3e4,
      // 30 seconds
      ...config,
      audience: config.audience || "",
      issuer: config.issuer || ""
    };
  }
  /**
   * Get the JWKS URI being used
   */
  getJwksUri() {
    return this.config.jwksUri;
  }
  /**
   * Refresh the JWKS cache
   * Useful if you need to force a key refresh
   */
  async refreshKeys() {
    await this.loadJose();
    this.jwksCache = this.jose.createRemoteJWKSet(
      new URL(this.config.jwksUri),
      {
        cacheMaxAge: this.config.cacheDuration,
        cooldownDuration: this.config.cooldownDuration
      }
    );
  }
  /**
   * Verify a JWT token using JWKS
   *
   * @param token - The JWT token to verify
   * @returns Verification result with claims if valid
   *
   * @example
   * ```typescript
   * const result = await verifier.verify(token);
   * if (result.valid) {
   *   console.log('User:', result.claims?.client_id);
   * } else {
   *   console.error('Invalid token:', result.error);
   * }
   * ```
   */
  async verify(token) {
    try {
      await this.loadJose();
      const verifyOptions = {};
      if (this.config.audience) {
        verifyOptions.audience = this.config.audience;
      }
      if (this.config.issuer) {
        verifyOptions.issuer = this.config.issuer;
      }
      const { payload } = await this.jose.jwtVerify(
        token,
        this.jwksCache,
        verifyOptions
      );
      const claims = {
        aud: payload.aud,
        client_id: payload.client_id || payload.sub,
        exp: payload.exp,
        iat: payload.iat,
        iss: payload.iss,
        jti: payload.jti || "",
        scope: this.parseScope(payload.scope),
        ...payload
        // Include all other claims
      };
      return {
        claims,
        valid: true
      };
    } catch (error) {
      return {
        error: error.message || "Token verification failed",
        valid: false
      };
    }
  }
  /**
   * Lazy load the jose library
   * Only loads when verification is first attempted
   */
  async loadJose() {
    if (this.joseLoaded) {
      return;
    }
    try {
      this.jose = await Promise.resolve().then(() => _interopRequireWildcard(require("jose")));
      this.joseLoaded = true;
      this.jwksCache = this.jose.createRemoteJWKSet(
        new URL(this.config.jwksUri),
        {
          cacheMaxAge: this.config.cacheDuration,
          cooldownDuration: this.config.cooldownDuration
        }
      );
    } catch (error) {
      throw new Error(
        `JWKS verification requires the 'jose' package.
Install it with: npm install jose

If you don't need JWKS support, use HS256 signing instead (default).

Original error: ${error.message}`
      );
    }
  }
  /**
   * Parse scope from token payload
   * Handles both string (space-separated) and array formats
   */
  parseScope(scope) {
    if (!scope) {
      return [];
    }
    if (typeof scope === "string") {
      return scope.split(" ").filter(Boolean);
    }
    if (Array.isArray(scope)) {
      return scope;
    }
    return [];
  }
}, _class6);



























exports.getAuthSession = getAuthSession; exports.requireAll = requireAll; exports.requireAny = requireAny; exports.requireAuth = requireAuth; exports.requireRole = requireRole; exports.requireScopes = requireScopes; exports.DEFAULT_ACCESS_TOKEN_TTL = DEFAULT_ACCESS_TOKEN_TTL; exports.DEFAULT_ACCESS_TOKEN_TTL_NO_REFRESH = DEFAULT_ACCESS_TOKEN_TTL_NO_REFRESH; exports.DEFAULT_REFRESH_TOKEN_TTL = DEFAULT_REFRESH_TOKEN_TTL; exports.DEFAULT_AUTHORIZATION_CODE_TTL = DEFAULT_AUTHORIZATION_CODE_TTL; exports.DEFAULT_TRANSACTION_TTL = DEFAULT_TRANSACTION_TTL; exports.ConsentManager = ConsentManager; exports.JWTIssuer = JWTIssuer; exports.PKCEUtils = PKCEUtils; exports.EncryptedTokenStorage = EncryptedTokenStorage; exports.MemoryTokenStorage = MemoryTokenStorage; exports.OAuthProxy = OAuthProxy; exports.OAuthProxyError = OAuthProxyError; exports.AuthProvider = AuthProvider; exports.AzureProvider = AzureProvider; exports.GitHubProvider = GitHubProvider; exports.GoogleProvider = GoogleProvider; exports.OAuthProvider = OAuthProvider; exports.DiskStore = DiskStore; exports.JWKSVerifier = JWKSVerifier;
//# sourceMappingURL=chunk-7UDY4VFQ.cjs.map