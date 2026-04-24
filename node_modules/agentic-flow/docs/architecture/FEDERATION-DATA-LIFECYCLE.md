# Federation Data Lifecycle: Persistent vs Ephemeral Storage

## Architecture Overview

The federation system uses a **hub-and-spoke** model with **persistent central storage** and **ephemeral agent storage**.

```
┌─────────────────────────────────────────────────────────┐
│              FEDERATION HUB (PERSISTENT)                │
│                                                          │
│  ┌────────────────────┐    ┌─────────────────────────┐ │
│  │   SQLite DB        │    │     AgentDB             │ │
│  │   (Metadata)       │    │  (Vector Memory)        │ │
│  │                    │    │                         │ │
│  │ • Episode metadata │    │ • Vector embeddings     │ │
│  │ • Agent registry   │    │ • HNSW index           │ │
│  │ • Change log       │    │ • Semantic search       │ │
│  │ • Tenant isolation │    │ • Pattern storage       │ │
│  └────────────────────┘    └─────────────────────────┘ │
│                                                          │
│  Storage: /data/hub.db and /data/hub-agentdb.db        │
│  Lifetime: PERMANENT (until manually deleted)           │
└─────────────────────────────────────────────────────────┘
                          ↑
                          │ WebSocket Sync
        ┌─────────────────┼─────────────────┐
        │                 │                 │
        ↓                 ↓                 ↓
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│   Agent 1    │  │   Agent 2    │  │   Agent 3    │
│ (Ephemeral)  │  │ (Ephemeral)  │  │ (Ephemeral)  │
│              │  │              │  │              │
│ Local AgentDB│  │ Local AgentDB│  │ Local AgentDB│
│ Storage:     │  │ Storage:     │  │ Storage:     │
│ :memory:     │  │ :memory:     │  │ :memory:     │
│              │  │              │  │              │
│ Lifetime:    │  │ Lifetime:    │  │ Lifetime:    │
│ 5s - 15min   │  │ 5s - 15min   │  │ 5s - 15min   │
└──────────────┘  └──────────────┘  └──────────────┘
      ↓                 ↓                 ↓
   DESTROYED         DESTROYED         DESTROYED
  (RAM freed)       (RAM freed)       (RAM freed)
```

---

## Data Flow: Complete Lifecycle

### Phase 1: Agent Spawns

```typescript
// Agent spawns with ephemeral memory
const agent = await EphemeralAgent.spawn({
  tenantId: 'acme-corp',
  lifetime: 300, // 5 minutes
  hubEndpoint: 'ws://hub:8443'
});

// Local AgentDB created in memory
agent.agentDB = new AgentDB({ path: ':memory:' });

// Connect to hub
await agent.connect();
```

**Storage State:**
- Hub: Empty (or has old memories from previous agents)
- Agent: Empty `:memory:` database

### Phase 2: Agent Pulls Memories from Hub

```typescript
await agent.execute(async (db) => {
  // PULL: Agent requests memories from hub
  const memories = await agent.queryMemories('task-name', 10);

  // Hub sends back all relevant memories from past agents
  // Agent stores them locally for fast semantic search
});
```

**Storage State:**
- Hub: Contains 1000 memories from previous agents (PERSISTENT)
- Agent: Downloaded 10 relevant memories to local `:memory:` (TEMPORARY)

**Data Transfer:**
```
Hub (Disk: 1000 episodes)
  → WebSocket
    → Agent (RAM: 10 relevant episodes)
```

### Phase 3: Agent Works and Learns

```typescript
await agent.execute(async (db) => {
  // Agent uses memories to inform decisions
  const context = await agent.queryMemories('similar-task');

  // Agent performs work
  const result = await processTask(context);

  // Agent stores NEW learning locally
  await agent.storeEpisode({
    task: 'task-name',
    input: 'data',
    output: result,
    reward: 0.95
  });
});
```

**Storage State:**
- Hub: Still 1000 memories (not updated yet)
- Agent: 10 old + 1 new = 11 memories in RAM

### Phase 4: Agent Syncs to Hub (PUSH)

```typescript
// Agent pushes new memories to hub
await agent.syncWithHub();

// Hub receives and stores permanently
hub.agentDB.storePattern({
  sessionId: 'acme-corp/agent-001',
  task: 'task-name',
  ...episode
});
```

**Storage State:**
- Hub: 1001 memories (NEW memory added to disk) ✅
- Agent: 11 memories in RAM

**Data Transfer:**
```
Agent (RAM: 1 new episode)
  → WebSocket
    → Hub (Disk: saves permanently)
```

### Phase 5: Agent Expires and Destroys

```typescript
// After 5 minutes (or manual cleanup)
await agent.destroy();

// Local memory is freed
agent.agentDB.close(); // :memory: database destroyed
```

**Storage State:**
- Hub: 1001 memories (PERSISTS on disk) ✅
- Agent: RAM freed, all local data GONE ❌

### Phase 6: New Agent Spawns (Hours/Days Later)

```typescript
// New agent spawns in the future
const newAgent = await EphemeralAgent.spawn({
  tenantId: 'acme-corp', // Same tenant
  lifetime: 300,
  hubEndpoint: 'ws://hub:8443'
});

await newAgent.execute(async (db) => {
  // NEW agent can access OLD memories!
  const memories = await newAgent.queryMemories('task-name', 10);

  // Returns memories from previous agents that died hours ago
  console.log(memories.length); // 10 memories (including episode from Agent 1)
});
```

**Storage State:**
- Hub: 1001 memories (still on disk from previous agents) ✅
- New Agent: Downloads 10 relevant memories from hub (including work from Agent 1)

**Key Insight**: Memory outlives the agents! 🎉

---

## Storage Locations

### Hub Storage (PERSISTENT)

**SQLite Database: `/data/hub.db`**
```sql
CREATE TABLE episodes (
  id INTEGER PRIMARY KEY,
  tenant_id TEXT NOT NULL,    -- Tenant isolation
  agent_id TEXT NOT NULL,      -- Which agent created this
  session_id TEXT NOT NULL,    -- Agent session
  task TEXT NOT NULL,          -- Task description
  input TEXT NOT NULL,         -- Task input
  output TEXT NOT NULL,        -- Task output
  reward REAL NOT NULL,        -- Success metric
  created_at INTEGER NOT NULL  -- Timestamp
);
```

**AgentDB Database: `/data/hub-agentdb.db`**
```typescript
{
  sessionId: 'acme-corp/agent-001',  // Tenant prefix for isolation
  task: 'implement-feature',
  embedding: [...384 dimensions...], // Vector for semantic search
  reward: 0.95,
  metadata: {
    tenantId: 'acme-corp',
    agentId: 'agent-001',
    timestamp: 1234567890
  }
}
```

**Lifetime**: PERMANENT until:
- Manually deleted
- Retention policy applied (e.g., delete after 90 days)
- Tenant requests data deletion (GDPR)

### Agent Storage (EPHEMERAL)

**Location**: `:memory:` (RAM only)

**Lifetime**: 5 seconds to 15 minutes

**Contents**:
- Downloaded memories from hub
- Local work in progress
- Temporary caches

**Destroyed when**:
- Agent reaches `lifetime` expiration
- Manual `agent.destroy()` call
- Process crash/restart
- Container shutdown

---

## Memory Persistence Guarantees

### ✅ What PERSISTS (Survives Agent Death)

1. **All Episodes**: Every `storeEpisode()` call that syncs to hub
2. **Vector Embeddings**: Semantic search index in hub AgentDB
3. **Metadata**: Agent ID, tenant ID, timestamps, rewards
4. **Tenant Isolation**: Sessions tagged with tenant prefix
5. **Change Log**: History of all modifications

### ❌ What is LOST (Agent Death)

1. **Local Cache**: Downloaded memories in agent's `:memory:` DB
2. **In-Progress Work**: Anything not yet synced to hub
3. **Temporary State**: Agent-specific runtime data
4. **Unsaved Episodes**: Episodes created but not synced

---

## Sync Timing: When Does Data Persist?

### Automatic Sync Points

```typescript
class EphemeralAgent {
  async execute(task) {
    // 1. PRE-SYNC: Pull latest from hub
    await this.syncWithHub(); // Download new memories

    // 2. WORK: Agent performs task
    const result = await task(this.db);

    // 3. POST-SYNC: Push new memories to hub
    await this.syncWithHub(); // Upload new episodes ✅

    return result;
  }

  async destroy() {
    // 4. FINAL SYNC: Ensure everything is saved
    await this.syncWithHub(); // Last chance to save ✅

    // 5. Local cleanup
    await this.agentDB.close(); // Memory freed
  }
}
```

### Manual Sync

```typescript
// Developer can force sync anytime
await agent.syncWithHub(); // Pushes all local episodes to hub
```

**Guarantee**: Any episode stored before `syncWithHub()` is PERMANENT.

---

## Example: Multi-Generation Learning

### Day 1: First Agent

```typescript
// 10:00 AM - Agent 1 spawns
const agent1 = await EphemeralAgent.spawn({
  tenantId: 'research-team',
  lifetime: 300
});

await agent1.execute(async () => {
  await agent1.storeEpisode({
    task: 'analyze-data',
    input: 'dataset-v1',
    output: 'Found pattern X',
    reward: 0.92
  });
});

// 10:05 AM - Agent 1 destroyed
await agent1.destroy(); // Episode saved to hub ✅
```

**Hub Storage**: 1 episode

### Day 2: Second Agent

```typescript
// 9:00 AM (next day) - Agent 2 spawns
const agent2 = await EphemeralAgent.spawn({
  tenantId: 'research-team', // Same tenant
  lifetime: 300
});

await agent2.execute(async () => {
  // Query memories (finds Agent 1's work from yesterday!)
  const memories = await agent2.queryMemories('analyze-data');

  console.log(memories[0].output); // "Found pattern X" ✅
  console.log(memories[0].agentId); // "agent-001" (from yesterday)

  // Build on previous work
  await agent2.storeEpisode({
    task: 'refine-pattern',
    input: 'pattern-x',
    output: 'Confirmed pattern X, found pattern Y',
    reward: 0.96
  });
});

await agent2.destroy();
```

**Hub Storage**: 2 episodes (Agent 1 + Agent 2)

### Day 30: Tenth Agent

```typescript
// 30 days later - Agent 10 spawns
const agent10 = await EphemeralAgent.spawn({
  tenantId: 'research-team',
  lifetime: 300
});

await agent10.execute(async () => {
  // Query all past work
  const memories = await agent10.queryMemories('pattern', 100);

  console.log(memories.length); // 50+ episodes from 9 previous agents ✅

  // Agent 10 learns from all past agents' successes
  const bestPatterns = memories
    .filter(m => m.reward > 0.90)
    .map(m => m.output);

  // Standing on the shoulders of giants 🚀
});
```

**Hub Storage**: 50+ episodes (cumulative learning)

---

## Retention Policies

### Default: Infinite Retention

Hub stores everything forever unless configured otherwise.

### Optional: Time-Based Retention

```typescript
// Delete episodes older than 90 days
hub.db.prepare(`
  DELETE FROM episodes
  WHERE created_at < ?
`).run(Date.now() - (90 * 24 * 60 * 60 * 1000));

// Delete from AgentDB too
await hub.agentDB.deleteOldPatterns({ maxAge: 90 * 24 * 60 * 60 });
```

### Optional: Reward-Based Retention

```typescript
// Keep only high-reward episodes
hub.db.prepare(`
  DELETE FROM episodes
  WHERE reward < 0.70
`).run();
```

---

## Disaster Recovery

### Hub Backup

```bash
# Backup hub databases
cp /data/hub.db /backup/hub-2025-10-31.db
cp /data/hub-agentdb.db /backup/hub-agentdb-2025-10-31.db
```

### Hub Restore

```bash
# Restore from backup
cp /backup/hub-2025-10-31.db /data/hub.db
cp /backup/hub-agentdb-2025-10-31.db /data/hub-agentdb.db

# Restart hub
docker restart federation-hub
```

### Agent Recovery

**Agents don't need backup** - They're ephemeral by design!

If an agent crashes, just spawn a new one:

```typescript
// Old agent crashed (no problem!)
// agent1 died unexpectedly ❌

// Spawn replacement
const agent2 = await EphemeralAgent.spawn({
  tenantId: 'acme-corp', // Same tenant
  lifetime: 300
});

// New agent has access to ALL old memories ✅
await agent2.execute(async () => {
  const memories = await agent2.queryMemories('task');
  // Gets memories from crashed agent + all previous agents
});
```

---

## Production Deployment

### Single Hub (Simple)

```yaml
# docker-compose.yml
services:
  federation-hub:
    image: federation-hub:latest
    volumes:
      - hub-data:/data  # PERSISTENT volume
    ports:
      - "8443:8443"

volumes:
  hub-data:
    driver: local  # Data survives container restarts
```

**Persistence**: Volume survives container restarts ✅

### Multi-Hub (High Availability)

```
┌──────────┐     ┌──────────┐     ┌──────────┐
│  Hub US  │────▶│ Hub EU   │────▶│ Hub AP   │
│ (Primary)│     │(Replica) │     │(Replica) │
└──────────┘     └──────────┘     └──────────┘
     │                │                │
     └────────────────┴────────────────┘
              Sync every 5 seconds
```

**Guarantee**: Data replicated across regions ✅

---

## Key Takeaways

1. **Hub = Permanent**: All memories stored on disk forever (until manually deleted)

2. **Agents = Temporary**: Local databases destroyed after 5-15 minutes

3. **Memory Outlives Agents**: New agents can access memories from agents that died hours/days/weeks ago

4. **Sync = Persist**: Any episode that syncs to hub is PERMANENT

5. **Tenant Isolation**: Memories are isolated by tenant, but persist across all agents in that tenant

6. **No Data Loss**: As long as hub is backed up, no memories are lost when agents die

7. **Infinite Generations**: Agents can learn from an unlimited chain of previous agents

8. **Docker Volumes**: Hub data persists across container restarts if using volumes

---

**Bottom Line**: The federation hub is the "source of truth" - it's a **persistent, centralized database** that outlives all ephemeral agents. Agents come and go, but the hub remembers everything. 🧠

This enables **continuous learning** where each new generation of agents builds on the collective knowledge of all previous generations, while maintaining the efficiency benefits of ephemeral, short-lived agents.
