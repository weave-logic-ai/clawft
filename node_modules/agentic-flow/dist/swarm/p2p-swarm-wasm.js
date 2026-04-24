/**
 * P2P Swarm V2 WASM Integration
 *
 * Provides WASM-accelerated components for P2P Swarm:
 * - Ed25519 identity (faster key generation)
 * - HNSW indexing (fast member/capability search)
 * - Semantic matching (intelligent task routing)
 */
import { initRuVectorWasm, isWasmInitialized, generateIdentity, signData, verifySignature, RuVectorHnswIndex, RuVectorSemanticMatcher, } from '../wasm/ruvector-edge.js';
import { logger } from '../utils/logger.js';
//=============================================================================
// WASM-Enhanced Identity Manager
//=============================================================================
/**
 * WASM-accelerated identity manager for P2P Swarm
 * Falls back to Node.js crypto when WASM unavailable
 */
export class WasmIdentityManager {
    publicKey = null;
    secretKey = null;
    publicKeyHex = '';
    initialized = false;
    sendCounter = 0;
    recvCounters = new Map();
    seenNonces = new Map();
    maxNonceAge = 300000; // 5 minutes
    async initialize() {
        if (this.initialized)
            return;
        // Try to initialize WASM
        await initRuVectorWasm();
        // Generate identity (uses WASM if available)
        const identity = await generateIdentity();
        this.publicKey = identity.publicKey;
        this.secretKey = identity.secretKey;
        this.publicKeyHex = identity.publicKeyHex;
        this.initialized = true;
        logger.info('WASM identity manager initialized', {
            wasmAccelerated: isWasmInitialized(),
            publicKeyPrefix: this.publicKeyHex.slice(0, 16),
        });
    }
    /**
     * Get public key as hex string
     */
    getPublicKeyHex() {
        return this.publicKeyHex;
    }
    /**
     * Get public key as Uint8Array
     */
    getPublicKey() {
        if (!this.publicKey)
            throw new Error('Identity not initialized');
        return this.publicKey;
    }
    /**
     * Sign data and return base64 signature
     */
    async sign(data) {
        if (!this.secretKey)
            throw new Error('Identity not initialized');
        const signature = await signData(new Uint8Array(Buffer.from(data)), this.secretKey);
        return Buffer.from(signature).toString('base64');
    }
    /**
     * Verify signature from peer
     */
    async verify(data, signature, peerPublicKeyHex) {
        try {
            const pubKey = new Uint8Array(Buffer.from(peerPublicKeyHex, 'hex'));
            const sig = new Uint8Array(Buffer.from(signature, 'base64'));
            const dataBytes = new Uint8Array(Buffer.from(data));
            return await verifySignature(dataBytes, sig, pubKey);
        }
        catch (error) {
            logger.warn('Signature verification failed', { error });
            return false;
        }
    }
    /**
     * Get next send counter (monotonic)
     */
    getNextCounter() {
        return ++this.sendCounter;
    }
    /**
     * Validate counter from peer (must be strictly increasing)
     */
    validateCounter(peerId, counter) {
        const lastSeen = this.recvCounters.get(peerId) || 0;
        if (counter <= lastSeen) {
            return false;
        }
        this.recvCounters.set(peerId, counter);
        return true;
    }
    /**
     * Generate and track nonce
     */
    generateNonce() {
        const crypto = require('crypto');
        return crypto.randomBytes(16).toString('hex');
    }
    /**
     * Validate nonce (replay protection)
     */
    validateNonce(senderId, nonce, timestamp) {
        // Reject old messages
        if (Date.now() - timestamp > this.maxNonceAge) {
            return false;
        }
        // Get or create nonce set for sender
        if (!this.seenNonces.has(senderId)) {
            this.seenNonces.set(senderId, new Map());
        }
        const senderNonces = this.seenNonces.get(senderId);
        // Reject replayed nonces
        if (senderNonces.has(nonce)) {
            return false;
        }
        senderNonces.set(nonce, timestamp);
        return true;
    }
    /**
     * Check if WASM acceleration is active
     */
    isWasmAccelerated() {
        return isWasmInitialized();
    }
}
/**
 * WASM-accelerated member index for fast capability search
 */
export class WasmMemberIndex {
    hnswIndex;
    members = new Map();
    indexToAgentId = new Map();
    dimensions;
    constructor(dimensions = 128) {
        this.dimensions = dimensions;
        this.hnswIndex = new RuVectorHnswIndex(dimensions, 16, 200);
    }
    /**
     * Add or update member
     */
    addMember(member) {
        this.members.set(member.agentId, member);
        // If member has embedding, add to HNSW index
        if (member.embedding) {
            const idx = this.hnswIndex.add(member.embedding);
            this.indexToAgentId.set(idx, member.agentId);
        }
    }
    /**
     * Remove member
     */
    removeMember(agentId) {
        this.members.delete(agentId);
        // Note: HNSW doesn't support removal, but member won't be returned in searches
    }
    /**
     * Get member by ID
     */
    getMember(agentId) {
        return this.members.get(agentId);
    }
    /**
     * Find members with specific capabilities
     */
    findByCapabilities(capabilities) {
        const results = [];
        for (const member of this.members.values()) {
            const hasAll = capabilities.every(cap => member.capabilities.includes(cap));
            if (hasAll) {
                results.push(member);
            }
        }
        return results;
    }
    /**
     * Find k nearest members by embedding similarity
     */
    findSimilar(queryEmbedding, k = 5) {
        const searchResults = this.hnswIndex.search(queryEmbedding, k);
        const members = [];
        for (const result of searchResults) {
            const agentId = this.indexToAgentId.get(result.index);
            if (agentId) {
                const member = this.members.get(agentId);
                if (member) {
                    members.push(member);
                }
            }
        }
        return members;
    }
    /**
     * Get all live members (last seen within threshold)
     */
    getLiveMembers(maxAge = 30000) {
        const now = Date.now();
        const live = [];
        for (const member of this.members.values()) {
            if (now - member.lastSeen < maxAge) {
                live.push(member);
            }
        }
        return live;
    }
    /**
     * Get total member count
     */
    size() {
        return this.members.size;
    }
    /**
     * Check if WASM acceleration is active
     */
    isWasmAccelerated() {
        return this.hnswIndex.isWasmAccelerated();
    }
}
//=============================================================================
// WASM-Enhanced Task Router
//=============================================================================
/**
 * WASM-accelerated task router for intelligent agent selection
 */
export class WasmTaskRouter {
    semanticMatcher;
    dimensions;
    constructor(dimensions = 128) {
        this.dimensions = dimensions;
        this.semanticMatcher = new RuVectorSemanticMatcher();
    }
    /**
     * Register agent for task routing
     */
    registerAgent(agentId, embedding, capabilities) {
        this.semanticMatcher.registerAgent(agentId, embedding, capabilities);
    }
    /**
     * Route task to best matching agents
     */
    routeTask(taskEmbedding, requiredCapabilities, topK = 3) {
        const matches = this.semanticMatcher.matchTask(taskEmbedding, topK * 2);
        // Filter by required capabilities if specified
        if (requiredCapabilities && requiredCapabilities.length > 0) {
            return matches
                .filter(m => requiredCapabilities.every(cap => m.capabilities.includes(cap)))
                .slice(0, topK)
                .map(m => ({
                agentId: m.agent,
                score: m.score,
                capabilities: m.capabilities,
            }));
        }
        return matches.slice(0, topK).map(m => ({
            agentId: m.agent,
            score: m.score,
            capabilities: m.capabilities,
        }));
    }
    /**
     * Check if WASM acceleration is active
     */
    isWasmAccelerated() {
        return this.semanticMatcher.isWasmAccelerated();
    }
}
//=============================================================================
// Factory Functions
//=============================================================================
/**
 * Create WASM-enhanced identity manager
 */
export async function createWasmIdentityManager() {
    const manager = new WasmIdentityManager();
    await manager.initialize();
    return manager;
}
/**
 * Create WASM-enhanced member index
 */
export function createWasmMemberIndex(dimensions = 128) {
    return new WasmMemberIndex(dimensions);
}
/**
 * Create WASM-enhanced task router
 */
export function createWasmTaskRouter(dimensions = 128) {
    return new WasmTaskRouter(dimensions);
}
/**
 * Get WASM acceleration status
 */
export function getWasmStatus() {
    return {
        initialized: isWasmInitialized(),
        features: isWasmInitialized()
            ? ['WasmIdentity', 'WasmHnswIndex', 'WasmSemanticMatcher']
            : ['JS Fallback'],
    };
}
export default {
    WasmIdentityManager,
    WasmMemberIndex,
    WasmTaskRouter,
    createWasmIdentityManager,
    createWasmMemberIndex,
    createWasmTaskRouter,
    getWasmStatus,
};
//# sourceMappingURL=p2p-swarm-wasm.js.map