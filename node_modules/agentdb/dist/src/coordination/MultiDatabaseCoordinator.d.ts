/**
 * MultiDatabaseCoordinator - Cross-Instance Synchronization and Coordination
 *
 * Orchestrates multiple AgentDB instances for distributed vector database operations.
 * Provides:
 * - Instance registration and lifecycle management
 * - Cross-instance synchronization with conflict resolution
 * - Health monitoring and automatic failover
 * - Distributed operations (broadcast insert/delete)
 * - Configurable replication strategies
 *
 * @module coordination/MultiDatabaseCoordinator
 */
import type { VectorBackend, VectorStats } from '../backends/VectorBackend.js';
/**
 * Status of a database instance
 */
export type InstanceStatus = 'online' | 'offline' | 'syncing';
/**
 * Conflict resolution strategies for synchronization
 */
export type ConflictResolutionStrategy = 'last-write-wins' | 'merge' | 'manual';
/**
 * Represents a remote database instance
 */
export interface DatabaseInstance {
    /** Unique identifier for this instance */
    id: string;
    /** URL or connection string for the instance */
    url: string;
    /** Current status of the instance */
    status: InstanceStatus;
    /** Timestamp of last successful sync */
    lastSyncAt: number;
    /** Number of vectors in this instance */
    vectorCount: number;
    /** Version string for compatibility checking */
    version: string;
    /** Optional metadata about the instance */
    metadata?: Record<string, any>;
}
/**
 * Options for synchronization operations
 */
export interface SyncOptions {
    /** Conflict resolution strategy */
    conflictResolution?: ConflictResolutionStrategy;
    /** Maximum number of items per batch */
    batchSize?: number;
    /** Timeout in milliseconds for sync operation */
    timeoutMs?: number;
    /** Whether to force full sync (ignore lastSyncAt) */
    forceFullSync?: boolean;
    /** Filter to sync only specific namespaces */
    namespaceFilter?: string[];
    /** Progress callback */
    onProgress?: (progress: SyncProgress) => void;
}
/**
 * Progress information during synchronization
 */
export interface SyncProgress {
    /** Current phase of synchronization */
    phase: 'preparing' | 'fetching' | 'comparing' | 'resolving' | 'applying' | 'completed' | 'error';
    /** Items processed so far */
    current: number;
    /** Total items to process */
    total: number;
    /** Optional message */
    message?: string;
    /** Error message if phase is 'error' */
    error?: string;
}
/**
 * Result of a synchronization operation
 */
export interface SyncResult {
    /** Whether sync completed successfully */
    success: boolean;
    /** Instance ID that was synced */
    instanceId: string;
    /** Number of items synced */
    itemsSynced: number;
    /** Number of conflicts detected */
    conflictsDetected: number;
    /** Number of conflicts resolved */
    conflictsResolved: number;
    /** Duration in milliseconds */
    durationMs: number;
    /** Bytes transferred */
    bytesTransferred: number;
    /** Error message if failed */
    error?: string;
    /** Detailed conflict information (for manual resolution) */
    unresolvedConflicts?: ConflictInfo[];
}
/**
 * Information about a sync conflict
 */
export interface ConflictInfo {
    /** Vector ID with conflict */
    vectorId: string;
    /** Local vector data */
    local: VectorData;
    /** Remote vector data */
    remote: VectorData;
    /** Suggested resolution */
    suggestion: 'keep-local' | 'keep-remote' | 'merge';
}
/**
 * Vector data for conflict resolution
 */
export interface VectorData {
    /** Vector embedding */
    embedding: Float32Array;
    /** Associated metadata */
    metadata?: Record<string, any>;
    /** Last modified timestamp */
    timestamp: number;
}
/**
 * Configuration for MultiDatabaseCoordinator
 */
export interface MultiDatabaseCoordinatorConfig {
    /** Replication factor (how many instances should have each vector) */
    replicationFactor?: number;
    /** Interval between automatic syncs (0 to disable) */
    syncIntervalMs?: number;
    /** Default conflict resolution strategy */
    conflictResolution?: ConflictResolutionStrategy;
    /** Health check interval in milliseconds */
    healthCheckIntervalMs?: number;
    /** Timeout for health checks */
    healthCheckTimeoutMs?: number;
    /** Whether to enable automatic failover */
    autoFailover?: boolean;
    /** Maximum retries for failed operations */
    maxRetries?: number;
    /** Retry delay in milliseconds */
    retryDelayMs?: number;
}
/**
 * Callback for instance status changes
 */
export type StatusChangeCallback = (instanceId: string, oldStatus: InstanceStatus, newStatus: InstanceStatus) => void;
/**
 * Result of an operation executed on all instances
 */
export interface DistributedOperationResult<T> {
    /** Results keyed by instance ID */
    results: Map<string, T>;
    /** Errors keyed by instance ID */
    errors: Map<string, Error>;
    /** Number of successful operations */
    successCount: number;
    /** Number of failed operations */
    failureCount: number;
}
/**
 * MultiDatabaseCoordinator - Manages multiple AgentDB instances
 *
 * Provides coordination, synchronization, and health monitoring for
 * distributed vector database deployments.
 *
 * @example
 * ```typescript
 * const coordinator = new MultiDatabaseCoordinator(primaryDb, {
 *   replicationFactor: 2,
 *   syncIntervalMs: 30000,
 *   conflictResolution: 'last-write-wins'
 * });
 *
 * // Register secondary instances
 * coordinator.registerInstance({
 *   id: 'secondary-1',
 *   url: 'http://db1.example.com:8080',
 *   status: 'online',
 *   lastSyncAt: 0,
 *   vectorCount: 0,
 *   version: '2.0.0'
 * });
 *
 * // Start health monitoring
 * coordinator.startHealthCheck(5000);
 *
 * // Sync all instances
 * const results = await coordinator.syncAll();
 * ```
 */
export declare class MultiDatabaseCoordinator {
    private primaryDb;
    private instances;
    private config;
    private healthCheckInterval;
    private syncInterval;
    private statusChangeCallbacks;
    private isSyncing;
    private vectorTimestamps;
    /**
     * Create a new MultiDatabaseCoordinator
     *
     * @param primaryDb - Primary vector database backend
     * @param config - Configuration options
     */
    constructor(primaryDb: VectorBackend, config?: MultiDatabaseCoordinatorConfig);
    /**
     * Register a new database instance
     *
     * @param instance - Instance configuration
     * @throws Error if instance with same ID already exists
     */
    registerInstance(instance: DatabaseInstance): void;
    /**
     * Unregister a database instance
     *
     * @param id - Instance ID to unregister
     * @throws Error if instance is currently syncing
     */
    unregisterInstance(id: string): void;
    /**
     * Get all registered instances
     *
     * @returns Array of database instances
     */
    getInstances(): DatabaseInstance[];
    /**
     * Get a specific instance by ID
     *
     * @param id - Instance ID
     * @returns Instance or null if not found
     */
    getInstanceStatus(id: string): DatabaseInstance | null;
    /**
     * Get only online instances
     *
     * @returns Array of online instances
     */
    getOnlineInstances(): DatabaseInstance[];
    /**
     * Update instance status
     *
     * @param id - Instance ID
     * @param status - New status
     */
    private updateInstanceStatus;
    /**
     * Synchronize data TO a remote instance
     *
     * @param targetId - Target instance ID
     * @param options - Sync options
     * @returns Sync result
     */
    syncToInstance(targetId: string, options?: SyncOptions): Promise<SyncResult>;
    /**
     * Synchronize data FROM a remote instance
     *
     * @param sourceId - Source instance ID
     * @param options - Sync options
     * @returns Sync result
     */
    syncFromInstance(sourceId: string, options?: SyncOptions): Promise<SyncResult>;
    /**
     * Synchronize all registered instances
     *
     * @param options - Sync options
     * @returns Map of instance ID to sync result
     */
    syncAll(options?: SyncOptions): Promise<Map<string, SyncResult>>;
    /**
     * Start periodic health checks
     *
     * @param intervalMs - Check interval in milliseconds (overrides config)
     */
    startHealthCheck(intervalMs?: number): void;
    /**
     * Stop periodic health checks
     */
    stopHealthCheck(): void;
    /**
     * Perform health check on all instances
     */
    private performHealthCheck;
    /**
     * Check if a specific instance is healthy
     */
    private checkInstanceHealth;
    /**
     * Register a callback for instance status changes
     *
     * @param callback - Callback function
     * @returns Unsubscribe function
     */
    onInstanceStatusChange(callback: StatusChangeCallback): () => void;
    /**
     * Broadcast an insert operation to all online instances
     *
     * @param id - Vector ID
     * @param vector - Vector embedding
     * @param metadata - Optional metadata
     */
    broadcastInsert(id: string, vector: Float32Array, metadata?: Record<string, any>): Promise<void>;
    /**
     * Broadcast a delete operation to all online instances
     *
     * @param id - Vector ID to delete
     */
    broadcastDelete(id: string): Promise<void>;
    /**
     * Execute an operation on all instances (including primary)
     *
     * @param operation - Operation to execute
     * @returns Map of instance ID to result
     */
    executeOnAll<T>(operation: (db: VectorBackend, instanceId: string) => Promise<T>): Promise<DistributedOperationResult<T>>;
    /**
     * Get current configuration
     */
    getConfig(): Readonly<Required<MultiDatabaseCoordinatorConfig>>;
    /**
     * Update configuration
     *
     * @param updates - Partial configuration updates
     */
    updateConfig(updates: Partial<MultiDatabaseCoordinatorConfig>): void;
    /**
     * Get coordinator statistics
     */
    getStats(): {
        primaryStats: VectorStats;
        instanceCount: number;
        onlineCount: number;
        offlineCount: number;
        syncingCount: number;
        totalVectors: number;
    };
    /**
     * Stop all background tasks and cleanup
     */
    close(): void;
    private startAutoSync;
    private stopAutoSync;
    private mergeOptions;
    private getLocalVectorIds;
    private createBatches;
    private selectReplicationTargets;
    private delay;
}
//# sourceMappingURL=MultiDatabaseCoordinator.d.ts.map