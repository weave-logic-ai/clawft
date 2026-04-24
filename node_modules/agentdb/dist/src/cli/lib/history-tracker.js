/**
 * History Tracker
 *
 * Tracks performance trends, detects regressions, and prepares
 * visualization data for simulation history.
 */
// ============================================================================
// History Tracker
// ============================================================================
export class HistoryTracker {
    store;
    constructor(store) {
        this.store = store;
    }
    // --------------------------------------------------------------------------
    // Trend Analysis
    // --------------------------------------------------------------------------
    /**
     * Get performance trend for a specific metric and scenario.
     */
    async getPerformanceTrend(scenarioId, metric) {
        const trendData = await this.store.getTrends(scenarioId, metric);
        const values = trendData.points.map(p => p.value);
        const statistics = this.calculateStatistics(values);
        // Calculate R-squared for trend line
        const rSquared = this.calculateRSquared(trendData.points);
        return {
            metric,
            scenario: scenarioId,
            dataPoints: trendData.points,
            statistics,
            trend: trendData.trend,
            slope: trendData.slope,
            rSquared
        };
    }
    /**
     * Get all performance trends for a scenario.
     */
    async getAllTrends(scenarioId) {
        // Get a sample run to discover metrics
        const runs = await this.store.findByScenario(scenarioId);
        if (runs.length === 0) {
            return [];
        }
        const metricNames = Object.keys(runs[0].metrics);
        const trends = [];
        for (const metric of metricNames) {
            const trend = await this.getPerformanceTrend(scenarioId, metric);
            trends.push(trend);
        }
        return trends;
    }
    /**
     * Calculate statistical measures.
     */
    calculateStatistics(values) {
        const sorted = [...values].sort((a, b) => a - b);
        const n = values.length;
        const mean = values.reduce((a, b) => a + b, 0) / n;
        const median = n % 2 === 0
            ? (sorted[n / 2 - 1] + sorted[n / 2]) / 2
            : sorted[Math.floor(n / 2)];
        const min = Math.min(...values);
        const max = Math.max(...values);
        // Standard deviation
        const variance = values.reduce((sum, v) => sum + Math.pow(v - mean, 2), 0) / n;
        const stdDev = Math.sqrt(variance);
        return { mean, median, min, max, stdDev };
    }
    /**
     * Calculate R-squared (coefficient of determination) for trend line.
     */
    calculateRSquared(points) {
        const n = points.length;
        if (n < 2)
            return 0;
        const x = points.map((_, i) => i); // Use index as x
        const y = points.map(p => p.value);
        // Calculate means
        const meanX = x.reduce((a, b) => a + b, 0) / n;
        const meanY = y.reduce((a, b) => a + b, 0) / n;
        // Calculate regression line coefficients
        const numerator = x.reduce((sum, xi, i) => sum + (xi - meanX) * (y[i] - meanY), 0);
        const denominator = x.reduce((sum, xi) => sum + Math.pow(xi - meanX, 2), 0);
        const slope = numerator / denominator;
        const intercept = meanY - slope * meanX;
        // Calculate R-squared
        const ssTotal = y.reduce((sum, yi) => sum + Math.pow(yi - meanY, 2), 0);
        const ssResidual = y.reduce((sum, yi, i) => {
            const predicted = slope * x[i] + intercept;
            return sum + Math.pow(yi - predicted, 2);
        }, 0);
        return 1 - (ssResidual / ssTotal);
    }
    // --------------------------------------------------------------------------
    // Regression Detection
    // --------------------------------------------------------------------------
    /**
     * Detect regressions using moving average baseline.
     */
    async detectRegressions(scenarioId, windowSize = 5, threshold = 0.1) {
        const runs = await this.store.findByScenario(scenarioId);
        if (runs.length < windowSize + 2) {
            return []; // Not enough data
        }
        const alerts = [];
        const metricNames = Object.keys(runs[0].metrics);
        for (const metric of metricNames) {
            const values = runs.map(r => r.metrics[metric] || 0);
            // Calculate baseline (moving average of first windowSize points)
            const baseline = values.slice(0, windowSize)
                .reduce((a, b) => a + b, 0) / windowSize;
            // Check recent runs (last 2)
            const recentRuns = runs.slice(-2);
            for (const run of recentRuns) {
                const current = run.metrics[metric] || 0;
                const degradation = (baseline - current) / baseline;
                if (degradation > threshold) {
                    alerts.push({
                        severity: this.getSeverity(degradation),
                        metric,
                        scenario: scenarioId,
                        message: `Performance degraded by ${(degradation * 100).toFixed(1)}% for ${metric}`,
                        degradation,
                        baseline,
                        current,
                        timestamp: run.timestamp
                    });
                }
            }
        }
        return alerts;
    }
    /**
     * Compare current run against baseline.
     */
    async compareToBaseline(scenarioId, currentRunId, baselineRunId) {
        const currentRun = await this.store.get(currentRunId);
        if (!currentRun) {
            throw new Error(`Run ${currentRunId} not found`);
        }
        let baseline;
        if (baselineRunId) {
            const baselineRun = await this.store.get(baselineRunId);
            if (!baselineRun) {
                throw new Error(`Baseline run ${baselineRunId} not found`);
            }
            baseline = baselineRun;
        }
        else {
            // Use average of last 5 runs as baseline
            const runs = await this.store.findByScenario(scenarioId);
            const recentRuns = runs.slice(-6, -1); // Exclude current run
            if (recentRuns.length === 0) {
                throw new Error('No baseline runs available');
            }
            // Calculate average metrics
            const avgMetrics = {};
            const metricNames = Object.keys(currentRun.metrics);
            for (const metric of metricNames) {
                const values = recentRuns.map(r => r.metrics[metric] || 0);
                avgMetrics[metric] = values.reduce((a, b) => a + b, 0) / values.length;
            }
            baseline = {
                ...currentRun,
                metrics: {
                    recall: 0,
                    latency: 0,
                    throughput: 0,
                    memoryUsage: 0,
                    ...avgMetrics
                }
            };
        }
        // Compare metrics
        const comparisons = [];
        for (const metric of Object.keys(currentRun.metrics)) {
            const baselineValue = baseline.metrics[metric] || 0;
            const currentValue = currentRun.metrics[metric] || 0;
            const change = currentValue - baselineValue;
            const changePercent = baselineValue !== 0
                ? (change / baselineValue) * 100
                : 0;
            comparisons.push({
                metric,
                baseline: baselineValue,
                current: currentValue,
                change,
                changePercent,
                improved: change > 0 // Assume higher is better
            });
        }
        return comparisons;
    }
    getSeverity(degradation) {
        if (degradation > 0.3)
            return 'critical';
        if (degradation > 0.15)
            return 'major';
        return 'minor';
    }
    // --------------------------------------------------------------------------
    // Visualization Data
    // --------------------------------------------------------------------------
    /**
     * Prepare data for Chart.js line chart.
     */
    async prepareLineChart(scenarioId, metrics) {
        const runs = await this.store.findByScenario(scenarioId);
        const labels = runs.map(r => r.timestamp.toLocaleDateString());
        const datasets = [];
        const colors = [
            '#FF6384', '#36A2EB', '#FFCE56', '#4BC0C0',
            '#9966FF', '#FF9F40', '#FF6384', '#C9CBCF'
        ];
        for (let i = 0; i < metrics.length; i++) {
            const metric = metrics[i];
            const data = runs.map(r => r.metrics[metric] || 0);
            datasets.push({
                label: metric,
                data,
                borderColor: colors[i % colors.length],
                backgroundColor: colors[i % colors.length] + '33' // 20% opacity
            });
        }
        return {
            type: 'line',
            labels,
            datasets
        };
    }
    /**
     * Prepare data for comparison bar chart.
     */
    async prepareComparisonChart(runIds) {
        const runs = [];
        for (const id of runIds) {
            const run = await this.store.get(id);
            if (run)
                runs.push(run);
        }
        if (runs.length === 0) {
            throw new Error('No valid runs for comparison');
        }
        // Collect all metrics
        const allMetrics = new Set();
        for (const run of runs) {
            Object.keys(run.metrics).forEach(m => allMetrics.add(m));
        }
        const labels = Array.from(allMetrics);
        const datasets = [];
        const colors = [
            '#FF6384', '#36A2EB', '#FFCE56', '#4BC0C0',
            '#9966FF', '#FF9F40'
        ];
        for (let i = 0; i < runs.length; i++) {
            const run = runs[i];
            const data = labels.map(metric => run.metrics[metric] || 0);
            datasets.push({
                label: `Run ${run.id}`,
                data,
                backgroundColor: colors[i % colors.length]
            });
        }
        return {
            type: 'bar',
            labels,
            datasets
        };
    }
    /**
     * Prepare scatter plot for correlation analysis.
     */
    async prepareScatterPlot(scenarioId, xMetric, yMetric) {
        const runs = await this.store.findByScenario(scenarioId);
        const data = runs.map(r => ({
            x: r.metrics[xMetric] || 0,
            y: r.metrics[yMetric] || 0
        }));
        return {
            type: 'scatter',
            labels: [],
            datasets: [{
                    label: `${xMetric} vs ${yMetric}`,
                    data: data,
                    backgroundColor: '#36A2EB'
                }]
        };
    }
    // --------------------------------------------------------------------------
    // Reports
    // --------------------------------------------------------------------------
    /**
     * Generate comprehensive trend report.
     */
    async generateTrendReport(scenarioId) {
        const trends = await this.getAllTrends(scenarioId);
        const regressions = await this.detectRegressions(scenarioId);
        let report = `# Performance Trend Report: ${scenarioId}\n\n`;
        report += `**Generated**: ${new Date().toLocaleString()}\n\n`;
        report += `## Summary\n\n`;
        report += `- **Total Metrics**: ${trends.length}\n`;
        report += `- **Improving**: ${trends.filter(t => t.trend === 'improving').length}\n`;
        report += `- **Degrading**: ${trends.filter(t => t.trend === 'degrading').length}\n`;
        report += `- **Stable**: ${trends.filter(t => t.trend === 'stable').length}\n`;
        report += `- **Regressions Detected**: ${regressions.length}\n\n`;
        if (regressions.length > 0) {
            report += `## ⚠️ Regressions\n\n`;
            for (const regression of regressions) {
                const icon = regression.severity === 'critical' ? '🔴' :
                    regression.severity === 'major' ? '🟠' : '🟡';
                report += `${icon} **${regression.metric}** (${regression.severity})\n`;
                report += `- Degradation: ${(regression.degradation * 100).toFixed(1)}%\n`;
                report += `- Baseline: ${regression.baseline.toFixed(2)}\n`;
                report += `- Current: ${regression.current.toFixed(2)}\n`;
                report += `- Detected: ${regression.timestamp.toLocaleString()}\n\n`;
            }
        }
        report += `## Detailed Trends\n\n`;
        for (const trend of trends) {
            const trendIcon = trend.trend === 'improving' ? '📈' :
                trend.trend === 'degrading' ? '📉' : '➡️';
            report += `### ${trendIcon} ${trend.metric}\n\n`;
            report += `- **Trend**: ${trend.trend}\n`;
            report += `- **Data Points**: ${trend.dataPoints.length}\n`;
            report += `- **Mean**: ${trend.statistics.mean.toFixed(2)}\n`;
            report += `- **Median**: ${trend.statistics.median.toFixed(2)}\n`;
            report += `- **Min**: ${trend.statistics.min.toFixed(2)}\n`;
            report += `- **Max**: ${trend.statistics.max.toFixed(2)}\n`;
            report += `- **Std Dev**: ${trend.statistics.stdDev.toFixed(2)}\n`;
            if (trend.slope !== undefined) {
                report += `- **Slope**: ${trend.slope.toFixed(4)}\n`;
            }
            if (trend.rSquared !== undefined) {
                report += `- **R²**: ${trend.rSquared.toFixed(4)}\n`;
            }
            report += '\n';
        }
        return report;
    }
}
// ============================================================================
// Factory
// ============================================================================
/**
 * Create history tracker instance.
 */
export function createHistoryTracker(store) {
    return new HistoryTracker(store);
}
//# sourceMappingURL=history-tracker.js.map