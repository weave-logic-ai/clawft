# Debug Streaming Guide

**Version**: 1.0.0
**Date**: 2025-11-01
**Status**: ✅ Production Ready

---

## 🔍 Overview

The Debug Streaming system provides **detailed, real-time visibility** into all agent operations with multiple verbosity levels, customizable output formats, and performance metrics.

**Perfect for**: Development, debugging, performance tuning, production monitoring

---

## 🚀 Quick Start

### Basic Usage

```typescript
import { createDebugStream, DebugLevel } from 'agentic-flow/federation/debug/debug-stream';

// Create debug stream
const debug = createDebugStream({
  level: DebugLevel.DETAILED,
  format: 'human',
  colorize: true,
});

// Log operations
debug.logDatabase('query', { table: 'sessions' }, 15);
debug.logMemory('store', 'agent-001', 'team-1', { id: 'mem-1' }, 12);

// Print metrics
debug.printMetrics();
```

### Environment Variables

```bash
# Set debug level
export DEBUG_LEVEL=VERBOSE

# Set output format
export DEBUG_FORMAT=human  # human | json | compact

# Set output destination
export DEBUG_OUTPUT=console  # console | file | both

# Optional: file path
export DEBUG_OUTPUT_FILE=debug.log
```

---

## 📊 Debug Levels

### SILENT (0)
No debug output. Production default.

```bash
DEBUG_LEVEL=SILENT
```

### BASIC (1)
Major events only: initialization, shutdown, errors.

```bash
DEBUG_LEVEL=BASIC
```

**Output**:
```
[2025-11-01T14:00:00.000Z] BASIC  CONNECTION server_start
[2025-11-01T14:00:01.000Z] BASIC  CONNECTION client_connected
```

**Best for**: Production monitoring, error tracking

### DETAILED (2)
Includes all database operations with timing.

```bash
DEBUG_LEVEL=DETAILED
```

**Output**:
```
[2025-11-01T14:00:00.100Z] DETAIL DATABASE query_memories 8.00ms
  Data: {
    "table": "agent_memories",
    "rows": 5
  }
[2025-11-01T14:00:00.120Z] DETAIL MEMORY store agent=agent-001 tenant=team-1 12.00ms
  Data: {
    "id": "mem-456",
    "content": "Task complete"
  }
```

**Best for**: Development, performance tuning, SQL debugging

### VERBOSE (3)
All events including real-time operations and tasks.

```bash
DEBUG_LEVEL=VERBOSE
```

**Output**:
```
[2025-11-01T14:00:00.000Z] VERBOS REALTIME agent=agent-001 agent_join
  Data: {
    "tenant": "team-alpha",
    "status": "online"
  }
[2025-11-01T14:00:00.050Z] VERBOS TASK agent=agent-001 task_assigned
  Data: {
    "taskId": "task-123",
    "description": "Process user request"
  }
```

**Best for**: Multi-agent debugging, coordination troubleshooting

### TRACE (4)
Everything including internal state changes.

```bash
DEBUG_LEVEL=TRACE
```

**Output**:
```
[2025-11-01T14:00:00.000Z] TRACE  TRACE state_change
  Data: {
    "component": "ConnectionPool",
    "old_state": "idle",
    "new_state": "connecting"
  }
```

**Best for**: Deep debugging, troubleshooting edge cases

---

## 🎨 Output Formats

### Human-Readable (Default)

```bash
DEBUG_FORMAT=human
```

**Features**:
- Color-coded output
- Formatted JSON
- Timestamps
- Duration highlighting
- Stack traces (optional)

**Example**:
```
[2025-11-01T14:00:00.100Z] DETAIL DATABASE query_memories 8.00ms
  Data: {
    "table": "agent_memories",
    "rows": 5
  }
```

### JSON

```bash
DEBUG_FORMAT=json
```

**Features**:
- Machine-parseable
- Structured data
- Easy to process
- Log aggregation friendly

**Example**:
```json
{"timestamp":"2025-11-01T14:00:00.100Z","level":2,"category":"database","operation":"query_memories","duration":8,"data":{"table":"agent_memories","rows":5}}
```

### Compact

```bash
DEBUG_FORMAT=compact
```

**Features**:
- Single line per event
- Minimal overhead
- Production friendly
- Easy to grep

**Example**:
```
2025-11-01T14:00:00.100Z | DETAIL | database | query_memories | 8ms
```

---

## 🎯 Usage Examples

### Example 1: Development Debugging

```typescript
import { createDebugStream, DebugLevel } from './debug-stream';

const debug = createDebugStream({
  level: DebugLevel.VERBOSE,
  format: 'human',
  colorize: true,
});

// Your code here
await performDatabaseOperations();

// Print metrics at end
debug.printMetrics();
```

### Example 2: Production Monitoring

```typescript
const debug = createDebugStream({
  level: DebugLevel.BASIC,
  format: 'json',
  output: 'file',
  outputFile: '/var/log/agent-debug.log',
  colorize: false,
});

// Monitor critical operations only
debug.logConnection('service_start', { version: '1.0.0' });
```

### Example 3: Performance Profiling

```typescript
const debug = createDebugStream({
  level: DebugLevel.DETAILED,
  format: 'human',
  includeTimestamps: true,
});

// Log operations with timing
const start = Date.now();
await database.query('SELECT * FROM ...');
const duration = Date.now() - start;

debug.logDatabase('query', { sql: '...' }, duration);

// Analyze performance
debug.printMetrics();
```

### Example 4: Event Filtering

```typescript
const debug = createDebugStream({
  level: DebugLevel.VERBOSE,
  filterCategories: ['database', 'memory'], // Only these
  filterAgents: ['agent-001'], // Only this agent
});

// Only matching events will be logged
```

### Example 5: Real-Time Monitoring

```typescript
const debug = createDebugStream({
  level: DebugLevel.DETAILED,
});

// Subscribe to events
debug.on('event', (event) => {
  if (event.duration && event.duration > 100) {
    console.log(`⚠️  SLOW: ${event.operation} took ${event.duration}ms`);
  }
});
```

---

## 📚 API Reference

### DebugStream Class

#### Methods

**`log(event)`**
Log a debug event manually.

**`logConnection(operation, data?, error?)`**
Log connection events.

**`logDatabase(operation, data?, duration?, error?)`**
Log database operations with timing.

**`logRealtime(operation, agentId?, data?, duration?)`**
Log realtime events.

**`logMemory(operation, agentId?, tenantId?, data?, duration?)`**
Log memory operations.

**`logTask(operation, agentId?, tenantId?, data?, duration?)`**
Log task operations.

**`logTrace(operation, data?)`**
Log internal state changes.

**`getMetrics()`**
Get performance metrics summary.

**`printMetrics()`**
Print formatted metrics to console.

**`getEvents(filter?)`**
Get buffered events with optional filter.

**`clearEvents()`**
Clear event buffer.

**`clearMetrics()`**
Clear performance metrics.

**`close()`**
Close file stream (if using file output).

#### Events

**`'event'`**
Emitted for every debug event.

```typescript
debug.on('event', (event) => {
  console.log('Event:', event);
});
```

---

## 🔧 Integration

### With Supabase Adapter

```typescript
import { SupabaseFederationAdapterDebug } from './supabase-adapter-debug';

const adapter = new SupabaseFederationAdapterDebug({
  url: process.env.SUPABASE_URL!,
  anonKey: process.env.SUPABASE_ANON_KEY!,
  debug: {
    enabled: true,
    level: DebugLevel.DETAILED,
    format: 'human',
  },
});

await adapter.initialize();
await adapter.storeMemory({...});

// Print performance metrics
adapter.printMetrics();
```

### With Real-Time Federation

```typescript
import { createRealtimeHub } from './realtime-federation';
import { createDebugStream, DebugLevel } from './debug-stream';

const debug = createDebugStream({
  level: DebugLevel.VERBOSE,
});

const hub = createRealtimeHub('agent-001', 'team-1');

// Integrate debug logging
hub.on('message:received', (msg) => {
  debug.logRealtime('message_received', 'agent-001', msg);
});
```

---

## 📊 Performance Metrics

### Automatic Tracking

All operations with duration are automatically tracked:

```typescript
debug.logDatabase('query', {}, 15);
debug.logDatabase('query', {}, 8);
debug.logDatabase('insert', {}, 12);

// Metrics are aggregated automatically
debug.printMetrics();
```

**Output**:
```
Performance Metrics Summary
============================================================
Operation                               Count     Avg Duration
------------------------------------------------------------
database:query                          2         11.50ms
database:insert                         1         12.00ms
```

### Custom Metrics

```typescript
const startTime = Date.now();
await yourOperation();
const duration = Date.now() - startTime;

debug.logDatabase('custom_operation', { details }, duration);
```

---

## 🎓 Best Practices

### Development

```bash
# Maximum visibility
DEBUG_LEVEL=TRACE
DEBUG_FORMAT=human
DEBUG_OUTPUT=console
```

### Staging

```bash
# Balanced monitoring
DEBUG_LEVEL=DETAILED
DEBUG_FORMAT=json
DEBUG_OUTPUT=both
DEBUG_OUTPUT_FILE=/var/log/staging-debug.log
```

### Production

```bash
# Minimal overhead
DEBUG_LEVEL=BASIC
DEBUG_FORMAT=compact
DEBUG_OUTPUT=file
DEBUG_OUTPUT_FILE=/var/log/production.log
```

---

## 🐛 Troubleshooting

### Issue: Too Much Output

**Solution**: Lower the debug level

```bash
# From VERBOSE to DETAILED
DEBUG_LEVEL=DETAILED
```

### Issue: Can't Find Slow Queries

**Solution**: Use event filtering with custom handler

```typescript
debug.on('event', (event) => {
  if (event.category === 'database' && event.duration! > 50) {
    console.log('SLOW:', event);
  }
});
```

### Issue: File Output Not Working

**Solution**: Check file permissions

```bash
# Ensure directory exists
mkdir -p /var/log/agent

# Set permissions
chmod 755 /var/log/agent

# Test with tmp
DEBUG_OUTPUT_FILE=/tmp/debug.log
```

---

## 🔗 Related Documentation

- [Supabase Integration](../supabase/SUPABASE-REALTIME-FEDERATION.md)
- [Federation Architecture](./FEDERATED-AGENTDB-EPHEMERAL-AGENTS.md)
- [Performance Tuning](../performance/OPTIMIZATION-GUIDE.md)

---

## ✅ Summary

**Debug Streaming provides**:
- ✅ 5 verbosity levels (SILENT to TRACE)
- ✅ 3 output formats (human, json, compact)
- ✅ Multiple destinations (console, file, both)
- ✅ Automatic performance tracking
- ✅ Event filtering
- ✅ Real-time streaming
- ✅ Color-coded output
- ✅ Stack traces (optional)

**Perfect for**:
- Development debugging
- Performance profiling
- Production monitoring
- Troubleshooting
- Learning and education

---

**Version**: 1.0.0
**Last Updated**: 2025-11-01
**Status**: ✅ Production Ready

🔍 **Start debugging with visibility!**
