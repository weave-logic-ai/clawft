/**
 * IntelligenceStore - SQLite persistence for RuVector intelligence layer
 *
 * Cross-platform (Linux, macOS, Windows) persistent storage for:
 * - Learning trajectories
 * - Routing patterns
 * - SONA adaptations
 * - HNSW vectors
 */
export interface StoredTrajectory {
    id: number;
    taskDescription: string;
    agent: string;
    steps: number;
    outcome: 'success' | 'failure' | 'partial';
    startTime: number;
    endTime: number;
    metadata?: string;
}
export interface StoredPattern {
    id: number;
    taskType: string;
    approach: string;
    embedding: Buffer;
    similarity: number;
    usageCount: number;
    successRate: number;
    createdAt: number;
    updatedAt: number;
}
export interface StoredRouting {
    id: number;
    task: string;
    recommendedAgent: string;
    confidence: number;
    latencyMs: number;
    wasSuccessful: boolean;
    timestamp: number;
}
export interface LearningStats {
    totalTrajectories: number;
    successfulTrajectories: number;
    totalRoutings: number;
    successfulRoutings: number;
    totalPatterns: number;
    sonaAdaptations: number;
    hnswQueries: number;
    lastUpdated: number;
}
export declare class IntelligenceStore {
    private db;
    private static instance;
    private constructor();
    /**
     * Get singleton instance
     */
    static getInstance(dbPath?: string): IntelligenceStore;
    /**
     * Get default database path (cross-platform)
     */
    static getDefaultPath(): string;
    /**
     * Initialize database schema
     */
    private initSchema;
    /**
     * Start a new trajectory
     */
    startTrajectory(taskDescription: string, agent: string): number;
    /**
     * Add step to trajectory
     */
    addTrajectoryStep(trajectoryId: number): void;
    /**
     * End trajectory with outcome
     */
    endTrajectory(trajectoryId: number, outcome: 'success' | 'failure' | 'partial', metadata?: Record<string, any>): void;
    /**
     * Get active trajectories (no end_time)
     */
    getActiveTrajectories(): StoredTrajectory[];
    /**
     * Get recent trajectories
     */
    getRecentTrajectories(limit?: number): StoredTrajectory[];
    /**
     * Store a pattern
     */
    storePattern(taskType: string, approach: string, embedding?: Float32Array): number;
    /**
     * Update pattern usage
     */
    updatePatternUsage(patternId: number, wasSuccessful: boolean): void;
    /**
     * Find patterns by task type
     */
    findPatterns(taskType: string, limit?: number): StoredPattern[];
    /**
     * Record a routing decision
     */
    recordRouting(task: string, recommendedAgent: string, confidence: number, latencyMs: number): number;
    /**
     * Update routing outcome
     */
    updateRoutingOutcome(routingId: number, wasSuccessful: boolean): void;
    /**
     * Get routing accuracy for an agent
     */
    getAgentAccuracy(agent: string): {
        total: number;
        successful: number;
        accuracy: number;
    };
    /**
     * Get all stats
     */
    getStats(): LearningStats;
    /**
     * Increment a stat counter
     */
    incrementStat(statName: string, amount?: number): void;
    /**
     * Record SONA adaptation
     */
    recordSonaAdaptation(): void;
    /**
     * Record HNSW query
     */
    recordHnswQuery(): void;
    /**
     * Get summary for display (simplified for UI)
     */
    getSummary(): {
        trajectories: number;
        routings: number;
        patterns: number;
        operations: number;
    };
    /**
     * Get detailed summary for reports
     */
    getDetailedSummary(): {
        trajectories: {
            total: number;
            active: number;
            successful: number;
        };
        routings: {
            total: number;
            accuracy: number;
        };
        patterns: number;
        operations: {
            sona: number;
            hnsw: number;
        };
    };
    /**
     * Close database connection
     */
    close(): void;
    /**
     * Reset all data (for testing)
     */
    reset(): void;
}
export declare function getIntelligenceStore(dbPath?: string): IntelligenceStore;
//# sourceMappingURL=IntelligenceStore.d.ts.map