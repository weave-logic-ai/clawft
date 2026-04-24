/**
 * SyncCoordinator - Orchestrate AgentDB Synchronization
 *
 * Coordinates bidirectional synchronization between local and remote AgentDB instances.
 * Handles change detection, conflict resolution, batching, and progress tracking.
 *
 * Features:
 * - Detect changes since last sync
 * - Bidirectional sync (push and pull)
 * - Conflict resolution strategies
 * - Batch operations for efficiency
 * - Progress tracking and reporting
 * - Comprehensive error handling
 * - Sync state persistence
 */
import { QUICClient } from './QUICClient.js';
import { QUICServer } from './QUICServer.js';
type Database = any;
export interface SyncCoordinatorConfig {
    db: Database;
    client?: QUICClient;
    server?: QUICServer;
    conflictStrategy?: 'local-wins' | 'remote-wins' | 'latest-wins' | 'merge';
    batchSize?: number;
    autoSync?: boolean;
    syncIntervalMs?: number;
}
export interface SyncState {
    lastSyncAt: number;
    lastEpisodeSync: number;
    lastSkillSync: number;
    lastEdgeSync: number;
    totalItemsSynced: number;
    totalBytesSynced: number;
    syncCount: number;
    lastError?: string;
}
export interface SyncProgress {
    phase: 'detecting' | 'pushing' | 'pulling' | 'resolving' | 'applying' | 'completed' | 'error';
    current: number;
    total: number;
    itemType?: string;
    message?: string;
    error?: string;
}
export interface SyncReport {
    success: boolean;
    startTime: number;
    endTime: number;
    durationMs: number;
    itemsPushed: number;
    itemsPulled: number;
    conflictsResolved: number;
    errors: string[];
    bytesTransferred: number;
}
export declare class SyncCoordinator {
    private db;
    private client?;
    private server?;
    private config;
    private syncState;
    private isSyncing;
    private autoSyncInterval;
    constructor(config: SyncCoordinatorConfig);
    /**
     * Perform bidirectional synchronization
     */
    sync(onProgress?: (progress: SyncProgress) => void): Promise<SyncReport>;
    /**
     * Detect changes since last sync
     */
    private detectChanges;
    /**
     * Push local changes to remote
     */
    private pushChanges;
    /**
     * Pull changes from remote
     */
    private pullChanges;
    /**
     * Resolve conflicts between local and remote data
     */
    private resolveConflicts;
    /**
     * Apply pulled changes to local database
     */
    private applyChanges;
    /**
     * Load sync state from database
     */
    private loadSyncState;
    /**
     * Save sync state to database
     */
    private saveSyncState;
    /**
     * Start automatic synchronization
     */
    private startAutoSync;
    /**
     * Stop automatic synchronization
     */
    stopAutoSync(): void;
    /**
     * Get sync state
     */
    getSyncState(): SyncState;
    /**
     * Get sync status
     */
    getStatus(): {
        isSyncing: boolean;
        autoSyncEnabled: boolean;
        state: SyncState;
    };
}
export {};
//# sourceMappingURL=SyncCoordinator.d.ts.map