# Agent Booster: Integration Guide

## 🔌 Agentic-Flow Integration

### Overview

Agent Booster integrates seamlessly with agentic-flow as an **optional performance accelerator** for code editing operations. When enabled, it replaces slow LLM-based code application with ultra-fast vector-based semantic merging.

### Integration Strategy

```
┌─────────────────────────────────────────────────────────────┐
│                    Agentic-Flow Agent                        │
│                                                               │
│  Agent receives task: "Add error handling to parseConfig"    │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│              Tool: edit_file (Enhanced)                      │
│                                                               │
│  Check: AGENT_BOOSTER_ENABLED?                              │
└────────┬───────────────────────────────────┬────────────────┘
         │ YES                                │ NO
         ▼                                    ▼
┌────────────────────────┐    ┌──────────────────────────────┐
│  Agent Booster         │    │  Original Behavior           │
│  (30ms, $0)           │    │  (LLM rewrite or API)        │
│                        │    │  (6000ms, $0.01)             │
│  ├─ Parse AST          │    │                              │
│  ├─ Vector search      │    │  ├─ Full file rewrite        │
│  ├─ Smart merge        │    │  │  or                       │
│  └─ Validate           │    │  └─ Morph LLM API call       │
│                        │    │                              │
│  Confidence >= 65%?    │    │                              │
│  ├─ YES: Return result │    │                              │
│  └─ NO: Fallback to → │────┘                              │
└────────────────────────┘                                    │
         │                                                     │
         └─────────────────────────────────────────────────────┘
                                 │
                                 ▼
                        ┌────────────────┐
                        │  Final Result  │
                        └────────────────┘
```

### Environment Variables

```bash
# .env

# ═══════════════════════════════════════════════════════════
# Agent Booster Configuration
# ═══════════════════════════════════════════════════════════

# Enable Agent Booster for code edits (default: false)
AGENT_BOOSTER_ENABLED=true

# Embedding model to use
# Options: jina-code-v2 (best), all-MiniLM-L6-v2 (fast)
# Default: jina-code-v2
AGENT_BOOSTER_MODEL=jina-code-v2

# Confidence threshold (0.0 - 1.0)
# If confidence < threshold, fallback to LLM
# Default: 0.65
AGENT_BOOSTER_CONFIDENCE_THRESHOLD=0.65

# Enable fallback to Morph LLM when confidence is low
# Default: true
AGENT_BOOSTER_FALLBACK_TO_MORPH=true

# Morph API key for fallback (optional)
MORPH_API_KEY=sk-morph-xxx

# Model cache directory (optional)
# Default: ~/.cache/agent-booster
AGENT_BOOSTER_CACHE_DIR=/path/to/cache

# Enable debug logging (default: false)
AGENT_BOOSTER_DEBUG=false

# Maximum number of chunks to extract per file (default: 100)
AGENT_BOOSTER_MAX_CHUNKS=100

# Enable embedding caching (default: true)
AGENT_BOOSTER_CACHE_EMBEDDINGS=true
```

### Tool Implementation

#### Enhanced `edit_file` Tool

```typescript
// src/tools/edit-file.ts

import { AgentBooster } from 'agent-booster';
import { readFileSync, writeFileSync } from 'fs';

let booster: AgentBooster | null = null;

// Initialize on first use
function getBooster(): AgentBooster | null {
  if (process.env.AGENT_BOOSTER_ENABLED !== 'true') {
    return null;
  }

  if (!booster) {
    booster = new AgentBooster({
      model: process.env.AGENT_BOOSTER_MODEL || 'jina-code-v2',
      confidenceThreshold: parseFloat(
        process.env.AGENT_BOOSTER_CONFIDENCE_THRESHOLD || '0.65'
      ),
      fallbackToMorph: process.env.AGENT_BOOSTER_FALLBACK_TO_MORPH === 'true',
      morphApiKey: process.env.MORPH_API_KEY,
      debug: process.env.AGENT_BOOSTER_DEBUG === 'true',
    });
  }

  return booster;
}

export const editFileTool = {
  name: 'edit_file',
  description: 'Apply semantic code edits to files',
  parameters: {
    type: 'object',
    properties: {
      file_path: {
        type: 'string',
        description: 'Path to the file to edit',
      },
      edit_description: {
        type: 'string',
        description: 'Description of the edit to apply',
      },
      language: {
        type: 'string',
        enum: ['javascript', 'typescript', 'python', 'rust'],
        description: 'Programming language',
      },
    },
    required: ['file_path', 'edit_description'],
  },

  async execute(params: {
    file_path: string;
    edit_description: string;
    language?: string;
  }) {
    const { file_path, edit_description, language } = params;

    // Read original file
    const originalCode = readFileSync(file_path, 'utf-8');

    // Try Agent Booster first if enabled
    const booster = getBooster();
    if (booster) {
      try {
        const result = await booster.applyEdit({
          originalCode,
          editSnippet: edit_description,
          language: language || detectLanguage(file_path),
        });

        // Check confidence
        if (result.confidence >= booster.config.confidenceThreshold) {
          // High confidence - use Agent Booster result
          writeFileSync(file_path, result.mergedCode, 'utf-8');

          return {
            success: true,
            method: 'agent-booster',
            confidence: result.confidence,
            strategy: result.strategy,
            latency_ms: result.metadata.latency_ms,
            cost: 0,
            message: `Applied edit using Agent Booster (${result.strategy}, confidence: ${(result.confidence * 100).toFixed(1)}%)`,
          };
        } else {
          // Low confidence - fallback
          console.log(
            `Agent Booster confidence too low (${(result.confidence * 100).toFixed(1)}%), falling back to LLM`
          );
        }
      } catch (error) {
        console.error('Agent Booster failed, falling back to LLM:', error);
      }
    }

    // Fallback to original behavior (LLM-based)
    return fallbackToLLM(originalCode, edit_description, file_path);
  },
};

// Fallback to LLM-based editing
async function fallbackToLLM(
  originalCode: string,
  editDescription: string,
  filePath: string
) {
  // Option 1: Use Morph LLM API
  if (process.env.MORPH_API_KEY) {
    const startTime = Date.now();
    const response = await fetch('https://api.morphllm.com/v1/chat/completions', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${process.env.MORPH_API_KEY}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model: 'morph-v3-large',
        messages: [{
          role: 'user',
          content: `<instruction>${editDescription}</instruction>\n<code>${originalCode}</code>\n<update>Apply the edit</update>`,
        }],
      }),
    });

    const data = await response.json();
    const mergedCode = data.choices[0].message.content;
    const latency = Date.now() - startTime;

    writeFileSync(filePath, mergedCode, 'utf-8');

    return {
      success: true,
      method: 'morph-llm',
      latency_ms: latency,
      cost: estimateCost(originalCode, mergedCode),
      message: `Applied edit using Morph LLM API`,
    };
  }

  // Option 2: Use main LLM to rewrite file
  // (existing implementation)
  return existingEditFileImplementation(originalCode, editDescription, filePath);
}

function detectLanguage(filePath: string): string {
  const ext = filePath.split('.').pop();
  const langMap: Record<string, string> = {
    'js': 'javascript',
    'jsx': 'javascript',
    'ts': 'typescript',
    'tsx': 'typescript',
    'py': 'python',
    'rs': 'rust',
    'go': 'go',
    'java': 'java',
  };
  return langMap[ext || ''] || 'javascript';
}

function estimateCost(original: string, merged: string): number {
  const tokens = (original.length + merged.length) / 4;
  return (tokens / 1000) * 0.003; // Rough estimate
}
```

### CLI Integration

```typescript
// src/cli/commands/edit.ts

import { Command } from 'commander';
import { AgentBooster } from 'agent-booster';
import { readFileSync, writeFileSync } from 'fs';

export const editCommand = new Command('edit')
  .description('Apply code edits using Agent Booster')
  .argument('<file>', 'File to edit')
  .argument('<description>', 'Edit description')
  .option('--model <model>', 'Embedding model', 'jina-code-v2')
  .option('--threshold <threshold>', 'Confidence threshold', '0.65')
  .option('--fallback', 'Fallback to Morph LLM if confidence low', false)
  .option('--dry-run', 'Show result without writing', false)
  .action(async (file: string, description: string, options) => {
    console.log(`📝 Editing ${file}...`);
    console.log(`📋 Edit: ${description}\n`);

    const booster = new AgentBooster({
      model: options.model,
      confidenceThreshold: parseFloat(options.threshold),
      fallbackToMorph: options.fallback,
    });

    const originalCode = readFileSync(file, 'utf-8');
    const startTime = Date.now();

    try {
      const result = await booster.applyEdit({
        originalCode,
        editSnippet: description,
        language: detectLanguage(file),
      });

      const latency = Date.now() - startTime;

      // Display results
      console.log(`✅ Success!`);
      console.log(`   Strategy: ${result.strategy}`);
      console.log(`   Confidence: ${(result.confidence * 100).toFixed(1)}%`);
      console.log(`   Latency: ${latency}ms`);
      console.log(`   Cost: $0.00\n`);

      if (result.confidence < parseFloat(options.threshold)) {
        console.log(`⚠️  Warning: Low confidence (${(result.confidence * 100).toFixed(1)}%)`);
        console.log(`   Consider using --fallback for better accuracy\n`);
      }

      if (options.dryRun) {
        console.log('Dry run - no changes written\n');
        console.log('Preview:');
        console.log('─'.repeat(80));
        console.log(result.mergedCode);
        console.log('─'.repeat(80));
      } else {
        writeFileSync(file, result.mergedCode, 'utf-8');
        console.log(`💾 Saved to ${file}`);
      }
    } catch (error) {
      console.error(`❌ Error: ${error.message}`);
      process.exit(1);
    }
  });
```

### Configuration Presets

```typescript
// src/config/agent-booster-presets.ts

export const presets = {
  // Maximum speed, acceptable accuracy
  fast: {
    model: 'all-MiniLM-L6-v2',
    confidenceThreshold: 0.60,
    fallbackToMorph: false,
    cacheEmbeddings: true,
  },

  // Balanced speed and accuracy (recommended)
  balanced: {
    model: 'jina-code-v2',
    confidenceThreshold: 0.65,
    fallbackToMorph: true,
    cacheEmbeddings: true,
  },

  // Maximum accuracy
  accurate: {
    model: 'jina-code-v2',
    confidenceThreshold: 0.75,
    fallbackToMorph: true,
    cacheEmbeddings: false,  // Always fresh
  },

  // Offline mode (no fallback)
  offline: {
    model: 'jina-code-v2',
    confidenceThreshold: 0.50,  // More lenient
    fallbackToMorph: false,
    cacheEmbeddings: true,
  },
};

// Usage in .env
// AGENT_BOOSTER_PRESET=balanced
```

### Metrics & Monitoring

```typescript
// src/utils/agent-booster-metrics.ts

interface EditMetrics {
  method: 'agent-booster' | 'morph-llm' | 'llm-rewrite';
  latency_ms: number;
  confidence?: number;
  strategy?: string;
  cost: number;
  success: boolean;
  timestamp: Date;
}

class MetricsCollector {
  private metrics: EditMetrics[] = [];

  record(metric: EditMetrics) {
    this.metrics.push(metric);

    // Optionally log to file
    if (process.env.AGENT_BOOSTER_METRICS_FILE) {
      appendFileSync(
        process.env.AGENT_BOOSTER_METRICS_FILE,
        JSON.stringify(metric) + '\n'
      );
    }
  }

  getSummary() {
    const total = this.metrics.length;
    const boosterEdits = this.metrics.filter(m => m.method === 'agent-booster');
    const morphEdits = this.metrics.filter(m => m.method === 'morph-llm');

    return {
      total_edits: total,
      agent_booster_usage: `${((boosterEdits.length / total) * 100).toFixed(1)}%`,
      avg_latency_booster: average(boosterEdits.map(m => m.latency_ms)),
      avg_latency_morph: average(morphEdits.map(m => m.latency_ms)),
      total_cost_saved: morphEdits.length * 0.01,  // Estimated
      avg_confidence: average(boosterEdits.map(m => m.confidence || 0)),
    };
  }
}

export const metricsCollector = new MetricsCollector();
```

### Usage Example

```typescript
// examples/agentic-flow-with-agent-booster.ts

import { AgenticFlow } from 'agentic-flow';
import { editFileTool } from './tools/edit-file';

// Configure via .env
// AGENT_BOOSTER_ENABLED=true
// AGENT_BOOSTER_MODEL=jina-code-v2
// AGENT_BOOSTER_CONFIDENCE_THRESHOLD=0.65
// AGENT_BOOSTER_FALLBACK_TO_MORPH=true

const agent = new AgenticFlow({
  model: 'claude-sonnet-4',
  tools: [editFileTool],
  systemPrompt: `You are a code refactoring assistant.
Use the edit_file tool to apply code changes.`,
});

// Agent will automatically use Agent Booster for fast edits
const result = await agent.run({
  task: 'Add error handling to all functions in src/utils/parser.ts',
});

console.log(result);

// Check metrics
import { metricsCollector } from './utils/agent-booster-metrics';
console.log('\nPerformance Summary:');
console.log(metricsCollector.getSummary());
```

---

## 🖥️ Model Context Protocol (MCP) Server

### Overview

Agent Booster provides a full-featured MCP server that enables seamless integration with MCP clients like Claude Desktop, Cursor, VS Code, and other AI assistants with workspace detection.

### MCP Server Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     MCP Client                               │
│        (Claude Desktop / Cursor / VS Code)                   │
└────────────────────┬────────────────────────────────────────┘
                     │ MCP Protocol (stdio/HTTP)
                     ▼
┌─────────────────────────────────────────────────────────────┐
│               Agent Booster MCP Server                       │
│                                                               │
│  Tools Exposed:                                              │
│  ├─ agent_booster_apply          (Apply single edit)        │
│  ├─ agent_booster_batch          (Batch edits)              │
│  ├─ agent_booster_analyze        (Analyze workspace)        │
│  └─ agent_booster_status         (Server status)            │
│                                                               │
│  Resources:                                                  │
│  └─ agent_booster://metrics      (Usage metrics)            │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│               Agent Booster Core                             │
│          (Rust library via native addon)                     │
└─────────────────────────────────────────────────────────────┘
```

### Starting the MCP Server

```bash
# Start stdio server (for Claude Desktop, etc.)
npx agent-booster mcp

# Start HTTP server
npx agent-booster mcp --http --port 3000

# With custom config
npx agent-booster mcp --config ./agent-booster.json
```

### MCP Client Configuration

#### Claude Desktop

```json
// ~/Library/Application Support/Claude/claude_desktop_config.json
{
  "mcpServers": {
    "agent-booster": {
      "command": "npx",
      "args": ["agent-booster", "mcp"],
      "env": {
        "AGENT_BOOSTER_MODEL": "jina-code-v2",
        "AGENT_BOOSTER_CONFIDENCE_THRESHOLD": "0.65"
      }
    }
  }
}
```

#### Cursor

```json
// .cursor/mcp.json
{
  "mcpServers": {
    "agent-booster": {
      "command": "npx",
      "args": ["agent-booster", "mcp"],
      "cwd": "${workspaceFolder}"
    }
  }
}
```

#### VS Code (via MCP Extension)

```json
// .vscode/mcp.json
{
  "servers": {
    "agent-booster": {
      "command": "npx agent-booster mcp",
      "workspaceDetection": true
    }
  }
}
```

### MCP Tools

#### 1. `agent_booster_apply`

Apply a single code edit to a file.

```json
{
  "name": "agent_booster_apply",
  "description": "Apply semantic code edit using vector similarity (200x faster than LLM)",
  "inputSchema": {
    "type": "object",
    "properties": {
      "file_path": {
        "type": "string",
        "description": "Relative path to file from workspace root"
      },
      "edit_description": {
        "type": "string",
        "description": "Description of the edit to apply"
      },
      "language": {
        "type": "string",
        "enum": ["javascript", "typescript", "python", "rust", "go"],
        "description": "Programming language (auto-detected if omitted)"
      }
    },
    "required": ["file_path", "edit_description"]
  }
}
```

**Example Usage (Claude Desktop):**

```
User: Add error handling to the parseConfig function in src/utils/config.ts

Claude: I'll use agent_booster_apply to add error handling.

[Calls agent_booster_apply with:
 file_path: "src/utils/config.ts"
 edit_description: "add try-catch error handling to parseConfig function"]

Response: {
  "success": true,
  "confidence": 0.92,
  "strategy": "exact_replace",
  "latency_ms": 35,
  "cost": 0
}

Claude: I've successfully added error handling to the parseConfig function
with 92% confidence in 35ms at zero cost!
```

#### 2. `agent_booster_batch`

Apply multiple edits in parallel.

```json
{
  "name": "agent_booster_batch",
  "description": "Apply multiple code edits in parallel (up to 10x faster)",
  "inputSchema": {
    "type": "object",
    "properties": {
      "edits": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "file_path": { "type": "string" },
            "edit_description": { "type": "string" }
          }
        },
        "maxItems": 100
      }
    },
    "required": ["edits"]
  }
}
```

#### 3. `agent_booster_analyze`

Analyze workspace for optimization opportunities.

```json
{
  "name": "agent_booster_analyze",
  "description": "Analyze workspace to identify files suitable for Agent Booster",
  "inputSchema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "Path to analyze (default: workspace root)"
      },
      "include_metrics": {
        "type": "boolean",
        "description": "Include detailed metrics",
        "default": true
      }
    }
  }
}
```

#### 4. `agent_booster_status`

Get server status and configuration.

```json
{
  "name": "agent_booster_status",
  "description": "Get Agent Booster server status and metrics",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

### MCP Resources

#### Metrics Resource

```
agent_booster://metrics
```

Returns JSON with usage statistics:

```json
{
  "total_edits": 1247,
  "agent_booster_usage": "82.3%",
  "morph_fallback_usage": "17.7%",
  "avg_latency_booster_ms": 38,
  "avg_latency_morph_ms": 5420,
  "total_cost_saved_usd": 22.14,
  "avg_confidence": 0.87,
  "uptime_hours": 12.4
}
```

### Workspace Detection

Agent Booster MCP server automatically detects workspace roots by looking for:

- `.git/` directory
- `package.json`
- `Cargo.toml`
- `.agent-booster.json` config file

Files are resolved relative to the detected workspace root.

### Performance Comparison (MCP Context)

```
Traditional MCP Flow (without Agent Booster):

User request: "Add logging to 10 functions"
  ↓
Claude analyzes workspace (5s)
  ↓
For each function:
  ├─ Read file (100ms)
  ├─ Generate edit via LLM (6000ms)
  ├─ Apply edit (100ms)
  └─ Write file (50ms)
  Total per file: ~6250ms

Total time: 10 files × 6250ms = 62.5 seconds
Total cost: 10 × $0.01 = $0.10

═════════════════════════════════════════════════════

Agent Booster MCP Flow:

User request: "Add logging to 10 functions"
  ↓
Claude analyzes workspace (5s)
  ↓
Calls agent_booster_batch with 10 edits:
  ├─ Parse all files in parallel (200ms)
  ├─ Generate embeddings in parallel (400ms)
  ├─ Apply all merges in parallel (300ms)
  └─ Validate all files in parallel (100ms)
  Total: ~1000ms = 1 second

Total time: 6 seconds (5s analysis + 1s edits)
Total cost: $0.00

Speedup: 10x faster
Savings: $0.10 (100%)
```

---

## 📊 Metrics Dashboard

Optional web dashboard for monitoring Agent Booster usage:

```bash
# Start metrics dashboard
npx agent-booster dashboard --port 8080
```

View at `http://localhost:8080`:

- Real-time edit metrics
- Cost savings calculator
- Performance graphs
- Confidence distribution
- Language breakdown
- Failure analysis
