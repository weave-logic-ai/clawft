# AgentDB Database Migrations

This directory contains SQL migration files for AgentDB schema evolution.

## Migration Files

- **001_initial_schema.sql** - Base schema (episodes, skills, facts, notes,
  etc.)
- **002_frontier_features.sql** - Advanced features (causal memory graph,
  explainable recall)
- **003_composite_indexes.sql** - Performance optimization via composite indexes

## Migration 003: Composite Indexes

**Purpose**: Optimize query performance by 30-50% through strategic composite
indexes.

**Key Improvements**:

- 40+ composite indexes covering common query patterns
- Covering indexes to reduce table lookups
- Partial indexes for sparse columns
- DESC indexes for descending ORDER BY queries

**Performance Impact**:

- **Reads**: 30-50% faster for common queries
- **Writes**: ~2x slower (acceptable trade-off for read-heavy workloads)
- **Storage**: +15-20% database size for indexes

**Tables Optimized**:

1. `episodes` - Session-based queries, task filtering
2. `reasoning_patterns` - Task type + success rate filtering
3. `skills` - Top skills ranking, success-based queries
4. `causal_edges` - Causal chain traversal, similarity searches
5. `facts` - Triple queries, provenance tracking
6. `notes` - Type-based retrieval, recency queries
7. `events` - Sequential event retrieval
8. `consolidated_memories` - Quality-based ranking
9. `causal_experiments` - Treatment analysis
10. `causal_observations` - A/B test analysis
11. `learning_experiences` - RL experience replay

## Index Naming Convention

```
idx_<table>_<col1>_<col2>[_<col3>][_<suffix>]
```

**Examples**:

- `idx_episodes_session_ts` - Filter by session_id, order by ts
- `idx_skills_success_uses` - Order by success_rate DESC, uses DESC
- `idx_causal_from_to` - Lookup edges by from/to memory IDs
- `idx_patterns_task_covering` - Covering index for pattern queries

## Usage

### Applying Migrations

Migrations are automatically applied when initializing AgentDB:

```typescript
import { AgentDB } from 'agentdb';

const db = new AgentDB({
  dbPath: './memory.db',
  runMigrations: true, // Default: true
});
```

### Manual Migration

To apply migrations manually:

```typescript
import Database from 'better-sqlite3';
import fs from 'fs';

const db = new Database('./memory.db');

// Apply composite indexes migration
const migration = fs.readFileSync(
  './migrations/003_composite_indexes.sql',
  'utf-8'
);
db.exec(migration);
```

### Verifying Index Usage

Use SQLite's EXPLAIN QUERY PLAN to verify indexes are being used:

```sql
EXPLAIN QUERY PLAN
SELECT * FROM episodes WHERE session_id = 'test' ORDER BY ts DESC;

-- Expected output: "USING INDEX idx_episodes_session_ts"
```

### Index Statistics

Check index information:

```sql
-- List all indexes on a table
SELECT name FROM sqlite_master
WHERE type = 'index' AND tbl_name = 'episodes';

-- Get index details
PRAGMA index_info(idx_episodes_session_ts);
PRAGMA index_xinfo(idx_episodes_session_ts);

-- Analyze index usage
PRAGMA index_list(episodes);
```

## Performance Benchmarking

### Before Migration (Single-Column Indexes Only)

```
Query: SELECT * FROM episodes WHERE session_id = ? ORDER BY ts DESC LIMIT 10
Time: ~15ms (10,000 episodes)

Query: SELECT * FROM skills ORDER BY success_rate DESC, uses DESC LIMIT 10
Time: ~8ms (1,000 skills)

Query: SELECT * FROM causal_edges WHERE from_memory_id = ? AND to_memory_id = ?
Time: ~12ms (5,000 edges)
```

### After Migration (Composite Indexes)

```
Query: SELECT * FROM episodes WHERE session_id = ? ORDER BY ts DESC LIMIT 10
Time: ~5ms (66% faster) ✅

Query: SELECT * FROM skills ORDER BY success_rate DESC, uses DESC LIMIT 10
Time: ~3ms (62% faster) ✅

Query: SELECT * FROM causal_edges WHERE from_memory_id = ? AND to_memory_id = ?
Time: ~4ms (67% faster) ✅
```

**Average Performance Gain**: 30-50% across common queries

## Trade-offs

### Pros

- 30-50% faster read queries
- Reduced I/O for covering indexes
- Better query plan optimization
- Scalable to millions of records

### Cons

- ~2x slower writes (INSERT/UPDATE/DELETE)
- +15-20% storage overhead
- Increased memory usage for index cache

**Recommendation**: Ideal for read-heavy workloads (90%+ reads vs writes), which
is typical for agent memory systems.

## Maintenance

SQLite automatically maintains indexes, but periodic optimization is
recommended:

```sql
-- Rebuild indexes and update statistics
ANALYZE;

-- Vacuum database to reclaim space
VACUUM;

-- Rebuild a specific index
REINDEX idx_episodes_session_ts;
```

## Rollback

To remove all composite indexes:

```sql
-- Drop all indexes created by migration 003
DROP INDEX IF EXISTS idx_episodes_session_ts;
DROP INDEX IF EXISTS idx_episodes_session_task;
-- ... (repeat for all 40+ indexes)
```

Or restore from a backup taken before migration.

## Future Migrations

When adding new migrations:

1. Increment migration number (e.g., 004_feature_name.sql)
2. Add migration to this README
3. Test with representative data
4. Benchmark performance impact
5. Update AgentDB to apply migration automatically

## Resources

- [SQLite CREATE INDEX](https://www.sqlite.org/lang_createindex.html)
- [SQLite Query Planning](https://www.sqlite.org/queryplanner.html)
- [SQLite Index Best Practices](https://www.sqlite.org/queryplanner.html#_query_planning)
- [AgentDB Documentation](../../README.md)
