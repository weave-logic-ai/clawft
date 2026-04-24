/**
 * IntelligenceStore - SQLite persistence for RuVector intelligence layer
 *
 * Cross-platform (Linux, macOS, Windows) persistent storage for:
 * - Learning trajectories
 * - Routing patterns
 * - SONA adaptations
 * - HNSW vectors
 */
import Database from 'better-sqlite3';
import { existsSync, mkdirSync } from 'fs';
import { dirname, join } from 'path';
import { homedir } from 'os';
export class IntelligenceStore {
    db;
    static instance = null;
    constructor(dbPath) {
        // Ensure directory exists
        const dir = dirname(dbPath);
        if (!existsSync(dir)) {
            mkdirSync(dir, { recursive: true });
        }
        this.db = new Database(dbPath);
        this.db.pragma('journal_mode = WAL'); // Better concurrent access
        this.db.pragma('synchronous = NORMAL'); // Good balance of speed/safety
        this.initSchema();
    }
    /**
     * Get singleton instance
     */
    static getInstance(dbPath) {
        if (!IntelligenceStore.instance) {
            const path = dbPath || IntelligenceStore.getDefaultPath();
            IntelligenceStore.instance = new IntelligenceStore(path);
        }
        return IntelligenceStore.instance;
    }
    /**
     * Get default database path (cross-platform)
     */
    static getDefaultPath() {
        // Check for project-local .agentic-flow directory first
        const localPath = join(process.cwd(), '.agentic-flow', 'intelligence.db');
        const localDir = dirname(localPath);
        if (existsSync(localDir)) {
            return localPath;
        }
        // Fall back to home directory
        const homeDir = homedir();
        return join(homeDir, '.agentic-flow', 'intelligence.db');
    }
    /**
     * Initialize database schema
     */
    initSchema() {
        this.db.exec(`
      -- Trajectories table
      CREATE TABLE IF NOT EXISTS trajectories (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        task_description TEXT NOT NULL,
        agent TEXT NOT NULL,
        steps INTEGER DEFAULT 0,
        outcome TEXT DEFAULT 'partial',
        start_time INTEGER NOT NULL,
        end_time INTEGER,
        metadata TEXT,
        created_at INTEGER DEFAULT (strftime('%s', 'now'))
      );

      -- Patterns table (for ReasoningBank)
      CREATE TABLE IF NOT EXISTS patterns (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        task_type TEXT NOT NULL,
        approach TEXT NOT NULL,
        embedding BLOB,
        similarity REAL DEFAULT 0,
        usage_count INTEGER DEFAULT 0,
        success_rate REAL DEFAULT 0,
        created_at INTEGER DEFAULT (strftime('%s', 'now')),
        updated_at INTEGER DEFAULT (strftime('%s', 'now'))
      );

      -- Routings table
      CREATE TABLE IF NOT EXISTS routings (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        task TEXT NOT NULL,
        recommended_agent TEXT NOT NULL,
        confidence REAL NOT NULL,
        latency_ms INTEGER NOT NULL,
        was_successful INTEGER DEFAULT 0,
        timestamp INTEGER DEFAULT (strftime('%s', 'now'))
      );

      -- Stats table (single row)
      CREATE TABLE IF NOT EXISTS stats (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        total_trajectories INTEGER DEFAULT 0,
        successful_trajectories INTEGER DEFAULT 0,
        total_routings INTEGER DEFAULT 0,
        successful_routings INTEGER DEFAULT 0,
        total_patterns INTEGER DEFAULT 0,
        sona_adaptations INTEGER DEFAULT 0,
        hnsw_queries INTEGER DEFAULT 0,
        last_updated INTEGER DEFAULT (strftime('%s', 'now'))
      );

      -- Initialize stats row if not exists
      INSERT OR IGNORE INTO stats (id) VALUES (1);

      -- Indexes for faster queries
      CREATE INDEX IF NOT EXISTS idx_trajectories_agent ON trajectories(agent);
      CREATE INDEX IF NOT EXISTS idx_trajectories_outcome ON trajectories(outcome);
      CREATE INDEX IF NOT EXISTS idx_patterns_task_type ON patterns(task_type);
      CREATE INDEX IF NOT EXISTS idx_routings_agent ON routings(recommended_agent);
      CREATE INDEX IF NOT EXISTS idx_routings_timestamp ON routings(timestamp);
    `);
    }
    // ============ Trajectory Methods ============
    /**
     * Start a new trajectory
     */
    startTrajectory(taskDescription, agent) {
        const stmt = this.db.prepare(`
      INSERT INTO trajectories (task_description, agent, start_time)
      VALUES (?, ?, ?)
    `);
        const result = stmt.run(taskDescription, agent, Date.now());
        this.incrementStat('total_trajectories');
        return result.lastInsertRowid;
    }
    /**
     * Add step to trajectory
     */
    addTrajectoryStep(trajectoryId) {
        const stmt = this.db.prepare(`
      UPDATE trajectories SET steps = steps + 1 WHERE id = ?
    `);
        stmt.run(trajectoryId);
    }
    /**
     * End trajectory with outcome
     */
    endTrajectory(trajectoryId, outcome, metadata) {
        const stmt = this.db.prepare(`
      UPDATE trajectories
      SET outcome = ?, end_time = ?, metadata = ?
      WHERE id = ?
    `);
        stmt.run(outcome, Date.now(), metadata ? JSON.stringify(metadata) : null, trajectoryId);
        if (outcome === 'success') {
            this.incrementStat('successful_trajectories');
        }
    }
    /**
     * Get active trajectories (no end_time)
     */
    getActiveTrajectories() {
        const stmt = this.db.prepare(`
      SELECT * FROM trajectories WHERE end_time IS NULL
    `);
        return stmt.all();
    }
    /**
     * Get recent trajectories
     */
    getRecentTrajectories(limit = 10) {
        const stmt = this.db.prepare(`
      SELECT * FROM trajectories ORDER BY start_time DESC LIMIT ?
    `);
        return stmt.all(limit);
    }
    // ============ Pattern Methods ============
    /**
     * Store a pattern
     */
    storePattern(taskType, approach, embedding) {
        const stmt = this.db.prepare(`
      INSERT INTO patterns (task_type, approach, embedding)
      VALUES (?, ?, ?)
    `);
        const embeddingBuffer = embedding ? Buffer.from(embedding.buffer) : null;
        const result = stmt.run(taskType, approach, embeddingBuffer);
        this.incrementStat('total_patterns');
        return result.lastInsertRowid;
    }
    /**
     * Update pattern usage
     */
    updatePatternUsage(patternId, wasSuccessful) {
        const stmt = this.db.prepare(`
      UPDATE patterns
      SET usage_count = usage_count + 1,
          success_rate = (success_rate * usage_count + ?) / (usage_count + 1),
          updated_at = strftime('%s', 'now')
      WHERE id = ?
    `);
        stmt.run(wasSuccessful ? 1 : 0, patternId);
    }
    /**
     * Find patterns by task type
     */
    findPatterns(taskType, limit = 5) {
        const stmt = this.db.prepare(`
      SELECT * FROM patterns
      WHERE task_type LIKE ?
      ORDER BY success_rate DESC, usage_count DESC
      LIMIT ?
    `);
        return stmt.all(`%${taskType}%`, limit);
    }
    // ============ Routing Methods ============
    /**
     * Record a routing decision
     */
    recordRouting(task, recommendedAgent, confidence, latencyMs) {
        const stmt = this.db.prepare(`
      INSERT INTO routings (task, recommended_agent, confidence, latency_ms)
      VALUES (?, ?, ?, ?)
    `);
        const result = stmt.run(task, recommendedAgent, confidence, latencyMs);
        this.incrementStat('total_routings');
        return result.lastInsertRowid;
    }
    /**
     * Update routing outcome
     */
    updateRoutingOutcome(routingId, wasSuccessful) {
        const stmt = this.db.prepare(`
      UPDATE routings SET was_successful = ? WHERE id = ?
    `);
        stmt.run(wasSuccessful ? 1 : 0, routingId);
        if (wasSuccessful) {
            this.incrementStat('successful_routings');
        }
    }
    /**
     * Get routing accuracy for an agent
     */
    getAgentAccuracy(agent) {
        const stmt = this.db.prepare(`
      SELECT
        COUNT(*) as total,
        SUM(was_successful) as successful
      FROM routings
      WHERE recommended_agent = ?
    `);
        const result = stmt.get(agent);
        return {
            total: result.total || 0,
            successful: result.successful || 0,
            accuracy: result.total > 0 ? (result.successful || 0) / result.total : 0,
        };
    }
    // ============ Stats Methods ============
    /**
     * Get all stats
     */
    getStats() {
        const stmt = this.db.prepare(`SELECT * FROM stats WHERE id = 1`);
        const row = stmt.get();
        return {
            totalTrajectories: row?.total_trajectories || 0,
            successfulTrajectories: row?.successful_trajectories || 0,
            totalRoutings: row?.total_routings || 0,
            successfulRoutings: row?.successful_routings || 0,
            totalPatterns: row?.total_patterns || 0,
            sonaAdaptations: row?.sona_adaptations || 0,
            hnswQueries: row?.hnsw_queries || 0,
            lastUpdated: row?.last_updated || Date.now(),
        };
    }
    /**
     * Increment a stat counter
     */
    incrementStat(statName, amount = 1) {
        const stmt = this.db.prepare(`
      UPDATE stats SET ${statName} = ${statName} + ?, last_updated = strftime('%s', 'now')
      WHERE id = 1
    `);
        stmt.run(amount);
    }
    /**
     * Record SONA adaptation
     */
    recordSonaAdaptation() {
        this.incrementStat('sona_adaptations');
    }
    /**
     * Record HNSW query
     */
    recordHnswQuery() {
        this.incrementStat('hnsw_queries');
    }
    // ============ Utility Methods ============
    /**
     * Get summary for display (simplified for UI)
     */
    getSummary() {
        const stats = this.getStats();
        return {
            trajectories: stats.totalTrajectories,
            routings: stats.totalRoutings,
            patterns: stats.totalPatterns,
            operations: stats.sonaAdaptations + stats.hnswQueries,
        };
    }
    /**
     * Get detailed summary for reports
     */
    getDetailedSummary() {
        const stats = this.getStats();
        const activeCount = this.getActiveTrajectories().length;
        return {
            trajectories: {
                total: stats.totalTrajectories,
                active: activeCount,
                successful: stats.successfulTrajectories,
            },
            routings: {
                total: stats.totalRoutings,
                accuracy: stats.totalRoutings > 0
                    ? stats.successfulRoutings / stats.totalRoutings
                    : 0,
            },
            patterns: stats.totalPatterns,
            operations: {
                sona: stats.sonaAdaptations,
                hnsw: stats.hnswQueries,
            },
        };
    }
    /**
     * Close database connection
     */
    close() {
        this.db.close();
        IntelligenceStore.instance = null;
    }
    /**
     * Reset all data (for testing)
     */
    reset() {
        this.db.exec(`
      DELETE FROM trajectories;
      DELETE FROM patterns;
      DELETE FROM routings;
      UPDATE stats SET
        total_trajectories = 0,
        successful_trajectories = 0,
        total_routings = 0,
        successful_routings = 0,
        total_patterns = 0,
        sona_adaptations = 0,
        hnsw_queries = 0,
        last_updated = strftime('%s', 'now')
      WHERE id = 1;
    `);
    }
}
// Export singleton getter
export function getIntelligenceStore(dbPath) {
    return IntelligenceStore.getInstance(dbPath);
}
//# sourceMappingURL=IntelligenceStore.js.map