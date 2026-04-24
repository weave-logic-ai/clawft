#!/usr/bin/env node
/**
 * Postinstall script for agentic-flow
 *
 * Automatically patches AgentDB v1.3.9 import resolution issues
 * This runs after npm install, npm install -g, and npx
 */

import { readFileSync, writeFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

function findAgentDBPath() {
  const possiblePaths = [
    // npm install (local)
    join(__dirname, '..', 'node_modules', 'agentdb'),
    // npm install -g
    join(__dirname, '..', '..', 'agentdb'),
    // npx (parent directory)
    join(__dirname, '..', '..', '..', 'agentdb')
  ];

  for (const path of possiblePaths) {
    if (existsSync(join(path, 'package.json'))) {
      try {
        const pkg = JSON.parse(readFileSync(join(path, 'package.json'), 'utf8'));
        if (pkg.name === 'agentdb') {
          return path;
        }
      } catch {}
    }
  }

  return null;
}

function applyPatch() {
  console.log('[agentic-flow] Checking AgentDB installation...');

  const agentdbPath = findAgentDBPath();
  if (!agentdbPath) {
    console.log('[agentic-flow] ⚠️  AgentDB not found - skipping patch');
    return;
  }

  const controllerIndexPath = join(agentdbPath, 'dist', 'controllers', 'index.js');
  if (!existsSync(controllerIndexPath)) {
    console.log('[agentic-flow] ⚠️  AgentDB controllers not found');
    return;
  }

  let content = readFileSync(controllerIndexPath, 'utf8');

  // Check if already patched
  if (content.includes("from './ReflexionMemory.js'")) {
    console.log('[agentic-flow] ✅ AgentDB already patched');
    return;
  }

  // Apply patches
  const patches = [
    { from: "from './ReflexionMemory'", to: "from './ReflexionMemory.js'" },
    { from: "from './SkillLibrary'", to: "from './SkillLibrary.js'" },
    { from: "from './EmbeddingService'", to: "from './EmbeddingService.js'" },
    { from: "from './CausalMemoryGraph'", to: "from './CausalMemoryGraph.js'" },
    { from: "from './CausalRecall'", to: "from './CausalRecall.js'" },
    { from: "from './NightlyLearner'", to: "from './NightlyLearner.js'" }
  ];

  let modified = false;
  for (const patch of patches) {
    if (content.includes(patch.from) && !content.includes(patch.to)) {
      content = content.replace(new RegExp(patch.from, 'g'), patch.to);
      modified = true;
    }
  }

  if (modified) {
    try {
      writeFileSync(controllerIndexPath, content, 'utf8');
      console.log('[agentic-flow] ✅ AgentDB imports patched successfully');
      console.log('[agentic-flow]    Fixed ESM import resolution for v1.3.9');
    } catch (error) {
      console.log('[agentic-flow] ⚠️  Could not write patch (read-only)');
      console.log('[agentic-flow]    This is OK for npx - runtime patch will handle it');
    }
  }
}

try {
  applyPatch();
} catch (error) {
  console.log('[agentic-flow] ⚠️  Postinstall patch failed:', error.message);
  console.log('[agentic-flow]    Runtime patch will attempt fix on first use');
}
