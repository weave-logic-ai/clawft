/**
 * ReflexionMemory - Episodic Replay Memory System
 *
 * Implements reflexion-style episodic replay for agent self-improvement.
 * Stores self-critiques and outcomes, retrieves relevant past experiences.
 *
 * Based on: "Reflexion: Language Agents with Verbal Reinforcement Learning"
 * https://arxiv.org/abs/2303.11366
 */
import { normalizeRowId } from '../types/database.types.js';
import { NodeIdMapper } from '../utils/NodeIdMapper.js';
import { QueryCache } from '../core/QueryCache.js';
export class ReflexionMemory {
    db;
    embedder;
    vectorBackend;
    learningBackend;
    graphBackend;
    queryCache;
    constructor(db, embedder, vectorBackend, learningBackend, graphBackend, cacheConfig) {
        this.db = db;
        this.embedder = embedder;
        this.vectorBackend = vectorBackend;
        this.learningBackend = learningBackend;
        this.graphBackend = graphBackend;
        this.queryCache = new QueryCache(cacheConfig);
    }
    /**
     * Store a new episode with its critique and outcome
     * Invalidates relevant cache entries
     */
    async storeEpisode(episode) {
        // Invalidate episode caches on write
        this.queryCache.invalidateCategory('episodes');
        this.queryCache.invalidateCategory('task-stats');
        // Use GraphDatabaseAdapter if available (AgentDB v2)
        if (this.graphBackend && 'storeEpisode' in this.graphBackend) {
            // GraphDatabaseAdapter has specialized storeEpisode method
            const graphAdapter = this.graphBackend;
            // Generate embedding for the task
            const taskEmbedding = await this.embedder.embed(episode.task);
            // Create episode node using GraphDatabaseAdapter
            const nodeId = await graphAdapter.storeEpisode({
                id: episode.id ? `episode-${episode.id}` : `episode-${Date.now()}-${Math.random()}`,
                sessionId: episode.sessionId,
                task: episode.task,
                reward: episode.reward,
                success: episode.success,
                input: episode.input,
                output: episode.output,
                critique: episode.critique,
                createdAt: episode.ts ? episode.ts * 1000 : Date.now(),
                tokensUsed: episode.tokensUsed,
                latencyMs: episode.latencyMs,
            }, taskEmbedding);
            // Return a numeric ID (parse from string ID)
            const numericId = parseInt(nodeId.split('-').pop() || '0', 36);
            // Register mapping for later use by CausalMemoryGraph
            NodeIdMapper.getInstance().register(numericId, nodeId);
            return numericId;
        }
        // Use generic GraphBackend if available
        if (this.graphBackend) {
            // Generate embedding for the task
            const taskEmbedding = await this.embedder.embed(episode.task);
            // Create episode node ID
            const nodeId = await this.graphBackend.createNode(['Episode'], {
                sessionId: episode.sessionId,
                task: episode.task,
                input: episode.input || '',
                output: episode.output || '',
                critique: episode.critique || '',
                reward: episode.reward,
                success: episode.success,
                latencyMs: episode.latencyMs || 0,
                tokensUsed: episode.tokensUsed || 0,
                tags: episode.tags ? JSON.stringify(episode.tags) : '[]',
                metadata: episode.metadata ? JSON.stringify(episode.metadata) : '{}',
                createdAt: Date.now(),
            });
            // Store embedding using vectorBackend if available
            if (this.vectorBackend && taskEmbedding) {
                this.vectorBackend.insert(nodeId, taskEmbedding, {
                    type: 'episode',
                    sessionId: episode.sessionId,
                });
            }
            // Return a numeric ID (parse from string ID)
            const numericId = parseInt(nodeId.split('-').pop() || '0', 36);
            // Register mapping for later use by CausalMemoryGraph
            NodeIdMapper.getInstance().register(numericId, nodeId);
            return numericId;
        }
        // Fallback to SQLite (v1 compatibility)
        const stmt = this.db.prepare(`
      INSERT INTO episodes (
        session_id, task, input, output, critique, reward, success,
        latency_ms, tokens_used, tags, metadata
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);
        const tags = episode.tags ? JSON.stringify(episode.tags) : null;
        const metadata = episode.metadata ? JSON.stringify(episode.metadata) : null;
        const result = stmt.run(episode.sessionId, episode.task, episode.input || null, episode.output || null, episode.critique || null, episode.reward, episode.success ? 1 : 0, episode.latencyMs || null, episode.tokensUsed || null, tags, metadata);
        const episodeId = normalizeRowId(result.lastInsertRowid);
        // Generate and store embedding
        const text = this.buildEpisodeText(episode);
        const embedding = await this.embedder.embed(text);
        // Use vector backend if available (150x faster retrieval)
        if (this.vectorBackend) {
            this.vectorBackend.insert(episodeId.toString(), embedding);
        }
        // Also store in SQL for fallback
        this.storeEmbedding(episodeId, embedding);
        // Create graph node for episode if graph backend available
        if (this.graphBackend) {
            await this.createEpisodeGraphNode(episodeId, episode, embedding);
        }
        // Add training sample if learning backend available
        if (this.learningBackend && episode.success !== undefined) {
            this.learningBackend.addSample({
                embedding,
                label: episode.success ? 1 : 0,
                weight: Math.abs(episode.reward),
                context: {
                    task: episode.task,
                    sessionId: episode.sessionId,
                    latencyMs: episode.latencyMs,
                    tokensUsed: episode.tokensUsed,
                },
            });
        }
        return episodeId;
    }
    /**
     * Retrieve relevant past episodes for a new task attempt
     * Results are cached for improved performance
     */
    async retrieveRelevant(query) {
        const { task, currentState = '', k = 5, minReward, onlyFailures = false, onlySuccesses = false, timeWindowDays, } = query;
        // Check cache first
        const cacheKey = this.queryCache.generateKey('retrieveRelevant', [task, currentState, k, minReward, onlyFailures, onlySuccesses, timeWindowDays], 'episodes');
        const cached = this.queryCache.get(cacheKey);
        if (cached) {
            return cached;
        }
        // Generate and enhance query embedding
        const queryEmbedding = await this.prepareQueryEmbedding(task, currentState, k);
        // Try different retrieval strategies in order of preference
        let episodes = [];
        if (this.graphBackend && 'searchSimilarEpisodes' in this.graphBackend) {
            episodes = await this.retrieveFromGraphAdapter(queryEmbedding, query);
        }
        else if (this.graphBackend && 'execute' in this.graphBackend) {
            episodes = await this.retrieveFromGenericGraph(query);
        }
        else if (this.vectorBackend) {
            episodes = await this.retrieveFromVectorBackend(queryEmbedding, query);
        }
        else {
            episodes = await this.retrieveFromSQLFallback(queryEmbedding, query);
        }
        // Cache and return results
        this.queryCache.set(cacheKey, episodes);
        return episodes;
    }
    /**
     * Prepare and enhance query embedding for search
     */
    async prepareQueryEmbedding(task, currentState, k) {
        const queryText = currentState ? `${task}\n${currentState}` : task;
        let queryEmbedding = await this.embedder.embed(queryText);
        // Enhance query with GNN if learning backend available
        if (this.learningBackend) {
            queryEmbedding = await this.enhanceQueryWithGNN(queryEmbedding, k);
        }
        return queryEmbedding;
    }
    /**
     * Retrieve episodes using GraphDatabaseAdapter (AgentDB v2)
     */
    async retrieveFromGraphAdapter(queryEmbedding, query) {
        const { k = 5, minReward, onlyFailures, onlySuccesses, timeWindowDays } = query;
        const graphAdapter = this.graphBackend;
        // Search using vector similarity
        const results = await graphAdapter.searchSimilarEpisodes(queryEmbedding, k * 3);
        // Apply filters
        const filtered = this.applyEpisodeFilters(results, {
            minReward,
            onlyFailures,
            onlySuccesses,
            timeWindowDays,
        });
        // Convert to EpisodeWithEmbedding format
        return filtered.slice(0, k).map((ep) => this.convertGraphEpisode(ep));
    }
    /**
     * Retrieve episodes using generic GraphBackend
     */
    async retrieveFromGenericGraph(query) {
        const { k = 5 } = query;
        const cypherQuery = this.buildCypherQuery(query);
        const result = await this.graphBackend.execute(cypherQuery);
        // Convert to EpisodeWithEmbedding format
        const episodes = result.rows.map((row) => this.convertCypherEpisode(row.e));
        return episodes.slice(0, k);
    }
    /**
     * Retrieve episodes using VectorBackend (150x faster)
     */
    async retrieveFromVectorBackend(queryEmbedding, query) {
        const { k = 5, minReward, onlyFailures, onlySuccesses, timeWindowDays } = query;
        // Get candidates from vector backend
        const searchResults = this.vectorBackend.search(queryEmbedding, k * 3, {
            threshold: 0.0,
        });
        // Fetch full episode data from DB
        const episodeIds = searchResults.map((r) => parseInt(r.id));
        if (episodeIds.length === 0) {
            return [];
        }
        const rows = this.fetchEpisodesByIds(episodeIds);
        const episodeMap = new Map(rows.map((r) => [r.id.toString(), r]));
        // Map results with similarity scores and apply filters
        const episodes = [];
        for (const result of searchResults) {
            const row = episodeMap.get(result.id);
            if (!row)
                continue;
            // Apply filters
            if (!this.passesEpisodeFilters(row, { minReward, onlyFailures, onlySuccesses, timeWindowDays })) {
                continue;
            }
            episodes.push(this.convertDatabaseEpisode(row, result.similarity));
            if (episodes.length >= k)
                break;
        }
        return episodes;
    }
    /**
     * Retrieve episodes using SQL-based similarity search (fallback)
     */
    async retrieveFromSQLFallback(queryEmbedding, query) {
        const { k = 5 } = query;
        const { whereClause, params } = this.buildSQLFilters(query);
        const stmt = this.db.prepare(`
      SELECT e.*, ee.embedding
      FROM episodes e
      JOIN episode_embeddings ee ON e.id = ee.episode_id
      ${whereClause}
      ORDER BY e.reward DESC
    `);
        const rows = stmt.all(...params);
        // Calculate similarities and convert
        const episodes = rows.map((row) => {
            const embedding = this.deserializeEmbedding(row.embedding);
            const similarity = this.cosineSimilarity(queryEmbedding, embedding);
            return this.convertDatabaseEpisode(row, similarity, embedding);
        });
        // Sort by similarity and return top-k
        episodes.sort((a, b) => (b.similarity || 0) - (a.similarity || 0));
        return episodes.slice(0, k);
    }
    /**
     * Apply episode filters to search results
     */
    applyEpisodeFilters(episodes, filters) {
        return episodes.filter((ep) => {
            if (filters.minReward !== undefined && ep.reward < filters.minReward)
                return false;
            if (filters.onlyFailures && ep.success)
                return false;
            if (filters.onlySuccesses && !ep.success)
                return false;
            if (filters.timeWindowDays && ep.createdAt < Date.now() - filters.timeWindowDays * 86400000)
                return false;
            return true;
        });
    }
    /**
     * Check if database row passes episode filters
     */
    passesEpisodeFilters(row, filters) {
        if (filters.minReward !== undefined && row.reward < filters.minReward)
            return false;
        if (filters.onlyFailures && row.success === 1)
            return false;
        if (filters.onlySuccesses && row.success === 0)
            return false;
        if (filters.timeWindowDays && row.ts < Date.now() / 1000 - filters.timeWindowDays * 86400)
            return false;
        return true;
    }
    /**
     * Build Cypher query with filters
     */
    buildCypherQuery(query) {
        const { k = 5, minReward, onlyFailures, onlySuccesses, timeWindowDays } = query;
        let cypherQuery = 'MATCH (e:Episode) WHERE 1=1';
        if (minReward !== undefined) {
            cypherQuery += ` AND e.reward >= ${minReward}`;
        }
        if (onlyFailures) {
            cypherQuery += ` AND e.success = false`;
        }
        if (onlySuccesses) {
            cypherQuery += ` AND e.success = true`;
        }
        if (timeWindowDays) {
            const cutoff = Date.now() - timeWindowDays * 86400000;
            cypherQuery += ` AND e.createdAt >= ${cutoff}`;
        }
        cypherQuery += ` RETURN e LIMIT ${k * 3}`;
        return cypherQuery;
    }
    /**
     * Build SQL WHERE clause and parameters for filters
     */
    buildSQLFilters(query) {
        const { minReward, onlyFailures, onlySuccesses, timeWindowDays } = query;
        const filters = [];
        const params = [];
        if (minReward !== undefined) {
            filters.push('e.reward >= ?');
            params.push(minReward);
        }
        if (onlyFailures) {
            filters.push('e.success = 0');
        }
        if (onlySuccesses) {
            filters.push('e.success = 1');
        }
        if (timeWindowDays) {
            filters.push('e.ts > strftime("%s", "now") - ?');
            params.push(timeWindowDays * 86400);
        }
        const whereClause = filters.length > 0 ? `WHERE ${filters.join(' AND ')}` : '';
        return { whereClause, params };
    }
    /**
     * Fetch episodes by IDs from database
     */
    fetchEpisodesByIds(episodeIds) {
        const placeholders = episodeIds.map(() => '?').join(',');
        const stmt = this.db.prepare(`
      SELECT * FROM episodes
      WHERE id IN (${placeholders})
    `);
        return stmt.all(...episodeIds);
    }
    /**
     * Convert GraphDatabaseAdapter episode to EpisodeWithEmbedding
     */
    convertGraphEpisode(ep) {
        return {
            id: parseInt(ep.id.split('-').pop() || '0', 36),
            sessionId: ep.sessionId,
            task: ep.task,
            input: ep.input,
            output: ep.output,
            critique: ep.critique,
            reward: ep.reward,
            success: ep.success,
            latencyMs: ep.latencyMs,
            tokensUsed: ep.tokensUsed,
            ts: Math.floor(ep.createdAt / 1000),
        };
    }
    /**
     * Convert Cypher query result to EpisodeWithEmbedding
     */
    convertCypherEpisode(node) {
        return {
            id: parseInt(node.id.split('-').pop() || '0', 36),
            sessionId: node.properties.sessionId,
            task: node.properties.task,
            input: node.properties.input,
            output: node.properties.output,
            critique: node.properties.critique,
            reward: typeof node.properties.reward === 'string'
                ? parseFloat(node.properties.reward)
                : node.properties.reward,
            success: typeof node.properties.success === 'string'
                ? node.properties.success === 'true'
                : node.properties.success,
            latencyMs: node.properties.latencyMs,
            tokensUsed: node.properties.tokensUsed,
            tags: node.properties.tags ? JSON.parse(node.properties.tags) : [],
            metadata: node.properties.metadata ? JSON.parse(node.properties.metadata) : {},
            ts: Math.floor(node.properties.createdAt / 1000),
        };
    }
    /**
     * Convert database row to EpisodeWithEmbedding
     */
    convertDatabaseEpisode(row, similarity, embedding) {
        return {
            id: row.id,
            ts: row.ts,
            sessionId: row.session_id,
            task: row.task,
            input: row.input ?? undefined,
            output: row.output ?? undefined,
            critique: row.critique ?? undefined,
            reward: row.reward,
            success: row.success === 1,
            latencyMs: row.latency_ms ?? undefined,
            tokensUsed: row.tokens_used ?? undefined,
            tags: row.tags ? JSON.parse(row.tags) : undefined,
            metadata: row.metadata ? JSON.parse(row.metadata) : undefined,
            embedding,
            similarity,
        };
    }
    /**
     * Get statistics for a task (cached)
     */
    getTaskStats(task, timeWindowDays) {
        // Check cache first
        const cacheKey = this.queryCache.generateKey('getTaskStats', [task, timeWindowDays], 'task-stats');
        const cached = this.queryCache.get(cacheKey);
        if (cached) {
            return cached;
        }
        const windowFilter = timeWindowDays
            ? `AND ts > strftime('%s', 'now') - ${timeWindowDays * 86400}`
            : '';
        const stmt = this.db.prepare(`
      SELECT
        COUNT(*) as total,
        AVG(CASE WHEN success = 1 THEN 1.0 ELSE 0.0 END) as success_rate,
        AVG(reward) as avg_reward,
        AVG(latency_ms) as avg_latency
      FROM episodes
      WHERE task = ? ${windowFilter}
    `);
        const stats = stmt.get(task);
        // Calculate improvement trend (recent vs older)
        const trendStmt = this.db.prepare(`
      SELECT
        AVG(CASE
          WHEN ts > strftime('%s', 'now') - ${7 * 86400} THEN reward
        END) as recent_reward,
        AVG(CASE
          WHEN ts <= strftime('%s', 'now') - ${7 * 86400} THEN reward
        END) as older_reward
      FROM episodes
      WHERE task = ? ${windowFilter}
    `);
        const trend = trendStmt.get(task);
        const improvementTrend = trend.recent_reward && trend.older_reward
            ? (trend.recent_reward - trend.older_reward) / trend.older_reward
            : 0;
        const results = {
            totalAttempts: stats?.total ?? 0,
            successRate: stats?.success_rate ?? 0,
            avgReward: stats?.avg_reward ?? 0,
            avgLatency: stats?.avg_latency ?? 0,
            improvementTrend,
        };
        // Cache the results
        this.queryCache.set(cacheKey, results);
        return results;
    }
    /**
     * Build critique summary from similar failed episodes (cached)
     */
    async getCritiqueSummary(query) {
        // Check cache first
        const cacheKey = this.queryCache.generateKey('getCritiqueSummary', [query.task, query.k], 'episodes');
        const cached = this.queryCache.get(cacheKey);
        if (cached) {
            return cached;
        }
        const failures = await this.retrieveRelevant({
            ...query,
            onlyFailures: true,
            k: 3,
        });
        if (failures.length === 0) {
            return 'No prior failures found for this task.';
        }
        const critiques = failures
            .filter((ep) => ep.critique)
            .map((ep, i) => `${i + 1}. ${ep.critique} (reward: ${ep.reward.toFixed(2)})`)
            .join('\n');
        const result = `Prior failures and lessons learned:\n${critiques}`;
        // Cache the result
        this.queryCache.set(cacheKey, result);
        return result;
    }
    /**
     * Get successful strategies for a task (cached)
     */
    async getSuccessStrategies(query) {
        // Check cache first
        const cacheKey = this.queryCache.generateKey('getSuccessStrategies', [query.task, query.k], 'episodes');
        const cached = this.queryCache.get(cacheKey);
        if (cached) {
            return cached;
        }
        const successes = await this.retrieveRelevant({
            ...query,
            onlySuccesses: true,
            minReward: 0.7,
            k: 3,
        });
        if (successes.length === 0) {
            return 'No successful strategies found for this task.';
        }
        const strategies = successes
            .map((ep, i) => {
            const approach = ep.output?.substring(0, 200) || 'No output recorded';
            return `${i + 1}. Approach (reward ${ep.reward.toFixed(2)}): ${approach}...`;
        })
            .join('\n');
        const result = `Successful strategies:\n${strategies}`;
        // Cache the result
        this.queryCache.set(cacheKey, result);
        return result;
    }
    /**
     * Get recent episodes for a session
     */
    async getRecentEpisodes(sessionId, limit = 10) {
        const stmt = this.db.prepare(`
      SELECT * FROM episodes
      WHERE session_id = ?
      ORDER BY ts DESC
      LIMIT ?
    `);
        const rows = stmt.all(sessionId, limit);
        return rows.map((row) => ({
            id: row.id,
            ts: row.ts,
            sessionId: row.session_id,
            task: row.task,
            input: row.input ?? undefined,
            output: row.output ?? undefined,
            critique: row.critique ?? undefined,
            reward: row.reward,
            success: row.success === 1,
            latencyMs: row.latency_ms ?? undefined,
            tokensUsed: row.tokens_used ?? undefined,
            tags: row.tags ? JSON.parse(row.tags) : undefined,
            metadata: row.metadata ? JSON.parse(row.metadata) : undefined,
        }));
    }
    /**
     * Prune low-quality episodes based on TTL and quality threshold
     * Invalidates cache on completion
     */
    pruneEpisodes(config) {
        const { minReward = 0.3, maxAgeDays = 30, keepMinPerTask = 5 } = config;
        // Keep high-reward episodes and minimum per task
        const stmt = this.db.prepare(`
      DELETE FROM episodes
      WHERE id IN (
        SELECT id FROM (
          SELECT
            id,
            reward,
            ts,
            ROW_NUMBER() OVER (PARTITION BY task ORDER BY reward DESC) as rank
          FROM episodes
          WHERE reward < ?
            AND ts < strftime('%s', 'now') - ?
        ) WHERE rank > ?
      )
    `);
        const result = stmt.run(minReward, maxAgeDays * 86400, keepMinPerTask);
        // Invalidate caches after pruning
        if (result.changes > 0) {
            this.queryCache.invalidateCategory('episodes');
            this.queryCache.invalidateCategory('task-stats');
        }
        return result.changes;
    }
    // ========================================================================
    // Private Helper Methods
    // ========================================================================
    buildEpisodeText(episode) {
        const parts = [episode.task];
        if (episode.critique)
            parts.push(episode.critique);
        if (episode.output)
            parts.push(episode.output);
        return parts.join('\n');
    }
    storeEmbedding(episodeId, embedding) {
        const stmt = this.db.prepare(`
      INSERT INTO episode_embeddings (episode_id, embedding)
      VALUES (?, ?)
    `);
        stmt.run(episodeId, this.serializeEmbedding(embedding));
    }
    serializeEmbedding(embedding) {
        // Handle empty/null embeddings
        if (!embedding || !embedding.buffer) {
            return Buffer.alloc(0);
        }
        return Buffer.from(embedding.buffer);
    }
    deserializeEmbedding(buffer) {
        return new Float32Array(buffer.buffer, buffer.byteOffset, buffer.length / 4);
    }
    cosineSimilarity(a, b) {
        let dotProduct = 0;
        let normA = 0;
        let normB = 0;
        for (let i = 0; i < a.length; i++) {
            dotProduct += a[i] * b[i];
            normA += a[i] * a[i];
            normB += b[i] * b[i];
        }
        return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
    }
    // ========================================================================
    // GNN and Graph Integration Methods
    // ========================================================================
    /**
     * Create graph node for episode with relationships
     */
    async createEpisodeGraphNode(episodeId, episode, embedding) {
        if (!this.graphBackend)
            return;
        // Create episode node
        const nodeId = await this.graphBackend.createNode(['Episode', episode.success ? 'Success' : 'Failure'], {
            episodeId,
            sessionId: episode.sessionId,
            task: episode.task,
            reward: episode.reward,
            success: episode.success,
            timestamp: episode.ts || Date.now(),
            latencyMs: episode.latencyMs,
            tokensUsed: episode.tokensUsed,
        });
        // Find similar episodes using graph vector search
        const similarEpisodes = await this.graphBackend.vectorSearch(embedding, 5, nodeId);
        // Create similarity relationships to similar episodes
        for (const similar of similarEpisodes) {
            if (similar.id !== nodeId && similar.properties.episodeId !== episodeId) {
                await this.graphBackend.createRelationship(nodeId, similar.id, 'SIMILAR_TO', {
                    similarity: this.cosineSimilarity(embedding, similar.embedding || new Float32Array()),
                    createdAt: Date.now(),
                });
            }
        }
        // Create session relationship
        const sessionNodes = await this.graphBackend.execute('MATCH (s:Session {sessionId: $sessionId}) RETURN s', { sessionId: episode.sessionId });
        let sessionNodeId;
        if (sessionNodes.rows.length === 0) {
            // Create session node if doesn't exist
            sessionNodeId = await this.graphBackend.createNode(['Session'], {
                sessionId: episode.sessionId,
                startTime: episode.ts || Date.now(),
            });
        }
        else {
            sessionNodeId = sessionNodes.rows[0].s.id;
        }
        await this.graphBackend.createRelationship(nodeId, sessionNodeId, 'BELONGS_TO_SESSION', {
            timestamp: episode.ts || Date.now(),
        });
        // If episode has critique, create causal relationship to previous failures
        if (episode.critique && !episode.success) {
            const previousFailures = await this.graphBackend.execute(`MATCH (e:Episode:Failure {sessionId: $sessionId})
         WHERE e.timestamp < $timestamp
         RETURN e
         ORDER BY e.timestamp DESC
         LIMIT 3`, { sessionId: episode.sessionId, timestamp: episode.ts || Date.now() });
            for (const prevFailure of previousFailures.rows) {
                await this.graphBackend.createRelationship(nodeId, prevFailure.e.id, 'LEARNED_FROM', {
                    critique: episode.critique,
                    improvementAttempt: true,
                });
            }
        }
    }
    /**
     * Enhance query embedding using GNN attention mechanism
     */
    async enhanceQueryWithGNN(queryEmbedding, k) {
        if (!this.learningBackend || !this.vectorBackend) {
            return queryEmbedding;
        }
        try {
            // Get initial neighbors
            const initialResults = this.vectorBackend.search(queryEmbedding, k * 2, {
                threshold: 0.0,
            });
            if (initialResults.length === 0) {
                return queryEmbedding;
            }
            // Fetch neighbor embeddings
            const neighborEmbeddings = [];
            const weights = [];
            const episodeIds = initialResults.map((r) => r.id);
            const placeholders = episodeIds.map(() => '?').join(',');
            const episodes = this.db
                .prepare(`
        SELECT ee.embedding, e.reward
        FROM episode_embeddings ee
        JOIN episodes e ON e.id = ee.episode_id
        WHERE ee.episode_id IN (${placeholders})
      `)
                .all(...episodeIds);
            for (const ep of episodes) {
                const embedding = this.deserializeEmbedding(ep.embedding);
                neighborEmbeddings.push(embedding);
                // Use reward as weight (higher reward = more important)
                weights.push(Math.max(0.1, ep.reward));
            }
            // Enhance query using GNN
            const enhanced = this.learningBackend.enhance(queryEmbedding, neighborEmbeddings, weights);
            return enhanced;
        }
        catch (error) {
            console.warn('[ReflexionMemory] GNN enhancement failed:', error);
            return queryEmbedding;
        }
    }
    /**
     * Get graph-based episode relationships
     */
    async getEpisodeRelationships(episodeId) {
        if (!this.graphBackend) {
            return { similar: [], session: '', learnedFrom: [] };
        }
        const result = await this.graphBackend.execute(`MATCH (e:Episode {episodeId: $episodeId})
       OPTIONAL MATCH (e)-[:SIMILAR_TO]->(similar:Episode)
       OPTIONAL MATCH (e)-[:BELONGS_TO_SESSION]->(s:Session)
       OPTIONAL MATCH (e)-[:LEARNED_FROM]->(learned:Episode)
       RETURN e, collect(DISTINCT similar.episodeId) as similar,
              s.sessionId as session,
              collect(DISTINCT learned.episodeId) as learnedFrom`, { episodeId });
        if (result.rows.length === 0) {
            return { similar: [], session: '', learnedFrom: [] };
        }
        const row = result.rows[0];
        return {
            similar: (row.similar || []).filter((id) => id != null),
            session: row.session || '',
            learnedFrom: (row.learnedFrom || []).filter((id) => id != null),
        };
    }
    /**
     * Train GNN model on accumulated samples
     */
    async trainGNN(options) {
        if (!this.learningBackend) {
            console.warn('[ReflexionMemory] No learning backend available for training');
            return;
        }
        const stats = this.learningBackend.getStats();
        if (stats.samplesCollected < 10) {
            console.warn('[ReflexionMemory] Not enough samples for training (need at least 10)');
            return;
        }
        const result = await this.learningBackend.train(options);
        console.log('[ReflexionMemory] GNN training complete:', {
            epochs: result.epochs,
            finalLoss: result.finalLoss.toFixed(4),
            improvement: `${(result.improvement * 100).toFixed(1)}%`,
            duration: `${result.duration}ms`,
        });
    }
    /**
     * Get learning backend statistics
     */
    getLearningStats() {
        if (!this.learningBackend) {
            return null;
        }
        return this.learningBackend.getStats();
    }
    /**
     * Get graph backend statistics
     */
    getGraphStats() {
        if (!this.graphBackend) {
            return null;
        }
        return this.graphBackend.getStats();
    }
    /**
     * Get query cache statistics
     */
    getCacheStats() {
        return this.queryCache.getStatistics();
    }
    /**
     * Clear query cache
     */
    clearCache() {
        this.queryCache.clear();
    }
    /**
     * Prune expired cache entries
     */
    pruneCache() {
        return this.queryCache.pruneExpired();
    }
    /**
     * Warm cache with common queries
     */
    async warmCache(sessionId) {
        await this.queryCache.warm(async (cache) => {
            // Warm cache with recent sessions if sessionId provided
            if (sessionId) {
                const recent = await this.getRecentEpisodes(sessionId, 10);
                // Episodes are already loaded, cache will be populated on next access
            }
        });
    }
}
//# sourceMappingURL=ReflexionMemory.js.map