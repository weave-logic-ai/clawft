/**
 * SDK Integration Test
 *
 * Validates the Claude Agent SDK integration features:
 * - Hooks bridge
 * - Session management
 * - Permission handler
 * - Agent converter
 */

import {
  getSdkHooks,
  getToolSpecificHooks,
  captureSessionId,
  getSessionStats,
  getResumeOptions,
  getForkOptions,
  clearAllSessions,
  customPermissionHandler,
  strictPermissionHandler,
  getPermissionHandler,
  initPermissionHandler,
  getEssentialAgents,
  getMergedAgents,
  convertAgentToSdkFormat
} from '../dist/sdk/index.js';

console.log('🧪 Testing SDK Integration...\n');

let testsPassed = 0;
let testsFailed = 0;

function test(name: string, fn: () => boolean | Promise<boolean>) {
  try {
    const result = fn();
    if (result instanceof Promise) {
      result.then(r => {
        if (r) {
          console.log(`  ✅ ${name}`);
          testsPassed++;
        } else {
          console.log(`  ❌ ${name}`);
          testsFailed++;
        }
      });
    } else if (result) {
      console.log(`  ✅ ${name}`);
      testsPassed++;
    } else {
      console.log(`  ❌ ${name}`);
      testsFailed++;
    }
  } catch (e) {
    console.log(`  ❌ ${name} - Error: ${(e as Error).message}`);
    testsFailed++;
  }
}

// =============================================================================
// Hooks Bridge Tests
// =============================================================================
console.log('📎 Hooks Bridge:');

test('getSdkHooks returns all expected hooks', () => {
  const hooks = getSdkHooks();
  const expected = ['PreToolUse', 'PostToolUse', 'PostToolUseFailure', 'SessionStart', 'SessionEnd', 'SubagentStart', 'SubagentStop'];
  return expected.every(h => h in hooks);
});

test('getToolSpecificHooks with matcher returns filtered hooks', () => {
  const hooks = getToolSpecificHooks('Edit|Write');
  return 'PreToolUse' in hooks && hooks.PreToolUse![0].matcher === 'Edit|Write';
});

// =============================================================================
// Session Manager Tests
// =============================================================================
console.log('\n📊 Session Manager:');

test('clearAllSessions resets session state', () => {
  clearAllSessions();
  const stats = getSessionStats();
  return stats.totalSessions === 0 && stats.currentSessionId === null;
});

test('captureSessionId captures from init message', () => {
  clearAllSessions();
  const mockMsg = { type: 'system', subtype: 'init', session_id: 'test-123', uuid: 'uuid-1' };
  const id = captureSessionId(mockMsg as any);
  return id === 'test-123';
});

test('getSessionStats returns correct counts after capture', () => {
  const stats = getSessionStats();
  return stats.totalSessions === 1 && stats.currentSessionId === 'test-123';
});

test('getResumeOptions returns resume config', () => {
  const options = getResumeOptions('test-123');
  return options.resume === 'test-123';
});

test('getForkOptions returns fork config', () => {
  const options = getForkOptions('test-123');
  return options.resume === 'test-123' && options.forkSession === true;
});

clearAllSessions();

// =============================================================================
// Permission Handler Tests
// =============================================================================
console.log('\n🔒 Permission Handler:');

test('getPermissionHandler returns handler for default mode', () => {
  const handler = getPermissionHandler('default');
  return typeof handler === 'function';
});

test('getPermissionHandler returns undefined for bypass mode', () => {
  const handler = getPermissionHandler('bypass');
  return handler === undefined;
});

test('customPermissionHandler allows read-only tools', async () => {
  const result = await customPermissionHandler('Read', { file_path: '/tmp/test.txt' }, { signal: new AbortController().signal });
  return result.behavior === 'allow';
});

test('customPermissionHandler blocks dangerous commands', async () => {
  const result = await customPermissionHandler('Bash', { command: 'rm -rf /' }, { signal: new AbortController().signal });
  return result.behavior === 'deny';
});

test('customPermissionHandler allows safe bash commands', async () => {
  const result = await customPermissionHandler('Bash', { command: 'ls -la' }, { signal: new AbortController().signal });
  return result.behavior === 'allow';
});

test('strictPermissionHandler blocks Edit tool', async () => {
  const result = await strictPermissionHandler('Edit', { file_path: '/tmp/test.txt' }, { signal: new AbortController().signal });
  return result.behavior === 'deny';
});

test('customPermissionHandler blocks git push --force', async () => {
  const result = await customPermissionHandler('Bash', { command: 'git push --force origin main' }, { signal: new AbortController().signal });
  return result.behavior === 'deny';
});

test('customPermissionHandler allows safe command substitution', async () => {
  const result = await customPermissionHandler('Bash', { command: 'echo $(date)' }, { signal: new AbortController().signal });
  return result.behavior === 'allow';
});

test('customPermissionHandler blocks npm publish', async () => {
  const result = await customPermissionHandler('Bash', { command: 'npm publish' }, { signal: new AbortController().signal });
  return result.behavior === 'deny';
});

// =============================================================================
// Agent Converter Tests
// =============================================================================
console.log('\n🤖 Agent Converter:');

test('getEssentialAgents returns 5 agents', () => {
  const agents = getEssentialAgents();
  return Object.keys(agents).length === 5;
});

test('Essential agents have required fields', () => {
  const agents = getEssentialAgents();
  const researcher = agents['researcher'];
  return researcher.description && researcher.prompt && Array.isArray(researcher.tools);
});

test('getMergedAgents includes essential agents', () => {
  const agents = getMergedAgents(false);
  return 'researcher' in agents && 'coder' in agents;
});

test('convertAgentToSdkFormat converts correctly', () => {
  const mockAgent = {
    name: 'test-agent',
    description: 'A test agent for research tasks',
    systemPrompt: 'You are a test agent',
    filePath: '/test/path'
  };
  const sdkAgent = convertAgentToSdkFormat(mockAgent as any);
  return sdkAgent.description === mockAgent.description &&
         sdkAgent.prompt === mockAgent.systemPrompt &&
         Array.isArray(sdkAgent.tools);
});

// =============================================================================
// Summary
// =============================================================================
setTimeout(() => {
  console.log('\n' + '='.repeat(50));
  console.log(`📊 Test Results: ${testsPassed} passed, ${testsFailed} failed`);
  console.log('='.repeat(50));

  if (testsFailed > 0) {
    console.log('\n❌ Some tests failed!');
    process.exit(1);
  } else {
    console.log('\n✅ All SDK integration tests passed!');
  }
}, 1000);
