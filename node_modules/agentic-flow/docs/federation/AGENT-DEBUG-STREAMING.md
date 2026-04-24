# Single Agent Debug Streaming

**Version**: 1.0.0
**Date**: 2025-11-01
**Status**: ✅ Production Ready

---

## 🔍 Overview

Track a **single agent's complete lifecycle** with detailed visibility into every operation, decision, and state change.

**Perfect for**: Understanding agent behavior, debugging issues, performance tuning, learning how agents work.

---

## ✨ What You Get

### Agent Lifecycle Tracking
- **Spawning** → **Initializing** → **Ready** → **Working** → **Idle** → **Shutting Down** → **Dead**
- Automatic phase transitions
- Timestamp every change
- Context for each phase

### Task Execution Tracing
- Task start/complete/fail
- Individual step tracking
- Duration measurement
- Result capture
- Error logging

### Decision Logging
- What decisions the agent makes
- Available options
- Selected choice
- Reasoning
- Confidence scores

### Communication Tracking
- Messages sent
- Messages received
- Target agents
- Message types and sizes

### Performance Metrics
- Operation timing
- Step duration
- Task completion time
- Average/min/max calculations
- Automatic aggregation

### Timeline Visualization
- Chronological event log
- Elapsed time markers
- Complete agent history

---

## 🚀 Quick Start

### Basic Usage

```typescript
import { createAgentDebugStream, DebugLevel } from 'agentic-flow/federation/debug/agent-debug-stream';

// Create debug stream for your agent
const agentDebug = createAgentDebugStream({
  agentId: 'my-agent-001',
  tenantId: 'my-team',
  level: DebugLevel.VERBOSE,
  timeline: true,
});

// Track lifecycle
agentDebug.logInitialization({ type: 'researcher' });
agentDebug.logReady();

// Track task
agentDebug.startTask('task-001', 'Research AI safety');
agentDebug.logTaskStep('task-001', 0, 'web_search', { query: '...' });
agentDebug.completeTaskStep('task-001', 0, 200);
agentDebug.completeTask('task-001', { findings: [...] });

// Track decision
agentDebug.logDecision(
  'select_source',
  [{ name: 'arxiv', score: 0.95 }, { name: 'scholar', score: 0.87 }],
  { name: 'arxiv', score: 0.95 },
  'Highest relevance',
  0.95
);

// Track communication
agentDebug.logCommunication('send', 'other-agent', { type: 'data', payload: {...} });

// Print summary
agentDebug.printSummary();
agentDebug.printTimeline();
```

---

## 📊 Sample Output

### Phase Transitions

```
[2025-11-01T14:00:00.000Z] BASIC  AGENT_LIFECYCLE phase_initializing
  Data: {
    "old_phase": "spawning",
    "new_phase": "initializing",
    "config": { "type": "researcher" }
  }

📍 Phase Change: spawning → initializing
```

### Task Execution

```
[2025-11-01T14:00:01.000Z] VERBOS TASK task_start
  Data: {
    "taskId": "research-001",
    "description": "Research AI safety frameworks"
  }

[2025-11-01T14:00:01.050Z] VERBOS TASK_STEP web_search
  Data: {
    "taskId": "research-001",
    "step": 0,
    "query": "AI safety frameworks 2024"
  }

[2025-11-01T14:00:01.250Z] VERBOS TASK_STEP step_complete
  Data: {
    "taskId": "research-001",
    "step": 0,
    "duration": 200,
    "results_found": 15
  }
```

### Decision Logging

```
[2025-11-01T14:00:02.000Z] VERBOS DECISION decision_made
  Data: {
    "context": "select_papers_to_analyze",
    "options_count": 3,
    "selected": { "title": "Paper A", "relevance": 0.95 },
    "reasoning": "Highest relevance score",
    "confidence": 0.95
  }

🤔 Decision Made: select_papers_to_analyze → {"title":"Paper A","relevance":0.95}
```

### Agent Summary

```
============================================================
📊 Agent Summary: research-agent-001
============================================================

Uptime:          1.68s
Tasks Completed: 2
Tasks Failed:    0
Decisions Made:  1
Messages Sent:   1
Messages Recv:   1

Performance Metrics:
------------------------------------------------------------
  step_web_search                1x  avg: 200.0ms  min: 200.0ms  max: 200.0ms
  step_analyze_document          1x  avg: 300.0ms  min: 300.0ms  max: 300.0ms
  memory_store                   1x  avg: 15.0ms  min: 15.0ms  max: 15.0ms
  task_completion                2x  avg: 538.0ms  min: 151.0ms  max: 925.0ms

============================================================
```

### Timeline View

```
============================================================
📅 Agent Timeline
============================================================

  +0.000s | phase_change         | {"from":"spawning","to":"initializing"}
  +0.100s | phase_change         | {"from":"initializing","to":"ready"}
  +0.200s | phase_change         | {"from":"ready","to":"working"}
  +0.250s | task_start           | {"taskId":"research-001"}
  +1.100s | task_complete        | {"taskId":"research-001","duration":925}
  +1.150s | phase_change         | {"from":"working","to":"idle"}

============================================================
```

---

## 🎯 Use Cases

### 1. Debug a Failing Agent

```typescript
const debug = createAgentDebugStream({
  agentId: 'buggy-agent',
  level: DebugLevel.TRACE, // Maximum detail
  trackDecisions: true,
});

// When task fails, you'll see:
// - Exact phase when failure occurred
// - All decisions leading to failure
// - Complete stack of operations
// - Performance metrics showing slow operations
```

### 2. Understand Agent Behavior

```typescript
const debug = createAgentDebugStream({
  agentId: 'learning-agent',
  timeline: true,
});

// After agent runs:
debug.printTimeline(); // See complete sequence of events
```

### 3. Performance Profiling

```typescript
const debug = createAgentDebugStream({
  agentId: 'slow-agent',
  level: DebugLevel.DETAILED,
});

// Track every operation with timing
// Metrics show which operations are slow
debug.printSummary(); // See performance breakdown
```

### 4. Multi-Agent Coordination

```typescript
// Track each agent individually
const agent1Debug = createAgentDebugStream({ agentId: 'agent-1' });
const agent2Debug = createAgentDebugStream({ agentId: 'agent-2' });
const agent3Debug = createAgentDebugStream({ agentId: 'agent-3' });

// See exactly how they interact
agent1Debug.logCommunication('send', 'agent-2', {...});
agent2Debug.logCommunication('receive', 'agent-1', {...});
```

---

## 🔧 Configuration

### AgentDebugConfig Options

```typescript
interface AgentDebugConfig {
  agentId: string;                    // Required: Agent identifier
  tenantId?: string;                   // Optional: Tenant ID
  level?: DebugLevel;                  // Verbosity level (default: VERBOSE)
  format?: 'human' | 'json' | 'compact' | 'timeline';
  output?: 'console' | 'file' | 'both';
  outputFile?: string;                 // File path for output
  colorize?: boolean;                  // Color-coded output (default: true)
  trackState?: boolean;                // Track state changes (default: true)
  trackDecisions?: boolean;            // Log decisions (default: false)
  trackCommunication?: boolean;        // Log messages (default: false)
  timeline?: boolean;                  // Build timeline (default: false)
}
```

### Environment Variables

```bash
DEBUG_LEVEL=VERBOSE                 # Verbosity level
DEBUG_FORMAT=human                  # Output format
DEBUG_OUTPUT=console                # Output destination
DEBUG_TRACK_DECISIONS=true          # Enable decision tracking
DEBUG_TRACK_COMMUNICATION=true      # Enable communication tracking
DEBUG_TIMELINE=true                 # Enable timeline
```

---

## 📚 API Reference

### AgentDebugStream Methods

#### Lifecycle

- **`logAgentPhase(phase, data?)`** - Log phase change
- **`logInitialization(config)`** - Log agent initialization
- **`logReady(capabilities?)`** - Log agent ready
- **`logShutdown(reason?)`** - Log shutdown

#### Tasks

- **`startTask(taskId, description, data?)`** - Start tracking task
- **`logTaskStep(taskId, step, operation, data?)`** - Log task step
- **`completeTaskStep(taskId, step, duration, data?)`** - Complete step
- **`completeTask(taskId, result?)`** - Complete task
- **`failTask(taskId, error)`** - Mark task as failed

#### Decisions & Communication

- **`logDecision(context, options, selected, reasoning?, confidence?)`** - Log decision
- **`logCommunication(type, target, message)`** - Log send/receive
- **`logMemoryOperation(operation, data, duration?)`** - Log memory ops
- **`logThought(thought, context?)`** - Log reasoning

#### Reporting

- **`printSummary()`** - Print agent summary
- **`printTimeline()`** - Print chronological timeline
- **`getState()`** - Get current state
- **`getTasks()`** - Get task history
- **`getDecisions()`** - Get decision history
- **`getCommunications()`** - Get communication history

---

## 🎓 Best Practices

### Development

```bash
# Maximum visibility
DEBUG_LEVEL=TRACE \\
DEBUG_TIMELINE=true \\
DEBUG_TRACK_DECISIONS=true \\
DEBUG_TRACK_COMMUNICATION=true \\
npm start
```

### Debugging

```typescript
// Enable all tracking
const debug = createAgentDebugStream({
  agentId: agent.id,
  level: DebugLevel.TRACE,
  trackDecisions: true,
  trackCommunication: true,
  timeline: true,
});

// Subscribe to events
debug.on('phase_change', (data) => {
  if (data.to === 'failed') {
    console.error('AGENT FAILED!', data);
  }
});
```

### Production

```bash
# Minimal overhead
DEBUG_LEVEL=BASIC \\
DEBUG_FORMAT=json \\
DEBUG_OUTPUT=file \\
DEBUG_OUTPUT_FILE=/var/log/agent.log \\
npm start
```

---

## ✅ Summary

**Single Agent Debug Streaming provides**:

✅ **Complete lifecycle tracking** (spawn → shutdown)
✅ **Task execution tracing** with steps
✅ **Decision logging** with reasoning
✅ **Communication tracking** (send/receive)
✅ **Memory operation logging**
✅ **Performance metrics** (automatic)
✅ **Timeline visualization**
✅ **Automatic summaries**
✅ **Event subscriptions**
✅ **Production-ready**

**Perfect for**:
- Debugging agent behavior
- Understanding decision-making
- Performance profiling
- Learning how agents work
- Multi-agent coordination debugging

---

**Version**: 1.0.0
**Last Updated**: 2025-11-01
**Status**: ✅ Production Ready

🔍 **Track your agents with complete visibility!**
