# Agent Booster CLI Integration Plan

## Current State

### What Works ✅
- **MCP Server**: 3 Agent Booster tools available via `standalone-stdio.js`
- **Standalone CLI**: `agent-booster` package works with JSON stdin
- **Integration**: Uses `npx --yes agent-booster` (no dependency needed)

### What Doesn't Work ❌
- **CLI `--agent` mode**: `npx agentic-flow --agent coder` doesn't use Agent Booster
- **Flag is cosmetic**: `--agent-booster` just prints info, doesn't integrate
- **Performance**: Takes 26s instead of <1s for simple var→const edits

## Problem Analysis

### Current Flow (CLI Mode)
```
User: npx agentic-flow --agent coder --task "convert var to const"
  ↓
CLI Wrapper (claude-code-wrapper.ts)
  ↓ Prints Agent Booster info (but doesn't integrate)
  ↓
Agent Executor (directApiAgent.ts)
  ↓ Calls LLM with Edit tool
  ↓
LLM Response (26 seconds)
  ↓
File modified using LLM-generated edit
```

### Desired Flow (with Agent Booster)
```
User: npx agentic-flow --agent coder --task "convert var to const" --agent-booster
  ↓
CLI Wrapper
  ↓ Pass agent-booster flag to agent
  ↓
Agent Executor
  ↓ Detect if task is code edit
  ↓
Try Agent Booster first
  ├─ High confidence (≥70%) → Apply edit (85ms)
  └─ Low confidence (<70%) → Fallback to LLM (26s)
```

## Implementation Challenges

### Challenge 1: CLI vs MCP Architecture

**Problem**: Agent Booster is designed for **exact code replacements**, but CLI agents receive **vague natural language** tasks.

Example:
```bash
# User input (vague)
npx agentic-flow --agent coder --task "convert var to const in utils.js"

# What Agent Booster needs (exact)
{"code":"var x = 1;","edit":"const x = 1;"}
```

**Solution**: We need an intermediary step that:
1. Reads the file
2. Uses LLM to generate exact code replacement
3. Passes to Agent Booster for fast application

But this doesn't save time! We still need the LLM to generate the exact edit.

### Challenge 2: When Agent Booster Actually Helps

Agent Booster is fast **only** when you already have the exact code replacement:

| Scenario | Agent Booster Helps? | Why |
|----------|---------------------|-----|
| User provides exact code | ✅ Yes | Skip LLM entirely (85ms vs 26s) |
| User provides file + vague task | ❌ No | Still need LLM to generate edit |
| Batch edits with pattern | ⚠️ Maybe | Need pattern detection first |

### Challenge 3: Remote Package Installation

**Current approach**: `npx --yes agent-booster` downloads on first use

**Issues**:
- 30s timeout for download
- Network dependency
- User doesn't know it's downloading
- Fails in air-gapped environments

**Better approach**: Add `agent-booster` as optional dependency

## Proposed Solutions

### Option 1: Add Pre-Processing Step (Hybrid)

**Pros:**
- Works with vague natural language
- Can leverage Agent Booster when applicable
- Graceful fallback to LLM

**Cons:**
- Complex flow
- LLM still needed for generating edits
- Minimal time savings (only skips LLM execution phase)

**Implementation:**
```typescript
async function executeWithAgentBooster(task: string, file: string) {
  // Step 1: Use fast LLM to generate exact edit
  const edit = await generateExactEdit(task, file);  // 5-10s

  // Step 2: Try Agent Booster
  const result = await tryAgentBooster(edit);

  if (result.confidence >= 0.7) {
    return result;  // 85ms for applying
  }

  // Step 3: Fallback to full LLM
  return await executeLLMAgent(task, file);  // 26s
}
```

**Time saved**: ~1-2s (only application phase)

### Option 2: Add Dependency (Simple)

**Pros:**
- No network delays
- Works offline
- Predictable behavior

**Cons:**
- Increases package size (1.4MB for WASM)
- Installation time increased
- Makes agent-booster non-optional

**Implementation:**
```json
{
  "dependencies": {
    "agent-booster": "^0.1.1"
  }
}
```

### Option 3: Keep MCP-Only (Current State)

**Pros:**
- Clean separation of concerns
- MCP tools work perfectly for exact edits
- CLI agents work perfectly for reasoning
- No complexity added

**Cons:**
- CLI users don't get Agent Booster performance
- `--agent-booster` flag is misleading

**Implementation:**
- Remove `--agent-booster` flag from CLI
- Update docs to clarify MCP-only
- Focus on MCP integration quality

### Option 4: Smart Pattern Detection

**Pros:**
- Detects simple patterns (var→const, add types)
- Can use Agent Booster without LLM
- Significant time savings for common tasks

**Cons:**
- Limited to known patterns
- Complex pattern detection logic
- May miss edge cases

**Implementation:**
```typescript
async function detectPatternAndApply(task: string, file: string) {
  // Detect common patterns
  if (task.includes('var') && task.includes('const')) {
    return await applyVarToConstPattern(file);  // 85ms
  }

  if (task.includes('add types') || task.includes('TypeScript')) {
    return await applyTypeAnnotationPattern(file);  // 85ms + pattern detection
  }

  // Fall back to full LLM
  return await executeLLMAgent(task, file);  // 26s
}
```

## Recommendation

**Choose Option 3: Keep MCP-Only**

**Reasoning:**

1. **Agent Booster's strength**: Exact code replacements
   - Perfect for MCP tools (Claude generates exact code)
   - Poor fit for CLI (user provides vague natural language)

2. **Time savings reality**:
   - MCP: 85ms (skip LLM entirely) ✅
   - CLI: Still need LLM for edit generation = minimal savings ❌

3. **User experience**:
   - MCP users get 728x speedup for exact edits
   - CLI users get full LLM reasoning for complex tasks
   - Each mode uses the right tool for the job

4. **Maintenance**:
   - Simple, clean architecture
   - No complex hybrid flows
   - Focus on quality, not gimmicks

### What to Change

1. **Remove misleading flag**:
   ```typescript
   // DELETE: .option('--agent-booster', ...)
   ```

2. **Update README**:
   ```markdown
   ## Agent Booster

   Agent Booster provides 728x faster code edits through the **MCP server** (for Claude Desktop/Cursor).

   ⚠️ **CLI users**: The `--agent` mode uses standard LLM reasoning and does NOT use Agent Booster. This is by design - use Claude Desktop/Cursor for Agent Booster performance.
   ```

3. **Focus on MCP quality**:
   - Improve MCP tool descriptions
   - Add more examples
   - Better error messages

## Alternative: Limited CLI Integration

If we **must** have CLI integration, here's the minimal viable approach:

### Pattern-Based CLI (Limited Scope)

Only support explicit patterns with `--pattern` flag:

```bash
# Explicit pattern (works with Agent Booster)
npx agentic-flow --agent coder --pattern var-to-const --file utils.js
# Result: 85ms

# Vague task (uses LLM)
npx agentic-flow --agent coder --task "improve error handling" --file utils.js
# Result: 26s
```

**Supported patterns**:
- `var-to-const`: Convert var to const/let
- `add-types`: Add TypeScript type annotations
- `arrow-functions`: Convert function() to () =>
- `async-await`: Convert callbacks to async/await

**Implementation**:
```typescript
const patterns = {
  'var-to-const': async (file: string) => {
    const code = fs.readFileSync(file, 'utf-8');
    const edit = code.replace(/\bvar\b/g, 'const'); // Simplified
    return await applyAgentBooster(code, edit);
  }
};

if (options.pattern && patterns[options.pattern]) {
  return await patterns[options.pattern](options.file);
}
```

**Pros**:
- Clear what works with Agent Booster
- Fast for supported patterns
- No false expectations

**Cons**:
- Limited pattern support
- Requires pattern knowledge
- Not natural language

## Validation Strategy

Before implementing ANY solution:

1. **Test locally** with exact use cases
2. **Publish to test npm registry** or local registry
3. **Install from npm** (not local files)
4. **Test in clean environment** (Docker container)
5. **Verify performance claims** with real benchmarks

### Test Checklist

- [ ] MCP server tools work remotely (Claude Desktop)
- [ ] CLI `--agent` mode works as expected
- [ ] `npx agent-booster` CLI works standalone
- [ ] No network timeouts or failures
- [ ] Performance matches claims
- [ ] Works in air-gapped environment (if dependency)

## Conclusion

**Recommended approach**: Keep Agent Booster as **MCP-only** (Option 3)

**Why**: Agent Booster is designed for exact code replacements, which works perfectly with MCP tools but poorly with CLI natural language tasks. Trying to force CLI integration would add complexity without significant benefit.

**Action items**:
1. Remove misleading `--agent-booster` CLI flag
2. Update documentation to clarify MCP-only
3. Focus on making MCP integration excellent
4. Let CLI mode use standard LLM reasoning (what it's good at)

This keeps the architecture clean, user expectations clear, and each mode focused on what it does best.
