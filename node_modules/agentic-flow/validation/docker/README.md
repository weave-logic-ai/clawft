# Docker Validation for ReasoningBank

This directory contains Docker-based validation scripts to test agentic-flow package in clean, isolated environments.

## Purpose

Validates that agentic-flow works correctly when installed via npm/npx in a fresh Node.js environment, testing:

1. **Package Installation** - npm install works correctly
2. **Backend Selector** - Environment detection functions properly
3. **Node.js Backend** - SQLite backend initializes and works
4. **WASM Backend** - WASM module loads and functions in Node.js
5. **Package Exports** - All export paths resolve correctly

## Quick Start

### Test Local Build

```bash
# From agentic-flow root directory

# 1. Build the package
npm run build

# 2. Create package tarball
npm pack

# 3. Run Docker validation
cd validation/docker
docker build -f Dockerfile.reasoningbank-local -t agentic-flow-test:local ../..
docker run --rm agentic-flow-test:local
```

### Test Published Version

```bash
# Test latest published version from npm
docker build -f Dockerfile.reasoningbank-test -t agentic-flow-test:latest ../..
docker run --rm agentic-flow-test:latest

# Or use docker-compose
docker-compose up reasoningbank-test-latest
```

## Files

| File | Purpose |
|------|---------|
| `Dockerfile.reasoningbank-test` | Tests published npm package |
| `Dockerfile.reasoningbank-local` | Tests local build (npm pack) |
| `test-reasoningbank-npx.mjs` | Validation test suite |
| `docker-compose.yml` | Orchestrates multiple test scenarios |
| `README.md` | This file |

## Test Suite

The validation script (`test-reasoningbank-npx.mjs`) runs:

### Test 1: Package Installation
- Initializes package.json
- Installs agentic-flow via npm
- Verifies installation success

### Test 2: Backend Selector
- Imports backend-selector module
- Tests environment detection (expects 'nodejs')
- Validates backend info structure
- Checks environment validation

### Test 3: Node.js Backend (SQLite)
- Creates ReasoningBank with optimal backend
- Verifies Node.js backend selected
- Checks db module is present
- Tests database initialization

### Test 4: WASM Backend
- Imports WASM adapter
- Creates WASM ReasoningBank instance
- Stores test pattern
- Performs category search
- Runs semantic similarity search
- Validates similarity scores
- Checks stats reporting

### Test 5: Package Exports
- Tests main export (`agentic-flow`)
- Tests reasoningbank export (conditional)
- Tests backend-selector export
- Tests wasm-adapter export

## Expected Output

```
🐳 Docker Validation: agentic-flow ReasoningBank

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ Created test directory: /test/validation-workspace

📦 Test 1: Package Installation via npm

   Installing agentic-flow@latest...
✅ Package installation
   Installed agentic-flow@1.5.13

🔍 Test 2: Backend Selector Environment Detection

✅ Backend selector import
   Detected: nodejs

✅ Environment detection
   Expected nodejs, got nodejs

💾 Test 3: Node.js Backend (SQLite)

✅ Node.js backend initialization
   SQLite backend loaded

✅ Node.js backend detection
   db module present

⚡ Test 4: WASM Backend (In-Memory)

✅ WASM backend initialization
   WASM module loaded

✅ WASM pattern storage
   In-memory storage works

✅ WASM semantic search
   Similarity matching works

✅ WASM similarity scoring
   Score: 0.5401

📦 Test 5: Package Exports

✅ Package exports
   All import paths valid

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 VALIDATION SUMMARY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Total Tests: 10
✅ Passed: 10
❌ Failed: 0
⏱️  Duration: 45.23s

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🎉 All tests passed! Package is working correctly.
```

## Troubleshooting

### Test Failures

If tests fail, check:

1. **Package installation fails**
   - Verify network connectivity
   - Check npm registry is accessible
   - Try with `--registry https://registry.npmjs.org`

2. **Backend selector fails**
   - Check package.json exports field
   - Verify conditional exports syntax
   - Test with `node --experimental-modules`

3. **WASM backend fails**
   - Ensure `--experimental-wasm-modules` flag is set
   - Check WASM binary is included in dist/
   - Verify wasm-pack build completed

4. **SQLite backend fails**
   - Check better-sqlite3 is in dependencies
   - Verify native module compilation
   - Test on matching Node.js version

### Local Testing

To test locally without Docker:

```bash
cd /tmp
mkdir reasoningbank-test && cd reasoningbank-test
npm init -y
npm install agentic-flow@latest

# Run individual tests
node --experimental-wasm-modules <<EOF
import { getRecommendedBackend } from 'agentic-flow/reasoningbank/backend-selector';
console.log('Backend:', getRecommendedBackend());
EOF
```

## CI/CD Integration

Add to GitHub Actions:

```yaml
- name: Validate agentic-flow in Docker
  run: |
    cd validation/docker
    docker build -f Dockerfile.reasoningbank-local -t test ../..
    docker run --rm test
```

## Performance Benchmarks

Expected performance in Docker:

| Operation | Time |
|-----------|------|
| Package install | 15-30s |
| Backend detection | <100ms |
| WASM initialization | 50-100ms |
| Pattern storage | 1-5ms |
| Semantic search | 50-100ms |

## Environment Details

- **Base Image**: `node:20-slim`
- **Node.js Version**: 20.x LTS
- **Architecture**: linux/amd64
- **Dependencies**: git, curl

## Support

If validation fails:

1. Check the [REASONINGBANK_BACKENDS.md](../../docs/REASONINGBANK_BACKENDS.md) guide
2. Review [IMPLEMENTATION_SUMMARY.md](../../docs/IMPLEMENTATION_SUMMARY.md)
3. Open an issue at https://github.com/ruvnet/agentic-flow/issues
