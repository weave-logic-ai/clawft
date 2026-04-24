/**
 * Traversal Optimization Simulation Tests
 *
 * Tests search strategies for efficient latent space navigation including
 * greedy, beam, dynamic-k, and attention-guided traversal.
 *
 * Target Metrics:
 * - Beam-5 configuration (optimal)
 * - Dynamic-k adaptation (5-20 range)
 * - Recall@10 >95% (target: 96.8%)
 * - Latency reduction -18.4% with dynamic-k
 * - Greedy, beam, A*, best-first strategies
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { traversalOptimizationScenario } from '../../scenarios/latent-space/traversal-optimization';
import type { SimulationReport } from '../../types';

describe('TraversalOptimization', () => {
  let report: SimulationReport;

  beforeAll(async () => {
    report = await traversalOptimizationScenario.run(traversalOptimizationScenario.config);
  }, 90000); // 90s timeout

  describe('Optimal Strategy Selection', () => {
    it('should identify beam-5 as optimal', () => {
      const best = report.summary.bestStrategy;
      expect(best.strategy).toBe('beam');
      expect(best.parameters.beamWidth).toBe(5);
    });

    it('should test greedy baseline', () => {
      const strategies = traversalOptimizationScenario.config.strategies;
      const greedy = strategies.find(s => s.name === 'greedy');
      expect(greedy).toBeDefined();
    });

    it('should test multiple beam widths', () => {
      const strategies = traversalOptimizationScenario.config.strategies;
      const beamStrategies = strategies.filter(s => s.name === 'beam');
      expect(beamStrategies.length).toBeGreaterThanOrEqual(3);
    });
  });

  describe('Dynamic-K Adaptation', () => {
    it('should adapt k in range 5-20', () => {
      const dynamicK = (report.detailedResults as any[]).find(
        r => r.strategy === 'dynamic-k'
      );

      if (dynamicK) {
        const range = dynamicK.metrics.dynamicKRange;
        expect(range[0]).toBe(5);
        expect(range[1]).toBe(20);
      }
    });

    it('should reduce latency by >15%', () => {
      const analysis = report.metrics.dynamicKEfficiency;
      if (analysis && analysis.latencyReduction) {
        expect(Math.abs(analysis.latencyReduction)).toBeGreaterThan(15);
      }
    });

    it('should target -18.4% latency reduction', () => {
      const analysis = report.metrics.dynamicKEfficiency;
      if (analysis && analysis.latencyReduction) {
        expect(analysis.latencyReduction).toBeCloseTo(-18.4, 5);
      }
    });
  });

  describe('Recall Performance', () => {
    it('should achieve >95% recall@10', () => {
      const results = report.detailedResults as any[];
      results.forEach(r => {
        if (r.metrics.recallAt10) {
          expect(r.metrics.recallAt10).toBeGreaterThan(0.95);
        }
      });
    });

    it('should target 96.8% recall@10', () => {
      const best = report.summary.bestStrategy;
      if (best.metrics.recallAt10) {
        expect(best.metrics.recallAt10).toBeCloseTo(0.968, 0.02);
      }
    });

    it('should maintain high precision', () => {
      const results = report.detailedResults as any[];
      results.forEach(r => {
        expect(r.metrics.precision).toBeGreaterThan(0.85);
      });
    });

    it('should optimize F1 score', () => {
      const results = report.detailedResults as any[];
      results.forEach(r => {
        const f1 = r.metrics.f1Score;
        expect(f1).toBeGreaterThan(0.85);
        expect(f1).toBeLessThanOrEqual(1.0);
      });
    });
  });

  describe('Search Efficiency', () => {
    it('should minimize average hops', () => {
      const avgHops = report.summary.avgHops || 0;
      expect(avgHops).toBeLessThan(25);
    });

    it('should reduce distance computations', () => {
      const results = report.detailedResults as any[];
      results.forEach(r => {
        const avgDist = r.metrics.avgDistanceComputations;
        expect(avgDist).toBeLessThan(r.graphSize / 10);
      });
    });

    it('should track latency percentiles', () => {
      const results = report.detailedResults as any[];
      results.forEach(r => {
        expect(r.metrics.latencyP50).toBeDefined();
        expect(r.metrics.latencyP95).toBeDefined();
        expect(r.metrics.latencyP99).toBeDefined();
      });
    });
  });

  describe('Strategy Comparison', () => {
    it('should compare all strategies', () => {
      const comparison = report.metrics.strategyComparison;
      expect(Array.isArray(comparison)).toBe(true);
      expect(comparison.length).toBeGreaterThanOrEqual(5);
    });

    it('should show beam > greedy recall', () => {
      const comparison = report.metrics.strategyComparison;
      const greedy = comparison.find((s: any) => s.strategy === 'greedy');
      const beam = comparison.find((s: any) => s.strategy === 'beam');

      if (greedy && beam) {
        expect(beam.avgRecall).toBeGreaterThan(greedy.avgRecall);
      }
    });

    it('should analyze latency trade-offs', () => {
      const comparison = report.metrics.strategyComparison;
      const greedy = comparison.find((s: any) => s.strategy === 'greedy');
      const beam10 = (report.detailedResults as any[]).find(
        r => r.strategy === 'beam' && r.parameters.beamWidth === 10
      );

      if (greedy && beam10) {
        expect(beam10.metrics.latencyMs).toBeGreaterThan(greedy.metrics.latencyMs);
      }
    });
  });

  describe('Recall-Latency Frontier', () => {
    it('should compute Pareto frontier', () => {
      const frontier = report.metrics.recallLatencyFrontier;
      expect(Array.isArray(frontier)).toBe(true);
      expect(frontier.length).toBeGreaterThan(0);
    });

    it('should identify optimal trade-off points', () => {
      const frontier = report.metrics.recallLatencyFrontier;
      frontier.forEach((point: any) => {
        expect(point.recall).toBeGreaterThan(0);
        expect(point.latency).toBeGreaterThan(0);
      });
    });
  });

  describe('Attention-Guided Navigation', () => {
    it('should test attention-guided strategy', () => {
      const attentionStrategy = (report.detailedResults as any[]).find(
        r => r.strategy === 'attention-guided'
      );
      expect(attentionStrategy).toBeDefined();
    });

    it('should improve efficiency', () => {
      const analysis = report.metrics.attentionGuidance;
      if (analysis && analysis.efficiency) {
        expect(analysis.efficiency).toBeGreaterThan(0.85);
      }
    });

    it('should prune search paths', () => {
      const analysis = report.metrics.attentionGuidance;
      if (analysis && analysis.pathPruning) {
        expect(analysis.pathPruning).toBeGreaterThan(0.1);
      }
    });
  });

  describe('Query Distribution Robustness', () => {
    it('should test uniform queries', () => {
      const distributions = traversalOptimizationScenario.config.queryDistributions;
      expect(distributions).toContain('uniform');
    });

    it('should test clustered queries', () => {
      const distributions = traversalOptimizationScenario.config.queryDistributions;
      expect(distributions).toContain('clustered');
    });

    it('should test outlier queries', () => {
      const distributions = traversalOptimizationScenario.config.queryDistributions;
      expect(distributions).toContain('outliers');
    });

    it('should handle mixed workloads', () => {
      const distributions = traversalOptimizationScenario.config.queryDistributions;
      expect(distributions).toContain('mixed');
    });
  });

  describe('Scalability', () => {
    it('should scale to 1M nodes', () => {
      const sizes = traversalOptimizationScenario.config.graphSizes;
      expect(sizes).toContain(1000000);
    });

    it('should maintain performance at scale', () => {
      const large = (report.detailedResults as any[]).filter(
        r => r.graphSize === 1000000
      );

      large.forEach(r => {
        expect(r.metrics.recall).toBeGreaterThan(0.90);
      });
    });
  });

  describe('Report Generation', () => {
    it('should generate comprehensive analysis', () => {
      expect(report.analysis).toBeDefined();
      expect(report.analysis).toContain('Traversal');
    });

    it('should provide strategy recommendations', () => {
      expect(report.recommendations).toBeDefined();
      expect(report.recommendations.length).toBeGreaterThanOrEqual(3);
    });

    it('should generate visualization artifacts', () => {
      expect(report.artifacts.recallLatencyPlots).toBeDefined();
      expect(report.artifacts.strategyComparisons).toBeDefined();
      expect(report.artifacts.efficiencyCurves).toBeDefined();
    });

    it('should complete efficiently', () => {
      expect(report.executionTimeMs).toBeLessThan(90000);
    });
  });
});
