/**
 * Database queries for ReasoningBank
 * Operates on Claude Flow's memory.db at .swarm/memory.db
 */
import Database from 'better-sqlite3';
import type { ReasoningMemory, PatternEmbedding, TaskTrajectory, MattsRun } from './schema.js';
/**
 * Run database migrations (create tables)
 */
export declare function runMigrations(): Promise<void>;
/**
 * Get database connection (singleton)
 */
export declare function getDb(): Database.Database;
/**
 * Fetch reasoning memory candidates for retrieval
 */
export declare function fetchMemoryCandidates(options: {
    domain?: string;
    agent?: string;
    minConfidence?: number;
}): Array<ReasoningMemory & {
    embedding: Float32Array;
    age_days: number;
}>;
/**
 * Store a new reasoning memory
 */
export declare function upsertMemory(memory: Omit<ReasoningMemory, 'created_at' | 'last_used'>): string;
/**
 * Store embedding for a memory
 */
export declare function upsertEmbedding(embedding: PatternEmbedding): void;
/**
 * Increment usage count for a memory
 */
export declare function incrementUsage(memoryId: string): void;
/**
 * Store task trajectory
 */
export declare function storeTrajectory(trajectory: Omit<TaskTrajectory, 'created_at'>): void;
/**
 * Store MaTTS run
 */
export declare function storeMattsRun(run: Omit<MattsRun, 'created_at'>): void;
/**
 * Log performance metric
 */
export declare function logMetric(name: string, value: number): void;
/**
 * Count new memories since last consolidation
 */
export declare function countNewMemoriesSinceConsolidation(): number;
/**
 * Get all active reasoning memories
 */
export declare function getAllActiveMemories(): ReasoningMemory[];
/**
 * Store memory link (relationship)
 */
export declare function storeLink(srcId: string, dstId: string, relation: 'entails' | 'contradicts' | 'refines' | 'duplicate_of', weight: number): void;
/**
 * Get contradictions for a memory
 */
export declare function getContradictions(memoryId: string): string[];
/**
 * Store consolidation run
 */
export declare function storeConsolidationRun(run: {
    run_id: string;
    items_processed: number;
    duplicates_found: number;
    contradictions_found: number;
    items_pruned: number;
    duration_ms: number;
}): void;
/**
 * Prune old, unused memories
 */
export declare function pruneOldMemories(options: {
    maxAgeDays: number;
    minConfidence: number;
}): number;
/**
 * Close database connection
 */
export declare function closeDb(): void;
//# sourceMappingURL=queries.d.ts.map