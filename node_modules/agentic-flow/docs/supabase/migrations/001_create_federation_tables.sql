-- Federation Hub Schema for Supabase
-- Version: 1.0.0
-- Date: 2025-10-31

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "vector";

-- =====================================================
-- Agent Sessions Table
-- =====================================================
CREATE TABLE IF NOT EXISTS agent_sessions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  session_id TEXT UNIQUE NOT NULL,
  tenant_id TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('active', 'completed', 'failed')),
  started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  ended_at TIMESTAMPTZ,
  metadata JSONB DEFAULT '{}',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for agent_sessions
CREATE INDEX idx_agent_sessions_tenant ON agent_sessions(tenant_id);
CREATE INDEX idx_agent_sessions_agent ON agent_sessions(agent_id);
CREATE INDEX idx_agent_sessions_status ON agent_sessions(status);
CREATE INDEX idx_agent_sessions_started ON agent_sessions(started_at DESC);

-- =====================================================
-- Agent Memories Table
-- =====================================================
CREATE TABLE IF NOT EXISTS agent_memories (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  tenant_id TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  content TEXT NOT NULL,
  embedding vector(1536), -- OpenAI ada-002 dimension
  metadata JSONB DEFAULT '{}',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ,

  -- Foreign key to session
  CONSTRAINT fk_session FOREIGN KEY (session_id)
    REFERENCES agent_sessions(session_id)
    ON DELETE CASCADE
);

-- Indexes for agent_memories
CREATE INDEX idx_agent_memories_tenant ON agent_memories(tenant_id);
CREATE INDEX idx_agent_memories_agent ON agent_memories(agent_id);
CREATE INDEX idx_agent_memories_session ON agent_memories(session_id);
CREATE INDEX idx_agent_memories_created ON agent_memories(created_at DESC);
CREATE INDEX idx_agent_memories_expires ON agent_memories(expires_at)
  WHERE expires_at IS NOT NULL;

-- HNSW vector index for semantic search
CREATE INDEX idx_agent_memories_embedding ON agent_memories
  USING hnsw (embedding vector_cosine_ops)
  WITH (m = 16, ef_construction = 64);

-- =====================================================
-- Agent Tasks Table
-- =====================================================
CREATE TABLE IF NOT EXISTS agent_tasks (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  task_id TEXT UNIQUE NOT NULL,
  tenant_id TEXT NOT NULL,
  assigned_to TEXT NOT NULL,
  assigned_by TEXT,
  description TEXT NOT NULL,
  priority TEXT NOT NULL CHECK (priority IN ('low', 'medium', 'high', 'critical')),
  status TEXT NOT NULL CHECK (status IN ('pending', 'assigned', 'in_progress', 'completed', 'failed')),
  result JSONB,
  dependencies TEXT[] DEFAULT '{}',
  deadline TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  metadata JSONB DEFAULT '{}'
);

-- Indexes for agent_tasks
CREATE INDEX idx_agent_tasks_tenant ON agent_tasks(tenant_id);
CREATE INDEX idx_agent_tasks_assigned_to ON agent_tasks(assigned_to);
CREATE INDEX idx_agent_tasks_status ON agent_tasks(status);
CREATE INDEX idx_agent_tasks_priority ON agent_tasks(priority);
CREATE INDEX idx_agent_tasks_created ON agent_tasks(created_at DESC);

-- =====================================================
-- Agent Events Table (Audit Log)
-- =====================================================
CREATE TABLE IF NOT EXISTS agent_events (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  tenant_id TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  payload JSONB NOT NULL DEFAULT '{}',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for agent_events
CREATE INDEX idx_agent_events_tenant ON agent_events(tenant_id);
CREATE INDEX idx_agent_events_agent ON agent_events(agent_id);
CREATE INDEX idx_agent_events_type ON agent_events(event_type);
CREATE INDEX idx_agent_events_created ON agent_events(created_at DESC);

-- Partition by month for efficient archival
-- CREATE TABLE agent_events_2025_11 PARTITION OF agent_events
--   FOR VALUES FROM ('2025-11-01') TO ('2025-12-01');

-- =====================================================
-- Row Level Security (RLS)
-- =====================================================

-- Enable RLS on all tables
ALTER TABLE agent_sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE agent_memories ENABLE ROW LEVEL SECURITY;
ALTER TABLE agent_tasks ENABLE ROW LEVEL SECURITY;
ALTER TABLE agent_events ENABLE ROW LEVEL SECURITY;

-- Policies for agent_sessions
CREATE POLICY tenant_isolation_sessions ON agent_sessions
  FOR ALL
  USING (tenant_id = current_setting('app.current_tenant', TRUE));

-- Policies for agent_memories
CREATE POLICY tenant_isolation_memories ON agent_memories
  FOR ALL
  USING (tenant_id = current_setting('app.current_tenant', TRUE));

-- Policies for agent_tasks
CREATE POLICY tenant_isolation_tasks ON agent_tasks
  FOR ALL
  USING (tenant_id = current_setting('app.current_tenant', TRUE));

-- Policies for agent_events
CREATE POLICY tenant_isolation_events ON agent_events
  FOR ALL
  USING (tenant_id = current_setting('app.current_tenant', TRUE));

-- Service role bypass (for server-side operations)
CREATE POLICY service_role_bypass_sessions ON agent_sessions
  FOR ALL
  TO service_role
  USING (TRUE);

CREATE POLICY service_role_bypass_memories ON agent_memories
  FOR ALL
  TO service_role
  USING (TRUE);

CREATE POLICY service_role_bypass_tasks ON agent_tasks
  FOR ALL
  TO service_role
  USING (TRUE);

CREATE POLICY service_role_bypass_events ON agent_events
  FOR ALL
  TO service_role
  USING (TRUE);

-- =====================================================
-- Functions
-- =====================================================

-- Semantic search function using pgvector
CREATE OR REPLACE FUNCTION search_memories(
  query_embedding vector(1536),
  query_tenant_id TEXT,
  match_count INT DEFAULT 10,
  similarity_threshold FLOAT DEFAULT 0.7
)
RETURNS TABLE (
  id UUID,
  tenant_id TEXT,
  agent_id TEXT,
  session_id TEXT,
  content TEXT,
  metadata JSONB,
  created_at TIMESTAMPTZ,
  similarity FLOAT
)
LANGUAGE plpgsql
AS $$
BEGIN
  RETURN QUERY
  SELECT
    m.id,
    m.tenant_id,
    m.agent_id,
    m.session_id,
    m.content,
    m.metadata,
    m.created_at,
    1 - (m.embedding <=> query_embedding) AS similarity
  FROM agent_memories m
  WHERE m.tenant_id = query_tenant_id
    AND m.embedding IS NOT NULL
    AND 1 - (m.embedding <=> query_embedding) > similarity_threshold
  ORDER BY m.embedding <=> query_embedding
  LIMIT match_count;
END;
$$;

-- Update updated_at timestamp automatically
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_agent_sessions_updated_at
  BEFORE UPDATE ON agent_sessions
  FOR EACH ROW
  EXECUTE FUNCTION update_updated_at();

-- Auto-delete expired memories
CREATE OR REPLACE FUNCTION delete_expired_memories()
RETURNS void AS $$
BEGIN
  DELETE FROM agent_memories
  WHERE expires_at IS NOT NULL
    AND expires_at < NOW();
END;
$$ LANGUAGE plpgsql;

-- Schedule periodic cleanup (requires pg_cron extension)
-- SELECT cron.schedule('cleanup-expired-memories', '0 * * * *', 'SELECT delete_expired_memories()');

-- =====================================================
-- Views
-- =====================================================

-- Active sessions view
CREATE OR REPLACE VIEW active_sessions AS
SELECT
  s.tenant_id,
  s.agent_id,
  s.session_id,
  s.started_at,
  s.metadata,
  COUNT(m.id) AS memory_count,
  MAX(m.created_at) AS last_memory_at
FROM agent_sessions s
LEFT JOIN agent_memories m ON s.session_id = m.session_id
WHERE s.status = 'active'
GROUP BY s.tenant_id, s.agent_id, s.session_id, s.started_at, s.metadata;

-- Hub statistics view
CREATE OR REPLACE VIEW hub_statistics AS
SELECT
  tenant_id,
  COUNT(DISTINCT agent_id) AS total_agents,
  COUNT(DISTINCT session_id) AS total_sessions,
  COUNT(*) AS total_memories,
  MIN(created_at) AS first_memory_at,
  MAX(created_at) AS last_memory_at
FROM agent_memories
GROUP BY tenant_id;

-- Task status view
CREATE OR REPLACE VIEW task_status_summary AS
SELECT
  tenant_id,
  status,
  priority,
  COUNT(*) AS task_count,
  AVG(EXTRACT(EPOCH FROM (completed_at - started_at))) AS avg_duration_seconds
FROM agent_tasks
GROUP BY tenant_id, status, priority;

-- =====================================================
-- Sample Data (Optional - for testing)
-- =====================================================

-- Uncomment to insert sample data:
/*
INSERT INTO agent_sessions (session_id, tenant_id, agent_id, status, metadata)
VALUES
  ('session-001', 'demo-tenant', 'agent-001', 'active', '{"type": "researcher"}'),
  ('session-002', 'demo-tenant', 'agent-002', 'active', '{"type": "analyst"}');

INSERT INTO agent_memories (tenant_id, agent_id, session_id, content, metadata)
VALUES
  ('demo-tenant', 'agent-001', 'session-001', 'Research finding: AI safety is critical', '{"topic": "AI safety"}'),
  ('demo-tenant', 'agent-002', 'session-002', 'Analysis: High confidence in results', '{"confidence": 0.95}');

INSERT INTO agent_tasks (task_id, tenant_id, assigned_to, description, priority, status)
VALUES
  ('task-001', 'demo-tenant', 'agent-001', 'Research AI safety frameworks', 'high', 'in_progress'),
  ('task-002', 'demo-tenant', 'agent-002', 'Analyze research findings', 'medium', 'pending');
*/

-- =====================================================
-- Grants
-- =====================================================

-- Grant necessary permissions
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO authenticated;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO service_role;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO authenticated;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO service_role;

-- =====================================================
-- Comments
-- =====================================================

COMMENT ON TABLE agent_sessions IS 'Tracks active and historical agent sessions';
COMMENT ON TABLE agent_memories IS 'Stores agent memories with vector embeddings for semantic search';
COMMENT ON TABLE agent_tasks IS 'Manages task assignments and coordination between agents';
COMMENT ON TABLE agent_events IS 'Audit log for all agent events and actions';

COMMENT ON FUNCTION search_memories IS 'Performs semantic search using pgvector cosine similarity';
COMMENT ON FUNCTION delete_expired_memories IS 'Removes memories that have passed their expiration time';

-- =====================================================
-- Completion Message
-- =====================================================

DO $$
BEGIN
  RAISE NOTICE '✅ Federation Hub schema created successfully!';
  RAISE NOTICE '';
  RAISE NOTICE 'Next steps:';
  RAISE NOTICE '1. Enable Realtime for tables in Supabase dashboard:';
  RAISE NOTICE '   - agent_sessions';
  RAISE NOTICE '   - agent_memories';
  RAISE NOTICE '   - agent_tasks';
  RAISE NOTICE '   - agent_events';
  RAISE NOTICE '';
  RAISE NOTICE '2. Configure RLS policies for your application';
  RAISE NOTICE '3. Set up pg_cron for automatic memory cleanup (optional)';
  RAISE NOTICE '4. Update your .env file with Supabase credentials';
END $$;
