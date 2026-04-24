-- ============================================================================
-- AgentDB Frontier Features Schema Extension
-- ============================================================================
-- Implements cutting-edge memory features:
-- 1. Causal Memory Graph - Store edges with causal strength, not just similarity
-- 2. Explainable Recall Certificates - Provenance and justification tracking
-- ============================================================================

-- ============================================================================
-- FEATURE 1: Causal Memory Graph
-- ============================================================================

-- Causal edges between memories with intervention effects
CREATE TABLE IF NOT EXISTS causal_edges (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  from_memory_id INTEGER NOT NULL,
  from_memory_type TEXT NOT NULL, -- 'episode', 'skill', 'note', 'fact'
  to_memory_id INTEGER NOT NULL,
  to_memory_type TEXT NOT NULL,

  -- Traditional similarity
  similarity REAL NOT NULL DEFAULT 0.0,

  -- Causal metrics
  uplift REAL, -- E[y|do(x)] - E[y]
  confidence REAL DEFAULT 0.5, -- Confidence in causal claim
  sample_size INTEGER, -- Number of observations

  -- Evidence and provenance
  evidence_ids TEXT, -- JSON array of proof IDs
  experiment_ids TEXT, -- JSON array of A/B test IDs
  confounder_score REAL, -- Likelihood of confounding

  -- Metadata
  mechanism TEXT, -- Hypothesized causal mechanism
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  last_validated_at INTEGER,

  metadata JSON

  -- Note: Foreign keys removed to allow flexible causal edges between any concepts
  -- from_memory_id and to_memory_id can be 0 for abstract causal relationships
);

CREATE INDEX IF NOT EXISTS idx_causal_edges_from ON causal_edges(from_memory_id, from_memory_type);
CREATE INDEX IF NOT EXISTS idx_causal_edges_to ON causal_edges(to_memory_id, to_memory_type);
CREATE INDEX IF NOT EXISTS idx_causal_edges_uplift ON causal_edges(uplift DESC);
CREATE INDEX IF NOT EXISTS idx_causal_edges_confidence ON causal_edges(confidence DESC);

-- Causal experiments (A/B tests for uplift estimation)
CREATE TABLE IF NOT EXISTS causal_experiments (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  hypothesis TEXT NOT NULL,
  treatment_id INTEGER NOT NULL, -- Memory used as treatment
  treatment_type TEXT NOT NULL,
  control_id INTEGER, -- Optional control memory

  -- Experiment design
  start_time INTEGER NOT NULL,
  end_time INTEGER,
  sample_size INTEGER DEFAULT 0,

  -- Results
  treatment_mean REAL,
  control_mean REAL,
  uplift REAL, -- treatment_mean - control_mean
  p_value REAL,
  confidence_interval_low REAL,
  confidence_interval_high REAL,

  -- Status
  status TEXT DEFAULT 'running', -- 'running', 'completed', 'failed'

  metadata JSON,
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_causal_experiments_status ON causal_experiments(status);
CREATE INDEX IF NOT EXISTS idx_causal_experiments_treatment ON causal_experiments(treatment_id, treatment_type);

-- Causal observations (individual data points)
CREATE TABLE IF NOT EXISTS causal_observations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  experiment_id INTEGER NOT NULL,
  episode_id INTEGER NOT NULL,

  -- Treatment assignment
  is_treatment BOOLEAN NOT NULL,

  -- Outcome
  outcome_value REAL NOT NULL,
  outcome_type TEXT, -- 'reward', 'success', 'latency'

  -- Context
  context JSON,
  observed_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

  FOREIGN KEY(experiment_id) REFERENCES causal_experiments(id) ON DELETE CASCADE,
  FOREIGN KEY(episode_id) REFERENCES episodes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_causal_observations_experiment ON causal_observations(experiment_id);
CREATE INDEX IF NOT EXISTS idx_causal_observations_treatment ON causal_observations(is_treatment);

-- ============================================================================
-- FEATURE 2: Explainable Recall Certificates
-- ============================================================================

-- Recall certificates for provenance and justification
CREATE TABLE IF NOT EXISTS recall_certificates (
  id TEXT PRIMARY KEY, -- UUID or hash
  query_id TEXT NOT NULL,
  query_text TEXT NOT NULL,

  -- Retrieved chunks
  chunk_ids TEXT NOT NULL, -- JSON array
  chunk_types TEXT NOT NULL, -- JSON array matching chunk_ids

  -- Minimal hitting set (justification)
  minimal_why TEXT, -- JSON array of chunk IDs that justify the answer
  redundancy_ratio REAL, -- len(chunk_ids) / len(minimal_why)
  completeness_score REAL, -- Fraction of query requirements met

  -- Provenance chain
  merkle_root TEXT NOT NULL,
  source_hashes TEXT, -- JSON array of source hashes
  proof_chain TEXT, -- JSON Merkle proof

  -- Policy compliance
  policy_proof TEXT, -- Proof of policy adherence
  policy_version TEXT,
  access_level TEXT, -- 'public', 'internal', 'confidential', 'restricted'

  -- Metadata
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  latency_ms INTEGER,

  metadata JSON
);

CREATE INDEX IF NOT EXISTS idx_recall_certificates_query ON recall_certificates(query_id);
CREATE INDEX IF NOT EXISTS idx_recall_certificates_created ON recall_certificates(created_at DESC);

-- Provenance sources (for Merkle tree construction)
CREATE TABLE IF NOT EXISTS provenance_sources (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source_type TEXT NOT NULL, -- 'episode', 'skill', 'note', 'fact', 'external'
  source_id INTEGER NOT NULL,

  -- Content hash
  content_hash TEXT NOT NULL UNIQUE,

  -- Lineage
  parent_hash TEXT, -- Previous version
  derived_from TEXT, -- JSON array of parent hashes

  -- Metadata
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  creator TEXT, -- User or system identifier

  metadata JSON
);

CREATE INDEX IF NOT EXISTS idx_provenance_sources_type ON provenance_sources(source_type, source_id);
CREATE INDEX IF NOT EXISTS idx_provenance_sources_hash ON provenance_sources(content_hash);
CREATE INDEX IF NOT EXISTS idx_provenance_sources_parent ON provenance_sources(parent_hash);

-- Justification paths (why a chunk was included)
CREATE TABLE IF NOT EXISTS justification_paths (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  certificate_id TEXT NOT NULL,
  chunk_id TEXT NOT NULL,
  chunk_type TEXT NOT NULL,

  -- Justification
  reason TEXT NOT NULL, -- 'semantic_match', 'causal_link', 'prerequisite', 'constraint'
  necessity_score REAL NOT NULL, -- How essential is this chunk (0-1)

  -- Path to query satisfaction
  path_elements TEXT, -- JSON array describing reasoning path

  FOREIGN KEY(certificate_id) REFERENCES recall_certificates(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_justification_paths_cert ON justification_paths(certificate_id);
CREATE INDEX IF NOT EXISTS idx_justification_paths_chunk ON justification_paths(chunk_id, chunk_type);

-- ============================================================================
-- Views for Causal Analysis
-- ============================================================================

-- High-confidence causal relationships
CREATE VIEW IF NOT EXISTS strong_causal_edges AS
SELECT
  ce.*,
  CASE
    WHEN ce.uplift > 0 THEN 'positive'
    WHEN ce.uplift < 0 THEN 'negative'
    ELSE 'neutral'
  END as effect_direction,
  ce.uplift * ce.confidence as causal_impact
FROM causal_edges ce
WHERE ce.confidence >= 0.7
  AND ce.uplift IS NOT NULL
  AND ABS(ce.uplift) >= 0.1
ORDER BY ABS(ce.uplift) * ce.confidence DESC;

-- Causal chains (multi-hop reasoning)
CREATE VIEW IF NOT EXISTS causal_chains AS
WITH RECURSIVE chain(from_id, to_id, depth, path, total_uplift) AS (
  SELECT from_memory_id, to_memory_id, 1,
         from_memory_id || '->' || to_memory_id,
         uplift
  FROM causal_edges
  WHERE confidence >= 0.6

  UNION ALL

  SELECT chain.from_id, ce.to_memory_id, chain.depth + 1,
         chain.path || '->' || ce.to_memory_id,
         chain.total_uplift + ce.uplift
  FROM chain
  JOIN causal_edges ce ON chain.to_id = ce.from_memory_id
  WHERE chain.depth < 5
    AND ce.confidence >= 0.6
    AND chain.path NOT LIKE '%' || ce.to_memory_id || '%'
)
SELECT * FROM chain
WHERE depth >= 2
ORDER BY total_uplift DESC, depth ASC;

-- ============================================================================
-- Views for Explainability
-- ============================================================================

-- Certificate quality metrics
CREATE VIEW IF NOT EXISTS certificate_quality AS
SELECT
  rc.id,
  rc.query_id,
  rc.completeness_score,
  rc.redundancy_ratio,
  COUNT(jp.id) as justification_count,
  AVG(jp.necessity_score) as avg_necessity,
  rc.latency_ms
FROM recall_certificates rc
LEFT JOIN justification_paths jp ON rc.id = jp.certificate_id
GROUP BY rc.id;

-- Provenance lineage depth
CREATE VIEW IF NOT EXISTS provenance_depth AS
WITH RECURSIVE lineage(hash, depth) AS (
  SELECT content_hash, 0
  FROM provenance_sources
  WHERE parent_hash IS NULL

  UNION ALL

  SELECT ps.content_hash, lineage.depth + 1
  FROM lineage
  JOIN provenance_sources ps ON lineage.hash = ps.parent_hash
)
SELECT
  ps.id,
  ps.source_type,
  ps.source_id,
  ps.content_hash,
  COALESCE(l.depth, 0) as lineage_depth
FROM provenance_sources ps
LEFT JOIN lineage l ON ps.content_hash = l.hash;

-- ============================================================================
-- Triggers for Automatic Maintenance
-- ============================================================================

-- Update causal edge timestamp
CREATE TRIGGER IF NOT EXISTS update_causal_edge_timestamp
AFTER UPDATE ON causal_edges
BEGIN
  UPDATE causal_edges
  SET updated_at = strftime('%s', 'now')
  WHERE id = NEW.id;
END;

-- Validate causal confidence bounds
CREATE TRIGGER IF NOT EXISTS validate_causal_confidence
BEFORE INSERT ON causal_edges
BEGIN
  SELECT CASE
    WHEN NEW.confidence < 0 OR NEW.confidence > 1 THEN
      RAISE(ABORT, 'Confidence must be between 0 and 1')
  END;
END;

-- ============================================================================
-- Functions for Causal Inference (as SQL helpers)
-- ============================================================================

-- These would typically be implemented in TypeScript, but we provide
-- SQL views that can assist with common causal queries

-- Instrumental variables (potential instruments for causal inference)
CREATE VIEW IF NOT EXISTS causal_instruments AS
SELECT
  e1.id as instrument_id,
  e1.task as instrument,
  e2.id as treatment_id,
  e2.task as treatment,
  e3.id as outcome_id,
  e3.task as outcome
FROM episodes e1
CROSS JOIN episodes e2
CROSS JOIN episodes e3
WHERE e1.id != e2.id AND e2.id != e3.id AND e1.id != e3.id
  -- Instrument affects treatment
  AND EXISTS (
    SELECT 1 FROM causal_edges
    WHERE from_memory_id = e1.id AND to_memory_id = e2.id
      AND ABS(uplift) > 0.1
  )
  -- Treatment affects outcome
  AND EXISTS (
    SELECT 1 FROM causal_edges
    WHERE from_memory_id = e2.id AND to_memory_id = e3.id
      AND ABS(uplift) > 0.1
  )
  -- Instrument doesn't directly affect outcome (exclusion restriction)
  AND NOT EXISTS (
    SELECT 1 FROM causal_edges
    WHERE from_memory_id = e1.id AND to_memory_id = e3.id
  );

-- ============================================================================
-- Schema Version
-- ============================================================================
-- Version: 2.0.0 (Frontier Features)
-- Features: Causal Memory Graph, Explainable Recall Certificates
-- Compatible with: AgentDB 1.x
--
-- Performance Optimization:
-- Apply composite index migration (003_composite_indexes.sql) for:
--   - 30-50% faster causal edge queries
--   - Optimized causal chain traversal (multi-hop reasoning)
--   - Faster experiment analysis and A/B testing
--   - See: db/migrations/README.md for details
-- ============================================================================

-- ============================================================================
-- FEATURE 3: Reinforcement Learning Experiences
-- ============================================================================

-- Learning experiences for RL training
CREATE TABLE IF NOT EXISTS learning_experiences (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL,
  state TEXT NOT NULL,
  action TEXT NOT NULL,
  reward REAL NOT NULL,
  next_state TEXT,
  success INTEGER NOT NULL DEFAULT 0,
  timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  metadata JSON
);

CREATE INDEX IF NOT EXISTS idx_learning_experiences_session ON learning_experiences(session_id);
CREATE INDEX IF NOT EXISTS idx_learning_experiences_timestamp ON learning_experiences(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_learning_experiences_reward ON learning_experiences(reward DESC);

-- Learning sessions table for RL session management
CREATE TABLE IF NOT EXISTS learning_sessions (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  session_type TEXT NOT NULL,
  config JSON NOT NULL,
  start_time INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  end_time INTEGER,
  status TEXT NOT NULL DEFAULT 'active',
  metadata JSON
);

CREATE INDEX IF NOT EXISTS idx_learning_sessions_user ON learning_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_learning_sessions_status ON learning_sessions(status);
CREATE INDEX IF NOT EXISTS idx_learning_sessions_start ON learning_sessions(start_time DESC);
