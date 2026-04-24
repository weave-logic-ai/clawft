/**
 * History Tracker
 *
 * Tracks performance trends, detects regressions, and prepares
 * visualization data for simulation history.
 */
import { ReportStore } from './report-store';
export interface PerformanceTrend {
    metric: string;
    scenario: string;
    dataPoints: Array<{
        timestamp: Date;
        value: number;
        runId: number;
    }>;
    statistics: {
        mean: number;
        median: number;
        min: number;
        max: number;
        stdDev: number;
    };
    trend: 'improving' | 'degrading' | 'stable';
    slope?: number;
    rSquared?: number;
}
export interface VisualizationData {
    type: 'line' | 'bar' | 'scatter';
    labels: string[];
    datasets: Array<{
        label: string;
        data: number[];
        backgroundColor?: string;
        borderColor?: string;
    }>;
}
export interface RegressionAlert {
    severity: 'minor' | 'major' | 'critical';
    metric: string;
    scenario: string;
    message: string;
    degradation: number;
    baseline: number;
    current: number;
    timestamp: Date;
}
export declare class HistoryTracker {
    private store;
    constructor(store: ReportStore);
    /**
     * Get performance trend for a specific metric and scenario.
     */
    getPerformanceTrend(scenarioId: string, metric: string): Promise<PerformanceTrend>;
    /**
     * Get all performance trends for a scenario.
     */
    getAllTrends(scenarioId: string): Promise<PerformanceTrend[]>;
    /**
     * Calculate statistical measures.
     */
    private calculateStatistics;
    /**
     * Calculate R-squared (coefficient of determination) for trend line.
     */
    private calculateRSquared;
    /**
     * Detect regressions using moving average baseline.
     */
    detectRegressions(scenarioId: string, windowSize?: number, threshold?: number): Promise<RegressionAlert[]>;
    /**
     * Compare current run against baseline.
     */
    compareToBaseline(scenarioId: string, currentRunId: number, baselineRunId?: number): Promise<{
        metric: string;
        baseline: number;
        current: number;
        change: number;
        changePercent: number;
        improved: boolean;
    }[]>;
    private getSeverity;
    /**
     * Prepare data for Chart.js line chart.
     */
    prepareLineChart(scenarioId: string, metrics: string[]): Promise<VisualizationData>;
    /**
     * Prepare data for comparison bar chart.
     */
    prepareComparisonChart(runIds: number[]): Promise<VisualizationData>;
    /**
     * Prepare scatter plot for correlation analysis.
     */
    prepareScatterPlot(scenarioId: string, xMetric: string, yMetric: string): Promise<VisualizationData>;
    /**
     * Generate comprehensive trend report.
     */
    generateTrendReport(scenarioId: string): Promise<string>;
}
/**
 * Create history tracker instance.
 */
export declare function createHistoryTracker(store: ReportStore): HistoryTracker;
//# sourceMappingURL=history-tracker.d.ts.map