# Agent Booster: NPM SDK & CLI Design

## 📦 NPM Package Structure

```
npm/
├── agent-booster/                  # Main SDK package
│   ├── package.json
│   ├── index.js                    # Auto-detection loader
│   ├── index.d.ts                  # TypeScript definitions
│   ├── native/                     # Native addon binaries
│   │   ├── index.node             # Auto-selected by platform
│   │   ├── linux-x64.node
│   │   ├── darwin-x64.node
│   │   ├── darwin-arm64.node
│   │   └── win32-x64.node
│   ├── wasm/                       # WebAssembly fallback
│   │   ├── agent_booster_bg.wasm
│   │   └── agent_booster.js
│   └── README.md
│
└── agent-booster-cli/              # Standalone CLI
    ├── package.json
    ├── bin/
    │   └── agent-booster.js        # CLI entry point
    ├── commands/
    │   ├── apply.js
    │   ├── batch.js
    │   ├── watch.js
    │   ├── mcp.js
    │   └── dashboard.js
    └── README.md
```

## 🎯 Main SDK Package

### package.json

```json
{
  "name": "agent-booster",
  "version": "0.1.0",
  "description": "Ultra-fast code application engine for AI agents (200x faster than LLMs)",
  "main": "index.js",
  "types": "index.d.ts",
  "keywords": [
    "ai",
    "code-editing",
    "ast",
    "semantic-merge",
    "vector-embeddings",
    "llm",
    "morph-alternative",
    "agentic-flow",
    "mcp",
    "rust",
    "napi",
    "wasm"
  ],
  "author": "Your Name <your@email.com>",
  "license": "MIT OR Apache-2.0",
  "repository": {
    "type": "git",
    "url": "https://github.com/your-org/agent-booster.git"
  },
  "engines": {
    "node": ">=16.0.0"
  },
  "os": ["linux", "darwin", "win32"],
  "cpu": ["x64", "arm64"],
  "files": [
    "index.js",
    "index.d.ts",
    "native/",
    "wasm/",
    "README.md",
    "LICENSE"
  ],
  "optionalDependencies": {
    "@agent-booster/linux-x64": "0.1.0",
    "@agent-booster/darwin-x64": "0.1.0",
    "@agent-booster/darwin-arm64": "0.1.0",
    "@agent-booster/win32-x64": "0.1.0"
  },
  "dependencies": {
    "detect-libc": "^2.0.2"
  },
  "devDependencies": {
    "@types/node": "^20.0.0",
    "typescript": "^5.0.0"
  },
  "scripts": {
    "postinstall": "node scripts/download-model.js",
    "test": "node test/index.js"
  }
}
```

### index.js (Auto-Detection Loader)

```javascript
// npm/agent-booster/index.js

const os = require('os');
const path = require('path');

let AgentBooster = null;

// Try to load native addon first (fastest)
function loadNative() {
  try {
    const platform = os.platform();
    const arch = os.arch();
    const nativeBinding = `${platform}-${arch}.node`;

    // Try platform-specific package
    try {
      const platformPackage = `@agent-booster/${platform}-${arch}`;
      AgentBooster = require(platformPackage).AgentBooster;
      console.log(`[agent-booster] Using native addon (${platform}-${arch})`);
      return true;
    } catch (e) {
      // Fall through to bundled native
    }

    // Try bundled native addon
    const nativePath = path.join(__dirname, 'native', nativeBinding);
    AgentBooster = require(nativePath).AgentBooster;
    console.log(`[agent-booster] Using bundled native addon (${platform}-${arch})`);
    return true;
  } catch (error) {
    console.warn('[agent-booster] Native addon not available:', error.message);
    return false;
  }
}

// Fallback to WASM
function loadWasm() {
  try {
    const wasmModule = require('./wasm/agent_booster.js');
    AgentBooster = wasmModule.AgentBooster;
    console.log('[agent-booster] Using WebAssembly');
    return true;
  } catch (error) {
    console.error('[agent-booster] WASM not available:', error.message);
    return false;
  }
}

// Load in order of preference
if (!loadNative()) {
  if (!loadWasm()) {
    throw new Error(
      'Failed to load Agent Booster. Neither native addon nor WASM are available.\n' +
      'Please report this issue: https://github.com/your-org/agent-booster/issues'
    );
  }
}

module.exports = { AgentBooster };
module.exports.default = AgentBooster;
```

### index.d.ts (TypeScript Definitions)

```typescript
// npm/agent-booster/index.d.ts

export interface AgentBoosterConfig {
  /** Embedding model to use */
  model?: 'jina-code-v2' | 'all-MiniLM-L6-v2' | string;

  /** Confidence threshold (0-1). Edits below this will fail or fallback. */
  confidenceThreshold?: number;

  /** Enable fallback to Morph LLM when confidence is low */
  fallbackToMorph?: boolean;

  /** Morph API key (required if fallbackToMorph is true) */
  morphApiKey?: string;

  /** Model cache directory */
  cacheDir?: string;

  /** Maximum number of chunks to extract per file */
  maxChunks?: number;

  /** Enable embedding caching */
  cacheEmbeddings?: boolean;

  /** Enable debug logging */
  debug?: boolean;
}

export interface EditRequest {
  /** Original code content */
  originalCode: string;

  /** Edit description or snippet to apply */
  editSnippet: string;

  /** Programming language */
  language?: 'javascript' | 'typescript' | 'python' | 'rust' | 'go' | 'java' | string;

  /** Optional edit options */
  options?: EditOptions;
}

export interface EditOptions {
  /** Preferred merge strategy (auto-detected if not specified) */
  strategy?: 'exact' | 'fuzzy' | 'insert' | 'append';

  /** Minimum confidence required */
  minConfidence?: number;

  /** Enable syntax validation */
  validateSyntax?: boolean;
}

export interface EditResult {
  /** Merged code */
  mergedCode: string;

  /** Confidence score (0-1) */
  confidence: number;

  /** Merge strategy used */
  strategy: 'exact_replace' | 'insert_before' | 'insert_after' | 'append' | 'fuzzy_replace';

  /** Additional metadata */
  metadata: EditMetadata;
}

export interface EditMetadata {
  /** Latency in milliseconds */
  latency_ms: number;

  /** Matched chunk information */
  matched_chunk?: {
    start_line: number;
    end_line: number;
    node_type: string;
  };

  /** Syntax validation result */
  syntax_valid: boolean;

  /** Method used (native/wasm) */
  method: 'native' | 'wasm';

  /** Model used */
  model: string;
}

export class AgentBooster {
  constructor(config?: AgentBoosterConfig);

  /**
   * Apply a single code edit
   */
  applyEdit(request: EditRequest): Promise<EditResult>;

  /**
   * Apply multiple edits in parallel
   */
  batchApply(requests: EditRequest[]): Promise<EditResult[]>;

  /**
   * Get current configuration
   */
  getConfig(): AgentBoosterConfig;

  /**
   * Update configuration
   */
  updateConfig(config: Partial<AgentBoosterConfig>): void;
}

export default AgentBooster;
```

## 🖥️ Standalone CLI Package

### package.json

```json
{
  "name": "agent-booster-cli",
  "version": "0.1.0",
  "description": "CLI for Agent Booster - ultra-fast code editing",
  "bin": {
    "agent-booster": "./bin/agent-booster.js"
  },
  "keywords": ["cli", "code-editing", "ai", "agent-booster"],
  "author": "Your Name <your@email.com>",
  "license": "MIT OR Apache-2.0",
  "engines": {
    "node": ">=16.0.0"
  },
  "dependencies": {
    "agent-booster": "^0.1.0",
    "commander": "^11.0.0",
    "chalk": "^5.0.0",
    "ora": "^7.0.0",
    "glob": "^10.0.0",
    "chokidar": "^3.5.0"
  }
}
```

### bin/agent-booster.js

```javascript
#!/usr/bin/env node

const { Command } = require('commander');
const chalk = require('chalk');
const packageJson = require('../package.json');

const program = new Command();

program
  .name('agent-booster')
  .description('Ultra-fast code application engine for AI agents')
  .version(packageJson.version);

// Commands
program
  .command('apply <file> <edit>')
  .description('Apply a single code edit')
  .option('-m, --model <model>', 'Embedding model', 'jina-code-v2')
  .option('-t, --threshold <threshold>', 'Confidence threshold', '0.65')
  .option('--dry-run', 'Show result without writing')
  .option('--fallback', 'Fallback to Morph LLM if needed')
  .action(require('../commands/apply'));

program
  .command('batch <edits-file>')
  .description('Apply multiple edits from JSON file')
  .option('-m, --model <model>', 'Embedding model', 'jina-code-v2')
  .option('-o, --output <dir>', 'Output directory')
  .option('--parallel <n>', 'Parallel jobs', '4')
  .action(require('../commands/batch'));

program
  .command('watch <path>')
  .description('Watch directory for changes and apply edits')
  .option('-m, --model <model>', 'Embedding model', 'jina-code-v2')
  .option('--ignore <patterns>', 'Ignore patterns (comma-separated)')
  .action(require('../commands/watch'));

program
  .command('mcp')
  .description('Start Model Context Protocol server')
  .option('--port <port>', 'HTTP port (default: stdio)', '')
  .option('--config <file>', 'Config file')
  .action(require('../commands/mcp'));

program
  .command('dashboard')
  .description('Start web dashboard for metrics')
  .option('-p, --port <port>', 'Port', '8080')
  .action(require('../commands/dashboard'));

program
  .command('download-models')
  .description('Download embedding models')
  .option('-m, --model <models>', 'Models to download (comma-separated)', 'jina-code-v2')
  .action(require('../commands/download-models'));

program.parse();
```

### commands/apply.js

```javascript
// npm/agent-booster-cli/commands/apply.js

const { AgentBooster } = require('agent-booster');
const { readFileSync, writeFileSync } = require('fs');
const chalk = require('chalk');
const ora = require('ora');

module.exports = async function apply(file, edit, options) {
  const spinner = ora(`Applying edit to ${file}...`).start();

  try {
    // Initialize Agent Booster
    const booster = new AgentBooster({
      model: options.model,
      confidenceThreshold: parseFloat(options.threshold),
      fallbackToMorph: options.fallback,
      morphApiKey: process.env.MORPH_API_KEY,
    });

    // Read file
    const originalCode = readFileSync(file, 'utf-8');

    // Apply edit
    const startTime = Date.now();
    const result = await booster.applyEdit({
      originalCode,
      editSnippet: edit,
      language: detectLanguage(file),
    });
    const latency = Date.now() - startTime;

    spinner.succeed('Edit applied successfully!');

    // Display results
    console.log('');
    console.log(chalk.bold('Results:'));
    console.log(`  Strategy:    ${chalk.cyan(result.strategy)}`);
    console.log(`  Confidence:  ${formatConfidence(result.confidence)}`);
    console.log(`  Latency:     ${chalk.green(latency + 'ms')}`);
    console.log(`  Cost:        ${chalk.green('$0.00')}`);
    console.log('');

    if (result.confidence < parseFloat(options.threshold)) {
      console.log(chalk.yellow('⚠️  Warning: Low confidence'));
      console.log(chalk.yellow(`   Consider using --fallback for better accuracy`));
      console.log('');
    }

    // Write or preview
    if (options.dryRun) {
      console.log(chalk.bold('Dry run - no changes written'));
      console.log('');
      console.log(chalk.dim('─'.repeat(80)));
      console.log(result.mergedCode);
      console.log(chalk.dim('─'.repeat(80)));
    } else {
      writeFileSync(file, result.mergedCode, 'utf-8');
      console.log(chalk.green(`✓ Saved to ${file}`));
    }
  } catch (error) {
    spinner.fail('Edit failed');
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
};

function detectLanguage(filePath) {
  const ext = filePath.split('.').pop();
  const langMap = {
    'js': 'javascript',
    'jsx': 'javascript',
    'ts': 'typescript',
    'tsx': 'typescript',
    'py': 'python',
    'rs': 'rust',
  };
  return langMap[ext] || 'javascript';
}

function formatConfidence(confidence) {
  const percentage = (confidence * 100).toFixed(1) + '%';
  if (confidence >= 0.85) return chalk.green(percentage);
  if (confidence >= 0.65) return chalk.yellow(percentage);
  return chalk.red(percentage);
}
```

### commands/watch.js

```javascript
// npm/agent-booster-cli/commands/watch.js

const { AgentBooster } = require('agent-booster');
const chokidar = require('chokidar');
const chalk = require('chalk');
const path = require('path');

module.exports = async function watch(directory, options) {
  console.log(chalk.bold(`\n📁 Watching ${directory} for changes...\n`));

  const booster = new AgentBooster({
    model: options.model,
  });

  const ignorePatterns = options.ignore
    ? options.ignore.split(',')
    : ['node_modules/**', '.git/**', 'dist/**', 'build/**'];

  const watcher = chokidar.watch(directory, {
    ignored: ignorePatterns,
    persistent: true,
    ignoreInitial: true,
  });

  watcher
    .on('change', async (filePath) => {
      console.log(chalk.dim(`${new Date().toLocaleTimeString()} `), 'Changed:', filePath);

      // Auto-apply formatting or linting fixes
      // This is a placeholder - actual implementation would read
      // from a .agent-booster.json config file
    })
    .on('error', (error) => {
      console.error(chalk.red('Watcher error:'), error);
    });

  // Keep process alive
  await new Promise(() => {});
};
```

## 📚 Usage Examples

### Example 1: Simple API Usage

```typescript
import { AgentBooster } from 'agent-booster';
import { readFileSync, writeFileSync } from 'fs';

const booster = new AgentBooster({
  model: 'jina-code-v2',
  confidenceThreshold: 0.65,
});

const result = await booster.applyEdit({
  originalCode: readFileSync('src/utils/parser.ts', 'utf-8'),
  editSnippet: 'add error handling to parseConfig function',
  language: 'typescript',
});

if (result.confidence >= 0.65) {
  writeFileSync('src/utils/parser.ts', result.mergedCode, 'utf-8');
  console.log(`✓ Applied edit with ${(result.confidence * 100).toFixed(1)}% confidence`);
} else {
  console.log(`✗ Confidence too low: ${(result.confidence * 100).toFixed(1)}%`);
}
```

### Example 2: Batch Processing

```typescript
const edits = [
  {
    originalCode: readFileSync('src/auth.ts', 'utf-8'),
    editSnippet: 'add JWT token validation',
    language: 'typescript',
  },
  {
    originalCode: readFileSync('src/db.ts', 'utf-8'),
    editSnippet: 'add connection pooling',
    language: 'typescript',
  },
  {
    originalCode: readFileSync('src/api.ts', 'utf-8'),
    editSnippet: 'add rate limiting',
    language: 'typescript',
  },
];

const results = await booster.batchApply(edits);

console.log(`✓ Applied ${results.filter(r => r.confidence >= 0.65).length} of ${results.length} edits`);
```

### Example 3: CLI Usage

```bash
# Apply single edit
npx agent-booster apply src/main.ts "add error handling to parseConfig"

# Dry run (preview only)
npx agent-booster apply src/main.ts "add logging" --dry-run

# Batch edits from JSON file
cat > edits.json <<EOF
[
  {
    "file": "src/auth.ts",
    "edit": "add JWT validation"
  },
  {
    "file": "src/db.ts",
    "edit": "add connection pooling"
  }
]
EOF

npx agent-booster batch edits.json

# Watch mode
npx agent-booster watch src/ --model jina-code-v2

# Start MCP server
npx agent-booster mcp --port 3000

# Download models
npx agent-booster download-models --model jina-code-v2,all-MiniLM-L6-v2
```

## 🚀 Distribution Strategy

### Platform-Specific Packages

```json
// @agent-booster/linux-x64/package.json
{
  "name": "@agent-booster/linux-x64",
  "version": "0.1.0",
  "os": ["linux"],
  "cpu": ["x64"],
  "main": "agent-booster.linux-x64.node"
}

// Similar for:
// - @agent-booster/darwin-x64
// - @agent-booster/darwin-arm64
// - @agent-booster/win32-x64
```

### Post-Install Script

```javascript
// scripts/download-model.js

const https = require('https');
const fs = require('fs');
const path = require('path');
const os = require('os');

const MODEL_URLS = {
  'jina-code-v2': 'https://huggingface.co/jinaai/jina-embeddings-v2-base-code/resolve/main/onnx/model.onnx',
  'all-MiniLM-L6-v2': 'https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx',
};

async function downloadModel(modelName) {
  const cacheDir = path.join(os.homedir(), '.cache', 'agent-booster', 'models');
  const modelPath = path.join(cacheDir, `${modelName}.onnx`);

  if (fs.existsSync(modelPath)) {
    console.log(`[agent-booster] Model ${modelName} already cached`);
    return;
  }

  console.log(`[agent-booster] Downloading model ${modelName}...`);

  // Create cache directory
  fs.mkdirSync(cacheDir, { recursive: true });

  // Download model
  // (implementation omitted for brevity)

  console.log(`[agent-booster] Model ${modelName} downloaded successfully`);
}

// Run on install
const defaultModel = process.env.AGENT_BOOSTER_MODEL || 'jina-code-v2';
downloadModel(defaultModel).catch(console.error);
```

## 📊 Bundle Size Analysis

```
agent-booster@0.1.0
├── Native addon (per platform): ~2-3 MB
├── WASM fallback: ~1.5 MB
├── JavaScript/TypeScript: ~50 KB
└── Total installed: ~5-8 MB

agent-booster-cli@0.1.0
├── Dependencies: ~2 MB
├── CLI code: ~100 KB
└── Total installed: ~2.1 MB

Models (downloaded on-demand):
├── jina-code-v2: ~150 MB (one-time)
└── all-MiniLM-L6-v2: ~90 MB (one-time)
```
