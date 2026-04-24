# Supabase Federation Quick Start

Get up and running with Supabase real-time federation in 5 minutes!

---

## ⚡ 5-Minute Setup

### Step 1: Create Supabase Project (2 minutes)

1. Go to [supabase.com](https://supabase.com)
2. Click **Start your project**
3. Create new project:
   - Name: `agentic-flow-federation`
   - Database Password: (save this!)
   - Region: Choose closest to you
4. Wait for project to provision

### Step 2: Get API Keys (30 seconds)

1. Go to **Project Settings** > **API**
2. Copy these values:
   ```
   Project URL: https://xxxxx.supabase.co
   anon/public key: eyJhbGc...
   service_role key: eyJhbGc... (keep secret!)
   ```

### Step 3: Run Database Migration (1 minute)

1. In Supabase dashboard, go to **SQL Editor**
2. Click **New query**
3. Copy the entire contents of: `docs/supabase/migrations/001_create_federation_tables.sql`
4. Paste and click **Run**
5. You should see: ✅ Federation Hub schema created successfully!

### Step 4: Enable Realtime (30 seconds)

1. Go to **Database** > **Replication**
2. Find and enable these tables:
   - ✅ `agent_sessions`
   - ✅ `agent_memories`
   - ✅ `agent_tasks`
   - ✅ `agent_events`
3. Click **Save**

### Step 5: Configure Environment (1 minute)

Create or update your `.env` file:

```bash
# Supabase credentials
SUPABASE_URL=https://xxxxx.supabase.co
SUPABASE_ANON_KEY=eyJhbGc...
SUPABASE_SERVICE_ROLE_KEY=eyJhbGc...

# Federation settings
FEDERATION_VECTOR_BACKEND=hybrid
FEDERATION_MEMORY_SYNC=true
FEDERATION_HEARTBEAT_INTERVAL=30000
FEDERATION_BROADCAST_LATENCY=low
```

### Step 6: Test It! (30 seconds)

```bash
# Install dependencies if needed
npm install @supabase/supabase-js

# Run the example
npx tsx examples/realtime-federation-example.ts
```

You should see agents joining, communicating, and collaborating in real-time!

---

## 🎯 What You Get

After setup, you have:

✅ **PostgreSQL database** with federation schema
✅ **Real-time subscriptions** for instant updates
✅ **Vector search** using pgvector
✅ **Multi-agent coordination** infrastructure
✅ **Presence tracking** for online agents
✅ **Task orchestration** system
✅ **Tenant isolation** via Row Level Security

---

## 🚀 Next Steps

### Try the Examples

```bash
# Multi-agent research team
npx tsx examples/realtime-federation-example.ts

# Or run specific examples programmatically
```

### Start Federation Hub

```bash
# Start the hub server
npx agentic-flow federation start --db-url $SUPABASE_URL

# In another terminal, spawn an agent
npx agentic-flow federation spawn \
  --agent-id my-agent \
  --tenant-id my-tenant \
  --hub-endpoint $SUPABASE_URL
```

### Build Your First Multi-Agent System

```typescript
import { createRealtimeHub } from 'agentic-flow/federation/integrations/realtime-federation';

// Create agent
const agent = createRealtimeHub('my-agent', 'my-team');
await agent.initialize();

// Listen for messages
agent.on('message:task_assignment', async (msg) => {
  console.log('Got task:', msg.payload.description);
  // Do work...
  await agent.reportTaskComplete(msg.payload.task_id, {
    status: 'success',
  });
});

// Update presence
await agent.updateStatus('online', 'Ready for tasks');

// Get team members
const team = agent.getActiveAgents();
console.log(`${team.length} agents online`);
```

---

## 💡 Common Use Cases

### 1. Research Team

Multiple agents collaborate on research:

```typescript
const researcher = createRealtimeHub('researcher', 'team');
const analyst = createRealtimeHub('analyst', 'team');
const writer = createRealtimeHub('writer', 'team');

// Researcher → Analyst → Writer workflow
```

### 2. Code Review

Agents review code in parallel:

```typescript
const reviewer1 = createRealtimeHub('reviewer-1', 'code-review');
const reviewer2 = createRealtimeHub('reviewer-2', 'code-review');

// Assign files to different reviewers
// Aggregate feedback
```

### 3. Customer Support

Agents handle support tickets:

```typescript
const router = createRealtimeHub('router', 'support');
const specialist1 = createRealtimeHub('billing-expert', 'support');
const specialist2 = createRealtimeHub('tech-expert', 'support');

// Route tickets to specialists based on type
```

### 4. Data Processing Pipeline

Distributed data processing:

```typescript
const coordinator = createRealtimeHub('coordinator', 'pipeline');
const worker1 = createRealtimeHub('worker-1', 'pipeline');
const worker2 = createRealtimeHub('worker-2', 'pipeline');
const worker3 = createRealtimeHub('worker-3', 'pipeline');

// Distribute work, track progress, aggregate results
```

---

## 🔧 Configuration Options

### Vector Backend

Choose storage strategy:

```bash
# AgentDB only (fastest, not persistent)
FEDERATION_VECTOR_BACKEND=agentdb

# Supabase pgvector only (persistent, slower)
FEDERATION_VECTOR_BACKEND=pgvector

# Hybrid (recommended - fast + persistent)
FEDERATION_VECTOR_BACKEND=hybrid
```

### Performance Tuning

```bash
# Lower latency, higher bandwidth
FEDERATION_BROADCAST_LATENCY=low
FEDERATION_HEARTBEAT_INTERVAL=15000  # 15s

# Higher latency, lower bandwidth
FEDERATION_BROADCAST_LATENCY=high
FEDERATION_HEARTBEAT_INTERVAL=60000  # 60s
```

### Memory Management

```bash
# Enable auto-sync to Supabase
FEDERATION_MEMORY_SYNC=true

# Disable for local-only
FEDERATION_MEMORY_SYNC=false
```

---

## 📊 Verify Setup

### Check Database

```sql
-- In Supabase SQL Editor
SELECT * FROM agent_sessions;
SELECT * FROM agent_memories;
SELECT * FROM agent_tasks;

-- Check realtime is enabled
SELECT schemaname, tablename,
       CASE WHEN oid IN (SELECT objid FROM pg_publication_tables WHERE pubname = 'supabase_realtime')
            THEN 'enabled'
            ELSE 'disabled'
       END as realtime_status
FROM pg_tables
WHERE tablename IN ('agent_sessions', 'agent_memories', 'agent_tasks', 'agent_events');
```

### Test Realtime

```typescript
import { createClient } from '@supabase/supabase-js';

const client = createClient(
  process.env.SUPABASE_URL!,
  process.env.SUPABASE_ANON_KEY!
);

// Subscribe to changes
const channel = client
  .channel('test')
  .on(
    'postgres_changes',
    {
      event: 'INSERT',
      schema: 'public',
      table: 'agent_sessions',
    },
    (payload) => {
      console.log('New session:', payload);
    }
  )
  .subscribe();

// Test: Insert a session
await client.from('agent_sessions').insert({
  session_id: 'test-session',
  tenant_id: 'test-tenant',
  agent_id: 'test-agent',
  status: 'active',
});

// You should see "New session:" logged
```

---

## 🐛 Troubleshooting

### "Connection failed"

Check your credentials:

```bash
echo $SUPABASE_URL
echo $SUPABASE_ANON_KEY
```

Make sure they're set correctly.

### "Table does not exist"

Run the migration again:

```bash
# Copy docs/supabase/migrations/001_create_federation_tables.sql
# Paste into Supabase SQL Editor
# Run it
```

### "Realtime not working"

Enable realtime in Supabase dashboard:

1. Database > Replication
2. Enable for all federation tables
3. Save

### "Permission denied"

Check Row Level Security:

```sql
-- Disable RLS for testing (not for production!)
ALTER TABLE agent_sessions DISABLE ROW LEVEL SECURITY;
ALTER TABLE agent_memories DISABLE ROW LEVEL SECURITY;
```

Or set tenant context:

```sql
SET app.current_tenant = 'your-tenant-id';
```

---

## 📚 Learn More

- [Full Documentation](./SUPABASE-REALTIME-FEDERATION.md)
- [Federation Architecture](../architecture/FEDERATED-AGENTDB-EPHEMERAL-AGENTS.md)
- [API Reference](./SUPABASE-REALTIME-FEDERATION.md#-api-reference)
- [Examples](../../examples/realtime-federation-example.ts)

---

## 🆘 Getting Help

- GitHub Issues: [github.com/ruvnet/agentic-flow/issues](https://github.com/ruvnet/agentic-flow/issues)
- Supabase Docs: [supabase.com/docs](https://supabase.com/docs)
- pgvector Guide: [github.com/pgvector/pgvector](https://github.com/pgvector/pgvector)

---

**Ready to build?** Start with the examples and customize for your use case!

🚀 Happy building!
