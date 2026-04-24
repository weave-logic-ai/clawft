/**
 * Metrics Hook - Learning dashboard and performance metrics
 */
import { z } from 'zod';
import { loadIntelligence } from './shared.js';
export const hookMetricsTool = {
    name: 'hook_metrics',
    description: 'Get learning metrics and performance dashboard',
    parameters: z.object({
        timeframe: z.enum(['1h', '24h', '7d', '30d']).optional().default('24h'),
        detailed: z.boolean().optional().default(false)
    }),
    execute: async ({ timeframe, detailed }, { onProgress }) => {
        const startTime = Date.now();
        const intel = loadIntelligence();
        // Parse timeframe
        const timeMs = {
            '1h': 60 * 60 * 1000,
            '24h': 24 * 60 * 60 * 1000,
            '7d': 7 * 24 * 60 * 60 * 1000,
            '30d': 30 * 24 * 60 * 60 * 1000
        };
        const cutoff = Date.now() - timeMs[timeframe];
        // Filter routing history by timeframe
        const recentHistory = intel.metrics.routingHistory.filter(h => new Date(h.timestamp).getTime() > cutoff);
        // Calculate routing accuracy
        const successCount = recentHistory.filter(h => h.success).length;
        const routingAccuracy = recentHistory.length > 0
            ? successCount / recentHistory.length
            : 0;
        // Calculate per-agent performance
        const agentPerformance = {};
        for (const entry of recentHistory) {
            if (!agentPerformance[entry.agent]) {
                agentPerformance[entry.agent] = { total: 0, successful: 0, rate: 0 };
            }
            agentPerformance[entry.agent].total++;
            if (entry.success) {
                agentPerformance[entry.agent].successful++;
            }
        }
        // Calculate rates
        for (const agent of Object.keys(agentPerformance)) {
            const perf = agentPerformance[agent];
            perf.rate = perf.total > 0 ? perf.successful / perf.total : 0;
        }
        // Get top patterns
        const patternStats = [];
        for (const [state, agents] of Object.entries(intel.patterns)) {
            const agentScores = agents;
            let topAgent = '';
            let topScore = 0;
            for (const [agent, score] of Object.entries(agentScores)) {
                if (score > topScore) {
                    topScore = score;
                    topAgent = agent;
                }
            }
            patternStats.push({
                state,
                agents: agentScores,
                topAgent,
                topScore
            });
        }
        patternStats.sort((a, b) => b.topScore - a.topScore);
        // Calculate learning velocity (improvement trend)
        let learningVelocity = 0;
        if (recentHistory.length >= 10) {
            const firstHalf = recentHistory.slice(0, Math.floor(recentHistory.length / 2));
            const secondHalf = recentHistory.slice(Math.floor(recentHistory.length / 2));
            const firstHalfRate = firstHalf.filter(h => h.success).length / firstHalf.length;
            const secondHalfRate = secondHalf.filter(h => h.success).length / secondHalf.length;
            learningVelocity = secondHalfRate - firstHalfRate;
        }
        // Memory utilization
        const memoryStats = {
            total: intel.memories.length,
            byType: {}
        };
        for (const mem of intel.memories) {
            memoryStats.byType[mem.type] = (memoryStats.byType[mem.type] || 0) + 1;
        }
        // Error pattern analysis
        const errorStats = {
            total: intel.errorPatterns.length,
            topErrors: intel.errorPatterns
                .map(ep => ({
                type: ep.errorType,
                context: ep.context.slice(0, 50),
                occurrences: Object.values(ep.agentSuccess).reduce((a, b) => Math.abs(a) + Math.abs(b), 0)
            }))
                .sort((a, b) => b.occurrences - a.occurrences)
                .slice(0, 5)
        };
        const latency = Date.now() - startTime;
        return {
            success: true,
            timeframe,
            routing: {
                accuracy: routingAccuracy,
                total: recentHistory.length,
                successful: successCount,
                failed: recentHistory.length - successCount
            },
            learning: {
                velocity: learningVelocity,
                improving: learningVelocity > 0,
                patternsLearned: Object.keys(intel.patterns).length,
                memoriesStored: intel.memories.length
            },
            agents: detailed ? agentPerformance : Object.fromEntries(Object.entries(agentPerformance)
                .sort((a, b) => b[1].total - a[1].total)
                .slice(0, 5)),
            topPatterns: patternStats.slice(0, detailed ? 10 : 5).map(p => ({
                state: p.state,
                topAgent: p.topAgent,
                score: p.topScore.toFixed(2)
            })),
            memory: memoryStats,
            errors: errorStats,
            pretrained: intel.pretrained || null,
            health: {
                status: routingAccuracy > 0.7 ? 'healthy' : routingAccuracy > 0.5 ? 'learning' : 'needs-data',
                dataPoints: recentHistory.length,
                recommendation: recentHistory.length < 10
                    ? 'Need more data points for accurate metrics'
                    : learningVelocity < 0
                        ? 'Learning velocity negative - consider retraining'
                        : 'System learning effectively'
            },
            latencyMs: latency,
            timestamp: new Date().toISOString()
        };
    }
};
//# sourceMappingURL=metrics.js.map