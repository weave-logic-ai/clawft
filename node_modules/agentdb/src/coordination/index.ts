/**
 * AgentDB Coordination Module
 *
 * Multi-database coordination, synchronization, and distributed operations
 * for AgentDB vector databases.
 *
 * @module coordination
 */

export {
  MultiDatabaseCoordinator,
  // Types
  type DatabaseInstance,
  type InstanceStatus,
  type ConflictResolutionStrategy,
  type SyncOptions,
  type SyncProgress,
  type SyncResult,
  type ConflictInfo,
  type VectorData,
  type MultiDatabaseCoordinatorConfig,
  type StatusChangeCallback,
  type DistributedOperationResult,
} from './MultiDatabaseCoordinator.js';
