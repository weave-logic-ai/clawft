/**
 * Report Store (SQLite)
 *
 * Persistent storage for simulation results with queryable history,
 * trend analysis, and comparison tools.
 */

import * as path from 'path';
import * as fs from 'fs';
import { AgentDBConfig, SimulationResult } from './simulation-registry';

// Use better-sqlite3 if available (optional dependency)
let Database: any;
try {
  Database = require('better-sqlite3');
} catch {
  // Fallback to in-memory store if better-sqlite3 not installed
  console.warn('better-sqlite3 not installed, using in-memory storage only');
}

// ============================================================================
// Types
// ============================================================================

export interface ComparisonReport {
  scenarios: string[];
  metrics: Record<string, {
    values: number[];
    best: number;
    worst: number;
    average: number;
    winner: number; // Index of best run
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

// ============================================================================
// Report Store
// ============================================================================

export class ReportStore {
  private db: any = null;
  private dbPath: string;

  constructor(dbPath: string = ':memory:') {
    this.dbPath = dbPath;
  }

  // --------------------------------------------------------------------------
  // Initialization
  // --------------------------------------------------------------------------

  /**
   * Initialize database connection and create schema.
   */
  async initialize(): Promise<void> {
    if (!Database) {
      throw new Error('better-sqlite3 not available. Install it with: npm install better-sqlite3');
    }

    // Ensure directory exists
    if (this.dbPath !== ':memory:') {
      const dir = path.dirname(this.dbPath);
      if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
      }
    }

    // Open database with better-sqlite3
    this.db = new Database(this.dbPath);

    // Create schema
    await this.createSchema();
  }

  /**
   * Create database schema.
   */
  private async createSchema(): Promise<void> {
    if (!this.db) throw new Error('Database not initialized');

    this.db.exec(`
      -- Simulation runs
      CREATE TABLE IF NOT EXISTS simulations (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        scenario_id TEXT NOT NULL,
        scenario_name TEXT NOT NULL,
        timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
        config_json TEXT NOT NULL,
        profile TEXT,
        agentdb_version TEXT,
        duration_ms INTEGER,
        iterations INTEGER,
        status TEXT CHECK(status IN ('running', 'completed', 'failed', 'cancelled')) DEFAULT 'completed'
      );

      -- Metrics (normalized for efficient queries)
      CREATE TABLE IF NOT EXISTS metrics (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        simulation_id INTEGER REFERENCES simulations(id) ON DELETE CASCADE,
        metric_name TEXT NOT NULL,
        metric_value REAL NOT NULL,
        iteration INTEGER DEFAULT 0,
        UNIQUE(simulation_id, metric_name, iteration)
      );

      -- Insights and recommendations
      CREATE TABLE IF NOT EXISTS insights (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        simulation_id INTEGER REFERENCES simulations(id) ON DELETE CASCADE,
        type TEXT CHECK(type IN ('insight', 'recommendation', 'warning')) NOT NULL,
        content TEXT NOT NULL,
        category TEXT
      );

      -- Comparison groups (for A/B testing)
      CREATE TABLE IF NOT EXISTS comparison_groups (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        description TEXT,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
      );

      CREATE TABLE IF NOT EXISTS comparison_members (
        group_id INTEGER REFERENCES comparison_groups(id) ON DELETE CASCADE,
        simulation_id INTEGER REFERENCES simulations(id) ON DELETE CASCADE,
        PRIMARY KEY(group_id, simulation_id)
      );

      -- Indexes for performance
      CREATE INDEX IF NOT EXISTS idx_simulations_scenario ON simulations(scenario_id);
      CREATE INDEX IF NOT EXISTS idx_simulations_timestamp ON simulations(timestamp);
      CREATE INDEX IF NOT EXISTS idx_simulations_status ON simulations(status);
      CREATE INDEX IF NOT EXISTS idx_metrics_simulation ON metrics(simulation_id);
      CREATE INDEX IF NOT EXISTS idx_metrics_name ON metrics(metric_name);
      CREATE INDEX IF NOT EXISTS idx_insights_simulation ON insights(simulation_id);
    `);
  }

  /**
   * Close database connection.
   */
  async close(): Promise<void> {
    if (this.db) {
      this.db.close();
      this.db = null;
    }
  }

  // --------------------------------------------------------------------------
  // CRUD Operations
  // --------------------------------------------------------------------------

  /**
   * Save a simulation result.
   */
  async save(result: SimulationResult): Promise<number> {
    if (!this.db) throw new Error('Database not initialized');

    // Insert simulation record
    const insertResult = this.db.prepare(`
      INSERT INTO simulations (
        scenario_id, scenario_name, timestamp, config_json, profile,
        agentdb_version, duration_ms, iterations, status
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    `).run(
      result.scenario,
      result.scenario,
      result.timestamp.toISOString(),
      JSON.stringify(result.config),
      result.config.profile,
      '2.0.0', // TODO: Get from package.json
      result.duration || 0,
      result.iterations || 1,
      'completed'
    );

    const simulationId = insertResult.lastInsertRowid as number;

    // Insert metrics
    const insertMetric = this.db.prepare(`
      INSERT INTO metrics (simulation_id, metric_name, metric_value)
      VALUES (?, ?, ?)
    `);
    for (const [metricName, metricValue] of Object.entries(result.metrics)) {
      insertMetric.run(simulationId, metricName, metricValue);
    }

    // Insert insights
    const insertInsight = this.db.prepare(`
      INSERT INTO insights (simulation_id, type, content, category)
      VALUES (?, ?, ?, ?)
    `);
    for (const insight of result.insights) {
      insertInsight.run(simulationId, 'insight', insight, 'general');
    }

    // Insert recommendations
    for (const recommendation of result.recommendations) {
      insertInsight.run(simulationId, 'recommendation', recommendation, 'general');
    }

    return simulationId;
  }

  /**
   * Get simulation by ID.
   */
  async get(id: number): Promise<SimulationResult | null> {
    if (!this.db) throw new Error('Database not initialized');

    const simulation = this.db.prepare(`
      SELECT * FROM simulations WHERE id = ?
    `).get(id);

    if (!simulation) return null;

    // Get metrics
    const metrics = this.db.prepare(`
      SELECT metric_name, metric_value FROM metrics
      WHERE simulation_id = ? AND iteration = 0
    `).all(id);

    // Get insights
    const insights = this.db.prepare(`
      SELECT content FROM insights
      WHERE simulation_id = ? AND type = 'insight'
    `).all(id);

    // Get recommendations
    const recommendations = this.db.prepare(`
      SELECT content FROM insights
      WHERE simulation_id = ? AND type = 'recommendation'
    `).all(id);

    return {
      id: simulation.id,
      scenario: simulation.scenario_id,
      timestamp: new Date(simulation.timestamp),
      config: JSON.parse(simulation.config_json),
      metrics: metrics.reduce((acc: any, m: any) => {
        acc[m.metric_name] = m.metric_value;
        return acc;
      }, {} as any),
      insights: insights.map((i: any) => i.content),
      recommendations: recommendations.map((r: any) => r.content),
      iterations: simulation.iterations,
      duration: simulation.duration_ms
    };
  }

  /**
   * List recent simulations.
   */
  async list(limit: number = 10): Promise<SimulationResult[]> {
    if (!this.db) throw new Error('Database not initialized');

    const simulations = this.db.prepare(`
      SELECT id FROM simulations
      ORDER BY timestamp DESC
      LIMIT ?
    `).all(limit);

    const results: SimulationResult[] = [];

    for (const sim of simulations) {
      const result = await this.get(sim.id);
      if (result) results.push(result);
    }

    return results;
  }

  /**
   * Find simulations by scenario ID.
   */
  async findByScenario(scenarioId: string): Promise<SimulationResult[]> {
    if (!this.db) throw new Error('Database not initialized');

    const simulations = this.db.prepare(`
      SELECT id FROM simulations
      WHERE scenario_id = ?
      ORDER BY timestamp DESC
    `).all(scenarioId);

    const results: SimulationResult[] = [];

    for (const sim of simulations) {
      const result = await this.get(sim.id);
      if (result) results.push(result);
    }

    return results;
  }

  // --------------------------------------------------------------------------
  // Comparison
  // --------------------------------------------------------------------------

  /**
   * Compare multiple simulation runs.
   */
  async compare(ids: number[]): Promise<ComparisonReport> {
    if (!this.db) throw new Error('Database not initialized');

    const results: SimulationResult[] = [];

    for (const id of ids) {
      const result = await this.get(id);
      if (result) results.push(result);
    }

    if (results.length === 0) {
      throw new Error('No valid simulations found for comparison');
    }

    // Collect all metric names
    const allMetrics = new Set<string>();
    for (const result of results) {
      Object.keys(result.metrics).forEach(m => allMetrics.add(m));
    }

    // Build comparison
    const metrics: ComparisonReport['metrics'] = {};

    for (const metricName of allMetrics) {
      const values = results.map(r => r.metrics[metricName] || 0);
      const best = Math.max(...values);
      const worst = Math.min(...values);
      const average = values.reduce((a, b) => a + b, 0) / values.length;
      const winner = values.indexOf(best);

      metrics[metricName] = { values, best, worst, average, winner };
    }

    // Generate insights
    const insights: string[] = [];

    for (const [metricName, data] of Object.entries(metrics)) {
      const improvement = ((data.best - data.worst) / data.worst * 100).toFixed(1);
      insights.push(
        `${metricName}: Best run improved by ${improvement}% over worst`
      );
    }

    return {
      scenarios: results.map(r => r.scenario),
      metrics,
      insights
    };
  }

  // --------------------------------------------------------------------------
  // Trend Analysis
  // --------------------------------------------------------------------------

  /**
   * Get performance trends for a metric.
   */
  async getTrends(scenarioId: string, metric: string): Promise<TrendData> {
    if (!this.db) throw new Error('Database not initialized');

    const rows = this.db.prepare(`
      SELECT s.id, s.timestamp, m.metric_value
      FROM simulations s
      JOIN metrics m ON s.id = m.simulation_id
      WHERE s.scenario_id = ? AND m.metric_name = ?
      ORDER BY s.timestamp ASC
    `).all(scenarioId, metric);

    const points = rows.map(r => ({
      timestamp: new Date(r.timestamp),
      value: r.metric_value,
      runId: r.id
    }));

    // Calculate trend (simple linear regression)
    const n = points.length;
    if (n < 2) {
      return { metric, points, trend: 'stable' };
    }

    const timestamps = points.map((p, i) => i); // Use index as x
    const values = points.map(p => p.value);

    const sumX = timestamps.reduce((a, b) => a + b, 0);
    const sumY = values.reduce((a, b) => a + b, 0);
    const sumXY = timestamps.reduce((sum, x, i) => sum + x * values[i], 0);
    const sumX2 = timestamps.reduce((sum, x) => sum + x * x, 0);

    const slope = (n * sumXY - sumX * sumY) / (n * sumX2 - sumX * sumX);

    let trend: 'improving' | 'degrading' | 'stable';
    if (slope > 0.01) trend = 'improving';
    else if (slope < -0.01) trend = 'degrading';
    else trend = 'stable';

    return { metric, points, trend, slope };
  }

  /**
   * Detect performance regressions.
   */
  async detectRegressions(
    scenarioId: string,
    threshold: number = 0.1
  ): Promise<Regression[]> {
    if (!this.db) throw new Error('Database not initialized');

    // Get all metrics for this scenario
    const metricNames = this.db.prepare(`
      SELECT DISTINCT m.metric_name
      FROM simulations s
      JOIN metrics m ON s.id = m.simulation_id
      WHERE s.scenario_id = ?
    `).all(scenarioId);

    const regressions: Regression[] = [];

    for (const { metric_name } of metricNames) {
      const trend = await this.getTrends(scenarioId, metric_name);

      if (trend.points.length < 5) continue; // Need at least 5 points

      // Calculate baseline (moving average of first 3 points)
      const baseline = trend.points.slice(0, 3)
        .reduce((sum, p) => sum + p.value, 0) / 3;

      // Check recent points (last 2)
      const recent = trend.points.slice(-2);

      for (const point of recent) {
        const degradation = (baseline - point.value) / baseline;

        if (degradation > threshold) {
          regressions.push({
            metric: metric_name,
            baseline,
            current: point.value,
            degradation,
            severity: this.getSeverity(degradation),
            firstDetected: point.timestamp,
            affectedRuns: [point.runId]
          });
        }
      }
    }

    return regressions;
  }

  private getSeverity(degradation: number): 'minor' | 'major' | 'critical' {
    if (degradation > 0.3) return 'critical';
    if (degradation > 0.15) return 'major';
    return 'minor';
  }

  // --------------------------------------------------------------------------
  // Import/Export
  // --------------------------------------------------------------------------

  /**
   * Export simulations to JSON.
   */
  async export(ids: number[]): Promise<string> {
    const results: SimulationResult[] = [];

    for (const id of ids) {
      const result = await this.get(id);
      if (result) results.push(result);
    }

    return JSON.stringify(results, null, 2);
  }

  /**
   * Import simulations from JSON.
   */
  async import(json: string): Promise<number[]> {
    const results: SimulationResult[] = JSON.parse(json);
    const ids: number[] = [];

    for (const result of results) {
      const id = await this.save(result);
      ids.push(id);
    }

    return ids;
  }

  /**
   * Backup database to file.
   * SECURITY FIX: Validate path to prevent SQL injection and path traversal
   */
  async backup(backupPath: string): Promise<void> {
    if (!this.db || this.dbPath === ':memory:') {
      throw new Error('Cannot backup in-memory database');
    }

    // Security: Validate backup path to prevent SQL injection and path traversal
    const normalizedPath = path.normalize(backupPath);

    // Check for path traversal attempts
    if (normalizedPath.includes('..') || path.isAbsolute(normalizedPath) && !normalizedPath.startsWith(process.cwd())) {
      throw new Error('Invalid backup path: path traversal not allowed');
    }

    // Validate path characters - only allow safe characters for file paths
    // This prevents SQL injection via the path parameter
    if (!/^[a-zA-Z0-9_\-./\\]+$/.test(normalizedPath)) {
      throw new Error('Invalid backup path: contains invalid characters');
    }

    // Ensure path ends with .db or .sqlite extension
    if (!normalizedPath.endsWith('.db') && !normalizedPath.endsWith('.sqlite')) {
      throw new Error('Invalid backup path: must end with .db or .sqlite extension');
    }

    // Ensure target directory exists
    const dir = path.dirname(normalizedPath);
    if (dir && dir !== '.' && !fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }

    // SQLite backup API - path is now validated
    // Note: VACUUM INTO requires the path as a string literal,
    // but we've validated it only contains safe characters
    this.db.exec(`VACUUM INTO '${normalizedPath}'`);
  }

  // --------------------------------------------------------------------------
  // Statistics
  // --------------------------------------------------------------------------

  /**
   * Get database statistics.
   */
  async getStats(): Promise<{
    totalSimulations: number;
    totalMetrics: number;
    totalInsights: number;
    scenarios: { id: string; count: number }[];
    profiles: { profile: string; count: number }[];
  }> {
    if (!this.db) throw new Error('Database not initialized');

    const totalSimulations = this.db.prepare(`
      SELECT COUNT(*) as count FROM simulations
    `).get();

    const totalMetrics = this.db.prepare(`
      SELECT COUNT(*) as count FROM metrics
    `).get();

    const totalInsights = this.db.prepare(`
      SELECT COUNT(*) as count FROM insights
    `).get();

    const scenarios = this.db.prepare(`
      SELECT scenario_id as id, COUNT(*) as count
      FROM simulations
      GROUP BY scenario_id
      ORDER BY count DESC
    `).all();

    const profiles = this.db.prepare(`
      SELECT profile, COUNT(*) as count
      FROM simulations
      GROUP BY profile
      ORDER BY count DESC
    `).all();

    return {
      totalSimulations: totalSimulations.count,
      totalMetrics: totalMetrics.count,
      totalInsights: totalInsights.count,
      scenarios,
      profiles
    };
  }
}

// ============================================================================
// Factory
// ============================================================================

/**
 * Create and initialize a report store.
 */
export async function createReportStore(
  dbPath: string = ':memory:'
): Promise<ReportStore> {
  const store = new ReportStore(dbPath);
  await store.initialize();
  return store;
}
