/**
 * QUIC Synchronization Types for AgentDB
 *
 * This file contains TypeScript interfaces and types for the QUIC-based
 * multi-node synchronization system. These types mirror the Protocol Buffer
 * definitions and provide type safety for the sync implementation.
 */

// ============================================================================
// Core Sync Types
// ============================================================================

/**
 * Vector clock for causal ordering of events across distributed nodes.
 * Maps node IDs to their logical clock values.
 */
export interface VectorClock {
  clocks: Map<string, number>;  // node_id -> logical_clock
}

/**
 * Compares two vector clocks to determine causal relationship
 */
export type VectorClockComparison =
  | 'before'      // local happened before remote
  | 'after'       // local happened after remote
  | 'concurrent'  // concurrent events (conflict)
  | 'equal';      // identical clocks

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
export type SyncPayload =
  | { type: 'episode_sync'; data: EpisodeSync }
  | { type: 'skill_sync'; data: SkillSync }
  | { type: 'causal_edge_sync'; data: CausalEdgeSync }
  | { type: 'reconciliation_request'; data: FullReconciliationRequest }
  | { type: 'reconciliation_response'; data: FullReconciliationResponse };

// ============================================================================
// Episode Synchronization
// ============================================================================

/**
 * Supported operations for episode sync
 */
export enum EpisodeSyncOperation {
  CREATE = 'CREATE',
  UPDATE = 'UPDATE',
  DELETE = 'DELETE'
}

/**
 * Episode synchronization message
 */
export interface EpisodeSync {
  operation: EpisodeSyncOperation;
  episodeId: number;
  episodeData: SyncableEpisode;
  causalClock: VectorClock;
  signature: Uint8Array;  // HMAC for integrity verification
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

// ============================================================================
// Skill Synchronization (CRDT-based)
// ============================================================================

/**
 * G-Counter (Grow-only Counter) for skill usage tracking
 */
export interface GCounter {
  nodeCounters: Map<string, number>;  // node_id -> local_count
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
  adds: Map<T, Set<string>>;  // element -> set of unique tags
  removes: Set<string>;        // set of removed tags
}

/**
 * Skill synchronization message with CRDT fields
 */
export interface SkillSync {
  skillId: number;
  skillName: string;
  description?: string;

  // CRDT fields
  uses: GCounter;                      // Total uses across nodes
  successRate: LWWRegister<number>;    // Success rate with timestamp
  avgReward: LWWRegister<number>;      // Average reward with timestamp
  avgLatencyMs: LWWRegister<number>;   // Average latency with timestamp
  sourceEpisodes: ORSet<number>;       // Set of source episode IDs

  // Metadata
  signature: Record<string, any>;      // Skill signature (inputs/outputs)
  version: VectorClock;
  metadata?: Record<string, any>;
}

// ============================================================================
// Causal Edge Synchronization
// ============================================================================

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

  // Causal metrics
  similarity: number;
  uplift?: number;
  confidence: number;
  sampleSize?: number;

  // Evidence and explanation
  evidenceIds?: number[];
  experimentIds?: number[];
  confounderScore?: number;
  mechanism?: string;

  // Sync metadata
  version: VectorClock;
  conflictMetadata: ConflictResolutionMetadata;
  metadata?: Record<string, any>;
}

// ============================================================================
// Full Reconciliation
// ============================================================================

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
  merkleRoot: string;  // For verification
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

// ============================================================================
// Authentication & Authorization
// ============================================================================

/**
 * JWT claims for API authorization
 */
export interface JWTClaims {
  iss: string;              // Issuer
  sub: string;              // Subject (node ID)
  exp: number;              // Expiration timestamp
  iat: number;              // Issued at timestamp
  roles: UserRole[];
  scopes: AuthScope[];
  networkId: string;
  metadata?: Record<string, any>;
}

/**
 * User roles
 */
export enum UserRole {
  ADMIN = 'admin',
  AGENT = 'agent',
  OBSERVER = 'observer',
  LEARNER = 'learner'
}

/**
 * Authorization scopes
 */
export type AuthScope =
  | 'episodes:read'
  | 'episodes:write'
  | 'episodes:delete'
  | 'skills:read'
  | 'skills:write'
  | 'skills:delete'
  | 'edges:read'
  | 'edges:write'
  | 'edges:delete'
  | 'experiments:read'
  | 'experiments:write'
  | 'reconciliation:request';

/**
 * Node registration data
 */
export interface NodeRegistration {
  nodeId: string;
  certificate: string;      // PEM-encoded X.509 certificate
  publicKey: string;        // PEM-encoded public key
  networkId: string;
  registeredAt: number;
  expiresAt: number;
}

// ============================================================================
// Configuration
// ============================================================================

/**
 * Network topology types
 */
export enum NetworkTopology {
  HUB_AND_SPOKE = 'hub_and_spoke',
  MESH = 'mesh',
  HIERARCHICAL = 'hierarchical'
}

/**
 * Conflict resolution strategies
 */
export enum ConflictResolutionStrategy {
  AUTO = 'auto',          // Automatic resolution using configured algorithms
  MANUAL = 'manual',      // Flag conflicts for manual resolution
  INTERACTIVE = 'interactive'  // Prompt user for resolution
}

/**
 * Sync mode
 */
export enum SyncMode {
  INCREMENTAL = 'incremental',
  FULL = 'full',
  HYBRID = 'hybrid'
}

/**
 * Server configuration
 */
export interface ServerConfig {
  port: number;
  host: string;
  maxConnections: number;
  maxStreamsPerConnection: number;

  // TLS/Security
  tlsCertPath: string;
  tlsKeyPath: string;
  caCertPath: string;
  jwtSecret: string;
  jwtExpirationMs: number;

  // Sync settings
  changelogRetentionDays: number;
  changelogMaxRecords: number;
  reconciliationIntervalMs: number;

  // Performance
  batchSize: number;
  compressionThreshold: number;
  maxMemoryPerConnection: number;

  // Topology
  topology: NetworkTopology;
  networkId: string;
}

/**
 * Client configuration
 */
export interface ClientConfig {
  nodeId: string;
  serverUrl: string;

  // TLS/Security
  clientCertPath: string;
  clientKeyPath: string;
  caCertPath: string;
  jwt: string;

  // Sync settings
  mode: SyncMode;
  incrementalIntervalMs: number;
  fullReconciliationIntervalMs: number;
  autoSync: boolean;

  // Conflict resolution
  conflictResolutionStrategy: ConflictResolutionStrategy;

  // Performance
  batchSize: number;
  compressionThreshold: number;
  retryMaxAttempts: number;
  retryBackoffMs: number;
}

// ============================================================================
// Status & Monitoring
// ============================================================================

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

  // Performance metrics
  avgSyncLatencyMs: number;
  throughputBytesPerSec: number;
  conflictsPerMinute: number;

  // Resource usage
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

  // Sync stats
  episodesSynced: number;
  skillsSynced: number;
  edgesSynced: number;
  conflictsEncountered: number;
  conflictsAutoResolved: number;

  // Connection health
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

  // Changes applied
  episodesAdded: number;
  episodesUpdated: number;
  episodesDeleted: number;
  skillsAdded: number;
  skillsUpdated: number;
  edgesAdded: number;
  edgesUpdated: number;

  // Conflicts
  conflictsTotal: number;
  conflictsAutoResolved: number;
  conflictsPending: number;

  // Errors
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

// ============================================================================
// Helper Functions (Type Guards & Utilities)
// ============================================================================

/**
 * Type guard for episode sync
 */
export function isEpisodeSync(payload: SyncPayload): payload is { type: 'episode_sync'; data: EpisodeSync } {
  return payload.type === 'episode_sync';
}

/**
 * Type guard for skill sync
 */
export function isSkillSync(payload: SyncPayload): payload is { type: 'skill_sync'; data: SkillSync } {
  return payload.type === 'skill_sync';
}

/**
 * Type guard for causal edge sync
 */
export function isCausalEdgeSync(payload: SyncPayload): payload is { type: 'causal_edge_sync'; data: CausalEdgeSync } {
  return payload.type === 'causal_edge_sync';
}

/**
 * Compare two vector clocks
 */
export function compareVectorClocks(a: VectorClock, b: VectorClock): VectorClockComparison {
  const allNodes = new Set([...a.clocks.keys(), ...b.clocks.keys()]);

  let aGreater = false;
  let bGreater = false;

  for (const node of allNodes) {
    const aClock = a.clocks.get(node) || 0;
    const bClock = b.clocks.get(node) || 0;

    if (aClock > bClock) aGreater = true;
    if (bClock > aClock) bGreater = true;
  }

  if (aGreater && !bGreater) return 'after';
  if (bGreater && !aGreater) return 'before';
  if (!aGreater && !bGreater) return 'equal';
  return 'concurrent';
}

/**
 * Merge two vector clocks (take max of each node)
 */
export function mergeVectorClocks(a: VectorClock, b: VectorClock): VectorClock {
  const merged = new Map(a.clocks);

  for (const [node, clock] of b.clocks) {
    const existingClock = merged.get(node) || 0;
    merged.set(node, Math.max(existingClock, clock));
  }

  return { clocks: merged };
}

/**
 * Increment vector clock for local node
 */
export function incrementVectorClock(clock: VectorClock, nodeId: string): VectorClock {
  const newClocks = new Map(clock.clocks);
  const currentClock = newClocks.get(nodeId) || 0;
  newClocks.set(nodeId, currentClock + 1);
  return { clocks: newClocks };
}

/**
 * Create empty vector clock
 */
export function createVectorClock(): VectorClock {
  return { clocks: new Map() };
}

// ============================================================================
// CRDT Operations
// ============================================================================

/**
 * Increment G-Counter for a node
 */
export function incrementGCounter(counter: GCounter, nodeId: string, delta: number = 1): GCounter {
  const newCounters = new Map(counter.nodeCounters);
  const current = newCounters.get(nodeId) || 0;
  newCounters.set(nodeId, current + delta);
  return { nodeCounters: newCounters };
}

/**
 * Get total value of G-Counter
 */
export function getGCounterValue(counter: GCounter): number {
  return Array.from(counter.nodeCounters.values()).reduce((sum, count) => sum + count, 0);
}

/**
 * Merge two G-Counters (take max per node)
 */
export function mergeGCounter(a: GCounter, b: GCounter): GCounter {
  const merged = new Map(a.nodeCounters);

  for (const [nodeId, count] of b.nodeCounters) {
    const existingCount = merged.get(nodeId) || 0;
    merged.set(nodeId, Math.max(existingCount, count));
  }

  return { nodeCounters: merged };
}

/**
 * Update LWW-Register with new value
 */
export function updateLWWRegister<T>(
  register: LWWRegister<T>,
  newValue: T,
  nodeId: string,
  timestamp: number = Date.now()
): LWWRegister<T> {
  if (timestamp > register.timestamp ||
      (timestamp === register.timestamp && nodeId > register.nodeId)) {
    return { value: newValue, timestamp, nodeId };
  }
  return register;
}

/**
 * Merge two LWW-Registers (keep most recent)
 */
export function mergeLWWRegister<T>(a: LWWRegister<T>, b: LWWRegister<T>): LWWRegister<T> {
  if (b.timestamp > a.timestamp) {
    return b;
  } else if (b.timestamp === a.timestamp) {
    return b.nodeId > a.nodeId ? b : a;
  }
  return a;
}

/**
 * Add element to OR-Set
 */
export function addToORSet<T>(set: ORSet<T>, element: T, uniqueTag: string): ORSet<T> {
  const newAdds = new Map(set.adds);
  if (!newAdds.has(element)) {
    newAdds.set(element, new Set());
  }
  newAdds.get(element)!.add(uniqueTag);

  return { adds: newAdds, removes: set.removes };
}

/**
 * Remove element from OR-Set
 */
export function removeFromORSet<T>(set: ORSet<T>, element: T): ORSet<T> {
  const tags = set.adds.get(element);
  if (!tags) return set;

  const newRemoves = new Set(set.removes);
  tags.forEach(tag => newRemoves.add(tag));

  return { adds: set.adds, removes: newRemoves };
}

/**
 * Get current elements in OR-Set
 */
export function getORSetElements<T>(set: ORSet<T>): Set<T> {
  const elements = new Set<T>();

  for (const [element, tags] of set.adds) {
    // Check if any tag is not in removes
    for (const tag of tags) {
      if (!set.removes.has(tag)) {
        elements.add(element);
        break;
      }
    }
  }

  return elements;
}

/**
 * Merge two OR-Sets
 */
export function mergeORSet<T>(a: ORSet<T>, b: ORSet<T>): ORSet<T> {
  const mergedAdds = new Map<T, Set<string>>();
  const mergedRemoves = new Set([...a.removes, ...b.removes]);

  // Merge adds from both sets
  const allElements = new Set([...a.adds.keys(), ...b.adds.keys()]);

  for (const element of allElements) {
    const aTags = a.adds.get(element) || new Set();
    const bTags = b.adds.get(element) || new Set();
    const mergedTags = new Set([...aTags, ...bTags]);

    // Remove tags that are in removes set
    for (const tag of mergedTags) {
      if (mergedRemoves.has(tag)) {
        mergedTags.delete(tag);
      }
    }

    if (mergedTags.size > 0) {
      mergedAdds.set(element, mergedTags);
    }
  }

  return { adds: mergedAdds, removes: mergedRemoves };
}

// ============================================================================
// Conflict Resolution Helpers
// ============================================================================

/**
 * Weighted average for numeric conflict resolution
 */
export function weightedAverage(v1: number, w1: number, v2: number, w2: number): number {
  if (w1 + w2 === 0) return 0;
  return (v1 * w1 + v2 * w2) / (w1 + w2);
}

/**
 * Determine authorization for operation
 */
export function isAuthorized(jwt: JWTClaims, requiredScope: AuthScope): boolean {
  return jwt.scopes.includes(requiredScope);
}

/**
 * Check if JWT is expired
 */
export function isJWTExpired(jwt: JWTClaims): boolean {
  return Date.now() >= jwt.exp * 1000;
}

/**
 * Generate unique tag for OR-Set operations
 */
export function generateUniqueTag(nodeId: string, timestamp: number = Date.now()): string {
  return `${nodeId}-${timestamp}-${Math.random().toString(36).substr(2, 9)}`;
}

// ============================================================================
// Event Types for Client SDK
// ============================================================================

/**
 * Events emitted by sync client
 */
export type SyncEvent =
  | { type: 'sync_started'; timestamp: number }
  | { type: 'sync_completed'; result: SyncResult }
  | { type: 'sync_failed'; error: SyncError }
  | { type: 'conflict_detected'; conflict: ConflictData }
  | { type: 'conflict_resolved'; conflict: ConflictData; resolution: any }
  | { type: 'connection_established'; nodeId: string; serverUrl: string }
  | { type: 'connection_lost'; reason: string }
  | { type: 'reconnecting'; attempt: number }
  | { type: 'reconciliation_started'; requestId: string }
  | { type: 'reconciliation_completed'; report: ReconciliationReport };

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
