/**
 * Report Store (SQLite)
 *
 * Persistent storage for simulation results with queryable history,
 * trend analysis, and comparison tools.
 */
import { SimulationResult } from './simulation-registry';
export interface ComparisonReport {
    scenarios: string[];
    metrics: Record<string, {
        values: number[];
        best: number;
        worst: number;
        average: number;
        winner: number;
    }>;
    insights: string[];
}
export interface TrendData {
    metric: string;
    points: Array<{
        timestamp: Date;
        value: number;
        runId: number;
    }>;
    trend: 'improving' | 'degrading' | 'stable';
    slope?: number;
}
export interface Regression {
    metric: string;
    baseline: number;
    current: number;
    degradation: number;
    severity: 'minor' | 'major' | 'critical';
    firstDetected: Date;
    affectedRuns: number[];
}
export declare class ReportStore {
    private db;
    private dbPath;
    constructor(dbPath?: string);
    /**
     * Initialize database connection and create schema.
     */
    initialize(): Promise<void>;
    /**
     * Create database schema.
     */
    private createSchema;
    /**
     * Close database connection.
     */
    close(): Promise<void>;
    /**
     * Save a simulation result.
     */
    save(result: SimulationResult): Promise<number>;
    /**
     * Get simulation by ID.
     */
    get(id: number): Promise<SimulationResult | null>;
    /**
     * List recent simulations.
     */
    list(limit?: number): Promise<SimulationResult[]>;
    /**
     * Find simulations by scenario ID.
     */
    findByScenario(scenarioId: string): Promise<SimulationResult[]>;
    /**
     * Compare multiple simulation runs.
     */
    compare(ids: number[]): Promise<ComparisonReport>;
    /**
     * Get performance trends for a metric.
     */
    getTrends(scenarioId: string, metric: string): Promise<TrendData>;
    /**
     * Detect performance regressions.
     */
    detectRegressions(scenarioId: string, threshold?: number): Promise<Regression[]>;
    private getSeverity;
    /**
     * Export simulations to JSON.
     */
    export(ids: number[]): Promise<string>;
    /**
     * Import simulations from JSON.
     */
    import(json: string): Promise<number[]>;
    /**
     * Backup database to file.
     * SECURITY FIX: Validate path to prevent SQL injection and path traversal
     */
    backup(backupPath: string): Promise<void>;
    /**
     * Get database statistics.
     */
    getStats(): Promise<{
        totalSimulations: number;
        totalMetrics: number;
        totalInsights: number;
        scenarios: {
            id: string;
            count: number;
        }[];
        profiles: {
            profile: string;
            count: number;
        }[];
    }>;
}
/**
 * Create and initialize a report store.
 */
export declare function createReportStore(dbPath?: string): Promise<ReportStore>;
//# sourceMappingURL=report-store.d.ts.map