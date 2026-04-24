/**
 * P2P Swarm v2 - Production Grade
 *
 * Fixes from code review:
 * 1. Two-layer key scheme (swarm envelope key + per-peer session keys)
 * 2. Ed25519 identity keys + X25519 ephemeral keys
 * 3. Message replay protection (nonces, counters, timestamps)
 * 4. Gun-based WebRTC signaling (no external PeerServer)
 * 5. IPFS CID pointers for large payloads
 * 6. Ed25519 signatures on all messages
 * 7. Relay health monitoring
 * 8. Task execution envelope with budgets
 */
/**
 * Stable canonical JSON stringify
 *
 * Unlike JSON.stringify, this:
 * - Sorts object keys recursively (alphabetically)
 * - Produces identical output regardless of insertion order
 * - Is safe to use for signing and hashing
 */
declare function stableStringify(obj: unknown): string;
/**
 * Identity and session key management
 *
 * Security fixes applied:
 * - Ed25519 uses crypto.sign(null, ...) - direct sign, not hash-then-sign
 * - Separate send counter (local) from receive counters (per-peer)
 * - Per-sender nonce tracking with timestamps for expiry
 * - X25519 ECDH + HKDF for real session key derivation
 */
declare class IdentityManager {
    private identityKey;
    private x25519Key;
    private peerSessionKeys;
    private seenNonces;
    private localSendCounter;
    private recvCounters;
    private maxNonceAge;
    private nonceCleanupInterval;
    constructor();
    /**
     * Get public identity key (PEM format for transport)
     */
    getPublicKey(): string;
    /**
     * Get X25519 public key for key exchange
     */
    getX25519PublicKey(): string;
    /**
     * Sign data with Ed25519 identity key
     * Ed25519 uses direct sign (no hash algorithm needed)
     */
    sign(data: string): string;
    /**
     * Verify Ed25519 signature from peer
     */
    verify(data: string, signature: string, peerPublicKey: string): boolean;
    /**
     * Derive session key with peer using real X25519 ECDH + HKDF
     *
     * Salt derivation: sha256(min(pubA, pubB) || max(pubA, pubB))
     * - Stable per pair regardless of who initiates
     * - High entropy from public key material
     *
     * Info: "p2p-swarm-v2:${swarmId}:${peerId}"
     * - Prevents cross-swarm key reuse
     * - Makes derived keys unique per pair and swarm
     */
    deriveSessionKey(peerX25519PubKey: string, peerId: string, swarmId?: string): Buffer;
    /**
     * Generate cryptographically secure nonce
     */
    generateNonce(): string;
    /**
     * Check and record nonce with per-sender tracking
     * Returns true if nonce is valid (not replayed, not expired)
     */
    checkNonce(nonce: string, timestamp: number, senderId: string): boolean;
    /**
     * Periodic cleanup of expired nonces
     */
    private startNonceCleanup;
    /**
     * Get next send counter (monotonically increasing)
     */
    getNextSendCounter(): number;
    /**
     * Validate received counter from peer (must be > last seen from that peer)
     */
    validateRecvCounter(peerId: string, counter: number): boolean;
    /**
     * Clear session key (for rotation)
     */
    rotateSessionKey(peerId: string): void;
    /**
     * Clean up resources
     */
    destroy(): void;
}
/**
 * Encryption with proper GCM handling
 */
declare class CryptoV2 {
    private algorithm;
    encrypt(data: string, key: Buffer): {
        ciphertext: string;
        iv: string;
        tag: string;
    };
    decrypt(ciphertext: string, key: Buffer, iv: string, tag: string): string | null;
    hash(data: string | Buffer): string;
    generateCID(data: string | Buffer): string;
}
/**
 * Signed message envelope for Gun pubsub
 *
 * Design rules:
 * - Gun = coordination bus (always use swarm envelope key)
 * - IPFS = artifact plane (content-addressed storage)
 * - WebRTC = private fast lane (can use per-peer session keys)
 *
 * Session keys are NOT used for Gun messages to avoid decryption
 * mismatch problems where only one receiver can decrypt.
 */
interface SignedEnvelope {
    messageId: string;
    topic: string;
    timestamp: number;
    senderId: string;
    senderPubKey: string;
    senderX25519Key: string;
    payloadHash: string;
    nonce: string;
    counter: number;
    signature: string;
    encrypted: {
        ciphertext: string;
        iv: string;
        tag: string;
    };
}
/**
 * Task execution envelope
 */
interface TaskEnvelope {
    taskId: string;
    moduleCID: string;
    entrypoint: string;
    inputCID: string;
    outputSchemaHash: string;
    budgets: {
        fuelLimit: number;
        memoryMB: number;
        timeoutMs: number;
    };
    requester: string;
    deadline: number;
    priority: number;
}
/**
 * Task result receipt with full execution binding
 * The signature covers ALL fields to prevent tampering
 *
 * Includes binding to the original TaskEnvelope for complete traceability
 */
interface TaskReceipt {
    taskId: string;
    executor: string;
    executorPubKey: string;
    resultCID: string;
    status: 'success' | 'error' | 'timeout' | 'oom';
    fuelUsed: number;
    memoryPeakMB: number;
    executionMs: number;
    inputHash: string;
    outputHash: string;
    moduleHash: string;
    startTimestamp: number;
    endTimestamp: number;
    moduleCID: string;
    inputCID: string;
    entrypoint: string;
    outputSchemaHash: string;
    taskEnvelopeHash: string;
    signature: string;
}
/**
 * Learning artifact pointer (small, goes to Gun)
 */
interface ArtifactPointer {
    type: 'q_table' | 'memory_vectors' | 'model_weights' | 'trajectory';
    agentId: string;
    cid: string;
    version: number;
    schemaHash: string;
    dimensions: string;
    checksum: string;
    timestamp: number;
    signature: string;
}
/**
 * Relay health tracker
 */
declare class RelayManager {
    private relays;
    private workingRelays;
    static readonly BOOTSTRAP_RELAYS: string[];
    constructor(customRelays?: string[]);
    /**
     * Get working relays
     */
    getWorkingRelays(): string[];
    /**
     * Mark relay as failed
     */
    markFailed(url: string): void;
    /**
     * Mark relay as successful
     */
    markSuccess(url: string, latencyMs: number): void;
    /**
     * Add new relay
     */
    addRelay(url: string): void;
    /**
     * Get health metrics
     */
    getMetrics(): {
        total: number;
        healthy: number;
        avgLatency: number;
    };
    /**
     * Persist working set
     */
    exportWorkingSet(): string[];
    /**
     * Import working set
     */
    importWorkingSet(relays: string[]): void;
}
/**
 * IPFS artifact manager (CID-based storage)
 *
 * IMPORTANT CID LIMITATION:
 * The current generateCID creates a simplified hash-based identifier (Qm prefix).
 * This is NOT a real IPFS CID and will NOT interop with:
 * - Real IPFS gateways
 * - Pinning services (Pinata, web3.storage, Filebase)
 * - Content routing
 *
 * For production, you need one of:
 * A) Real IPFS client: add bytes to IPFS node, take returned CID
 * B) Multiformats library: create CIDv1 from raw bytes offline
 *
 * The current implementation works for local coordination and can be
 * upgraded to real IPFS when needed.
 *
 * Gateway fetch is DISABLED by default because our fake CIDs won't work.
 * Enable only when using real multiformats CIDv1 generation.
 */
declare class ArtifactStore {
    private localCache;
    private crypto;
    private maxCacheSize;
    private currentCacheSize;
    private maxArtifactSize;
    private enableGatewayFetch;
    static readonly CHUNK_SIZE: number;
    static readonly IPFS_GATEWAYS: string[];
    constructor(enableGatewayFetch?: boolean);
    /**
     * Store artifact and get CID
     */
    store(data: Buffer | string, compress?: boolean): Promise<{
        cid: string;
        size: number;
        chunks: number;
    }>;
    /**
     * Retrieve artifact by CID
     *
     * Strategy:
     * 1. Check local cache
     * 2. If enableGatewayFetch=true AND CID is real (bafy prefix), try gateways
     * 3. Otherwise return null (local-only mode)
     *
     * Gateway fetch is DISABLED by default because our simplified CIDs
     * are not real IPFS CIDs and won't work with gateways.
     */
    retrieve(cid: string): Promise<Buffer | null>;
    /**
     * Enable gateway fetch (only when using real IPFS CIDs)
     */
    setEnableGatewayFetch(enable: boolean): void;
    /**
     * Store in local cache with eviction
     */
    private cacheStore;
    /**
     * Create artifact pointer for Gun
     * Uses stableStringify for deterministic signing
     */
    createPointer(type: ArtifactPointer['type'], agentId: string, cid: string, dimensions: string, identity: IdentityManager): ArtifactPointer;
}
/**
 * Production-grade P2P Swarm Coordinator
 */
export declare class P2PSwarmV2 {
    private identity;
    private crypto;
    private relayManager;
    private artifactStore;
    private swarmKey;
    private swarmId;
    private agentId;
    private gun;
    private swarmNode;
    private connected;
    private messageHandlers;
    private pendingOffers;
    private memberRegistry;
    private heartbeatInterval;
    private taskExecutorActive;
    private claimedTasks;
    private readonly HEARTBEAT_INTERVAL_MS;
    private readonly MEMBER_TIMEOUT_MS;
    private negativeMemberCache;
    private readonly NEGATIVE_CACHE_TTL_MS;
    private readonly CLAIM_TTL_MS;
    constructor(agentId: string, swarmKey?: string);
    /**
     * Connect to Gun relays
     *
     * Starts autonomy features:
     * - Membership watcher (subscribe to members, verify registrations)
     * - Heartbeat publishing (every 20 seconds)
     * - Task executor loop (optional, enable with startTaskExecutor())
     */
    connect(): Promise<boolean>;
    /**
     * Register self with public key and X25519 key
     * Uses canonical serialization for signature
     * Field order must match verifyMemberRegistration
     */
    private registerSelf;
    /**
     * Verified member entry type
     */
    private static readonly MEMBER_REQUIRED_FIELDS;
    /**
     * Verify and cache a member registration from Gun
     * STRICT: Requires all fields including x25519PublicKey
     * Returns the verified member or null if invalid
     */
    private verifyMemberRegistration;
    /**
     * Resolve full verified member from the registry
     * Returns the complete member entry (with both Ed25519 and X25519 keys)
     * Returns null if member not found or not verified
     *
     * Uses negative cache to prevent spam from repeated lookups of unknown agents
     */
    private resolveMember;
    /**
     * Convenience: Resolve just the Ed25519 public key
     */
    private resolveMemberKey;
    /**
     * Subscribe to WebRTC signaling (Gun-based, no PeerServer needed)
     * SECURITY: Verifies signatures using registry key, not envelope key
     */
    private subscribeToSignaling;
    /**
     * Handle incoming WebRTC offer (via Gun signaling)
     */
    private handleOffer;
    /**
     * Handle incoming WebRTC answer
     */
    private handleAnswer;
    /**
     * Handle incoming ICE candidate
     */
    private handleICE;
    /**
     * Create canonical signaling message and signature
     * Uses stableStringify for deterministic serialization
     */
    private createCanonicalSignal;
    /**
     * Send WebRTC offer via Gun (no PeerServer needed)
     * Uses canonical signature for verification
     */
    sendOffer(targetAgentId: string, offer: any): Promise<void>;
    /**
     * Send WebRTC answer via Gun
     */
    sendAnswer(targetAgentId: string, answer: any): Promise<void>;
    /**
     * Send ICE candidate via Gun
     */
    sendICE(targetAgentId: string, candidate: any): Promise<void>;
    /**
     * Publish signed and encrypted message
     *
     * Gun messages ALWAYS use swarm envelope key (not session keys)
     * This ensures all swarm members can decrypt and process messages.
     * Uses canonical serialization for signature stability.
     */
    publish(topic: string, payload: any): Promise<string>;
    /**
     * Subscribe to topic with strict identity verification
     *
     * SECURITY: Never trust keys from envelopes. Always resolve from registry.
     *
     * Verification steps:
     * 1. Check nonce and timestamp (replay protection)
     * 2. Check counter (ordering)
     * 3. Resolve sender's public key from verified member registry
     * 4. Reject if envelope key differs from registry key
     * 5. Verify signature using registry key
     * 6. Decrypt with swarm key
     * 7. Verify payload hash
     */
    subscribe(topic: string, callback: (data: any, from: string) => void): void;
    /**
     * Store Q-table and publish pointer
     */
    syncQTable(qTable: number[][]): Promise<ArtifactPointer>;
    /**
     * Store memory vectors and publish pointer
     */
    syncMemory(vectors: number[][], namespace: string): Promise<ArtifactPointer>;
    /**
     * Submit task for execution
     */
    submitTask(task: Omit<TaskEnvelope, 'requester' | 'deadline' | 'priority'>): Promise<string>;
    /**
     * Submit task result with full execution binding
     * Signs ALL fields including TaskEnvelope binding for complete traceability
     */
    submitResult(receipt: Omit<TaskReceipt, 'signature' | 'executorPubKey'>): Promise<string>;
    /**
     * Get status
     */
    getStatus(): {
        connected: boolean;
        swarmId: string;
        agentId: string;
        publicKey: string;
        relays: {
            total: number;
            healthy: number;
            avgLatency: number;
        };
    };
    /**
     * Get swarm key for sharing
     */
    getSwarmKey(): string;
    /**
     * Start membership watcher
     * Continuously monitors the members map, verifies registrations, tracks liveness
     */
    private startMembershipWatcher;
    /**
     * Subscribe to heartbeat messages to track liveness
     * Explicitly rejects agentId mismatches to prevent confusion attacks
     */
    private subscribeToHeartbeats;
    /**
     * Start publishing heartbeats
     * Every HEARTBEAT_INTERVAL_MS, publish a signed heartbeat message
     */
    private startHeartbeat;
    /**
     * Publish a single heartbeat
     * Belt-and-suspenders: check both connected and swarmNode
     */
    private publishHeartbeat;
    /**
     * Get list of live members (heartbeat within MEMBER_TIMEOUT_MS)
     */
    getLiveMembers(): Array<{
        agentId: string;
        capabilities: string[];
        lastSeen: number;
        isAlive: boolean;
    }>;
    /**
     * Get count of live members
     */
    getLiveMemberCount(): number;
    /**
     * Verify a task receipt
     *
     * Checks:
     * 1. Executor is in verified member registry
     * 2. Executor public key matches registry
     * 3. Signature is valid over the canonical receipt fields
     * 4. Optionally verifies taskEnvelopeHash if original task is provided
     */
    verifyReceipt(receipt: TaskReceipt, originalTask?: TaskEnvelope): Promise<{
        valid: boolean;
        reason?: string;
        executor?: {
            agentId: string;
            capabilities: string[];
            verified: boolean;
        };
    }>;
    /**
     * Start the task executor loop
     * Subscribes to tasks, claims them, and (in production) executes in Wasmtime
     *
     * Security checks:
     * - Requester must be verified member
     * - Requester in envelope must match sender
     * - Executor must have 'executor' capability
     */
    startTaskExecutor(): void;
    /**
     * Claim a task with conflict resolution
     *
     * Before claiming:
     * 1. Check for existing claim
     * 2. If claim exists and is fresh (< CLAIM_TTL_MS), skip
     * 3. If no claim or stale, write our claim
     *
     * Uses single timestamp to ensure signature matches stored data
     */
    private claimTask;
    /**
     * Execute a task
     *
     * NOTE: This is a stub. In production, this would:
     * 1. Fetch WASM module from IPFS using moduleCID
     * 2. Fetch input data from IPFS using inputCID
     * 3. Execute in Wasmtime with fuel, memory, and timeout limits
     * 4. Store output to IPFS, get resultCID
     * 5. Return signed receipt
     *
     * Hashes actual bytes when available for meaningful receipts
     */
    private executeTask;
    /**
     * Create error receipt for failed task
     * Uses same hashing semantics as executeTask for consistency
     */
    private createErrorReceipt;
    /**
     * Stop task executor
     */
    stopTaskExecutor(): void;
    /**
     * Disconnect and cleanup resources
     */
    disconnect(): void;
}
/**
 * RuVector optimization for Gun message handling
 *
 * Provides:
 * - HNSW indexing for O(log n) message lookup by topic similarity
 * - Batch message operations for reduced overhead
 * - PQ quantization for compressed message metadata storage
 * - Vector-based topic routing for intelligent distribution
 */
declare class RuVectorGunOptimizer {
    private topicVectors;
    private messageIndex;
    private batchQueue;
    private batchFlushInterval;
    private readonly batchSize;
    private readonly flushIntervalMs;
    constructor();
    /**
     * Generate topic embedding using simple hash-based vectors
     * In production: use actual embedding model via ruvector
     */
    private generateTopicVector;
    /**
     * Find similar topics using cosine similarity
     * In production: use HNSW index via ruvector for O(log n) lookup
     */
    findSimilarTopics(topic: string, topK?: number): Array<{
        topic: string;
        similarity: number;
    }>;
    /**
     * Cosine similarity between two vectors
     */
    private cosineSimilarity;
    /**
     * Index a topic for similarity search
     */
    indexTopic(topic: string): void;
    /**
     * Add message to batch queue for efficient Gun writes
     */
    queueMessage(topic: string, payload: any): void;
    /**
     * Get batched messages for efficient Gun write
     */
    flushBatch(): Array<{
        topic: string;
        payload: any;
    }>;
    /**
     * Start periodic batch flushing
     */
    private startBatchFlusher;
    /**
     * Get optimization metrics
     */
    getMetrics(): {
        indexedTopics: number;
        batchQueueSize: number;
        totalMessages: number;
    };
    /**
     * Cleanup resources
     */
    destroy(): void;
}
/**
 * Create and connect a P2P swarm
 */
export declare function createP2PSwarmV2(agentId: string, swarmKey?: string): Promise<P2PSwarmV2>;
export { stableStringify, IdentityManager, CryptoV2, RelayManager, ArtifactStore, RuVectorGunOptimizer, };
export type { SignedEnvelope, TaskEnvelope, TaskReceipt, ArtifactPointer, };
//# sourceMappingURL=p2p-swarm-v2.d.ts.map