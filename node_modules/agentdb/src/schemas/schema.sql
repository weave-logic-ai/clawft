-- ============================================================================
-- AgentDB State-of-the-Art Memory Schema
-- ============================================================================
-- Implements 5 cutting-edge memory patterns for autonomous agents:
-- 1. Reflexion-style episodic replay
-- 2. Skill library from trajectories
-- 3. Structured mixed memory (facts + summaries)
-- 4. Episodic segmentation and consolidation
-- 5. Graph-aware recall
-- ============================================================================

-- Enable foreign keys
PRAGMA foreign_keys = ON;

-- ============================================================================
-- Pattern 1: Reflexion-Style Episodic Replay
-- ============================================================================
-- Store self-critique and outcomes after each attempt.
-- Retrieve nearest failures and fixes before the next run.

CREATE TABLE IF NOT EXISTS episodes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  ts INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  session_id TEXT NOT NULL,
  task TEXT NOT NULL,
  input TEXT,
  output TEXT,
  critique TEXT,
  reward REAL DEFAULT 0.0,
  success BOOLEAN DEFAULT 0,
  latency_ms INTEGER,
  tokens_used INTEGER,
  tags TEXT, -- JSON array of tags
  metadata JSON,
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_episodes_ts ON episodes(ts DESC);
CREATE INDEX IF NOT EXISTS idx_episodes_session ON episodes(session_id);
CREATE INDEX IF NOT EXISTS idx_episodes_reward ON episodes(reward DESC);
CREATE INDEX IF NOT EXISTS idx_episodes_task ON episodes(task);

-- Vector embeddings for episodes (384-dim for all-MiniLM-L6-v2)
-- Will use sqlite-vec when available, fallback to JSON storage
CREATE TABLE IF NOT EXISTS episode_embeddings (
  episode_id INTEGER PRIMARY KEY,
  embedding BLOB NOT NULL, -- Float32Array as BLOB
  embedding_model TEXT DEFAULT 'all-MiniLM-L6-v2',
  FOREIGN KEY(episode_id) REFERENCES episodes(id) ON DELETE CASCADE
);

-- ============================================================================
-- Pattern 2: Skill Library from Trajectories
-- ============================================================================
-- Promote high-reward traces into reusable "skills" with typed IO.

CREATE TABLE IF NOT EXISTS skills (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT UNIQUE NOT NULL,
  description TEXT,
  signature JSON NOT NULL, -- {inputs: {...}, outputs: {...}}
  code TEXT, -- Tool call manifest or code template
  success_rate REAL DEFAULT 0.0,
  uses INTEGER DEFAULT 0,
  avg_reward REAL DEFAULT 0.0,
  avg_latency_ms INTEGER DEFAULT 0,
  created_from_episode INTEGER, -- Source episode ID
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  last_used_at INTEGER,
  metadata JSON,
  FOREIGN KEY(created_from_episode) REFERENCES episodes(id)
);

CREATE INDEX IF NOT EXISTS idx_skills_success ON skills(success_rate DESC);
CREATE INDEX IF NOT EXISTS idx_skills_uses ON skills(uses DESC);
CREATE INDEX IF NOT EXISTS idx_skills_name ON skills(name);

-- Skill relationships and composition
CREATE TABLE IF NOT EXISTS skill_links (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  parent_skill_id INTEGER NOT NULL,
  child_skill_id INTEGER NOT NULL,
  relationship TEXT NOT NULL, -- 'prerequisite', 'alternative', 'refinement', 'composition'
  weight REAL DEFAULT 1.0,
  metadata JSON,
  FOREIGN KEY(parent_skill_id) REFERENCES skills(id) ON DELETE CASCADE,
  FOREIGN KEY(child_skill_id) REFERENCES skills(id) ON DELETE CASCADE,
  UNIQUE(parent_skill_id, child_skill_id, relationship)
);

CREATE INDEX IF NOT EXISTS idx_skill_links_parent ON skill_links(parent_skill_id);
CREATE INDEX IF NOT EXISTS idx_skill_links_child ON skill_links(child_skill_id);

-- Skill embeddings for semantic search
CREATE TABLE IF NOT EXISTS skill_embeddings (
  skill_id INTEGER PRIMARY KEY,
  embedding BLOB NOT NULL,
  embedding_model TEXT DEFAULT 'all-MiniLM-L6-v2',
  FOREIGN KEY(skill_id) REFERENCES skills(id) ON DELETE CASCADE
);

-- ============================================================================
-- Pattern 3: Structured Mixed Memory (Facts + Summaries)
-- ============================================================================
-- Combine facts, summaries, and vectors to avoid over-embedding.

-- Atomic facts as triples (subject-predicate-object)
CREATE TABLE IF NOT EXISTS facts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  subject TEXT NOT NULL,
  predicate TEXT NOT NULL,
  object TEXT NOT NULL,
  source_type TEXT, -- 'episode', 'skill', 'external', 'inferred'
  source_id INTEGER,
  confidence REAL DEFAULT 1.0,
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  expires_at INTEGER, -- TTL for temporal facts
  metadata JSON
);

CREATE INDEX IF NOT EXISTS idx_facts_subject ON facts(subject);
CREATE INDEX IF NOT EXISTS idx_facts_predicate ON facts(predicate);
CREATE INDEX IF NOT EXISTS idx_facts_object ON facts(object);
CREATE INDEX IF NOT EXISTS idx_facts_source ON facts(source_type, source_id);
CREATE INDEX IF NOT EXISTS idx_facts_expires ON facts(expires_at) WHERE expires_at IS NOT NULL;

-- Notes and summaries with semantic embeddings
CREATE TABLE IF NOT EXISTS notes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT,
  text TEXT NOT NULL,
  summary TEXT, -- Condensed version for context
  note_type TEXT DEFAULT 'general', -- 'insight', 'constraint', 'goal', 'observation'
  importance REAL DEFAULT 0.5,
  access_count INTEGER DEFAULT 0,
  last_accessed_at INTEGER,
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  metadata JSON
);

CREATE INDEX IF NOT EXISTS idx_notes_type ON notes(note_type);
CREATE INDEX IF NOT EXISTS idx_notes_importance ON notes(importance DESC);
CREATE INDEX IF NOT EXISTS idx_notes_accessed ON notes(last_accessed_at DESC);

-- Note embeddings (only for summaries to reduce storage)
CREATE TABLE IF NOT EXISTS note_embeddings (
  note_id INTEGER PRIMARY KEY,
  embedding BLOB NOT NULL,
  embedding_model TEXT DEFAULT 'all-MiniLM-L6-v2',
  FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
);

-- ============================================================================
-- Pattern 4: Episodic Segmentation and Consolidation
-- ============================================================================
-- Segment long tasks into events and consolidate into compact memories.

CREATE TABLE IF NOT EXISTS events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL,
  episode_id INTEGER, -- Link to parent episode
  step INTEGER NOT NULL,
  phase TEXT, -- 'planning', 'execution', 'reflection', 'learning'
  role TEXT, -- 'user', 'assistant', 'system', 'tool'
  content TEXT NOT NULL,
  features JSON, -- Extracted features for learning
  tool_calls JSON, -- Tool invocations in this event
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  FOREIGN KEY(episode_id) REFERENCES episodes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id, step);
CREATE INDEX IF NOT EXISTS idx_events_phase ON events(phase);
CREATE INDEX IF NOT EXISTS idx_events_episode ON events(episode_id);

-- Consolidated memories from event windows
CREATE TABLE IF NOT EXISTS consolidated_memories (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL,
  start_event_id INTEGER NOT NULL,
  end_event_id INTEGER NOT NULL,
  phase TEXT,
  summary TEXT NOT NULL,
  key_insights JSON, -- Extracted learnings
  success_patterns JSON, -- What worked
  failure_patterns JSON, -- What didn't work
  quality_score REAL DEFAULT 0.5,
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  FOREIGN KEY(start_event_id) REFERENCES events(id),
  FOREIGN KEY(end_event_id) REFERENCES events(id)
);

CREATE INDEX IF NOT EXISTS idx_consolidated_session ON consolidated_memories(session_id);
CREATE INDEX IF NOT EXISTS idx_consolidated_quality ON consolidated_memories(quality_score DESC);

-- ============================================================================
-- Pattern 5: Graph-Aware Recall (Lightweight GraphRAG)
-- ============================================================================
-- Build a lightweight GraphRAG overlay for experiences.

CREATE TABLE IF NOT EXISTS exp_nodes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  kind TEXT NOT NULL, -- 'task', 'skill', 'concept', 'tool', 'outcome'
  label TEXT NOT NULL,
  payload JSON,
  centrality REAL DEFAULT 0.0, -- Graph importance metric
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_exp_nodes_kind ON exp_nodes(kind);
CREATE INDEX IF NOT EXISTS idx_exp_nodes_label ON exp_nodes(label);
CREATE INDEX IF NOT EXISTS idx_exp_nodes_centrality ON exp_nodes(centrality DESC);

CREATE TABLE IF NOT EXISTS exp_edges (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  src_node_id INTEGER NOT NULL,
  dst_node_id INTEGER NOT NULL,
  relationship TEXT NOT NULL, -- 'requires', 'produces', 'similar_to', 'refines', 'part_of'
  weight REAL DEFAULT 1.0,
  metadata JSON,
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  FOREIGN KEY(src_node_id) REFERENCES exp_nodes(id) ON DELETE CASCADE,
  FOREIGN KEY(dst_node_id) REFERENCES exp_nodes(id) ON DELETE CASCADE,
  UNIQUE(src_node_id, dst_node_id, relationship)
);

CREATE INDEX IF NOT EXISTS idx_exp_edges_src ON exp_edges(src_node_id);
CREATE INDEX IF NOT EXISTS idx_exp_edges_dst ON exp_edges(dst_node_id);
CREATE INDEX IF NOT EXISTS idx_exp_edges_rel ON exp_edges(relationship);

-- Node embeddings for graph-augmented retrieval
CREATE TABLE IF NOT EXISTS exp_node_embeddings (
  node_id INTEGER PRIMARY KEY,
  embedding BLOB NOT NULL,
  embedding_model TEXT DEFAULT 'all-MiniLM-L6-v2',
  FOREIGN KEY(node_id) REFERENCES exp_nodes(id) ON DELETE CASCADE
);

-- ============================================================================
-- Memory Management and Scoring
-- ============================================================================

-- Track memory quality scores and usage statistics
CREATE TABLE IF NOT EXISTS memory_scores (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  memory_type TEXT NOT NULL, -- 'episode', 'skill', 'note', 'consolidated'
  memory_id INTEGER NOT NULL,
  quality_score REAL NOT NULL,
  novelty_score REAL,
  relevance_score REAL,
  utility_score REAL,
  computed_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  metadata JSON
);

CREATE INDEX IF NOT EXISTS idx_memory_scores_type ON memory_scores(memory_type, memory_id);
CREATE INDEX IF NOT EXISTS idx_memory_scores_quality ON memory_scores(quality_score DESC);

-- Memory access patterns for adaptive retrieval
CREATE TABLE IF NOT EXISTS memory_access_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  memory_type TEXT NOT NULL,
  memory_id INTEGER NOT NULL,
  query TEXT,
  relevance_score REAL,
  was_useful BOOLEAN,
  feedback JSON,
  accessed_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_access_log_type ON memory_access_log(memory_type, memory_id);
CREATE INDEX IF NOT EXISTS idx_access_log_time ON memory_access_log(accessed_at DESC);

-- ============================================================================
-- Consolidation and Maintenance
-- ============================================================================

-- Track consolidation jobs and their results
CREATE TABLE IF NOT EXISTS consolidation_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  job_type TEXT NOT NULL, -- 'episode_to_skill', 'event_to_memory', 'deduplication', 'pruning'
  records_processed INTEGER DEFAULT 0,
  records_created INTEGER DEFAULT 0,
  records_deleted INTEGER DEFAULT 0,
  duration_ms INTEGER,
  status TEXT DEFAULT 'pending', -- 'pending', 'running', 'completed', 'failed'
  error TEXT,
  started_at INTEGER,
  completed_at INTEGER,
  metadata JSON
);

CREATE INDEX IF NOT EXISTS idx_consolidation_status ON consolidation_runs(status);
CREATE INDEX IF NOT EXISTS idx_consolidation_type ON consolidation_runs(job_type);

-- ============================================================================
-- Views for Common Queries
-- ============================================================================

-- High-value episodes for skill creation
CREATE VIEW IF NOT EXISTS skill_candidates AS
SELECT
  task,
  COUNT(*) as attempt_count,
  AVG(reward) as avg_reward,
  AVG(success) as success_rate,
  MAX(id) as latest_episode_id,
  GROUP_CONCAT(id) as episode_ids
FROM episodes
WHERE ts > strftime('%s', 'now') - 86400 * 7 -- Last 7 days
GROUP BY task
HAVING attempt_count >= 3 AND avg_reward >= 0.7;

-- Top performing skills
CREATE VIEW IF NOT EXISTS top_skills AS
SELECT
  s.*,
  COALESCE(s.success_rate, 0) * 0.4 +
  COALESCE(s.uses, 0) * 0.0001 +
  COALESCE(s.avg_reward, 0) * 0.6 as composite_score
FROM skills s
ORDER BY composite_score DESC;

-- Recent high-quality memories
CREATE VIEW IF NOT EXISTS recent_quality_memories AS
SELECT
  'episode' as type, id, task as title, critique as content, reward as score, created_at
FROM episodes
WHERE reward >= 0.7 AND ts > strftime('%s', 'now') - 86400 * 3
UNION ALL
SELECT
  'note' as type, id, title, summary as content, importance as score, created_at
FROM notes
WHERE importance >= 0.7 AND created_at > strftime('%s', 'now') - 86400 * 3
UNION ALL
SELECT
  'consolidated' as type, id, session_id as title, summary as content, quality_score as score, created_at
FROM consolidated_memories
WHERE quality_score >= 0.7 AND created_at > strftime('%s', 'now') - 86400 * 3
ORDER BY created_at DESC;

-- ============================================================================
-- Triggers for Auto-Maintenance
-- ============================================================================

-- Update skill usage statistics
CREATE TRIGGER IF NOT EXISTS update_skill_last_used
AFTER UPDATE OF uses ON skills
BEGIN
  UPDATE skills SET last_used_at = strftime('%s', 'now') WHERE id = NEW.id;
END;

-- Update note access tracking
CREATE TRIGGER IF NOT EXISTS update_note_access
AFTER UPDATE OF access_count ON notes
BEGIN
  UPDATE notes SET last_accessed_at = strftime('%s', 'now') WHERE id = NEW.id;
END;

-- Auto-update timestamps
CREATE TRIGGER IF NOT EXISTS update_skill_timestamp
AFTER UPDATE ON skills
BEGIN
  UPDATE skills SET updated_at = strftime('%s', 'now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_note_timestamp
AFTER UPDATE ON notes
BEGIN
  UPDATE notes SET updated_at = strftime('%s', 'now') WHERE id = NEW.id;
END;

-- ============================================================================
-- Initialization Complete
-- ============================================================================
-- Schema version: 1.0.0
-- Compatible with: SQLite 3.35+, sqlite-vec (optional), sqlite-vss (optional)
-- WASM compatible: Yes (via SQLite-WASM + OPFS)
--
-- Performance Optimization:
-- For production deployments, apply composite index migration for 30-50% query speedup:
--   - Migration file: db/migrations/003_composite_indexes.sql
--   - Adds 40+ composite indexes for common query patterns
--   - Trade-off: 2x slower writes, +15-20% storage (acceptable for read-heavy workloads)
--   - See: db/migrations/README.md for details
-- ============================================================================
