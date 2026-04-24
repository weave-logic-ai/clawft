# 🔧 MCP Tools: Complete Reference Guide

**213 tools across 4 servers • Universal AI agent capabilities**

---

## 📑 Quick Navigation

[← Back to Main README](https://github.com/ruvnet/agentic-flow/blob/main/README.md) | [Multi-Model Router ←](https://github.com/ruvnet/agentic-flow/blob/main/docs/guides/MULTI-MODEL-ROUTER.md) | [Deployment Options →](https://github.com/ruvnet/agentic-flow/blob/main/docs/guides/DEPLOYMENT.md)

---

## 🎯 What are MCP Tools?

MCP (Model Context Protocol) tools are standardized AI agent capabilities that enable agents to interact with external systems, services, and data sources. Think of them as "skills" or "abilities" that agents can use to perform tasks beyond text generation.

### The Problem

Traditional AI agents are limited to:
- **Text-only responses**: Can't interact with real systems
- **No memory**: Forget everything between sessions
- **No external data**: Limited to training data
- **No automation**: Can't execute actions
- **Isolated**: Can't coordinate with other agents

**Example**: An agent reviewing code can't actually run tests, check GitHub PRs, or update documentation - it can only suggest these actions.

### The Solution

MCP tools provide standardized interfaces for:
- **System Integration**: File I/O, Git operations, API calls
- **Data Access**: Databases, search engines, web scraping
- **Agent Coordination**: Swarm management, task orchestration
- **Memory & Learning**: Persistent storage, pattern recognition
- **Automation**: Workflow execution, event handling

**Results**:
- Agents can execute real actions, not just suggest them
- Cross-session memory enables learning and improvement
- Multi-agent coordination for complex tasks
- Integration with 1000+ external services

---

## 🚀 MCP Server Overview

### Available Servers

| Server | Tools | Focus Area | Installation |
|--------|-------|------------|--------------|
| **Claude Flow** | 101 tools | Agent orchestration, memory, neural | `claude mcp add claude-flow npx claude-flow@alpha mcp start` |
| **Flow Nexus** | 96 tools | Cloud execution, sandboxes, payments | `claude mcp add flow-nexus npx flow-nexus@latest mcp start` |
| **Agentic Payments** | 12 tools | Payment authorization, mandates | Built-in with agentic-flow |
| **Claude Flow SDK** | 4 tools | Low-level SDK integration | Built-in with agentic-flow |

**Total**: 213 MCP tools available

---

## 📚 Tool Categories

### 1️⃣ Swarm & Agent Orchestration (20 tools)

Coordinate multiple agents for complex tasks.

#### **Core Swarm Tools**

```javascript
// Initialize swarm with specific topology
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'swarm_init',
    params: {
      topology: 'mesh',      // mesh, hierarchical, ring, star
      maxAgents: 8,
      strategy: 'balanced'   // balanced, specialized, adaptive
    }
  }
});

// Spawn specialized agent
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'agent_spawn',
    params: {
      type: 'coder',         // researcher, analyst, optimizer, coordinator
      name: 'backend-dev',
      capabilities: ['api-design', 'database', 'testing']
    }
  }
});

// Orchestrate complex task
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'task_orchestrate',
    params: {
      task: 'Build REST API with authentication',
      strategy: 'adaptive',  // parallel, sequential, adaptive
      priority: 'high',
      maxAgents: 5
    }
  }
});
```

#### **Monitoring & Status**

```javascript
// Get swarm status
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'swarm_status',
    params: { swarmId: 'swarm-123' }
  }
});

// List active agents
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'agent_list',
    params: { filter: 'active' }  // all, active, idle, busy
  }
});

// Get agent performance metrics
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'agent_metrics',
    params: {
      agentId: 'agent-456',
      metric: 'performance'  // cpu, memory, tasks, performance
    }
  }
});
```

#### **Advanced Swarm Operations**

```javascript
// Scale swarm up/down
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'swarm_scale',
    params: {
      swarmId: 'swarm-123',
      targetSize: 15
    }
  }
});

// Optimize topology dynamically
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'topology_optimize',
    params: { swarmId: 'swarm-123' }
  }
});

// Destroy swarm and cleanup
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'swarm_destroy',
    params: { swarmId: 'swarm-123' }
  }
});
```

---

### 2️⃣ Memory & Learning (18 tools)

Persistent memory across sessions with learning capabilities.

#### **Memory Storage & Retrieval**

```javascript
// Store memory with TTL and namespace
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'memory_usage',
    params: {
      action: 'store',
      key: 'api-design-pattern',
      value: JSON.stringify({
        pattern: 'REST with JWT auth',
        successRate: 0.95,
        usageCount: 47
      }),
      namespace: 'backend-patterns',
      ttl: 2592000  // 30 days in seconds
    }
  }
});

// Retrieve memory
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'memory_usage',
    params: {
      action: 'retrieve',
      key: 'api-design-pattern',
      namespace: 'backend-patterns'
    }
  }
});

// Search memories with pattern
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'memory_search',
    params: {
      pattern: 'authentication',
      namespace: 'backend-patterns',
      limit: 10
    }
  }
});
```

#### **Memory Management**

```javascript
// List all memories in namespace
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'memory_usage',
    params: {
      action: 'list',
      namespace: 'backend-patterns'
    }
  }
});

// Delete memory
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'memory_usage',
    params: {
      action: 'delete',
      key: 'old-pattern',
      namespace: 'backend-patterns'
    }
  }
});

// Create memory backup
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'memory_backup',
    params: { path: '/backups/memory-20241012.db' }
  }
});

// Restore from backup
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'memory_restore',
    params: { backupPath: '/backups/memory-20241012.db' }
  }
});
```

---

### 3️⃣ Neural Networks & AI (15 tools)

Train and deploy neural networks with WASM acceleration.

#### **Neural Training**

```javascript
// Train neural patterns
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'neural_train',
    params: {
      pattern_type: 'coordination',  // optimization, prediction
      training_data: JSON.stringify({
        inputs: [...],
        outputs: [...]
      }),
      epochs: 50
    }
  }
});

// Check training status
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'neural_status',
    params: { modelId: 'model-123' }
  }
});
```

#### **Neural Inference**

```javascript
// Run inference
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'neural_predict',
    params: {
      modelId: 'model-123',
      input: JSON.stringify({ features: [...] })
    }
  }
});

// Analyze cognitive patterns
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'neural_patterns',
    params: {
      action: 'analyze',  // learn, predict
      operation: 'code-review',
      outcome: 'success'
    }
  }
});
```

#### **Model Management**

```javascript
// Load pre-trained model
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'model_load',
    params: { modelPath: '/models/pretrained-v1.onnx' }
  }
});

// Save trained model
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'model_save',
    params: {
      modelId: 'model-123',
      path: '/models/custom-v1.onnx'
    }
  }
});

// Compress model for faster inference
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'neural_compress',
    params: {
      modelId: 'model-123',
      ratio: 0.5  // 50% compression
    }
  }
});
```

---

### 4️⃣ Cloud Execution & Sandboxes (24 tools)

Execute code in isolated cloud environments.

#### **Sandbox Management (Flow Nexus)**

```javascript
// Create execution sandbox
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'sandbox_create',
    params: {
      template: 'node',  // python, react, nextjs, claude-code
      name: 'api-dev',
      env_vars: {
        DATABASE_URL: 'postgresql://...',
        API_KEY: 'sk-...'
      },
      timeout: 3600  // 1 hour
    }
  }
});

// Execute code in sandbox
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'sandbox_execute',
    params: {
      sandbox_id: 'sandbox-789',
      code: 'console.log("Hello from sandbox!")',
      language: 'javascript',
      timeout: 60
    }
  }
});

// Upload file to sandbox
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'sandbox_upload',
    params: {
      sandbox_id: 'sandbox-789',
      file_path: '/app/config.json',
      content: JSON.stringify({ port: 3000 })
    }
  }
});
```

#### **Sandbox Operations**

```javascript
// Get sandbox status
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'sandbox_status',
    params: { sandbox_id: 'sandbox-789' }
  }
});

// List all sandboxes
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'sandbox_list',
    params: { status: 'running' }  // running, stopped, all
  }
});

// Configure sandbox environment
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'sandbox_configure',
    params: {
      sandbox_id: 'sandbox-789',
      env_vars: { NEW_VAR: 'value' },
      install_packages: ['express', 'axios']
    }
  }
});

// Stop sandbox
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'sandbox_stop',
    params: { sandbox_id: 'sandbox-789' }
  }
});

// Delete sandbox
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'sandbox_delete',
    params: { sandbox_id: 'sandbox-789' }
  }
});
```

---

### 5️⃣ GitHub Integration (16 tools)

Comprehensive GitHub workflow automation.

#### **Repository Management**

```javascript
// Analyze repository
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'github_repo_analyze',
    params: {
      repo: 'ruvnet/agentic-flow',
      analysis_type: 'code_quality'  // performance, security
    }
  }
});

// Manage pull requests
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'github_pr_manage',
    params: {
      repo: 'ruvnet/agentic-flow',
      pr_number: 123,
      action: 'review'  // merge, close
    }
  }
});

// Code review with AI
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'github_code_review',
    params: {
      repo: 'ruvnet/agentic-flow',
      pr: 123
    }
  }
});
```

#### **Issue & Project Management**

```javascript
// Track issues intelligently
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'github_issue_track',
    params: {
      repo: 'ruvnet/agentic-flow',
      action: 'triage'  // assign, label, close
    }
  }
});

// Coordinate releases
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'github_release_coord',
    params: {
      repo: 'ruvnet/agentic-flow',
      version: 'v1.6.0'
    }
  }
});

// Automate workflows
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'github_workflow_auto',
    params: {
      repo: 'ruvnet/agentic-flow',
      workflow: {
        trigger: 'push',
        actions: ['test', 'build', 'deploy']
      }
    }
  }
});
```

---

### 6️⃣ Payment Authorization (12 tools)

AI-native payment authorization with Active Mandates.

#### **Mandate Management (Agentic Payments)**

```javascript
// Create payment mandate
await query({
  mcp: {
    server: 'agentic-payments',
    tool: 'create_active_mandate',
    params: {
      agent: 'shopping-bot@agentics',
      holder: 'user-123',
      amount: 12000,  // $120.00 in cents
      currency: 'USD',
      period: 'monthly',
      kind: 'intent',
      merchant_allow: ['amazon.com', 'ebay.com'],
      expires_at: '2025-01-12T00:00:00Z'
    }
  }
});

// Sign mandate with Ed25519
await query({
  mcp: {
    server: 'agentic-payments',
    tool: 'sign_mandate',
    params: {
      mandate: { ... },
      private_key: 'ed25519-private-key-hex'
    }
  }
});

// Verify mandate signature
await query({
  mcp: {
    server: 'agentic-payments',
    tool: 'verify_mandate',
    params: {
      signed_mandate: { ... },
      check_guards: true  // Check expiration, revocation
    }
  }
});
```

#### **Payment Operations**

```javascript
// Create intent-based mandate
await query({
  mcp: {
    server: 'agentic-payments',
    tool: 'create_intent_mandate',
    params: {
      merchant_id: 'shop-456',
      customer_id: 'user-123',
      intent: 'Purchase office supplies under $100',
      max_amount: 100.00,
      currency: 'USD'
    }
  }
});

// Create cart-based mandate
await query({
  mcp: {
    server: 'agentic-payments',
    tool: 'create_cart_mandate',
    params: {
      merchant_id: 'shop-456',
      customer_id: 'user-123',
      items: [
        { id: 'item-1', name: 'Laptop', quantity: 1, unit_price: 120000 },
        { id: 'item-2', name: 'Mouse', quantity: 2, unit_price: 2500 }
      ],
      currency: 'USD'
    }
  }
});

// Revoke mandate
await query({
  mcp: {
    server: 'agentic-payments',
    tool: 'revoke_mandate',
    params: {
      mandate_id: 'mandate-789',
      reason: 'User cancelled subscription'
    }
  }
});
```

---

### 7️⃣ Workflow Automation (22 tools)

Event-driven workflow execution with message queues.

#### **Workflow Creation & Execution**

```javascript
// Create workflow
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'workflow_create',
    params: {
      name: 'code-review-pipeline',
      steps: [
        { type: 'checkout', params: { branch: 'main' } },
        { type: 'test', params: { coverage: 80 } },
        { type: 'review', params: { reviewers: 2 } },
        { type: 'merge', params: { strategy: 'squash' } }
      ],
      triggers: ['pull_request.opened', 'push.main']
    }
  }
});

// Execute workflow
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'workflow_execute',
    params: {
      workflow_id: 'workflow-456',
      input_data: { pr_number: 123 },
      async: true  // Run via message queue
    }
  }
});

// Get workflow status
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'workflow_status',
    params: {
      workflow_id: 'workflow-456',
      include_metrics: true
    }
  }
});
```

#### **Advanced Workflow Features**

```javascript
// Assign optimal agent to task
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'workflow_agent_assign',
    params: {
      task_id: 'task-789',
      agent_type: 'coder',
      use_vector_similarity: true
    }
  }
});

// Check message queue status
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'workflow_queue_status',
    params: {
      queue_name: 'code-review-queue',
      include_messages: true
    }
  }
});

// Get workflow audit trail
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'workflow_audit_trail',
    params: {
      workflow_id: 'workflow-456',
      start_time: '2024-10-01T00:00:00Z',
      limit: 50
    }
  }
});
```

---

### 8️⃣ Performance & Monitoring (18 tools)

Real-time metrics, benchmarks, and optimization.

#### **Performance Tracking**

```javascript
// Generate performance report
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'performance_report',
    params: {
      timeframe: '24h',  // 7d, 30d
      format: 'detailed'  // summary, json
    }
  }
});

// Analyze bottlenecks
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'bottleneck_analyze',
    params: {
      component: 'swarm-coordinator',
      metrics: ['latency', 'throughput', 'error_rate']
    }
  }
});

// Track token usage
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'token_usage',
    params: {
      operation: 'code-review',
      timeframe: '24h'
    }
  }
});
```

#### **Benchmarking**

```javascript
// Run performance benchmarks
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'benchmark_run',
    params: {
      suite: 'swarm',  // wasm, agent, task
      iterations: 10
    }
  }
});

// Collect system metrics
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'metrics_collect',
    params: {
      components: ['gateway', 'agents', 'memory']
    }
  }
});

// Analyze performance trends
await query({
  mcp: {
    server: 'claude-flow',
    tool: 'trend_analysis',
    params: {
      metric: 'response_time',
      period: '7d'
    }
  }
});
```

---

### 9️⃣ App Store & Templates (12 tools)

Deploy pre-built applications and templates.

#### **Template Management**

```javascript
// List available templates
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'template_list',
    params: {
      category: 'web-apps',
      featured: true,
      limit: 20
    }
  }
});

// Get template details
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'template_get',
    params: {
      template_id: 'template-123'
    }
  }
});

// Deploy template
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'template_deploy',
    params: {
      template_id: 'template-123',
      deployment_name: 'my-app',
      variables: {
        anthropic_api_key: 'sk-ant-...',
        port: 3000
      }
    }
  }
});
```

#### **App Publishing**

```javascript
// Publish app to store
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'app_store_publish_app',
    params: {
      name: 'Code Review Bot',
      description: 'Automated code review with AI',
      category: 'development',
      source_code: '...',
      tags: ['code-review', 'ai', 'automation'],
      version: '1.0.0'
    }
  }
});

// List user's apps
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'app_installed',
    params: { user_id: 'user-123' }
  }
});

// Get app analytics
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'app_analytics',
    params: {
      app_id: 'app-456',
      timeframe: '30d'
    }
  }
});
```

---

### 🔟 User Management & Auth (16 tools)

Authentication, authorization, and user operations.

#### **Authentication (Flow Nexus)**

```javascript
// Register new user
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'user_register',
    params: {
      email: 'user@example.com',
      password: 'secure-password',
      full_name: 'John Doe'
    }
  }
});

// Login user
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'user_login',
    params: {
      email: 'user@example.com',
      password: 'secure-password'
    }
  }
});

// Logout user
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'user_logout',
    params: {}
  }
});
```

#### **User Profile Management**

```javascript
// Get user profile
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'user_profile',
    params: { user_id: 'user-123' }
  }
});

// Update profile
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'user_update_profile',
    params: {
      user_id: 'user-123',
      updates: {
        full_name: 'John Smith',
        avatar_url: 'https://...'
      }
    }
  }
});

// Get user statistics
await query({
  mcp: {
    server: 'flow-nexus',
    tool: 'user_stats',
    params: { user_id: 'user-123' }
  }
});
```

---

## 🛠️ Tool Usage Patterns

### Pattern 1: Sequential Task Execution

```javascript
// Step 1: Initialize swarm
const swarm = await query({ mcp: { server: 'claude-flow', tool: 'swarm_init', params: { topology: 'mesh' } }});

// Step 2: Spawn agents
await query({ mcp: { server: 'claude-flow', tool: 'agent_spawn', params: { type: 'coder' } }});
await query({ mcp: { server: 'claude-flow', tool: 'agent_spawn', params: { type: 'tester' } }});

// Step 3: Execute task
const result = await query({ mcp: { server: 'claude-flow', tool: 'task_orchestrate', params: { task: '...' } }});

// Step 4: Clean up
await query({ mcp: { server: 'claude-flow', tool: 'swarm_destroy', params: { swarmId: swarm.id } }});
```

### Pattern 2: Memory-Augmented Execution

```javascript
// Retrieve relevant memories
const memories = await query({
  mcp: { server: 'claude-flow', tool: 'memory_search', params: { pattern: 'api-design' } }
});

// Execute task with memory context
const result = await executeTask({ context: memories });

// Store new experience
await query({
  mcp: { server: 'claude-flow', tool: 'memory_usage', params: {
    action: 'store',
    key: 'new-pattern',
    value: JSON.stringify(result)
  }}
});
```

### Pattern 3: Cloud-Native Development

```javascript
// Create sandbox
const sandbox = await query({
  mcp: { server: 'flow-nexus', tool: 'sandbox_create', params: { template: 'node' } }
});

// Execute code
await query({
  mcp: { server: 'flow-nexus', tool: 'sandbox_execute', params: {
    sandbox_id: sandbox.id,
    code: 'npm install && npm test'
  }}
});

// Get results
const logs = await query({
  mcp: { server: 'flow-nexus', tool: 'sandbox_logs', params: { sandbox_id: sandbox.id } }
});

// Cleanup
await query({
  mcp: { server: 'flow-nexus', tool: 'sandbox_delete', params: { sandbox_id: sandbox.id } }
});
```

---

## 📊 Tool Performance Benchmarks

### Latency Comparison

| Tool Category | Average Latency | P99 Latency | Throughput |
|---------------|----------------|-------------|------------|
| **Memory Operations** | 5ms | 12ms | 10K ops/sec |
| **Swarm Management** | 50ms | 120ms | 500 ops/sec |
| **Neural Inference** | 15ms | 35ms | 2K ops/sec |
| **Sandbox Creation** | 2s | 5s | 50 ops/min |
| **GitHub Integration** | 200ms | 800ms | 100 ops/sec |

### Cost Optimization

| Operation | Without MCP | With MCP | Savings |
|-----------|------------|----------|---------|
| **Code Review** | $0.15 (LLM) | $0.02 (cached) | 87% |
| **Memory Retrieval** | $0.05 (LLM) | $0.00 (local) | 100% |
| **Task Orchestration** | $0.30 (manual) | $0.05 (auto) | 83% |

---

## 🔗 Related Documentation

### Core Components
- [← Back to Main README](https://github.com/ruvnet/agentic-flow/blob/main/README.md)
- [Agent Booster (Code Transformations)](https://github.com/ruvnet/agentic-flow/blob/main/docs/guides/AGENT-BOOSTER.md)
- [ReasoningBank (Learning Memory)](https://github.com/ruvnet/agentic-flow/blob/main/docs/guides/REASONINGBANK.md)
- [Multi-Model Router (Cost Optimization)](https://github.com/ruvnet/agentic-flow/blob/main/docs/guides/MULTI-MODEL-ROUTER.md)

### Advanced Topics
- [Deployment Options →](https://github.com/ruvnet/agentic-flow/blob/main/docs/guides/DEPLOYMENT.md)
- [Performance Benchmarks](https://github.com/ruvnet/agentic-flow/blob/main/docs/agentic-flow/benchmarks/README.md)
- [API Reference](https://docs.rs/agentic-flow)

### External Resources
- [MCP Specification](https://modelcontextprotocol.io)
- [Claude Agent SDK](https://docs.claude.com/en/api/agent-sdk)
- [Flow Nexus Platform](https://flow-nexus.ruv.io)

---

## 🤝 Contributing

MCP tools are part of the agentic-flow ecosystem. Contributions welcome!

**Areas for Contribution:**
- Additional tool implementations
- Performance optimizations
- New MCP server integrations
- Documentation improvements
- Usage examples

See [CONTRIBUTING.md](https://github.com/ruvnet/agentic-flow/blob/main/CONTRIBUTING.md) for guidelines.

---

## 📄 License

MIT License - see [LICENSE](https://github.com/ruvnet/agentic-flow/blob/main/LICENSE) for details.

---

**Access 213 AI agent capabilities across 4 MCP servers. Universal integration.** 🔧

[← Back to Main README](https://github.com/ruvnet/agentic-flow/blob/main/README.md)
