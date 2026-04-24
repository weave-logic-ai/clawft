/**
 * QUIC Synchronization Types for AgentDB
 *
 * This file contains TypeScript interfaces and types for the QUIC-based
 * multi-node synchronization system. These types mirror the Protocol Buffer
 * definitions and provide type safety for the sync implementation.
 */
/**
 * Vector clock for causal ordering of events across distributed nodes.
 * Maps node IDs to their logical clock values.
 */
export interface VectorClock {
    clocks: Map<string, number>;
}
/**
 * Compares two vector clocks to determine causal relationship
 */
export type VectorClockComparison = 'before' | 'after' | 'concurrent' | 'equal';
/**
 * Main sync message envelope wrapping all sync operations
 */
export interface SyncMessage {
    sequenceNumber: number;
    timestampMs: number;
    nodeId: string;
    vectorClock: VectorClock;
    payload: SyncPayload;
}
/**
 * Union type for different sync payload types
 */
export type SyncPayload = {
    type: 'episode_sync';
    data: EpisodeSync;
} | {
    type: 'skill_sync';
    data: SkillSync;
} | {
    type: 'causal_edge_sync';
    data: CausalEdgeSync;
} | {
    type: 'reconciliation_request';
    data: FullReconciliationRequest;
} | {
    type: 'reconciliation_response';
    data: FullReconciliationResponse;
};
/**
 * Supported operations for episode sync
 */
export declare enum EpisodeSyncOperation {
    CREATE = "CREATE",
    UPDATE = "UPDATE",
    DELETE = "DELETE"
}
/**
 * Episode synchronization message
 */
export interface EpisodeSync {
    operation: EpisodeSyncOperation;
    episodeId: number;
    episodeData: SyncableEpisode;
    causalClock: VectorClock;
    signature: Uint8Array;
}
/**
 * Serializable episode data for sync
 */
export interface SyncableEpisode {
    id?: number;
    agentId: string;
    sessionId: string;
    task: string;
    input: string;
    output: string;
    critique?: string;
    reward: number;
    success: boolean;
    latencyMs: number;
    timestamp: number;
    metadata?: Record<string, any>;
    vectorClock: VectorClock;
}
/**
 * G-Counter (Grow-only Counter) for skill usage tracking
 */
export interface GCounter {
    nodeCounters: Map<string, number>;
}
/**
 * LWW-Register (Last-Write-Wins Register) for scalar values
 */
export interface LWWRegister<T> {
    value: T;
    timestamp: number;
    nodeId: string;
}
/**
 * OR-Set (Observed-Remove Set) for set-based values
 */
export interface ORSet<T> {
    adds: Map<T, Set<string>>;
    removes: Set<string>;
}
/**
 * Skill synchronization message with CRDT fields
 */
export interface SkillSync {
    skillId: number;
    skillName: string;
    description?: string;
    uses: GCounter;
    successRate: LWWRegister<number>;
    avgReward: LWWRegister<number>;
    avgLatencyMs: LWWRegister<number>;
    sourceEpisodes: ORSet<number>;
    signature: Record<string, any>;
    version: VectorClock;
    metadata?: Record<string, any>;
}
/**
 * Metadata for conflict resolution in causal edges
 */
export interface ConflictResolutionMetadata {
    experimentIds?: number[];
    evidenceCount: number;
    lastModifiedBy: string;
    lastModifiedAt: number;
}
/**
 * Causal edge synchronization message
 */
export interface CausalEdgeSync {
    edgeId: number;
    fromMemoryId: number;
    fromMemoryType: 'episode' | 'skill' | 'note' | 'fact';
    toMemoryId: number;
    toMemoryType: 'episode' | 'skill' | 'note' | 'fact';
    similarity: number;
    uplift?: number;
    confidence: number;
    sampleSize?: number;
    evidenceIds?: number[];
    experimentIds?: number[];
    confounderScore?: number;
    mechanism?: string;
    version: VectorClock;
    conflictMetadata: ConflictResolutionMetadata;
    metadata?: Record<string, any>;
}
/**
 * Data types that can be reconciled
 */
export type ReconciliableDataType = 'episodes' | 'skills' | 'edges' | 'experiments';
/**
 * Request for full reconciliation
 */
export interface FullReconciliationRequest {
    lastSyncTimestamp: number;
    currentState: VectorClock;
    dataTypes: ReconciliableDataType[];
    requestId: string;
}
/**
 * Response with full state for reconciliation
 */
export interface FullReconciliationResponse {
    requestId: string;
    episodes: EpisodeSync[];
    skills: SkillSync[];
    edges: CausalEdgeSync[];
    authoritativeClock: VectorClock;
    merkleRoot: string;
}
/**
 * State summary for efficient reconciliation
 */
export interface StateSummary {
    episodes: {
        count: number;
        merkleRoot: string;
        vectorClock: VectorClock;
    };
    skills: {
        count: number;
        merkleRoot: string;
        vectorClock: VectorClock;
    };
    edges: {
        count: number;
        merkleRoot: string;
        vectorClock: VectorClock;
    };
}
/**
 * Reconciliation report with results
 */
export interface ReconciliationReport {
    success: boolean;
    startTime: number;
    endTime: number;
    duration: number;
    recordsAdded: number;
    recordsUpdated: number;
    recordsDeleted: number;
    conflictsResolved: number;
    conflictsUnresolved: number;
    errors: string[];
}
/**
 * JWT claims for API authorization
 */
export interface JWTClaims {
    iss: string;
    sub: string;
    exp: number;
    iat: number;
    roles: UserRole[];
    scopes: AuthScope[];
    networkId: string;
    metadata?: Record<string, any>;
}
/**
 * User roles
 */
export declare enum UserRole {
    ADMIN = "admin",
    AGENT = "agent",
    OBSERVER = "observer",
    LEARNER = "learner"
}
/**
 * Authorization scopes
 */
export type AuthScope = 'episodes:read' | 'episodes:write' | 'episodes:delete' | 'skills:read' | 'skills:write' | 'skills:delete' | 'edges:read' | 'edges:write' | 'edges:delete' | 'experiments:read' | 'experiments:write' | 'reconciliation:request';
/**
 * Node registration data
 */
export interface NodeRegistration {
    nodeId: string;
    certificate: string;
    publicKey: string;
    networkId: string;
    registeredAt: number;
    expiresAt: number;
}
/**
 * Network topology types
 */
export declare enum NetworkTopology {
    HUB_AND_SPOKE = "hub_and_spoke",
    MESH = "mesh",
    HIERARCHICAL = "hierarchical"
}
/**
 * Conflict resolution strategies
 */
export declare enum ConflictResolutionStrategy {
    AUTO = "auto",// Automatic resolution using configured algorithms
    MANUAL = "manual",// Flag conflicts for manual resolution
    INTERACTIVE = "interactive"
}
/**
 * Sync mode
 */
export declare enum SyncMode {
    INCREMENTAL = "incremental",
    FULL = "full",
    HYBRID = "hybrid"
}
/**
 * Server configuration
 */
export interface ServerConfig {
    port: number;
    host: string;
    maxConnections: number;
    maxStreamsPerConnection: number;
    tlsCertPath: string;
    tlsKeyPath: string;
    caCertPath: string;
    jwtSecret: string;
    jwtExpirationMs: number;
    changelogRetentionDays: number;
    changelogMaxRecords: number;
    reconciliationIntervalMs: number;
    batchSize: number;
    compressionThreshold: number;
    maxMemoryPerConnection: number;
    topology: NetworkTopology;
    networkId: string;
}
/**
 * Client configuration
 */
export interface ClientConfig {
    nodeId: string;
    serverUrl: string;
    clientCertPath: string;
    clientKeyPath: string;
    caCertPath: string;
    jwt: string;
    mode: SyncMode;
    incrementalIntervalMs: number;
    fullReconciliationIntervalMs: number;
    autoSync: boolean;
    conflictResolutionStrategy: ConflictResolutionStrategy;
    batchSize: number;
    compressionThreshold: number;
    retryMaxAttempts: number;
    retryBackoffMs: number;
}
/**
 * Server status
 */
export interface ServerStatus {
    uptime: number;
    activeConnections: number;
    totalConnectionsHandled: number;
    activeStreams: number;
    changelogSize: number;
    lastReconciliation: number;
    avgSyncLatencyMs: number;
    throughputBytesPerSec: number;
    conflictsPerMinute: number;
    cpuUsagePercent: number;
    memoryUsageBytes: number;
    diskUsageBytes: number;
}
/**
 * Client status
 */
export interface ClientStatus {
    connected: boolean;
    nodeId: string;
    serverUrl: string;
    lastSyncTimestamp: number;
    nextSyncScheduled: number;
    episodesSynced: number;
    skillsSynced: number;
    edgesSynced: number;
    conflictsEncountered: number;
    conflictsAutoResolved: number;
    connectionUptimeMs: number;
    reconnectAttempts: number;
    lastError?: string;
}
/**
 * Sync result for a single operation
 */
export interface SyncResult {
    success: boolean;
    duration: number;
    episodesAdded: number;
    episodesUpdated: number;
    episodesDeleted: number;
    skillsAdded: number;
    skillsUpdated: number;
    edgesAdded: number;
    edgesUpdated: number;
    conflictsTotal: number;
    conflictsAutoResolved: number;
    conflictsPending: number;
    errors: SyncError[];
}
/**
 * Sync error details
 */
export interface SyncError {
    code: string;
    message: string;
    dataType: ReconciliableDataType;
    recordId?: number;
    timestamp: number;
    retryable: boolean;
}
/**
 * Type guard for episode sync
 */
export declare function isEpisodeSync(payload: SyncPayload): payload is {
    type: 'episode_sync';
    data: EpisodeSync;
};
/**
 * Type guard for skill sync
 */
export declare function isSkillSync(payload: SyncPayload): payload is {
    type: 'skill_sync';
    data: SkillSync;
};
/**
 * Type guard for causal edge sync
 */
export declare function isCausalEdgeSync(payload: SyncPayload): payload is {
    type: 'causal_edge_sync';
    data: CausalEdgeSync;
};
/**
 * Compare two vector clocks
 */
export declare function compareVectorClocks(a: VectorClock, b: VectorClock): VectorClockComparison;
/**
 * Merge two vector clocks (take max of each node)
 */
export declare function mergeVectorClocks(a: VectorClock, b: VectorClock): VectorClock;
/**
 * Increment vector clock for local node
 */
export declare function incrementVectorClock(clock: VectorClock, nodeId: string): VectorClock;
/**
 * Create empty vector clock
 */
export declare function createVectorClock(): VectorClock;
/**
 * Increment G-Counter for a node
 */
export declare function incrementGCounter(counter: GCounter, nodeId: string, delta?: number): GCounter;
/**
 * Get total value of G-Counter
 */
export declare function getGCounterValue(counter: GCounter): number;
/**
 * Merge two G-Counters (take max per node)
 */
export declare function mergeGCounter(a: GCounter, b: GCounter): GCounter;
/**
 * Update LWW-Register with new value
 */
export declare function updateLWWRegister<T>(register: LWWRegister<T>, newValue: T, nodeId: string, timestamp?: number): LWWRegister<T>;
/**
 * Merge two LWW-Registers (keep most recent)
 */
export declare function mergeLWWRegister<T>(a: LWWRegister<T>, b: LWWRegister<T>): LWWRegister<T>;
/**
 * Add element to OR-Set
 */
export declare function addToORSet<T>(set: ORSet<T>, element: T, uniqueTag: string): ORSet<T>;
/**
 * Remove element from OR-Set
 */
export declare function removeFromORSet<T>(set: ORSet<T>, element: T): ORSet<T>;
/**
 * Get current elements in OR-Set
 */
export declare function getORSetElements<T>(set: ORSet<T>): Set<T>;
/**
 * Merge two OR-Sets
 */
export declare function mergeORSet<T>(a: ORSet<T>, b: ORSet<T>): ORSet<T>;
/**
 * Weighted average for numeric conflict resolution
 */
export declare function weightedAverage(v1: number, w1: number, v2: number, w2: number): number;
/**
 * Determine authorization for operation
 */
export declare function isAuthorized(jwt: JWTClaims, requiredScope: AuthScope): boolean;
/**
 * Check if JWT is expired
 */
export declare function isJWTExpired(jwt: JWTClaims): boolean;
/**
 * Generate unique tag for OR-Set operations
 */
export declare function generateUniqueTag(nodeId: string, timestamp?: number): string;
/**
 * Events emitted by sync client
 */
export type SyncEvent = {
    type: 'sync_started';
    timestamp: number;
} | {
    type: 'sync_completed';
    result: SyncResult;
} | {
    type: 'sync_failed';
    error: SyncError;
} | {
    type: 'conflict_detected';
    conflict: ConflictData;
} | {
    type: 'conflict_resolved';
    conflict: ConflictData;
    resolution: any;
} | {
    type: 'connection_established';
    nodeId: string;
    serverUrl: string;
} | {
    type: 'connection_lost';
    reason: string;
} | {
    type: 'reconnecting';
    attempt: number;
} | {
    type: 'reconciliation_started';
    requestId: string;
} | {
    type: 'reconciliation_completed';
    report: ReconciliationReport;
};
/**
 * Conflict data for manual resolution
 */
export interface ConflictData {
    dataType: ReconciliableDataType;
    recordId: number;
    localVersion: any;
    remoteVersion: any;
    localVectorClock: VectorClock;
    remoteVectorClock: VectorClock;
    detectedAt: number;
}
/**
 * Event handler type
 */
export type SyncEventHandler = (event: SyncEvent) => void;
//# sourceMappingURL=quic.d.ts.map