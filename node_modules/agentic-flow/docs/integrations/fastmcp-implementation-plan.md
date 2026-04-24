# FastMCP Implementation Plan for Agentic-Flow

## Executive Summary

This document outlines a comprehensive plan to integrate the **fastmcp** TypeScript framework into agentic-flow, enhancing the Model Context Protocol (MCP) server infrastructure with improved performance, real-time streaming capabilities via HTTP transport, and optional security features.

### Current State
- **Architecture**: 4 MCP servers (claude-flow-sdk in-process, claude-flow, flow-nexus, agentic-payments as subprocesses)
- **Tools**: 203+ MCP tools across multiple domains
- **Implementation**: Custom MCP implementation using `@anthropic-ai/claude-agent-sdk` and `@modelcontextprotocol/sdk`
- **Transport**: Primarily stdio for local/subprocess communication
- **Deployment**: Local, Docker, and cloud environments

### Target State
- **Framework**: fastmcp TypeScript framework for standardized MCP server implementation
- **Transport**: Dual support for stdio (local) and HTTP streaming (network/cloud)
- **Security**: Optional authentication, authorization, and rate limiting layers
- **Performance**: Reduced boilerplate, improved tooling, better debugging
- **Maintainability**: Standardized patterns across all MCP servers

### Benefits
1. **Developer Experience**: Simplified tool definition with Zod schema validation
2. **Performance**: Built-in optimizations, streaming support, progress notifications
3. **Security**: Session-based authentication, tool-level authorization, rate limiting
4. **Observability**: Enhanced logging, typed server events, health-check endpoints
5. **Flexibility**: Support for both stdio and HTTP transports with automatic CORS
6. **Testing**: Built-in CLI tools for development and debugging (`npx fastmcp dev`)

---

## 1. Research Summary

### FastMCP Overview

FastMCP (v3.19.0+) is a TypeScript framework built on top of `@modelcontextprotocol/sdk` that eliminates boilerplate and provides intuitive APIs for building MCP servers.

**Key Features:**
- Simple tool, resource, and prompt definition
- Schema validation support (Zod, ArkType, Valibot, Standard Schema)
- Session-based authentication with tool-level access control
- Multiple transport types: stdio, HTTP streaming (SSE deprecated)
- Stateless mode for serverless deployments
- Progress notifications and streaming output
- Typed server events and prompt auto-completion
- CORS enabled by default
- Health-check endpoints
- Built-in development CLI

**Package Info:**
- **npm**: `npm install fastmcp`
- **GitHub**: https://github.com/punkpeye/fastmcp
- **Documentation**: https://gofastmcp.com/
- **Usage**: 382+ projects in npm registry

### Transport Mechanisms

#### 1. Stdio Transport (Default)
- **Use Case**: Local command-line tools, desktop applications (Claude Desktop)
- **Communication**: Standard input/output streams
- **Advantages**: Simple, lightweight, perfect for subprocess communication
- **Current Match**: Aligns with current agentic-flow subprocess servers

#### 2. HTTP Streaming Transport (Recommended for Web)
- **Use Case**: Network-based deployments, multiple clients, cloud services
- **Communication**: Streamable HTTP with optional SSE for server-to-client streaming
- **Advantages**: Bidirectional communication, scalable, web-friendly
- **Target Use**: Flow-nexus cloud features, remote agent coordination

#### 3. SSE Transport (Deprecated)
- **Status**: Maintained for backward compatibility only
- **Recommendation**: Use HTTP streaming for all new projects
- **Migration Path**: Clients can fall back from HTTP to SSE for old servers

### Security Features

#### Authentication
```typescript
const server = new FastMCP({
  name: "Secure Server",
  authenticate: async (req) => {
    const apiKey = req.headers.get('x-api-key');
    if (!apiKey || apiKey !== process.env.API_KEY) {
      throw new Response('Unauthorized', { status: 401 });
    }
    return { userId: 'user-123', role: 'admin' };
  }
});
```

#### Tool-Level Authorization
```typescript
server.addTool({
  name: "admin-dashboard",
  canAccess: (auth) => auth.role === 'admin',
  execute: async () => { /* ... */ }
});
```

#### Rate Limiting
- TypeScript version: Middleware pattern support (not built-in like Python version)
- Python version: Built-in token bucket algorithms, per-client identification
- Implementation: Custom middleware or third-party libraries (express-rate-limit)

#### OAuth Support
- Built-in OAuth discovery endpoints
- Supports MCP Specification 2025-03-26 and 2025-06-18

### Comparison: Current vs FastMCP

| Feature | Current Implementation | FastMCP Implementation |
|---------|----------------------|----------------------|
| **Tool Definition** | Manual SDK calls with verbose handlers | Simple `addTool()` with Zod schemas |
| **Schema Validation** | Custom validation logic | Built-in Zod/ArkType/Valibot support |
| **Authentication** | Not implemented | Session-based with tool-level control |
| **Transport** | Stdio only | Stdio + HTTP streaming |
| **Progress Reporting** | Not available | Built-in progress notifications |
| **Streaming** | Not available | Built-in streaming output |
| **Development Tools** | Manual testing | `npx fastmcp dev` and `npx fastmcp inspect` |
| **Error Handling** | Custom error handling | Standardized error responses |
| **Type Safety** | Partial | Full TypeScript support |
| **CORS** | Manual setup | Enabled by default |
| **Health Checks** | Not available | Built-in health-check endpoint |
| **Session Management** | Not available | Built-in session support |
| **OAuth** | Not implemented | Built-in OAuth discovery |

---

## 2. Technical Architecture

### 2.1 Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Agentic-Flow Core                         │
│                                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │ Goal Planner│  │ Orchestrator│  │  Agent SDK  │        │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘        │
│         │                 │                 │               │
│         └─────────────────┴─────────────────┘               │
│                           │                                  │
└───────────────────────────┼──────────────────────────────────┘
                            │
                    ┌───────▼────────┐
                    │  MCP Gateway   │
                    │  (FastMCP Hub) │
                    └───────┬────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
   ┌────▼─────┐      ┌─────▼──────┐     ┌─────▼──────┐
   │ In-Process│      │ Subprocess │     │   HTTP     │
   │  FastMCP  │      │  FastMCP   │     │  FastMCP   │
   │  Servers  │      │  Servers   │     │  Servers   │
   └────┬─────┘      └─────┬──────┘     └─────┬──────┘
        │                  │                   │
   ┌────▼─────┐      ┌─────▼──────┐     ┌─────▼──────┐
   │ claude-  │      │ claude-    │     │ flow-nexus │
   │ flow-sdk │      │ flow       │     │  (cloud)   │
   │          │      │ agentic-   │     │            │
   │ (stdio)  │      │ payments   │     │  (HTTP)    │
   └──────────┘      │ (stdio)    │     └────────────┘
                     └────────────┘

Transport Layer:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│  Stdio Transport │     │  HTTP Streaming  │     │  SSE (Legacy)    │
│                  │     │                  │     │                  │
│  • stdin/stdout  │     │  • Bidirectional │     │  • Server→Client │
│  • Local process │     │  • Network       │     │  • Deprecated    │
│  • Claude Desktop│     │  • Multi-client  │     │  • Backward compat│
│  • Subprocess    │     │  • Cloud ready   │     │  • Migration only│
└──────────────────┘     └──────────────────┘     └──────────────────┘

Security Layer (Optional):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

┌─────────────────────────────────────────────────────────────┐
│  Authentication   │  Authorization  │  Rate Limiting        │
│  ───────────────  │  ─────────────  │  ────────────        │
│  • API Key        │  • Tool-level   │  • Per-client        │
│  • Bearer Token   │  • Role-based   │  • Per-tool          │
│  • OAuth 2.0      │  • Custom logic │  • Token bucket      │
│  • Session-based  │  • canAccess()  │  • Middleware        │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Server Migration Plan

#### Phase 1: In-Process Server (claude-flow-sdk)
**Current**: Uses `@anthropic-ai/claude-agent-sdk` with custom tool definitions
**Target**: FastMCP with stdio transport, no authentication needed (in-process)

```typescript
// Before (current)
import { createSdkMcpServer, tool } from '@anthropic-ai/claude-agent-sdk';
import { z } from 'zod';

export const claudeFlowSdkServer = createSdkMcpServer({
  name: 'claude-flow-sdk',
  version: '1.0.0',
  tools: [
    tool('memory_store', '...', { key: z.string(), ... }, async (args) => { ... })
  ]
});

// After (FastMCP)
import { FastMCP } from 'fastmcp';
import { z } from 'zod';

const server = new FastMCP({
  name: 'claude-flow-sdk',
  version: '1.0.0'
});

server.addTool({
  name: 'memory_store',
  description: 'Store a value in persistent memory',
  parameters: z.object({
    key: z.string().describe('Memory key'),
    value: z.string().describe('Value to store'),
    namespace: z.string().optional().default('default'),
    ttl: z.number().optional()
  }),
  execute: async ({ key, value, namespace, ttl }) => {
    // Implementation
    return `Stored ${key} in ${namespace}`;
  }
});

server.start({ transportType: 'stdio' });
```

#### Phase 2: Subprocess Servers (claude-flow, agentic-payments)
**Current**: Spawned as subprocesses with stdio communication
**Target**: FastMCP with stdio transport, optional authentication

```typescript
// claude-flow server with optional auth
import { FastMCP } from 'fastmcp';
import { z } from 'zod';

const server = new FastMCP({
  name: 'claude-flow',
  version: '2.0.0',
  // Optional: Enable authentication for secure environments
  authenticate: process.env.SECURE_MODE === 'true'
    ? async (req) => {
        const token = req.headers.get('authorization')?.replace('Bearer ', '');
        if (!token || token !== process.env.CLAUDE_FLOW_TOKEN) {
          throw new Response('Unauthorized', { status: 401 });
        }
        return { authenticated: true };
      }
    : undefined
});

// Tool definitions with progress reporting
server.addTool({
  name: 'swarm_init',
  description: 'Initialize multi-agent swarm',
  parameters: z.object({
    topology: z.enum(['mesh', 'hierarchical', 'ring', 'star']),
    maxAgents: z.number().default(8),
    strategy: z.enum(['balanced', 'specialized', 'adaptive']).default('balanced')
  }),
  execute: async ({ topology, maxAgents, strategy }, { onProgress }) => {
    onProgress?.({ progress: 0.2, message: 'Initializing topology...' });
    // Initialize swarm
    onProgress?.({ progress: 0.5, message: 'Spawning agents...' });
    // Spawn agents
    onProgress?.({ progress: 1.0, message: 'Swarm ready' });
    return { swarmId: 'swarm-123', agents: maxAgents, topology };
  }
});

server.start({ transportType: 'stdio' });
```

#### Phase 3: Cloud/Network Server (flow-nexus)
**Current**: HTTP-based custom implementation
**Target**: FastMCP with HTTP streaming transport, full authentication

```typescript
// flow-nexus cloud server with full security
import { FastMCP } from 'fastmcp';
import { z } from 'zod';
import { verifyJWT, checkRateLimit } from './security';

const server = new FastMCP({
  name: 'flow-nexus',
  version: '1.0.0',
  authenticate: async (req) => {
    // JWT-based authentication
    const token = req.headers.get('authorization')?.replace('Bearer ', '');
    if (!token) {
      throw new Response('Missing token', { status: 401 });
    }

    const payload = await verifyJWT(token);
    if (!payload) {
      throw new Response('Invalid token', { status: 401 });
    }

    // Rate limiting check
    const allowed = await checkRateLimit(payload.userId);
    if (!allowed) {
      throw new Response('Rate limit exceeded', { status: 429 });
    }

    return {
      userId: payload.userId,
      tier: payload.tier,
      role: payload.role
    };
  }
});

// Tool with role-based access control
server.addTool({
  name: 'sandbox_create',
  description: 'Create isolated code execution sandbox',
  canAccess: (auth) => ['pro', 'enterprise'].includes(auth.tier),
  parameters: z.object({
    template: z.enum(['node', 'python', 'react', 'nextjs']),
    name: z.string().optional(),
    env_vars: z.record(z.string()).optional()
  }),
  execute: async ({ template, name, env_vars }, { onProgress, auth }) => {
    onProgress?.({ progress: 0.1, message: 'Allocating sandbox...' });
    // Create sandbox
    onProgress?.({ progress: 0.5, message: 'Installing dependencies...' });
    // Setup environment
    onProgress?.({ progress: 1.0, message: 'Sandbox ready' });

    return {
      sandboxId: 'sandbox-456',
      template,
      userId: auth.userId,
      url: `https://sandbox-456.flow-nexus.io`
    };
  }
});

// Start with HTTP streaming transport
server.start({
  transportType: 'http',
  port: process.env.PORT || 8080,
  host: '0.0.0.0'
});
```

### 2.3 Security Architecture

#### Authentication Layer

```typescript
// security/auth.ts
import jwt from 'jsonwebtoken';

export interface AuthContext {
  userId: string;
  tier: 'free' | 'pro' | 'enterprise';
  role: 'user' | 'admin';
  permissions: string[];
}

export async function verifyJWT(token: string): Promise<AuthContext | null> {
  try {
    const payload = jwt.verify(token, process.env.JWT_SECRET!) as any;
    return {
      userId: payload.sub,
      tier: payload.tier || 'free',
      role: payload.role || 'user',
      permissions: payload.permissions || []
    };
  } catch {
    return null;
  }
}

export async function verifyAPIKey(apiKey: string): Promise<AuthContext | null> {
  // Check against database or cache
  const user = await db.users.findByAPIKey(apiKey);
  if (!user) return null;

  return {
    userId: user.id,
    tier: user.tier,
    role: user.role,
    permissions: user.permissions
  };
}
```

#### Rate Limiting

```typescript
// security/rate-limit.ts
import { RedisClient } from 'redis';

interface RateLimitConfig {
  windowMs: number;
  maxRequests: number;
  keyPrefix: string;
}

export class RateLimiter {
  private redis: RedisClient;

  constructor(private config: RateLimitConfig) {
    this.redis = new RedisClient(process.env.REDIS_URL);
  }

  async checkLimit(userId: string): Promise<boolean> {
    const key = `${this.config.keyPrefix}:${userId}`;
    const now = Date.now();
    const windowStart = now - this.config.windowMs;

    // Remove old entries
    await this.redis.zremrangebyscore(key, 0, windowStart);

    // Count requests in current window
    const count = await this.redis.zcard(key);

    if (count >= this.config.maxRequests) {
      return false;
    }

    // Add current request
    await this.redis.zadd(key, now, `${now}`);
    await this.redis.expire(key, Math.ceil(this.config.windowMs / 1000));

    return true;
  }
}

// Usage
const limiter = new RateLimiter({
  windowMs: 60000, // 1 minute
  maxRequests: 100,
  keyPrefix: 'rate-limit'
});

export async function checkRateLimit(userId: string): Promise<boolean> {
  return await limiter.checkLimit(userId);
}
```

#### Tool Authorization

```typescript
// security/authorization.ts
export interface Permission {
  resource: string;
  action: 'read' | 'write' | 'execute';
}

export function hasPermission(
  auth: AuthContext,
  required: Permission
): boolean {
  // Admin has all permissions
  if (auth.role === 'admin') return true;

  // Check tier-based permissions
  const tierPermissions = {
    free: ['swarm:read', 'memory:read', 'agent:read'],
    pro: ['swarm:*', 'memory:*', 'agent:*', 'sandbox:*'],
    enterprise: ['*']
  };

  const permissions = tierPermissions[auth.tier];
  const pattern = `${required.resource}:${required.action}`;

  return permissions.includes(pattern) ||
         permissions.includes(`${required.resource}:*`) ||
         permissions.includes('*');
}

// Tool-level authorization helper
export function requirePermission(resource: string, action: string) {
  return (auth: AuthContext) => hasPermission(auth, { resource, action });
}
```

### 2.4 Transport Selection Strategy

```typescript
// config/transport.ts
export interface TransportConfig {
  type: 'stdio' | 'http';
  port?: number;
  host?: string;
  enableAuth?: boolean;
  enableCORS?: boolean;
}

export function selectTransport(): TransportConfig {
  // Environment-based transport selection
  const env = process.env.NODE_ENV || 'development';
  const deploymentMode = process.env.DEPLOYMENT_MODE || 'local';

  // Local subprocess: stdio
  if (deploymentMode === 'local' || deploymentMode === 'subprocess') {
    return {
      type: 'stdio',
      enableAuth: false
    };
  }

  // Docker container: stdio or HTTP based on port
  if (deploymentMode === 'docker') {
    return {
      type: process.env.MCP_PORT ? 'http' : 'stdio',
      port: parseInt(process.env.MCP_PORT || '8080'),
      host: '0.0.0.0',
      enableAuth: env === 'production',
      enableCORS: true
    };
  }

  // Cloud deployment: HTTP with auth
  if (deploymentMode === 'cloud') {
    return {
      type: 'http',
      port: parseInt(process.env.PORT || '8080'),
      host: '0.0.0.0',
      enableAuth: true,
      enableCORS: true
    };
  }

  // Default to stdio
  return {
    type: 'stdio',
    enableAuth: false
  };
}
```

---

## 3. Migration Strategy

### 3.1 Migration Phases

#### Phase 0: Preparation (Week 1)
**Goals:**
- Install and evaluate fastmcp in development environment
- Create proof-of-concept with single tool
- Benchmark performance vs current implementation
- Document API differences

**Tasks:**
1. Install fastmcp: `npm install fastmcp`
2. Create test server with 1-2 tools
3. Compare stdio performance
4. Test HTTP streaming transport
5. Evaluate authentication options
6. Document findings

**Deliverables:**
- POC server implementation
- Performance comparison report
- API migration guide draft

#### Phase 1: In-Process Server Migration (Week 2-3)
**Target:** claude-flow-sdk server (in-process, stdio)

**Migration Steps:**
1. Create new `src/mcp/fastmcp/` directory structure
2. Implement base FastMCP server class
3. Migrate tools one category at a time:
   - Memory tools (store, retrieve, search)
   - Swarm tools (init, spawn, status)
   - Coordination tools (orchestrate, metrics)
4. Add comprehensive error handling
5. Implement logging integration
6. Create migration tests
7. Run parallel with old implementation
8. Gradual rollout with feature flags

**Code Structure:**
```
src/mcp/fastmcp/
├── servers/
│   ├── base.ts                 # Base FastMCP server class
│   ├── claude-flow-sdk.ts      # In-process server
│   └── index.ts
├── tools/
│   ├── memory/                 # Memory tools
│   │   ├── store.ts
│   │   ├── retrieve.ts
│   │   └── search.ts
│   ├── swarm/                  # Swarm tools
│   │   ├── init.ts
│   │   ├── spawn.ts
│   │   └── status.ts
│   └── index.ts
├── security/
│   ├── auth.ts                 # Authentication
│   ├── rate-limit.ts           # Rate limiting
│   └── authorization.ts        # Tool-level auth
├── config/
│   ├── transport.ts            # Transport selection
│   └── server-config.ts        # Server configuration
└── index.ts
```

**Success Criteria:**
- All existing tools work with FastMCP
- No performance degradation
- Improved error messages
- Better TypeScript types
- Tests passing

#### Phase 2: Subprocess Servers Migration (Week 4-5)
**Target:** claude-flow and agentic-payments servers (subprocess, stdio)

**Migration Steps:**
1. Migrate claude-flow server
   - Convert all swarm coordination tools
   - Add progress reporting
   - Implement streaming for long operations
   - Add optional authentication
2. Migrate agentic-payments server
   - Convert payment tools
   - Add transaction streaming
   - Implement webhook notifications
   - Add authentication
3. Update subprocess spawning logic
4. Test inter-server communication
5. Validate memory coordination

**Authentication:**
- **Development**: No authentication
- **Production**: Optional token-based authentication via environment variable

**Success Criteria:**
- Subprocess servers work seamlessly
- Progress reporting functional
- Streaming operations work
- Optional auth doesn't break existing workflows

#### Phase 3: HTTP/Cloud Server (Week 6-7)
**Target:** flow-nexus server (HTTP streaming, full auth)

**Migration Steps:**
1. Implement HTTP streaming transport
2. Add JWT-based authentication
3. Implement rate limiting with Redis
4. Add role-based tool access control
5. Setup CORS configuration
6. Add health-check endpoints
7. Implement session management
8. Deploy to staging environment
9. Load testing
10. Production deployment

**Security Features:**
- JWT authentication
- API key support
- OAuth 2.0 integration
- Per-user rate limiting
- Tool-level authorization
- Request logging
- Audit trail

**Success Criteria:**
- HTTP transport working in cloud
- Authentication prevents unauthorized access
- Rate limiting prevents abuse
- Multi-client support verified
- Health checks operational
- Performance acceptable under load

#### Phase 4: Testing & Validation (Week 8)
**Goals:**
- Comprehensive integration testing
- Performance benchmarking
- Security audit
- Documentation update

**Testing Matrix:**
| Server | Transport | Auth | Tests |
|--------|-----------|------|-------|
| claude-flow-sdk | stdio | None | Unit, Integration |
| claude-flow | stdio | Optional | Unit, Integration, E2E |
| agentic-payments | stdio | Optional | Unit, Integration |
| flow-nexus | HTTP | Required | Unit, Integration, E2E, Load |

**Performance Benchmarks:**
- Tool invocation latency
- Throughput (tools/second)
- Memory usage
- CPU utilization
- Startup time
- Subprocess spawn time
- HTTP request latency

**Security Tests:**
- Authentication bypass attempts
- Rate limit effectiveness
- Authorization checks
- Token expiration
- CORS validation
- Input sanitization

#### Phase 5: Documentation & Rollout (Week 9-10)
**Goals:**
- Complete documentation
- Migration guide for users
- Training materials
- Staged rollout

**Documentation:**
1. Architecture overview
2. API reference for each server
3. Authentication guide
4. Transport selection guide
5. Security best practices
6. Migration guide from v1.x
7. Troubleshooting guide
8. Performance tuning guide

**Rollout Strategy:**
1. Beta release (10% users)
2. Feedback collection
3. Bug fixes
4. Expanded rollout (50% users)
5. Final validation
6. Full rollout (100% users)
7. Deprecate old implementation

### 3.2 Backward Compatibility

**Strategy:**
- Maintain old implementation alongside FastMCP
- Feature flag: `USE_FASTMCP=true/false`
- Gradual migration path
- Support both implementations for 2 releases

**Feature Flag Implementation:**
```typescript
// src/mcp/index.ts
import { getServerConfig } from './config';
import { createLegacyServer } from './legacy';
import { createFastMCPServer } from './fastmcp';

export function createMCPServer(name: string) {
  const config = getServerConfig();

  if (config.useFastMCP) {
    return createFastMCPServer(name);
  } else {
    return createLegacyServer(name);
  }
}
```

**Migration Timeline:**
- **v2.0.0**: FastMCP available, old implementation default
- **v2.1.0**: FastMCP default, old implementation available via flag
- **v2.2.0**: FastMCP only, old implementation removed

### 3.3 Rollback Plan

**Triggers:**
- Critical bugs in production
- Performance degradation > 20%
- Authentication vulnerabilities
- Data loss or corruption

**Rollback Procedure:**
1. Set `USE_FASTMCP=false` environment variable
2. Restart affected servers
3. Validate old implementation working
4. Investigate issue
5. Fix and retest
6. Re-enable FastMCP

**Rollback Testing:**
- Practice rollback in staging
- Document time to rollback (target: < 5 minutes)
- Validate no data loss during rollback

---

## 4. Implementation Details

### 4.1 Code Structure

```
src/mcp/
├── fastmcp/                    # New FastMCP implementation
│   ├── servers/
│   │   ├── base.ts             # Base server class with common setup
│   │   ├── claude-flow-sdk.ts  # In-process server
│   │   ├── claude-flow.ts      # Subprocess server
│   │   ├── agentic-payments.ts # Payments subprocess server
│   │   ├── flow-nexus.ts       # Cloud HTTP server
│   │   └── index.ts
│   ├── tools/
│   │   ├── memory/
│   │   │   ├── store.ts
│   │   │   ├── retrieve.ts
│   │   │   ├── search.ts
│   │   │   ├── list.ts
│   │   │   └── delete.ts
│   │   ├── swarm/
│   │   │   ├── init.ts
│   │   │   ├── spawn.ts
│   │   │   ├── status.ts
│   │   │   ├── scale.ts
│   │   │   └── destroy.ts
│   │   ├── agent/
│   │   │   ├── list.ts
│   │   │   ├── metrics.ts
│   │   │   └── monitor.ts
│   │   ├── task/
│   │   │   ├── orchestrate.ts
│   │   │   ├── status.ts
│   │   │   └── results.ts
│   │   ├── neural/
│   │   │   ├── train.ts
│   │   │   ├── predict.ts
│   │   │   ├── status.ts
│   │   │   └── patterns.ts
│   │   ├── github/
│   │   │   ├── analyze.ts
│   │   │   ├── pr-manage.ts
│   │   │   └── issue-track.ts
│   │   ├── sandbox/
│   │   │   ├── create.ts
│   │   │   ├── execute.ts
│   │   │   ├── upload.ts
│   │   │   └── logs.ts
│   │   └── index.ts
│   ├── security/
│   │   ├── auth.ts             # Authentication handlers
│   │   ├── rate-limit.ts       # Rate limiting
│   │   ├── authorization.ts    # Tool authorization
│   │   └── index.ts
│   ├── middleware/
│   │   ├── logging.ts          # Request/response logging
│   │   ├── metrics.ts          # Performance metrics
│   │   ├── error-handler.ts    # Error handling
│   │   └── index.ts
│   ├── config/
│   │   ├── transport.ts        # Transport configuration
│   │   ├── server-config.ts    # Server settings
│   │   ├── security-config.ts  # Security settings
│   │   └── index.ts
│   ├── utils/
│   │   ├── progress.ts         # Progress reporting helpers
│   │   ├── streaming.ts        # Streaming helpers
│   │   ├── validation.ts       # Schema validation
│   │   └── index.ts
│   ├── types/
│   │   ├── auth.ts             # Auth types
│   │   ├── tools.ts            # Tool types
│   │   ├── config.ts           # Config types
│   │   └── index.ts
│   └── index.ts
├── legacy/                     # Old implementation (deprecated)
│   └── ...
└── index.ts                    # Main entry point with feature flag
```

### 4.2 Base Server Implementation

```typescript
// src/mcp/fastmcp/servers/base.ts
import { FastMCP } from 'fastmcp';
import { z } from 'zod';
import type { AuthContext, TransportConfig } from '../types';
import { setupLogging } from '../middleware/logging';
import { setupMetrics } from '../middleware/metrics';
import { setupErrorHandler } from '../middleware/error-handler';

export interface BaseServerConfig {
  name: string;
  version: string;
  transport: TransportConfig;
  enableAuth?: boolean;
  enableMetrics?: boolean;
  enableLogging?: boolean;
}

export abstract class BaseMCPServer {
  protected server: FastMCP;
  protected config: BaseServerConfig;

  constructor(config: BaseServerConfig) {
    this.config = config;

    this.server = new FastMCP({
      name: config.name,
      version: config.version,
      authenticate: config.enableAuth
        ? this.authenticate.bind(this)
        : undefined
    });

    // Setup middleware
    if (config.enableLogging) {
      setupLogging(this.server);
    }
    if (config.enableMetrics) {
      setupMetrics(this.server);
    }
    setupErrorHandler(this.server);

    // Register tools
    this.registerTools();
  }

  protected abstract registerTools(): void;

  protected async authenticate(req: Request): Promise<AuthContext> {
    // Override in subclasses for custom auth
    throw new Response('Authentication not implemented', { status: 501 });
  }

  public async start() {
    const { transport } = this.config;

    if (transport.type === 'stdio') {
      await this.server.start({ transportType: 'stdio' });
    } else {
      await this.server.start({
        transportType: 'http',
        port: transport.port,
        host: transport.host
      });
    }

    console.log(`${this.config.name} server started on ${transport.type}`);
  }

  public async stop() {
    // Cleanup logic
  }
}
```

### 4.3 Tool Implementation Example

```typescript
// src/mcp/fastmcp/tools/memory/store.ts
import { z } from 'zod';
import type { ToolDefinition } from '../../types';
import { execSync } from 'child_process';

export const memoryStoreTool: ToolDefinition = {
  name: 'memory_store',
  description: 'Store a value in persistent memory with optional namespace and TTL',
  parameters: z.object({
    key: z.string()
      .min(1)
      .describe('Memory key'),
    value: z.string()
      .describe('Value to store'),
    namespace: z.string()
      .optional()
      .default('default')
      .describe('Memory namespace'),
    ttl: z.number()
      .positive()
      .optional()
      .describe('Time-to-live in seconds')
  }),
  execute: async ({ key, value, namespace, ttl }, { onProgress, auth }) => {
    try {
      // Progress reporting
      onProgress?.({ progress: 0.2, message: 'Validating input...' });

      // Build command
      const cmd = [
        'npx claude-flow@alpha memory store',
        `"${key}"`,
        `"${value}"`,
        `--namespace "${namespace}"`,
        ttl ? `--ttl ${ttl}` : ''
      ].filter(Boolean).join(' ');

      onProgress?.({ progress: 0.5, message: 'Storing value...' });

      // Execute
      const result = execSync(cmd, {
        encoding: 'utf-8',
        maxBuffer: 10 * 1024 * 1024
      });

      onProgress?.({ progress: 1.0, message: 'Stored successfully' });

      // Return formatted result
      return {
        success: true,
        key,
        namespace,
        size: value.length,
        ttl,
        userId: auth?.userId
      };
    } catch (error: any) {
      throw new Error(`Failed to store memory: ${error.message}`);
    }
  }
};
```

### 4.4 Authentication Implementation

```typescript
// src/mcp/fastmcp/security/auth.ts
import jwt from 'jsonwebtoken';
import type { AuthContext } from '../types';

export async function authenticateJWT(req: Request): Promise<AuthContext> {
  const authHeader = req.headers.get('authorization');
  if (!authHeader) {
    throw new Response('Missing Authorization header', { status: 401 });
  }

  const token = authHeader.replace('Bearer ', '');
  if (!token) {
    throw new Response('Invalid Authorization format', { status: 401 });
  }

  try {
    const payload = jwt.verify(token, process.env.JWT_SECRET!) as any;

    return {
      userId: payload.sub,
      tier: payload.tier || 'free',
      role: payload.role || 'user',
      permissions: payload.permissions || []
    };
  } catch (error) {
    throw new Response('Invalid or expired token', { status: 401 });
  }
}

export async function authenticateAPIKey(req: Request): Promise<AuthContext> {
  const apiKey = req.headers.get('x-api-key');
  if (!apiKey) {
    throw new Response('Missing API key', { status: 401 });
  }

  // In production, check against database
  if (apiKey !== process.env.API_KEY) {
    throw new Response('Invalid API key', { status: 401 });
  }

  // Return static context for demo
  return {
    userId: 'api-key-user',
    tier: 'pro',
    role: 'user',
    permissions: ['*']
  };
}
```

### 4.5 Server-Specific Implementations

#### Claude Flow SDK Server (In-Process)
```typescript
// src/mcp/fastmcp/servers/claude-flow-sdk.ts
import { BaseMCPServer } from './base';
import { selectTransport } from '../config/transport';
import * as memoryTools from '../tools/memory';
import * as swarmTools from '../tools/swarm';
import * as agentTools from '../tools/agent';
import * as taskTools from '../tools/task';

export class ClaudeFlowSdkServer extends BaseMCPServer {
  constructor() {
    super({
      name: 'claude-flow-sdk',
      version: '1.0.0',
      transport: selectTransport(),
      enableAuth: false, // In-process, no auth needed
      enableMetrics: true,
      enableLogging: true
    });
  }

  protected registerTools() {
    // Memory tools
    this.server.addTool(memoryTools.memoryStoreTool);
    this.server.addTool(memoryTools.memoryRetrieveTool);
    this.server.addTool(memoryTools.memorySearchTool);
    this.server.addTool(memoryTools.memoryListTool);
    this.server.addTool(memoryTools.memoryDeleteTool);

    // Swarm tools
    this.server.addTool(swarmTools.swarmInitTool);
    this.server.addTool(swarmTools.swarmStatusTool);
    this.server.addTool(swarmTools.swarmDestroyTool);

    // Agent tools
    this.server.addTool(agentTools.agentSpawnTool);
    this.server.addTool(agentTools.agentListTool);
    this.server.addTool(agentTools.agentMetricsTool);

    // Task tools
    this.server.addTool(taskTools.taskOrchestrateTool);
    this.server.addTool(taskTools.taskStatusTool);
    this.server.addTool(taskTools.taskResultsTool);
  }
}
```

#### Flow Nexus Server (HTTP with Auth)
```typescript
// src/mcp/fastmcp/servers/flow-nexus.ts
import { BaseMCPServer } from './base';
import { authenticateJWT } from '../security/auth';
import { checkRateLimit } from '../security/rate-limit';
import * as sandboxTools from '../tools/sandbox';
import * as neuralTools from '../tools/neural';
import * as githubTools from '../tools/github';

export class FlowNexusServer extends BaseMCPServer {
  constructor() {
    super({
      name: 'flow-nexus',
      version: '1.0.0',
      transport: {
        type: 'http',
        port: parseInt(process.env.PORT || '8080'),
        host: '0.0.0.0',
        enableAuth: true,
        enableCORS: true
      },
      enableAuth: true,
      enableMetrics: true,
      enableLogging: true
    });
  }

  protected async authenticate(req: Request) {
    const auth = await authenticateJWT(req);

    // Check rate limit
    const allowed = await checkRateLimit(auth.userId);
    if (!allowed) {
      throw new Response('Rate limit exceeded', { status: 429 });
    }

    return auth;
  }

  protected registerTools() {
    // Sandbox tools (Pro+ only)
    this.server.addTool({
      ...sandboxTools.sandboxCreateTool,
      canAccess: (auth) => ['pro', 'enterprise'].includes(auth.tier)
    });
    this.server.addTool({
      ...sandboxTools.sandboxExecuteTool,
      canAccess: (auth) => ['pro', 'enterprise'].includes(auth.tier)
    });

    // Neural tools (All authenticated users)
    this.server.addTool(neuralTools.neuralTrainTool);
    this.server.addTool(neuralTools.neuralPredictTool);
    this.server.addTool(neuralTools.neuralStatusTool);

    // GitHub tools (All authenticated users)
    this.server.addTool(githubTools.githubAnalyzeTool);
    this.server.addTool(githubTools.githubPRManageTool);

    // Admin tools (Admin only)
    this.server.addTool({
      name: 'admin_metrics',
      description: 'View system-wide metrics',
      canAccess: (auth) => auth.role === 'admin',
      parameters: z.object({}),
      execute: async () => {
        // Return metrics
      }
    });
  }
}
```

---

## 5. Testing Strategy

### 5.1 Unit Tests

```typescript
// tests/unit/tools/memory/store.test.ts
import { describe, it, expect, vi } from 'vitest';
import { memoryStoreTool } from '../../../../src/mcp/fastmcp/tools/memory/store';

describe('Memory Store Tool', () => {
  it('should store value successfully', async () => {
    const result = await memoryStoreTool.execute(
      { key: 'test', value: 'data', namespace: 'default' },
      { onProgress: vi.fn(), auth: undefined }
    );

    expect(result.success).toBe(true);
    expect(result.key).toBe('test');
  });

  it('should validate input schema', () => {
    expect(() => {
      memoryStoreTool.parameters.parse({ key: '', value: 'data' });
    }).toThrow();
  });

  it('should report progress', async () => {
    const onProgress = vi.fn();
    await memoryStoreTool.execute(
      { key: 'test', value: 'data', namespace: 'default' },
      { onProgress, auth: undefined }
    );

    expect(onProgress).toHaveBeenCalledWith(
      expect.objectContaining({ progress: 0.2 })
    );
  });
});
```

### 5.2 Integration Tests

```typescript
// tests/integration/servers/claude-flow-sdk.test.ts
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { ClaudeFlowSdkServer } from '../../../src/mcp/fastmcp/servers/claude-flow-sdk';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

describe('Claude Flow SDK Server', () => {
  let server: ClaudeFlowSdkServer;
  let client: any;

  beforeAll(async () => {
    server = new ClaudeFlowSdkServer();
    await server.start();

    // Create client
    const transport = new StdioClientTransport({
      command: 'node',
      args: ['dist/mcp/fastmcp/servers/claude-flow-sdk.js']
    });
    client = await transport.connect();
  });

  afterAll(async () => {
    await server.stop();
  });

  it('should list available tools', async () => {
    const tools = await client.listTools();
    expect(tools).toContain('memory_store');
    expect(tools).toContain('swarm_init');
  });

  it('should execute memory_store tool', async () => {
    const result = await client.callTool('memory_store', {
      key: 'integration-test',
      value: 'test-data',
      namespace: 'test'
    });

    expect(result.success).toBe(true);
  });
});
```

### 5.3 E2E Tests

```typescript
// tests/e2e/multi-server.test.ts
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { ClaudeFlowSdkServer } from '../../src/mcp/fastmcp/servers/claude-flow-sdk';
import { FlowNexusServer } from '../../src/mcp/fastmcp/servers/flow-nexus';

describe('Multi-Server Coordination', () => {
  let sdkServer: ClaudeFlowSdkServer;
  let nexusServer: FlowNexusServer;

  beforeAll(async () => {
    sdkServer = new ClaudeFlowSdkServer();
    nexusServer = new FlowNexusServer();
    await Promise.all([sdkServer.start(), nexusServer.start()]);
  });

  afterAll(async () => {
    await Promise.all([sdkServer.stop(), nexusServer.stop()]);
  });

  it('should coordinate swarm initialization', async () => {
    // Initialize swarm via SDK
    const swarmResult = await sdkClient.callTool('swarm_init', {
      topology: 'mesh',
      maxAgents: 5
    });

    // Create sandbox via Nexus
    const sandboxResult = await nexusClient.callTool('sandbox_create', {
      template: 'node',
      name: 'test-sandbox'
    });

    // Verify coordination
    const status = await sdkClient.callTool('swarm_status');
    expect(status.agents).toHaveLength(5);
  });
});
```

### 5.4 Performance Tests

```typescript
// tests/performance/throughput.test.ts
import { describe, it, expect } from 'vitest';
import { performance } from 'perf_hooks';

describe('Performance Benchmarks', () => {
  it('should handle 1000 tool calls within 5 seconds', async () => {
    const start = performance.now();

    const promises = Array.from({ length: 1000 }, () =>
      client.callTool('memory_store', {
        key: `perf-${Math.random()}`,
        value: 'test-data',
        namespace: 'perf'
      })
    );

    await Promise.all(promises);
    const duration = performance.now() - start;

    expect(duration).toBeLessThan(5000);
    console.log(`Throughput: ${(1000 / duration * 1000).toFixed(2)} tools/sec`);
  });

  it('should maintain low latency under load', async () => {
    const latencies: number[] = [];

    for (let i = 0; i < 100; i++) {
      const start = performance.now();
      await client.callTool('swarm_status');
      latencies.push(performance.now() - start);
    }

    const avg = latencies.reduce((a, b) => a + b) / latencies.length;
    const p95 = latencies.sort()[Math.floor(latencies.length * 0.95)];

    expect(avg).toBeLessThan(50); // 50ms average
    expect(p95).toBeLessThan(100); // 100ms p95

    console.log(`Avg latency: ${avg.toFixed(2)}ms, P95: ${p95.toFixed(2)}ms`);
  });
});
```

### 5.5 Security Tests

```typescript
// tests/security/auth.test.ts
import { describe, it, expect } from 'vitest';
import { FlowNexusServer } from '../../src/mcp/fastmcp/servers/flow-nexus';

describe('Authentication Security', () => {
  it('should reject requests without token', async () => {
    const response = await fetch('http://localhost:8080/tools/list');
    expect(response.status).toBe(401);
  });

  it('should reject invalid tokens', async () => {
    const response = await fetch('http://localhost:8080/tools/list', {
      headers: { 'Authorization': 'Bearer invalid-token' }
    });
    expect(response.status).toBe(401);
  });

  it('should accept valid tokens', async () => {
    const token = generateValidToken();
    const response = await fetch('http://localhost:8080/tools/list', {
      headers: { 'Authorization': `Bearer ${token}` }
    });
    expect(response.status).toBe(200);
  });

  it('should enforce rate limits', async () => {
    const token = generateValidToken();

    // Make 100 requests (rate limit)
    for (let i = 0; i < 100; i++) {
      await fetch('http://localhost:8080/tools/list', {
        headers: { 'Authorization': `Bearer ${token}` }
      });
    }

    // 101st request should be rate limited
    const response = await fetch('http://localhost:8080/tools/list', {
      headers: { 'Authorization': `Bearer ${token}` }
    });
    expect(response.status).toBe(429);
  });
});
```

---

## 6. Development & Deployment

### 6.1 Development Setup

```bash
# Install dependencies
npm install fastmcp zod jsonwebtoken redis

# Install dev dependencies
npm install -D @types/jsonwebtoken vitest

# Run in development
npm run dev

# Test FastMCP server
npx fastmcp dev src/mcp/fastmcp/servers/claude-flow-sdk.ts

# Inspect server
npx fastmcp inspect src/mcp/fastmcp/servers/claude-flow-sdk.ts
```

### 6.2 Environment Configuration

```bash
# .env.development
NODE_ENV=development
DEPLOYMENT_MODE=local
USE_FASTMCP=true
ENABLE_AUTH=false
ENABLE_METRICS=true
LOG_LEVEL=debug

# .env.production
NODE_ENV=production
DEPLOYMENT_MODE=cloud
USE_FASTMCP=true
ENABLE_AUTH=true
ENABLE_METRICS=true
LOG_LEVEL=info
JWT_SECRET=your-secret-key
API_KEY=your-api-key
REDIS_URL=redis://localhost:6379
PORT=8080
```

### 6.3 Docker Configuration

```dockerfile
# Dockerfile
FROM node:18-alpine

WORKDIR /app

# Install dependencies
COPY package*.json ./
RUN npm ci --only=production

# Copy source
COPY dist ./dist
COPY .env.production ./.env

# Expose ports
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD node -e "require('http').get('http://localhost:8080/health', (r) => r.statusCode === 200 ? process.exit(0) : process.exit(1))"

# Start server
CMD ["node", "dist/mcp/fastmcp/servers/flow-nexus.js"]
```

```yaml
# docker-compose.yml
version: '3.8'

services:
  # In-process server (not containerized, runs with main app)

  # Claude Flow subprocess server
  claude-flow:
    build: .
    command: node dist/mcp/fastmcp/servers/claude-flow.js
    environment:
      - NODE_ENV=production
      - DEPLOYMENT_MODE=docker
      - ENABLE_AUTH=false
    volumes:
      - ./data:/app/data

  # Flow Nexus HTTP server
  flow-nexus:
    build: .
    command: node dist/mcp/fastmcp/servers/flow-nexus.js
    ports:
      - "8080:8080"
    environment:
      - NODE_ENV=production
      - DEPLOYMENT_MODE=cloud
      - ENABLE_AUTH=true
      - JWT_SECRET=${JWT_SECRET}
      - REDIS_URL=redis://redis:6379
      - PORT=8080
    depends_on:
      - redis
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 3s
      retries: 3

  # Redis for rate limiting
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data

volumes:
  redis-data:
```

### 6.4 Kubernetes Deployment

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: flow-nexus
spec:
  replicas: 3
  selector:
    matchLabels:
      app: flow-nexus
  template:
    metadata:
      labels:
        app: flow-nexus
    spec:
      containers:
      - name: flow-nexus
        image: flow-nexus:latest
        ports:
        - containerPort: 8080
        env:
        - name: NODE_ENV
          value: "production"
        - name: DEPLOYMENT_MODE
          value: "cloud"
        - name: ENABLE_AUTH
          value: "true"
        - name: JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: flow-nexus-secrets
              key: jwt-secret
        - name: REDIS_URL
          value: "redis://redis-service:6379"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
---
apiVersion: v1
kind: Service
metadata:
  name: flow-nexus-service
spec:
  type: LoadBalancer
  selector:
    app: flow-nexus
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8080
```

### 6.5 CI/CD Pipeline

```yaml
# .github/workflows/deploy.yml
name: Deploy FastMCP Servers

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'
      - run: npm ci
      - run: npm run build
      - run: npm test
      - run: npm run test:integration
      - run: npm run test:security

  benchmark:
    runs-on: ubuntu-latest
    needs: test
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'
      - run: npm ci
      - run: npm run build
      - run: npm run benchmark
      - name: Compare performance
        run: |
          node scripts/compare-benchmarks.js

  deploy-staging:
    runs-on: ubuntu-latest
    needs: [test, benchmark]
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v3
      - name: Deploy to staging
        run: |
          kubectl apply -f k8s/staging/
      - name: Run smoke tests
        run: npm run test:e2e:staging

  deploy-production:
    runs-on: ubuntu-latest
    needs: deploy-staging
    if: github.ref == 'refs/heads/main'
    environment: production
    steps:
      - uses: actions/checkout@v3
      - name: Deploy to production
        run: |
          kubectl apply -f k8s/production/
      - name: Run smoke tests
        run: npm run test:e2e:production
      - name: Monitor rollout
        run: kubectl rollout status deployment/flow-nexus
```

---

## 7. Code Examples

### 7.1 Basic Stdio Server

```typescript
// examples/stdio-server.ts
import { FastMCP } from 'fastmcp';
import { z } from 'zod';

// Create server
const server = new FastMCP({
  name: 'example-stdio-server',
  version: '1.0.0'
});

// Add tool with Zod schema
server.addTool({
  name: 'calculate',
  description: 'Perform basic arithmetic operations',
  parameters: z.object({
    operation: z.enum(['add', 'subtract', 'multiply', 'divide']),
    a: z.number(),
    b: z.number()
  }),
  execute: async ({ operation, a, b }) => {
    switch (operation) {
      case 'add':
        return a + b;
      case 'subtract':
        return a - b;
      case 'multiply':
        return a * b;
      case 'divide':
        if (b === 0) throw new Error('Division by zero');
        return a / b;
    }
  }
});

// Start with stdio transport
server.start({ transportType: 'stdio' });
```

### 7.2 HTTP Server with Authentication

```typescript
// examples/http-server-auth.ts
import { FastMCP } from 'fastmcp';
import { z } from 'zod';
import jwt from 'jsonwebtoken';

// Create server with authentication
const server = new FastMCP({
  name: 'secure-http-server',
  version: '1.0.0',
  authenticate: async (req) => {
    const token = req.headers.get('authorization')?.replace('Bearer ', '');
    if (!token) {
      throw new Response('Missing token', { status: 401 });
    }

    try {
      const payload = jwt.verify(token, process.env.JWT_SECRET!) as any;
      return {
        userId: payload.sub,
        role: payload.role
      };
    } catch {
      throw new Response('Invalid token', { status: 401 });
    }
  }
});

// Add public tool (no auth required)
server.addTool({
  name: 'health',
  description: 'Health check endpoint',
  parameters: z.object({}),
  execute: async () => ({
    status: 'healthy',
    timestamp: new Date().toISOString()
  })
});

// Add authenticated tool with role check
server.addTool({
  name: 'admin_action',
  description: 'Admin-only action',
  canAccess: (auth) => auth.role === 'admin',
  parameters: z.object({
    action: z.string()
  }),
  execute: async ({ action }, { auth }) => {
    console.log(`Admin ${auth.userId} performed: ${action}`);
    return { success: true };
  }
});

// Start with HTTP transport
server.start({
  transportType: 'http',
  port: 8080,
  host: '0.0.0.0'
});
```

### 7.3 Server with Progress Reporting

```typescript
// examples/progress-server.ts
import { FastMCP } from 'fastmcp';
import { z } from 'zod';

const server = new FastMCP({
  name: 'progress-server',
  version: '1.0.0'
});

server.addTool({
  name: 'process_data',
  description: 'Process large dataset with progress updates',
  parameters: z.object({
    dataSize: z.number().min(1).max(1000000)
  }),
  execute: async ({ dataSize }, { onProgress }) => {
    const chunks = Math.ceil(dataSize / 1000);

    for (let i = 0; i < chunks; i++) {
      // Simulate processing
      await new Promise(resolve => setTimeout(resolve, 100));

      // Report progress
      const progress = (i + 1) / chunks;
      onProgress?.({
        progress,
        message: `Processing chunk ${i + 1}/${chunks}`
      });
    }

    return {
      processed: dataSize,
      chunks,
      duration: chunks * 100
    };
  }
});

server.start({ transportType: 'stdio' });
```

### 7.4 Server with Streaming Output

```typescript
// examples/streaming-server.ts
import { FastMCP } from 'fastmcp';
import { z } from 'zod';

const server = new FastMCP({
  name: 'streaming-server',
  version: '1.0.0'
});

server.addTool({
  name: 'generate_report',
  description: 'Generate report with streaming output',
  parameters: z.object({
    sections: z.array(z.string())
  }),
  execute: async function* ({ sections }) {
    yield '# Report\n\n';

    for (const section of sections) {
      yield `## ${section}\n\n`;

      // Simulate section generation
      await new Promise(resolve => setTimeout(resolve, 500));

      yield `Content for ${section}...\n\n`;
    }

    yield '---\nReport complete\n';
  }
});

server.start({ transportType: 'http', port: 8080 });
```

### 7.5 Multi-Transport Server

```typescript
// examples/multi-transport.ts
import { FastMCP } from 'fastmcp';
import { z } from 'zod';

function createServer() {
  const server = new FastMCP({
    name: 'multi-transport-server',
    version: '1.0.0'
  });

  // Add tools...
  server.addTool({
    name: 'echo',
    description: 'Echo a message',
    parameters: z.object({ message: z.string() }),
    execute: async ({ message }) => message
  });

  return server;
}

// Determine transport from environment
const transportType = process.env.TRANSPORT_TYPE || 'stdio';

if (transportType === 'stdio') {
  const server = createServer();
  server.start({ transportType: 'stdio' });
  console.error('Started with stdio transport');
} else {
  const server = createServer();
  server.start({
    transportType: 'http',
    port: parseInt(process.env.PORT || '8080'),
    host: process.env.HOST || '0.0.0.0'
  });
  console.error(`Started with HTTP transport on ${process.env.PORT || 8080}`);
}
```

---

## 8. Performance Optimization

### 8.1 Connection Pooling

```typescript
// src/mcp/fastmcp/utils/connection-pool.ts
export class ConnectionPool {
  private connections: Map<string, any> = new Map();
  private maxSize: number;

  constructor(maxSize = 10) {
    this.maxSize = maxSize;
  }

  async getConnection(key: string, factory: () => Promise<any>) {
    if (this.connections.has(key)) {
      return this.connections.get(key);
    }

    if (this.connections.size >= this.maxSize) {
      // Evict oldest connection
      const firstKey = this.connections.keys().next().value;
      const conn = this.connections.get(firstKey);
      await conn?.close?.();
      this.connections.delete(firstKey);
    }

    const conn = await factory();
    this.connections.set(key, conn);
    return conn;
  }

  async closeAll() {
    for (const [key, conn] of this.connections) {
      await conn?.close?.();
    }
    this.connections.clear();
  }
}
```

### 8.2 Caching Layer

```typescript
// src/mcp/fastmcp/middleware/cache.ts
import { createHash } from 'crypto';

interface CacheEntry {
  value: any;
  timestamp: number;
  ttl: number;
}

export class ToolCache {
  private cache = new Map<string, CacheEntry>();

  private getCacheKey(toolName: string, args: any): string {
    const hash = createHash('sha256');
    hash.update(JSON.stringify({ toolName, args }));
    return hash.digest('hex');
  }

  get(toolName: string, args: any): any | null {
    const key = this.getCacheKey(toolName, args);
    const entry = this.cache.get(key);

    if (!entry) return null;

    const now = Date.now();
    if (now - entry.timestamp > entry.ttl) {
      this.cache.delete(key);
      return null;
    }

    return entry.value;
  }

  set(toolName: string, args: any, value: any, ttl = 60000) {
    const key = this.getCacheKey(toolName, args);
    this.cache.set(key, {
      value,
      timestamp: Date.now(),
      ttl
    });
  }

  clear() {
    this.cache.clear();
  }
}

// Wrap tool execution with caching
export function withCache(cache: ToolCache, ttl?: number) {
  return (tool: any) => ({
    ...tool,
    execute: async (args: any, context: any) => {
      // Check cache
      const cached = cache.get(tool.name, args);
      if (cached) {
        return cached;
      }

      // Execute tool
      const result = await tool.execute(args, context);

      // Store in cache
      cache.set(tool.name, args, result, ttl);

      return result;
    }
  });
}
```

### 8.3 Request Batching

```typescript
// src/mcp/fastmcp/utils/batch.ts
export class RequestBatcher {
  private pending: Map<string, Promise<any>> = new Map();
  private batchDelay: number;

  constructor(batchDelay = 10) {
    this.batchDelay = batchDelay;
  }

  async batch<T>(key: string, executor: () => Promise<T>): Promise<T> {
    // Check if request with same key is pending
    if (this.pending.has(key)) {
      return this.pending.get(key);
    }

    // Create new batch
    const promise = new Promise<T>((resolve, reject) => {
      setTimeout(async () => {
        try {
          const result = await executor();
          this.pending.delete(key);
          resolve(result);
        } catch (error) {
          this.pending.delete(key);
          reject(error);
        }
      }, this.batchDelay);
    });

    this.pending.set(key, promise);
    return promise;
  }
}
```

---

## 9. Monitoring & Observability

### 9.1 Metrics Collection

```typescript
// src/mcp/fastmcp/middleware/metrics.ts
import { Counter, Histogram, Registry } from 'prom-client';

export class MetricsCollector {
  private registry: Registry;
  private toolCallCounter: Counter;
  private toolDurationHistogram: Histogram;
  private errorCounter: Counter;

  constructor() {
    this.registry = new Registry();

    this.toolCallCounter = new Counter({
      name: 'mcp_tool_calls_total',
      help: 'Total number of tool calls',
      labelNames: ['tool', 'status'],
      registers: [this.registry]
    });

    this.toolDurationHistogram = new Histogram({
      name: 'mcp_tool_duration_seconds',
      help: 'Tool execution duration',
      labelNames: ['tool'],
      buckets: [0.001, 0.01, 0.1, 0.5, 1, 5, 10],
      registers: [this.registry]
    });

    this.errorCounter = new Counter({
      name: 'mcp_errors_total',
      help: 'Total errors',
      labelNames: ['tool', 'error_type'],
      registers: [this.registry]
    });
  }

  recordToolCall(tool: string, duration: number, success: boolean) {
    this.toolCallCounter.inc({ tool, status: success ? 'success' : 'error' });
    this.toolDurationHistogram.observe({ tool }, duration);
  }

  recordError(tool: string, errorType: string) {
    this.errorCounter.inc({ tool, error_type: errorType });
  }

  async getMetrics(): Promise<string> {
    return this.registry.metrics();
  }
}

// Add metrics endpoint
export function setupMetrics(server: FastMCP) {
  const collector = new MetricsCollector();

  server.addTool({
    name: 'metrics',
    description: 'Get Prometheus metrics',
    parameters: z.object({}),
    execute: async () => {
      return await collector.getMetrics();
    }
  });

  return collector;
}
```

### 9.2 Distributed Tracing

```typescript
// src/mcp/fastmcp/middleware/tracing.ts
import { trace, context, SpanStatusCode } from '@opentelemetry/api';
import { NodeTracerProvider } from '@opentelemetry/sdk-trace-node';
import { JaegerExporter } from '@opentelemetry/exporter-jaeger';

const tracer = trace.getTracer('fastmcp-server');

export function withTracing(tool: any) {
  return {
    ...tool,
    execute: async (args: any, ctx: any) => {
      const span = tracer.startSpan(`tool.${tool.name}`);

      try {
        span.setAttributes({
          'tool.name': tool.name,
          'tool.args': JSON.stringify(args),
          'user.id': ctx.auth?.userId
        });

        const result = await tool.execute(args, ctx);

        span.setStatus({ code: SpanStatusCode.OK });
        return result;
      } catch (error: any) {
        span.setStatus({
          code: SpanStatusCode.ERROR,
          message: error.message
        });
        span.recordException(error);
        throw error;
      } finally {
        span.end();
      }
    }
  };
}
```

---

## 10. Migration Checklist

### 10.1 Pre-Migration

- [ ] Install fastmcp package
- [ ] Review current MCP tool inventory (203+ tools)
- [ ] Identify authentication requirements per server
- [ ] Plan transport strategy (stdio vs HTTP)
- [ ] Setup development environment
- [ ] Create feature flag system
- [ ] Backup current implementation
- [ ] Document current API contracts

### 10.2 Phase 1: In-Process Server

- [ ] Create fastmcp directory structure
- [ ] Implement base server class
- [ ] Migrate memory tools (5 tools)
- [ ] Migrate swarm tools (3 tools)
- [ ] Migrate agent tools (3 tools)
- [ ] Migrate task tools (3 tools)
- [ ] Add error handling
- [ ] Add logging integration
- [ ] Write unit tests
- [ ] Write integration tests
- [ ] Performance benchmark
- [ ] Code review
- [ ] Deploy to development
- [ ] Gradual rollout with feature flag
- [ ] Monitor for issues
- [ ] Full rollout

### 10.3 Phase 2: Subprocess Servers

- [ ] Migrate claude-flow server
  - [ ] Convert all tools to FastMCP
  - [ ] Add progress reporting
  - [ ] Add streaming support
  - [ ] Optional authentication
  - [ ] Tests
- [ ] Migrate agentic-payments server
  - [ ] Convert payment tools
  - [ ] Add transaction streaming
  - [ ] Authentication
  - [ ] Tests
- [ ] Update subprocess spawning
- [ ] Integration tests
- [ ] Deploy to development
- [ ] Testing period (1 week)
- [ ] Production deployment

### 10.4 Phase 3: HTTP Server

- [ ] Implement HTTP streaming transport
- [ ] JWT authentication
- [ ] API key authentication
- [ ] OAuth integration
- [ ] Rate limiting with Redis
- [ ] Tool authorization
- [ ] CORS configuration
- [ ] Health-check endpoints
- [ ] Session management
- [ ] Load testing
- [ ] Security audit
- [ ] Staging deployment
- [ ] Production deployment

### 10.5 Phase 4: Testing & Validation

- [ ] Unit tests (100+ tests)
- [ ] Integration tests (30+ tests)
- [ ] E2E tests (10+ tests)
- [ ] Performance benchmarks
- [ ] Security tests
- [ ] Load tests
- [ ] Chaos testing
- [ ] Documentation review
- [ ] Migration guide
- [ ] Troubleshooting guide

### 10.6 Phase 5: Documentation & Rollout

- [ ] Architecture documentation
- [ ] API reference
- [ ] Authentication guide
- [ ] Security best practices
- [ ] Migration guide
- [ ] Troubleshooting guide
- [ ] Performance tuning guide
- [ ] Beta release (10%)
- [ ] Expanded rollout (50%)
- [ ] Full rollout (100%)
- [ ] Deprecate old implementation

---

## 11. Risk Assessment & Mitigation

### 11.1 Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Performance degradation | Low | High | Benchmark before migration, feature flag rollback |
| Authentication vulnerabilities | Medium | Critical | Security audit, penetration testing |
| Tool compatibility issues | Medium | High | Parallel testing, gradual rollout |
| Transport reliability | Low | High | Connection pooling, retry logic, fallback |
| Memory leaks | Low | Medium | Load testing, monitoring, profiling |
| Breaking API changes | Medium | High | Backward compatibility layer, versioning |

### 11.2 Operational Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Deployment failure | Low | High | Rollback plan, staging testing |
| Downtime during migration | Medium | High | Blue-green deployment, feature flags |
| Training needs | High | Low | Documentation, examples, workshops |
| Support burden | Medium | Medium | Comprehensive docs, troubleshooting guide |
| Third-party dependency | Low | Medium | Pin versions, vendor evaluation |

### 11.3 Mitigation Strategies

**Performance Degradation:**
- Benchmark current implementation
- Set performance SLAs (latency, throughput)
- Implement automatic rollback if SLAs breached
- Load testing before production
- Gradual rollout with monitoring

**Authentication Security:**
- Security audit by external firm
- Penetration testing
- Rate limiting to prevent brute force
- JWT expiration and rotation
- Audit logging for security events
- Regular security reviews

**Tool Compatibility:**
- Parallel testing (old vs new)
- Schema validation
- Comprehensive test suite
- Gradual migration (tool by tool)
- Fallback to old implementation

**Deployment Risks:**
- Blue-green deployment
- Feature flags for instant rollback
- Automated rollback triggers
- Staging environment testing
- Canary deployments

---

## 12. Success Metrics

### 12.1 Performance Metrics

**Target Improvements:**
- Tool invocation latency: < 50ms (p95)
- Throughput: > 1000 tools/sec
- Startup time: < 2 seconds
- Memory usage: < 500MB baseline
- CPU usage: < 50% under load

**Current Baseline:**
- Measure current performance
- Establish benchmarks
- Set improvement targets
- Monitor regression

### 12.2 Developer Experience Metrics

- Lines of code reduction: Target 30-40%
- Tool definition time: Target 50% reduction
- Bug density: Target 50% reduction
- Time to add new tool: Target 60% reduction
- Developer satisfaction score: Target 8+/10

### 12.3 Reliability Metrics

- Uptime: > 99.9%
- Error rate: < 0.1%
- Mean time to recovery: < 5 minutes
- Authentication success rate: > 99%
- Rate limit false positives: < 1%

### 12.4 Security Metrics

- Authentication failures: < 1% legitimate requests
- Rate limit effectiveness: 100% abuse prevention
- Security incidents: 0
- Audit log completeness: 100%
- Vulnerability response time: < 24 hours

---

## 13. Timeline & Resources

### 13.1 Timeline

**Total Duration: 10 weeks**

| Phase | Duration | Start | End |
|-------|----------|-------|-----|
| Phase 0: Preparation | 1 week | Week 1 | Week 1 |
| Phase 1: In-Process Server | 2 weeks | Week 2 | Week 3 |
| Phase 2: Subprocess Servers | 2 weeks | Week 4 | Week 5 |
| Phase 3: HTTP Server | 2 weeks | Week 6 | Week 7 |
| Phase 4: Testing | 1 week | Week 8 | Week 8 |
| Phase 5: Documentation & Rollout | 2 weeks | Week 9 | Week 10 |

### 13.2 Resource Requirements

**Team:**
- 1 Senior Backend Engineer (full-time)
- 1 DevOps Engineer (50%)
- 1 Security Engineer (25%)
- 1 QA Engineer (50%)
- 1 Technical Writer (25%)

**Infrastructure:**
- Development environment
- Staging environment (identical to production)
- Load testing infrastructure
- Redis for rate limiting
- Monitoring stack (Prometheus, Grafana)
- Tracing infrastructure (Jaeger)

**Tools & Services:**
- fastmcp npm package
- JWT library (jsonwebtoken)
- Redis client
- Testing framework (vitest)
- Load testing (k6)
- Security scanning tools

### 13.3 Budget Estimate

- Personnel: $40,000 (10 weeks * combined effort)
- Infrastructure: $2,000 (staging, testing)
- Tools & Services: $500
- Security audit: $5,000
- Contingency (20%): $9,500
- **Total: ~$57,000**

---

## 14. Conclusion

### 14.1 Summary

This implementation plan provides a comprehensive roadmap for migrating agentic-flow's MCP infrastructure to the fastmcp TypeScript framework. The migration will be executed in five phases over 10 weeks, with each phase building on the previous one.

**Key Benefits:**
1. **Simplified Development**: Reduced boilerplate, intuitive APIs
2. **Enhanced Security**: Built-in authentication, authorization, rate limiting
3. **Better Performance**: Optimized transport, connection pooling, caching
4. **Improved Observability**: Metrics, tracing, health checks
5. **Cloud-Ready**: HTTP streaming transport for scalable deployments
6. **Future-Proof**: Standardized framework, active community

### 14.2 Next Steps

1. **Week 1**: Get approval and allocate resources
2. **Week 1**: Setup development environment and POC
3. **Week 2-3**: Begin Phase 1 (in-process server migration)
4. **Ongoing**: Weekly status reviews and risk assessment
5. **Week 8**: Comprehensive testing and validation
6. **Week 9-10**: Documentation and staged rollout

### 14.3 Long-Term Vision

Post-migration, agentic-flow will have:
- Modern, maintainable MCP server infrastructure
- Flexible transport options (stdio, HTTP)
- Production-ready security features
- Scalable architecture for cloud deployment
- Improved developer experience
- Foundation for future enhancements

### 14.4 Recommendations

1. **Start with POC**: Validate assumptions before full commitment
2. **Gradual Rollout**: Use feature flags for safe deployment
3. **Monitor Closely**: Set up comprehensive monitoring from day 1
4. **Document Everything**: Maintain migration journal and lessons learned
5. **Engage Community**: Contribute improvements back to fastmcp
6. **Plan for Scale**: Design for 10x growth from the start

---

## 15. Appendix

### 15.1 References

- **FastMCP TypeScript**: https://github.com/punkpeye/fastmcp
- **FastMCP Documentation**: https://gofastmcp.com/
- **MCP Specification**: https://modelcontextprotocol.io/
- **MCP TypeScript SDK**: https://github.com/modelcontextprotocol/typescript-sdk
- **Zod Documentation**: https://zod.dev/

### 15.2 Glossary

- **MCP**: Model Context Protocol - standardized protocol for AI tool integration
- **FastMCP**: TypeScript framework for building MCP servers
- **Stdio**: Standard input/output transport for local communication
- **SSE**: Server-Sent Events (deprecated in favor of HTTP streaming)
- **HTTP Streaming**: Bidirectional HTTP transport with optional SSE
- **Tool**: Executable function exposed via MCP
- **Resource**: Data source exposed via MCP
- **Prompt**: Reusable template for LLM interactions
- **Transport**: Communication mechanism between client and server
- **Authentication**: Verifying user identity
- **Authorization**: Controlling access to tools
- **Rate Limiting**: Preventing abuse through request throttling

### 15.3 Contact Information

- **Project Lead**: [Name] <email>
- **Tech Lead**: [Name] <email>
- **Security Lead**: [Name] <email>
- **DevOps Lead**: [Name] <email>

### 15.4 Change Log

| Version | Date | Changes | Author |
|---------|------|---------|--------|
| 1.0 | 2025-10-03 | Initial plan | Claude |
| | | | |

---

**Document Status**: Draft for Review
**Last Updated**: 2025-10-03
**Next Review**: After Phase 0 completion
