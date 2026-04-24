# QUIC Swarm Coordination - Quick Start Guide

This guide will help you get started with QUIC-enabled multi-agent swarm coordination in agentic-flow.

## Table of Contents

1. [Installation](#installation)
2. [Basic Usage](#basic-usage)
3. [Topology Selection](#topology-selection)
4. [Transport Configuration](#transport-configuration)
5. [Agent Registration](#agent-registration)
6. [Statistics & Monitoring](#statistics--monitoring)
7. [Common Patterns](#common-patterns)
8. [Troubleshooting](#troubleshooting)

## Installation

```bash
npm install agentic-flow
```

## Basic Usage

### Initialize a Simple Swarm

```typescript
import { initSwarm } from 'agentic-flow';

const swarm = await initSwarm({
  swarmId: 'my-swarm',
  topology: 'mesh',
  transport: 'quic',
  quicPort: 4433
});
```

### Register Agents

```typescript
await swarm.registerAgent({
  id: 'agent-1',
  role: 'worker',
  host: 'localhost',
  port: 4434,
  capabilities: ['compute', 'analyze']
});
```

### Get Statistics

```typescript
const stats = await swarm.getStats();
console.log('Swarm stats:', stats);
```

### Cleanup

```typescript
await swarm.shutdown();
```

## Topology Selection

### Mesh Topology (Peer-to-Peer)

Best for: Small swarms (<20 agents), maximum redundancy

```typescript
const swarm = await initSwarm({
  swarmId: 'mesh-swarm',
  topology: 'mesh',
  transport: 'quic',
  maxAgents: 10
});
```

**Characteristics**:
- All agents connect to all others
- O(n²) connections
- No single point of failure
- Ideal for distributed consensus

### Hierarchical Topology (Coordinator-Worker)

Best for: Large swarms (100+ agents), centralized task distribution

```typescript
const swarm = await initSwarm({
  swarmId: 'hierarchical-swarm',
  topology: 'hierarchical',
  transport: 'quic',
  maxAgents: 100
});

// Register coordinators
await swarm.registerAgent({
  id: 'coordinator-1',
  role: 'coordinator',
  host: 'localhost',
  port: 4433,
  capabilities: ['orchestrate']
});

// Register workers
await swarm.registerAgent({
  id: 'worker-1',
  role: 'worker',
  host: 'localhost',
  port: 4434,
  capabilities: ['compute']
});
```

**Characteristics**:
- Workers communicate through coordinators
- O(n) connections
- Scalable to 100+ agents
- Ideal for task distribution

### Ring Topology (Circular)

Best for: Ordered processing, pipeline architectures

```typescript
const swarm = await initSwarm({
  swarmId: 'ring-swarm',
  topology: 'ring',
  transport: 'quic',
  maxAgents: 8
});
```

**Characteristics**:
- Each agent connects to next in circle
- O(n) connections
- Predictable message flow
- Ideal for pipelines

### Star Topology (Hub and Spoke)

Best for: Simple coordination, fan-out/fan-in patterns

```typescript
const swarm = await initSwarm({
  swarmId: 'star-swarm',
  topology: 'star',
  transport: 'quic',
  maxAgents: 20
});

// Register central coordinator
await swarm.registerAgent({
  id: 'central',
  role: 'coordinator',
  host: 'localhost',
  port: 4433,
  capabilities: ['coordinate']
});

// Register spoke agents
await swarm.registerAgent({
  id: 'spoke-1',
  role: 'worker',
  host: 'localhost',
  port: 4434,
  capabilities: ['compute']
});
```

**Characteristics**:
- All messages through central coordinator
- O(n) connections
- Single coordination point
- Simple to manage

## Transport Configuration

### QUIC Transport (Recommended)

Fastest option with 0-RTT connection establishment:

```typescript
const swarm = await initSwarm({
  swarmId: 'quic-swarm',
  topology: 'mesh',
  transport: 'quic',
  quicPort: 4433,
  quicHost: 'localhost'
});
```

**Features**:
- 0-RTT connection establishment
- 100+ concurrent streams per connection
- No head-of-line blocking
- Connection migration support

### HTTP/2 Transport (Fallback)

Compatible fallback option:

```typescript
const swarm = await initSwarm({
  swarmId: 'http2-swarm',
  topology: 'mesh',
  transport: 'http2',
  quicPort: 8443  // HTTP/2 port
});
```

**Features**:
- Universal compatibility
- Proven reliability
- Standard TLS security

### Auto Transport (Default)

Automatically select best available transport:

```typescript
const swarm = await initSwarm({
  swarmId: 'auto-swarm',
  topology: 'mesh',
  transport: 'auto',  // Try QUIC, fallback to HTTP/2
  enableFallback: true
});
```

**Features**:
- Automatic protocol selection
- Transparent fallback
- Continuous health monitoring
- Best performance available

## Agent Registration

### Basic Agent

```typescript
await swarm.registerAgent({
  id: 'agent-1',
  role: 'worker',
  host: 'localhost',
  port: 4434,
  capabilities: ['compute']
});
```

### Agent with Metadata

```typescript
await swarm.registerAgent({
  id: 'agent-2',
  role: 'worker',
  host: 'agent-2.example.com',
  port: 4435,
  capabilities: ['compute', 'analyze', 'aggregate'],
  metadata: {
    region: 'us-east-1',
    instance_type: 'c5.2xlarge',
    max_concurrent_tasks: 10
  }
});
```

### Unregister Agent

```typescript
await swarm.unregisterAgent('agent-1');
```

## Statistics & Monitoring

### Swarm Statistics

```typescript
const stats = await swarm.getStats();

console.log('Swarm ID:', stats.swarmId);
console.log('Topology:', stats.topology);
console.log('Transport:', stats.transport);
console.log('QUIC Available:', stats.quicAvailable);
console.log('Total Agents:', stats.coordinatorStats?.totalAgents);
console.log('Active Agents:', stats.coordinatorStats?.activeAgents);
console.log('Total Messages:', stats.coordinatorStats?.totalMessages);
console.log('Avg Latency:', stats.coordinatorStats?.averageLatency, 'ms');
```

### Transport Statistics

```typescript
const stats = await swarm.getStats();

if (stats.transportStats instanceof Map) {
  const quicStats = stats.transportStats.get('quic');
  const http2Stats = stats.transportStats.get('http2');

  console.log('QUIC Stats:', quicStats);
  console.log('HTTP/2 Stats:', http2Stats);
}
```

### Per-Agent Statistics

```typescript
// Access through coordinator if available
const coordinator = swarm.router?.getCoordinator();
if (coordinator) {
  const agentStats = coordinator.getAgentStats('agent-1');
  console.log('Agent Stats:', agentStats);

  const allStats = coordinator.getAllAgentStats();
  console.log('All Agent Stats:', allStats);
}
```

## Common Patterns

### Pattern 1: Distributed Computation

```typescript
// Initialize mesh swarm for distributed computation
const swarm = await initSwarm({
  swarmId: 'compute-swarm',
  topology: 'mesh',
  transport: 'quic',
  maxAgents: 10
});

// Register compute nodes
for (let i = 1; i <= 10; i++) {
  await swarm.registerAgent({
    id: `compute-${i}`,
    role: 'worker',
    host: `compute-${i}.local`,
    port: 4433 + i,
    capabilities: ['compute', 'verify']
  });
}

// Agents can now communicate peer-to-peer
// for distributed computation tasks
```

### Pattern 2: Task Distribution Pipeline

```typescript
// Initialize hierarchical swarm for task distribution
const swarm = await initSwarm({
  swarmId: 'pipeline-swarm',
  topology: 'hierarchical',
  transport: 'quic',
  maxAgents: 50
});

// Register task coordinators
for (let i = 1; i <= 5; i++) {
  await swarm.registerAgent({
    id: `coordinator-${i}`,
    role: 'coordinator',
    host: `coordinator-${i}.local`,
    port: 4433,
    capabilities: ['distribute', 'aggregate']
  });
}

// Register processing workers
for (let i = 1; i <= 40; i++) {
  await swarm.registerAgent({
    id: `worker-${i}`,
    role: 'worker',
    host: `worker-${i}.local`,
    port: 4434 + i,
    capabilities: ['process']
  });
}
```

### Pattern 3: Data Processing Pipeline

```typescript
// Initialize ring swarm for sequential processing
const swarm = await initSwarm({
  swarmId: 'data-pipeline',
  topology: 'ring',
  transport: 'quic',
  maxAgents: 5
});

// Register pipeline stages
const stages = ['ingest', 'validate', 'transform', 'enrich', 'store'];
for (let i = 0; i < stages.length; i++) {
  await swarm.registerAgent({
    id: `stage-${stages[i]}`,
    role: 'worker',
    host: `stage-${i}.local`,
    port: 4433 + i,
    capabilities: [stages[i]]
  });
}

// Data flows through stages in order
```

### Pattern 4: Hub-Based Coordination

```typescript
// Initialize star swarm for centralized coordination
const swarm = await initSwarm({
  swarmId: 'hub-swarm',
  topology: 'star',
  transport: 'quic',
  maxAgents: 20
});

// Register central hub
await swarm.registerAgent({
  id: 'hub',
  role: 'coordinator',
  host: 'hub.local',
  port: 4433,
  capabilities: ['coordinate', 'aggregate', 'monitor']
});

// Register spoke agents
for (let i = 1; i <= 15; i++) {
  await swarm.registerAgent({
    id: `spoke-${i}`,
    role: 'worker',
    host: `spoke-${i}.local`,
    port: 4434 + i,
    capabilities: ['process']
  });
}

// All communication goes through hub
```

## Troubleshooting

### QUIC Not Available

**Problem**: Swarm falls back to HTTP/2 unexpectedly

**Solutions**:
1. Check WASM module is loaded:
   ```typescript
   import { checkQuicAvailability } from 'agentic-flow';
   const available = await checkQuicAvailability();
   console.log('QUIC available:', available);
   ```

2. Verify TLS certificates (if using peer verification):
   ```bash
   ls -la ./certs/cert.pem
   ls -la ./certs/key.pem
   ```

3. Check firewall allows UDP traffic on QUIC port:
   ```bash
   sudo ufw allow 4433/udp
   ```

### High Message Latency

**Problem**: Messages taking longer than expected

**Solutions**:
1. Check transport being used:
   ```typescript
   const stats = await swarm.getStats();
   console.log('Current transport:', stats.transport);
   ```

2. Enable QUIC explicitly:
   ```typescript
   const swarm = await initSwarm({
     transport: 'quic',
     enableFallback: false
   });
   ```

3. Monitor QUIC statistics:
   ```typescript
   const quicStats = stats.transportStats?.get('quic');
   console.log('RTT:', quicStats?.rttMs, 'ms');
   console.log('Packet loss:', quicStats?.packetsLost);
   ```

### Connection Pool Exhaustion

**Problem**: "Maximum connections reached" errors

**Solutions**:
1. Increase max agents:
   ```typescript
   const swarm = await initSwarm({
     maxAgents: 50  // Increase from default 10
   });
   ```

2. Use hierarchical topology for better scaling:
   ```typescript
   const swarm = await initSwarm({
     topology: 'hierarchical'  // Better for large swarms
   });
   ```

### Agent Registration Failures

**Problem**: Cannot register new agents

**Solutions**:
1. Check max agents limit:
   ```typescript
   const stats = await swarm.getStats();
   console.log('Total agents:', stats.coordinatorStats?.totalAgents);
   console.log('Max agents:', stats.coordinatorStats?.maxAgents);
   ```

2. Verify agent connectivity:
   ```bash
   nc -zv agent-1.local 4434
   ```

3. Check for duplicate agent IDs:
   ```typescript
   await swarm.unregisterAgent('existing-agent-id');
   await swarm.registerAgent({ id: 'existing-agent-id', ... });
   ```

## Next Steps

- Read the [Architecture Documentation](../architecture/QUIC-SWARM-INTEGRATION.md)
- Explore [Example Applications](../../examples/)
- Learn about [Transport Configuration](../guides/QUIC-TRANSPORT-CONFIG.md)
- See [Performance Benchmarks](../validation-reports/QUIC-PERFORMANCE.md)

## Support

- GitHub Issues: https://github.com/ruvnet/agentic-flow/issues
- Documentation: https://github.com/ruvnet/agentic-flow/docs
- Examples: https://github.com/ruvnet/agentic-flow/examples
