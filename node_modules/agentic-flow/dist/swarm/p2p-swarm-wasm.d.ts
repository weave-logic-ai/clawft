/**
 * P2P Swarm V2 WASM Integration
 *
 * Provides WASM-accelerated components for P2P Swarm:
 * - Ed25519 identity (faster key generation)
 * - HNSW indexing (fast member/capability search)
 * - Semantic matching (intelligent task routing)
 */
/**
 * WASM-accelerated identity manager for P2P Swarm
 * Falls back to Node.js crypto when WASM unavailable
 */
export declare class WasmIdentityManager {
    private publicKey;
    private secretKey;
    private publicKeyHex;
    private initialized;
    private sendCounter;
    private recvCounters;
    private seenNonces;
    private maxNonceAge;
    initialize(): Promise<void>;
    /**
     * Get public key as hex string
     */
    getPublicKeyHex(): string;
    /**
     * Get public key as Uint8Array
     */
    getPublicKey(): Uint8Array;
    /**
     * Sign data and return base64 signature
     */
    sign(data: string): Promise<string>;
    /**
     * Verify signature from peer
     */
    verify(data: string, signature: string, peerPublicKeyHex: string): Promise<boolean>;
    /**
     * Get next send counter (monotonic)
     */
    getNextCounter(): number;
    /**
     * Validate counter from peer (must be strictly increasing)
     */
    validateCounter(peerId: string, counter: number): boolean;
    /**
     * Generate and track nonce
     */
    generateNonce(): string;
    /**
     * Validate nonce (replay protection)
     */
    validateNonce(senderId: string, nonce: string, timestamp: number): boolean;
    /**
     * Check if WASM acceleration is active
     */
    isWasmAccelerated(): boolean;
}
export interface SwarmMember {
    agentId: string;
    publicKeyHex: string;
    capabilities: string[];
    embedding?: Float32Array;
    lastSeen: number;
}
/**
 * WASM-accelerated member index for fast capability search
 */
export declare class WasmMemberIndex {
    private hnswIndex;
    private members;
    private indexToAgentId;
    private dimensions;
    constructor(dimensions?: number);
    /**
     * Add or update member
     */
    addMember(member: SwarmMember): void;
    /**
     * Remove member
     */
    removeMember(agentId: string): void;
    /**
     * Get member by ID
     */
    getMember(agentId: string): SwarmMember | undefined;
    /**
     * Find members with specific capabilities
     */
    findByCapabilities(capabilities: string[]): SwarmMember[];
    /**
     * Find k nearest members by embedding similarity
     */
    findSimilar(queryEmbedding: Float32Array, k?: number): SwarmMember[];
    /**
     * Get all live members (last seen within threshold)
     */
    getLiveMembers(maxAge?: number): SwarmMember[];
    /**
     * Get total member count
     */
    size(): number;
    /**
     * Check if WASM acceleration is active
     */
    isWasmAccelerated(): boolean;
}
/**
 * WASM-accelerated task router for intelligent agent selection
 */
export declare class WasmTaskRouter {
    private semanticMatcher;
    private dimensions;
    constructor(dimensions?: number);
    /**
     * Register agent for task routing
     */
    registerAgent(agentId: string, embedding: Float32Array, capabilities: string[]): void;
    /**
     * Route task to best matching agents
     */
    routeTask(taskEmbedding: Float32Array, requiredCapabilities?: string[], topK?: number): Array<{
        agentId: string;
        score: number;
        capabilities: string[];
    }>;
    /**
     * Check if WASM acceleration is active
     */
    isWasmAccelerated(): boolean;
}
/**
 * Create WASM-enhanced identity manager
 */
export declare function createWasmIdentityManager(): Promise<WasmIdentityManager>;
/**
 * Create WASM-enhanced member index
 */
export declare function createWasmMemberIndex(dimensions?: number): WasmMemberIndex;
/**
 * Create WASM-enhanced task router
 */
export declare function createWasmTaskRouter(dimensions?: number): WasmTaskRouter;
/**
 * Get WASM acceleration status
 */
export declare function getWasmStatus(): {
    initialized: boolean;
    features: string[];
};
declare const _default: {
    WasmIdentityManager: typeof WasmIdentityManager;
    WasmMemberIndex: typeof WasmMemberIndex;
    WasmTaskRouter: typeof WasmTaskRouter;
    createWasmIdentityManager: typeof createWasmIdentityManager;
    createWasmMemberIndex: typeof createWasmMemberIndex;
    createWasmTaskRouter: typeof createWasmTaskRouter;
    getWasmStatus: typeof getWasmStatus;
};
export default _default;
//# sourceMappingURL=p2p-swarm-wasm.d.ts.map