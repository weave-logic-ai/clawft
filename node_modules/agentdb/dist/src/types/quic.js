/**
 * QUIC Synchronization Types for AgentDB
 *
 * This file contains TypeScript interfaces and types for the QUIC-based
 * multi-node synchronization system. These types mirror the Protocol Buffer
 * definitions and provide type safety for the sync implementation.
 */
// ============================================================================
// Episode Synchronization
// ============================================================================
/**
 * Supported operations for episode sync
 */
export var EpisodeSyncOperation;
(function (EpisodeSyncOperation) {
    EpisodeSyncOperation["CREATE"] = "CREATE";
    EpisodeSyncOperation["UPDATE"] = "UPDATE";
    EpisodeSyncOperation["DELETE"] = "DELETE";
})(EpisodeSyncOperation || (EpisodeSyncOperation = {}));
/**
 * User roles
 */
export var UserRole;
(function (UserRole) {
    UserRole["ADMIN"] = "admin";
    UserRole["AGENT"] = "agent";
    UserRole["OBSERVER"] = "observer";
    UserRole["LEARNER"] = "learner";
})(UserRole || (UserRole = {}));
// ============================================================================
// Configuration
// ============================================================================
/**
 * Network topology types
 */
export var NetworkTopology;
(function (NetworkTopology) {
    NetworkTopology["HUB_AND_SPOKE"] = "hub_and_spoke";
    NetworkTopology["MESH"] = "mesh";
    NetworkTopology["HIERARCHICAL"] = "hierarchical";
})(NetworkTopology || (NetworkTopology = {}));
/**
 * Conflict resolution strategies
 */
export var ConflictResolutionStrategy;
(function (ConflictResolutionStrategy) {
    ConflictResolutionStrategy["AUTO"] = "auto";
    ConflictResolutionStrategy["MANUAL"] = "manual";
    ConflictResolutionStrategy["INTERACTIVE"] = "interactive"; // Prompt user for resolution
})(ConflictResolutionStrategy || (ConflictResolutionStrategy = {}));
/**
 * Sync mode
 */
export var SyncMode;
(function (SyncMode) {
    SyncMode["INCREMENTAL"] = "incremental";
    SyncMode["FULL"] = "full";
    SyncMode["HYBRID"] = "hybrid";
})(SyncMode || (SyncMode = {}));
// ============================================================================
// Helper Functions (Type Guards & Utilities)
// ============================================================================
/**
 * Type guard for episode sync
 */
export function isEpisodeSync(payload) {
    return payload.type === 'episode_sync';
}
/**
 * Type guard for skill sync
 */
export function isSkillSync(payload) {
    return payload.type === 'skill_sync';
}
/**
 * Type guard for causal edge sync
 */
export function isCausalEdgeSync(payload) {
    return payload.type === 'causal_edge_sync';
}
/**
 * Compare two vector clocks
 */
export function compareVectorClocks(a, b) {
    const allNodes = new Set([...a.clocks.keys(), ...b.clocks.keys()]);
    let aGreater = false;
    let bGreater = false;
    for (const node of allNodes) {
        const aClock = a.clocks.get(node) || 0;
        const bClock = b.clocks.get(node) || 0;
        if (aClock > bClock)
            aGreater = true;
        if (bClock > aClock)
            bGreater = true;
    }
    if (aGreater && !bGreater)
        return 'after';
    if (bGreater && !aGreater)
        return 'before';
    if (!aGreater && !bGreater)
        return 'equal';
    return 'concurrent';
}
/**
 * Merge two vector clocks (take max of each node)
 */
export function mergeVectorClocks(a, b) {
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
export function incrementVectorClock(clock, nodeId) {
    const newClocks = new Map(clock.clocks);
    const currentClock = newClocks.get(nodeId) || 0;
    newClocks.set(nodeId, currentClock + 1);
    return { clocks: newClocks };
}
/**
 * Create empty vector clock
 */
export function createVectorClock() {
    return { clocks: new Map() };
}
// ============================================================================
// CRDT Operations
// ============================================================================
/**
 * Increment G-Counter for a node
 */
export function incrementGCounter(counter, nodeId, delta = 1) {
    const newCounters = new Map(counter.nodeCounters);
    const current = newCounters.get(nodeId) || 0;
    newCounters.set(nodeId, current + delta);
    return { nodeCounters: newCounters };
}
/**
 * Get total value of G-Counter
 */
export function getGCounterValue(counter) {
    return Array.from(counter.nodeCounters.values()).reduce((sum, count) => sum + count, 0);
}
/**
 * Merge two G-Counters (take max per node)
 */
export function mergeGCounter(a, b) {
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
export function updateLWWRegister(register, newValue, nodeId, timestamp = Date.now()) {
    if (timestamp > register.timestamp ||
        (timestamp === register.timestamp && nodeId > register.nodeId)) {
        return { value: newValue, timestamp, nodeId };
    }
    return register;
}
/**
 * Merge two LWW-Registers (keep most recent)
 */
export function mergeLWWRegister(a, b) {
    if (b.timestamp > a.timestamp) {
        return b;
    }
    else if (b.timestamp === a.timestamp) {
        return b.nodeId > a.nodeId ? b : a;
    }
    return a;
}
/**
 * Add element to OR-Set
 */
export function addToORSet(set, element, uniqueTag) {
    const newAdds = new Map(set.adds);
    if (!newAdds.has(element)) {
        newAdds.set(element, new Set());
    }
    newAdds.get(element).add(uniqueTag);
    return { adds: newAdds, removes: set.removes };
}
/**
 * Remove element from OR-Set
 */
export function removeFromORSet(set, element) {
    const tags = set.adds.get(element);
    if (!tags)
        return set;
    const newRemoves = new Set(set.removes);
    tags.forEach(tag => newRemoves.add(tag));
    return { adds: set.adds, removes: newRemoves };
}
/**
 * Get current elements in OR-Set
 */
export function getORSetElements(set) {
    const elements = new Set();
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
export function mergeORSet(a, b) {
    const mergedAdds = new Map();
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
export function weightedAverage(v1, w1, v2, w2) {
    if (w1 + w2 === 0)
        return 0;
    return (v1 * w1 + v2 * w2) / (w1 + w2);
}
/**
 * Determine authorization for operation
 */
export function isAuthorized(jwt, requiredScope) {
    return jwt.scopes.includes(requiredScope);
}
/**
 * Check if JWT is expired
 */
export function isJWTExpired(jwt) {
    return Date.now() >= jwt.exp * 1000;
}
/**
 * Generate unique tag for OR-Set operations
 */
export function generateUniqueTag(nodeId, timestamp = Date.now()) {
    return `${nodeId}-${timestamp}-${Math.random().toString(36).substr(2, 9)}`;
}
//# sourceMappingURL=quic.js.map