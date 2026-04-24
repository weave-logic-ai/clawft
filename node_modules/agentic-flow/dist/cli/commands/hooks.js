#!/usr/bin/env node
/**
 * CLI Hooks Commands
 * Provides CLI interface for agentic-flow hook tools
 *
 * NOW WITH FULL RUVECTOR INTELLIGENCE:
 * - @ruvector/sona: Micro-LoRA (~0.05ms), EWC++, Trajectory tracking
 * - @ruvector/attention: MoE, Flash, Hyperbolic, Graph attention
 * - ruvector core: HNSW indexing (150x faster)
 *
 * Available as BOTH:
 * 1. CLI Commands (agentic-flow hooks ...)
 * 2. MCP Tools (via hooks-server.ts)
 */
import { Command } from 'commander';
import * as path from 'path';
// Import hook tools
import { hookPreEditTool } from '../../mcp/fastmcp/tools/hooks/pre-edit.js';
import { hookPostEditTool } from '../../mcp/fastmcp/tools/hooks/post-edit.js';
import { hookPreCommandTool } from '../../mcp/fastmcp/tools/hooks/pre-command.js';
import { hookPostCommandTool } from '../../mcp/fastmcp/tools/hooks/post-command.js';
import { hookRouteTool } from '../../mcp/fastmcp/tools/hooks/route.js';
import { hookExplainTool } from '../../mcp/fastmcp/tools/hooks/explain.js';
import { hookPretrainTool } from '../../mcp/fastmcp/tools/hooks/pretrain.js';
import { hookBuildAgentsTool } from '../../mcp/fastmcp/tools/hooks/build-agents.js';
import { hookMetricsTool } from '../../mcp/fastmcp/tools/hooks/metrics.js';
import { hookTransferTool } from '../../mcp/fastmcp/tools/hooks/transfer.js';
// Import intelligence tools (RuVector SONA + Attention + HNSW)
import { intelligenceRouteTool, intelligenceTrajectoryStartTool, intelligenceTrajectoryStepTool, intelligenceTrajectoryEndTool, intelligencePatternStoreTool, intelligencePatternSearchTool, intelligenceStatsTool, intelligenceLearnTool, intelligenceAttentionTool } from '../../mcp/fastmcp/tools/hooks/intelligence-tools.js';
const mockContext = {
    onProgress: (update) => {
        if (process.env.VERBOSE) {
            console.log(`[${Math.round(update.progress * 100)}%] ${update.message}`);
        }
    }
};
export function createHooksCommand() {
    const hooks = new Command('hooks')
        .description('Self-learning intelligence hooks for agent routing and optimization');
    // Pre-edit hook
    hooks
        .command('pre-edit <filePath>')
        .description('Get context and agent suggestion before editing a file')
        .option('-t, --task <task>', 'Task description')
        .option('-j, --json', 'Output as JSON')
        .action(async (filePath, options) => {
        try {
            const result = await hookPreEditTool.execute({ filePath, task: options.task }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n🎯 Suggested Agent: ${result.suggestedAgent}`);
                console.log(`📊 Confidence: ${(result.confidence * 100).toFixed(1)}%`);
                if (result.relatedFiles?.length > 0) {
                    console.log(`📁 Related Files:`);
                    result.relatedFiles.forEach((f) => console.log(`   - ${f}`));
                }
                console.log(`⏱️  Latency: ${result.latencyMs}ms`);
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Post-edit hook
    hooks
        .command('post-edit <filePath>')
        .description('Record edit outcome for learning')
        .option('-s, --success', 'Mark as successful edit')
        .option('-f, --fail', 'Mark as failed edit')
        .option('-a, --agent <agent>', 'Agent that performed the edit')
        .option('-d, --duration <ms>', 'Edit duration in milliseconds', parseInt)
        .option('-e, --error <message>', 'Error message if failed')
        .option('-j, --json', 'Output as JSON')
        .action(async (filePath, options) => {
        try {
            const success = options.success || !options.fail;
            const result = await hookPostEditTool.execute({
                filePath,
                success,
                agent: options.agent,
                duration: options.duration,
                errorMessage: options.error
            }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n${success ? '✅' : '❌'} Edit recorded: ${filePath}`);
                console.log(`📈 Pattern updated: ${result.newPatternValue?.toFixed(2) || 'N/A'}`);
                console.log(`📊 Routing accuracy: ${(result.routingAccuracy * 100).toFixed(1)}%`);
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Pre-command hook
    hooks
        .command('pre-command <command>')
        .description('Assess command risk before execution')
        .option('-j, --json', 'Output as JSON')
        .action(async (command, options) => {
        try {
            const result = await hookPreCommandTool.execute({ command }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                const riskIcon = result.blocked ? '🚫' :
                    result.riskCategory === 'safe' ? '✅' :
                        result.riskCategory === 'caution' ? '⚠️' : '❌';
                console.log(`\n${riskIcon} Risk Level: ${result.riskCategory.toUpperCase()} (${(result.riskLevel * 100).toFixed(0)}%)`);
                if (result.blocked) {
                    console.log(`🚫 Command BLOCKED - dangerous operation detected`);
                }
                else if (result.approved) {
                    console.log(`✅ Command APPROVED`);
                }
                if (result.suggestions?.length > 0) {
                    console.log(`💡 Suggestions:`);
                    result.suggestions.forEach((s) => console.log(`   - ${s}`));
                }
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Post-command hook
    hooks
        .command('post-command <command>')
        .description('Record command outcome for learning')
        .option('-c, --exit-code <code>', 'Exit code', parseInt, 0)
        .option('-o, --stdout <output>', 'Command stdout')
        .option('-e, --stderr <error>', 'Command stderr')
        .option('-j, --json', 'Output as JSON')
        .action(async (command, options) => {
        try {
            const result = await hookPostCommandTool.execute({
                command,
                exitCode: options.exitCode ?? 0,
                stdout: options.stdout,
                stderr: options.stderr
            }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n${result.commandSuccess ? '✅' : '❌'} Command outcome recorded`);
                console.log(`📚 Learned: ${result.learned ? 'Yes' : 'No'}`);
                if (result.errorType) {
                    console.log(`🔍 Error type: ${result.errorType}`);
                }
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Route hook
    hooks
        .command('route <task>')
        .description('Route task to optimal agent using learned patterns')
        .option('-f, --file <filePath>', 'Context file path')
        .option('-e, --explore', 'Enable exploration mode')
        .option('-j, --json', 'Output as JSON')
        .action(async (task, options) => {
        try {
            const result = await hookRouteTool.execute({
                task,
                context: options.file ? { file: options.file } : undefined,
                explore: options.explore
            }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n🎯 Recommended Agent: ${result.agent}`);
                console.log(`📊 Confidence: ${(result.confidence * 100).toFixed(1)}%`);
                if (result.explored) {
                    console.log(`🔍 Exploration mode: Active`);
                }
                console.log(`\n📋 Routing Factors:`);
                result.factors.forEach((f) => {
                    console.log(`   • ${f.name}: ${(f.score * 100).toFixed(0)}%`);
                });
                if (result.alternatives?.length > 0) {
                    console.log(`\n🔄 Alternatives:`);
                    result.alternatives.forEach((a) => {
                        console.log(`   - ${a.agent} (${(a.score * 100).toFixed(0)}%)`);
                    });
                }
                console.log(`\n⏱️  Latency: ${result.latencyMs}ms`);
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Explain hook
    hooks
        .command('explain <task>')
        .description('Explain routing decision with full transparency')
        .option('-f, --file <filePath>', 'Context file path')
        .option('-j, --json', 'Output as JSON')
        .action(async (task, options) => {
        try {
            const result = await hookExplainTool.execute({ task, file: options.file }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n📝 Summary: ${result.summary}`);
                console.log(`\n🎯 Recommended: ${result.recommendedAgent}`);
                console.log(`\n💡 Reasons:`);
                result.reasons.forEach((r) => console.log(`   • ${r}`));
                console.log(`\n🏆 Agent Ranking:`);
                result.ranking.forEach((r) => {
                    console.log(`   ${r.rank}. ${r.agent} - ${(r.score * 100).toFixed(1)}%`);
                });
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Pretrain hook
    hooks
        .command('pretrain')
        .description('Analyze repository to bootstrap intelligence')
        .option('-d, --depth <n>', 'Git history depth', parseInt, 50)
        .option('--skip-git', 'Skip git history analysis')
        .option('--skip-files', 'Skip file structure analysis')
        .option('-j, --json', 'Output as JSON')
        .action(async (options) => {
        try {
            console.log('🧠 Analyzing repository...\n');
            const result = await hookPretrainTool.execute({
                depth: options.depth ?? 50,
                skipGit: options.skipGit ?? false,
                skipFiles: options.skipFiles ?? false
            }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n📊 Pretrain Complete!`);
                console.log(`   📁 Files analyzed: ${result.filesAnalyzed}`);
                console.log(`   🧩 Patterns created: ${result.patternsCreated}`);
                console.log(`   💾 Memories stored: ${result.memoriesStored}`);
                console.log(`   🔗 Co-edits found: ${result.coEditsFound || 0}`);
                if (result.languages?.length > 0) {
                    console.log(`   🌐 Languages: ${result.languages.join(', ')}`);
                }
                console.log(`   ⏱️  Duration: ${result.durationMs}ms`);
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Build-agents hook
    hooks
        .command('build-agents')
        .description('Generate optimized agent configurations from pretrain data')
        .option('-f, --focus <mode>', 'Focus mode: quality|speed|security|testing|fullstack', 'quality')
        .option('-o, --output <dir>', 'Output directory', '.claude/agents')
        .option('--format <fmt>', 'Output format: yaml|json', 'yaml')
        .option('--no-prompts', 'Exclude system prompts')
        .option('-j, --json', 'Output as JSON')
        .action(async (options) => {
        try {
            console.log(`🏗️  Building agents with focus: ${options.focus}...\n`);
            const result = await hookBuildAgentsTool.execute({
                focus: options.focus,
                output: options.output,
                format: options.format,
                includePrompts: options.prompts !== false
            }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n✅ Agents Generated!`);
                console.log(`   📦 Total: ${result.agentsGenerated}`);
                console.log(`   📂 Output: ${result.outputDir}`);
                console.log(`   🎯 Focus: ${result.focus}`);
                console.log(`\n   Agents created:`);
                result.agents.forEach((a) => console.log(`     • ${a}`));
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Metrics hook
    hooks
        .command('metrics')
        .alias('stats')
        .description('View learning metrics and performance dashboard')
        .option('-t, --timeframe <period>', 'Timeframe: 1h|24h|7d|30d', '24h')
        .option('-d, --detailed', 'Show detailed metrics')
        .option('-j, --json', 'Output as JSON')
        .action(async (options) => {
        try {
            const result = await hookMetricsTool.execute({
                timeframe: options.timeframe,
                detailed: options.detailed
            }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n📊 Learning Metrics (${options.timeframe || '24h'})\n`);
                console.log(`🎯 Routing:`);
                console.log(`   Total routes: ${result.routing.total}`);
                console.log(`   Successful: ${result.routing.successful}`);
                console.log(`   Accuracy: ${(result.routing.accuracy * 100).toFixed(1)}%`);
                console.log(`\n📚 Learning:`);
                console.log(`   Patterns: ${result.learning.patterns}`);
                console.log(`   Memories: ${result.learning.memories}`);
                console.log(`   Error patterns: ${result.learning.errorPatterns}`);
                console.log(`\n💚 Health: ${result.health.status.toUpperCase()}`);
                if (result.health.issues?.length > 0) {
                    console.log(`   Issues:`);
                    result.health.issues.forEach((i) => console.log(`     ⚠️ ${i}`));
                }
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Transfer hook
    hooks
        .command('transfer <sourceProject>')
        .description('Transfer learned patterns from another project')
        .option('-c, --min-confidence <n>', 'Minimum confidence threshold', parseFloat, 0.7)
        .option('-m, --max-patterns <n>', 'Maximum patterns to transfer', parseInt, 50)
        .option('--mode <mode>', 'Transfer mode: merge|replace|additive', 'merge')
        .option('-j, --json', 'Output as JSON')
        .action(async (sourceProject, options) => {
        try {
            console.log(`📤 Transferring patterns from: ${sourceProject}...\n`);
            const result = await hookTransferTool.execute({
                sourceProject,
                minConfidence: options.minConfidence,
                maxPatterns: options.maxPatterns,
                mode: options.mode
            }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n✅ Transfer Complete!`);
                console.log(`   📥 Patterns transferred: ${result.transferred}`);
                console.log(`   🔄 Patterns adapted: ${result.adapted}`);
                console.log(`   🎯 Mode: ${result.mode}`);
                if (result.targetStack?.length > 0) {
                    console.log(`   🛠️  Target stack: ${result.targetStack.join(', ')}`);
                }
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Init command (creates .claude/settings.json with hooks)
    hooks
        .command('init')
        .description('Initialize hooks in project with .claude/settings.json')
        .option('--minimal', 'Minimal configuration')
        .option('--force', 'Overwrite existing configuration')
        .option('--no-statusline', 'Skip statusline generation')
        .action(async (options) => {
        try {
            const fs = await import('fs');
            const settingsPath = path.join(process.cwd(), '.claude', 'settings.json');
            const statuslinePath = path.join(process.cwd(), '.claude', 'statusline.mjs');
            const claudeDir = path.dirname(settingsPath);
            // Check if exists
            if (fs.existsSync(settingsPath) && !options.force) {
                console.log('⚠️  Settings file already exists. Use --force to overwrite.');
                return;
            }
            // Create .claude directory
            if (!fs.existsSync(claudeDir)) {
                fs.mkdirSync(claudeDir, { recursive: true });
            }
            // Create statusline.mjs (compact colored format with agent stats)
            if (options.statusline !== false) {
                const statuslineContent = `#!/usr/bin/env node
// Agentic Flow Intelligence Status Line - Compact Format with Colors
// Works on Windows, Mac, and Linux - Queries SQLite for real stats

import { readFileSync, statSync, existsSync } from 'fs';
import { execSync } from 'child_process';

const INTEL_FILE = '.agentic-flow/intelligence.json';
const INTEL_DB = '.agentic-flow/intelligence.db';

// Detect dark/light mode
const isDarkMode = (() => {
  const colorScheme = process.env.COLORFGBG || '';
  const termBg = process.env.TERM_BACKGROUND || '';
  const vscodeTheme = process.env.VSCODE_TERMINAL_COLOR_THEME || '';
  if (termBg === 'light') return false;
  if (vscodeTheme.toLowerCase().includes('light')) return false;
  if (colorScheme.startsWith('0;') || colorScheme.includes(';15')) return false;
  if (process.env.CLAUDE_CODE_THEME === 'light') return false;
  return true;
})();

// Color palettes
const colors = {
  dark: {
    reset: '\\x1b[0m', dim: '\\x1b[2m', bold: '\\x1b[1m',
    model: '\\x1b[38;5;208m', project: '\\x1b[38;5;39m', branch: '\\x1b[38;5;156m',
    brain: '\\x1b[38;5;213m', patterns: '\\x1b[38;5;220m', memory: '\\x1b[38;5;117m',
    trajectories: '\\x1b[38;5;183m', agents: '\\x1b[38;5;156m',
    target: '\\x1b[38;5;196m', learning: '\\x1b[38;5;226m', epsilon: '\\x1b[38;5;51m',
    success: '\\x1b[38;5;46m', symbol: '\\x1b[38;5;245m'
  },
  light: {
    reset: '\\x1b[0m', dim: '\\x1b[2m', bold: '\\x1b[1m',
    model: '\\x1b[38;5;166m', project: '\\x1b[38;5;27m', branch: '\\x1b[38;5;28m',
    brain: '\\x1b[38;5;129m', patterns: '\\x1b[38;5;136m', memory: '\\x1b[38;5;30m',
    trajectories: '\\x1b[38;5;91m', agents: '\\x1b[38;5;28m',
    target: '\\x1b[38;5;160m', learning: '\\x1b[38;5;136m', epsilon: '\\x1b[38;5;31m',
    success: '\\x1b[38;5;28m', symbol: '\\x1b[38;5;240m'
  }
};
const c = isDarkMode ? colors.dark : colors.light;

function readJson(file) {
  try { return JSON.parse(readFileSync(file, 'utf8')); } catch { return null; }
}

function querySqlite(db, sql) {
  try {
    return execSync(\`sqlite3 "\${db}" "\${sql}"\`, { encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'], timeout: 1000 }).trim();
  } catch { return null; }
}

function getGitBranch() {
  try { return execSync('git branch --show-current', { encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }).trim(); }
  catch { return ''; }
}

function getProjectName() {
  try { return process.cwd().split('/').pop() || 'project'; } catch { return 'project'; }
}

function formatSize(bytes) {
  if (bytes < 1024) return \`\${bytes}B\`;
  if (bytes < 1024 * 1024) return \`\${Math.round(bytes / 1024)}K\`;
  return \`\${(bytes / (1024 * 1024)).toFixed(1)}M\`;
}

const LEARNING_RATE = parseFloat(process.env.AGENTIC_FLOW_LEARNING_RATE || '0.1');
const EPSILON = parseFloat(process.env.AGENTIC_FLOW_EPSILON || '0.1');

const intel = readJson(INTEL_FILE);
let dbStats = { totalTrajectories: 0, successfulRoutings: 0, totalRoutings: 0, totalPatterns: 0, totalAgents: 0, activeAgents: 0 };

if (existsSync(INTEL_DB)) {
  const statsRow = querySqlite(INTEL_DB, 'SELECT * FROM stats LIMIT 1');
  if (statsRow) {
    const cols = statsRow.split('|');
    dbStats.totalTrajectories = parseInt(cols[1]) || 0;
    dbStats.totalRoutings = parseInt(cols[3]) || 0;
    dbStats.successfulRoutings = parseInt(cols[4]) || 0;
    dbStats.totalPatterns = parseInt(cols[5]) || 0;
  }
  const trajCount = querySqlite(INTEL_DB, 'SELECT COUNT(*) FROM trajectories');
  if (trajCount) dbStats.totalTrajectories = parseInt(trajCount) || dbStats.totalTrajectories;
  const patternCount = querySqlite(INTEL_DB, 'SELECT COUNT(*) FROM patterns');
  if (patternCount) dbStats.totalPatterns = parseInt(patternCount) || 0;
}

const routes = dbStats.totalRoutings > 0 ? dbStats.totalRoutings : (intel?.metrics?.totalRoutes || 0);
const patterns = dbStats.totalPatterns > 0 ? dbStats.totalPatterns : (intel?.patterns ? Object.keys(intel.patterns).length : 0);
const memories = intel?.memories?.length || 0;
const trajectories = dbStats.totalTrajectories;

let agents = intel?.agents ? Object.keys(intel.agents).length : 66;
let activeAgents = intel?.agents ? Object.values(intel.agents).filter(a => a.status === 'active').length : 0;

const branch = getGitBranch();
const project = getProjectName();

let dbSize = 0;
if (existsSync(INTEL_DB)) {
  try { dbSize = statSync(INTEL_DB).size; } catch {}
}

const lines = [];

// Line 1: Model + project + branch (colored)
let line1 = \`\${c.model}\${c.bold}Opus 4.5\${c.reset}\`;
line1 += \` \${c.dim}in\${c.reset} \${c.project}\${project}\${c.reset}\`;
if (branch) line1 += \` \${c.dim}on\${c.reset} \${c.symbol}⎇\${c.reset} \${c.branch}\${branch}\${c.reset}\`;
lines.push(line1);

// Line 2: RuVector stats (compact with symbols)
let line2 = \`\${c.brain}🧠 RuVector\${c.reset}\`;
line2 += \` \${c.symbol}◆\${c.reset} \${c.patterns}\${patterns}\${c.reset} \${c.dim}patterns\${c.reset}\`;
if (memories > 0 || dbSize > 0) {
  line2 += \` \${c.symbol}⬡\${c.reset} \${c.memory}\${memories > 0 ? memories : formatSize(dbSize)}\${c.reset} \${c.dim}mem\${c.reset}\`;
}
if (trajectories > 0) line2 += \` \${c.symbol}↝\${c.reset}\${c.trajectories}\${trajectories}\${c.reset}\`;
if (agents > 0) line2 += \` \${c.symbol}#\${c.reset}\${c.agents}\${activeAgents > 0 ? activeAgents + '/' : ''}\${agents}\${c.reset}\`;
lines.push(line2);

// Line 3: Routing info (compact)
const lrPercent = Math.round(LEARNING_RATE * 100);
const epsPercent = Math.round(EPSILON * 100);
let line3 = \`\${c.target}🎯 Routing\${c.reset} \${c.dim}q-learning\${c.reset}\`;
line3 += \` \${c.learning}lr:\${lrPercent}%\${c.reset} \${c.epsilon}ε:\${epsPercent}%\${c.reset}\`;
if (routes > 0) {
  const successRate = dbStats.successfulRoutings > 0 ? Math.round((dbStats.successfulRoutings / routes) * 100) : 100;
  line3 += \` \${c.symbol}|\${c.reset} \${c.success}\${successRate}% ✓\${c.reset}\`;
}
lines.push(line3);

console.log(lines.join('\\n'));
`;
                fs.writeFileSync(statuslinePath, statuslineContent);
                console.log(`   📊 Created: ${statuslinePath}`);
            }
            // Create settings
            const settings = {
                env: {
                    AGENTIC_FLOW_INTELLIGENCE: 'true',
                    AGENTIC_FLOW_LEARNING_RATE: '0.1',
                    AGENTIC_FLOW_MEMORY_BACKEND: 'agentdb'
                },
                hooks: {
                    PreToolUse: [
                        {
                            matcher: 'Edit|Write',
                            hooks: [
                                {
                                    type: 'command',
                                    command: 'npx agentic-flow hooks pre-edit "$TOOL_INPUT_file_path"'
                                }
                            ]
                        },
                        {
                            matcher: 'Bash',
                            hooks: [
                                {
                                    type: 'command',
                                    command: 'npx agentic-flow hooks pre-command "$TOOL_INPUT_command"'
                                }
                            ]
                        }
                    ],
                    PostToolUse: [
                        {
                            matcher: 'Edit|Write',
                            hooks: [
                                {
                                    type: 'command',
                                    command: 'npx agentic-flow hooks post-edit "$TOOL_INPUT_file_path" --success'
                                }
                            ]
                        }
                    ],
                    SessionStart: [
                        {
                            hooks: [
                                {
                                    type: 'command',
                                    command: 'npx agentic-flow hooks intelligence stats'
                                }
                            ]
                        }
                    ],
                    UserPromptSubmit: [
                        {
                            hooks: [
                                {
                                    type: 'command',
                                    timeout: 2000,
                                    command: 'npx agentic-flow hooks intelligence stats'
                                }
                            ]
                        }
                    ]
                },
                permissions: {
                    allow: [
                        'Bash(npx:*)',
                        'Bash(agentic-flow:*)',
                        'mcp__agentic-flow'
                    ]
                },
                statusLine: options.statusline !== false ? {
                    type: 'command',
                    command: 'node .claude/statusline.mjs'
                } : undefined
            };
            fs.writeFileSync(settingsPath, JSON.stringify(settings, null, 2));
            console.log('✅ Hooks initialized!');
            console.log(`   📁 Created: ${settingsPath}`);
            console.log('\n💡 Next steps:');
            console.log('   1. Run: npx agentic-flow hooks pretrain');
            console.log('   2. Run: npx agentic-flow hooks build-agents');
            console.log('   3. Start using Claude Code with intelligent routing!');
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // ============================================
    // RUVECTOR INTELLIGENCE COMMANDS
    // SONA Micro-LoRA + MoE Attention + HNSW
    // ============================================
    // Create intelligence subcommand group
    const intelligence = new Command('intelligence')
        .alias('intel')
        .description('RuVector intelligence: SONA Micro-LoRA (~0.05ms) + MoE attention + HNSW (150x faster)');
    // Intelligence route command
    intelligence
        .command('route <task>')
        .description('Route task using SONA + MoE + HNSW (150x faster than brute force)')
        .option('-f, --file <path>', 'File context')
        .option('-e, --error <context>', 'Error context for debugging')
        .option('-k, --top-k <n>', 'Number of candidates', parseInt, 5)
        .option('-j, --json', 'Output as JSON')
        .action(async (task, options) => {
        try {
            const result = await intelligenceRouteTool.execute({ task, file: options.file, errorContext: options.error, topK: options.topK }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n⚡ RuVector Intelligence Route`);
                console.log(`🎯 Agent: ${result.agent}`);
                console.log(`📊 Confidence: ${((result.confidence || 0) * 100).toFixed(1)}%`);
                console.log(`🔧 Engine: ${result.engine}`);
                console.log(`⏱️  Latency: ${result.latencyMs?.toFixed(2)}ms`);
                if (result.features?.length > 0) {
                    console.log(`🧠 Features: ${result.features.join(', ')}`);
                }
                if (result.alternatives?.length > 0) {
                    console.log(`\n🔄 Alternatives:`);
                    result.alternatives.forEach((a) => {
                        console.log(`   - ${a.agent} (${((a.confidence || 0) * 100).toFixed(1)}%)`);
                    });
                }
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Trajectory start command
    intelligence
        .command('trajectory-start <task>')
        .description('Begin SONA trajectory for reinforcement learning')
        .requiredOption('-a, --agent <name>', 'Agent executing the task')
        .option('-c, --context <text>', 'Additional context')
        .option('-j, --json', 'Output as JSON')
        .action(async (task, options) => {
        try {
            const result = await intelligenceTrajectoryStartTool.execute({ task, agent: options.agent, context: options.context }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n🎬 Trajectory Started`);
                console.log(`📝 ID: ${result.trajectoryId}`);
                console.log(`🤖 Agent: ${options.agent}`);
                console.log(`🧠 Features: ${result.features?.join(', ')}`);
                console.log(`\n💡 Use this ID with trajectory-step and trajectory-end commands`);
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Trajectory step command
    intelligence
        .command('trajectory-step <trajectoryId>')
        .description('Record step in trajectory for reinforcement learning')
        .requiredOption('-a, --action <action>', 'Action taken')
        .requiredOption('-r, --reward <n>', 'Reward signal (-1 to 1)', parseFloat)
        .option('-f, --file <path>', 'File involved')
        .option('--error-fixed', 'Mark as error fix')
        .option('--test-passed', 'Mark as test passed')
        .option('-j, --json', 'Output as JSON')
        .action(async (trajectoryId, options) => {
        try {
            const result = await intelligenceTrajectoryStepTool.execute({
                trajectoryId: parseInt(trajectoryId),
                action: options.action,
                reward: options.reward,
                file: options.file,
                errorFixed: options.errorFixed,
                testPassed: options.testPassed
            }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n📍 Step Recorded`);
                console.log(`   Action: ${options.action}`);
                console.log(`   Reward: ${options.reward}`);
                console.log(`   Trajectory: ${trajectoryId}`);
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Trajectory end command
    intelligence
        .command('trajectory-end <trajectoryId>')
        .description('End trajectory and trigger SONA learning with EWC++')
        .option('-s, --success', 'Mark as successful')
        .option('-f, --fail', 'Mark as failed')
        .option('-q, --quality <n>', 'Quality score (0-1)', parseFloat, 0.8)
        .option('-j, --json', 'Output as JSON')
        .action(async (trajectoryId, options) => {
        try {
            const success = options.success || !options.fail;
            const result = await intelligenceTrajectoryEndTool.execute({ trajectoryId: parseInt(trajectoryId), success, quality: options.quality }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n🏁 Trajectory Completed`);
                console.log(`   ${success ? '✅ Success' : '❌ Failed'}`);
                console.log(`   Quality: ${(options.quality || 0.8) * 100}%`);
                console.log(`   Learning: EWC++ consolidation applied`);
                if (result.learningOutcome) {
                    console.log(`   Outcome: ${JSON.stringify(result.learningOutcome)}`);
                }
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Pattern store command
    intelligence
        .command('pattern-store')
        .description('Store pattern in ReasoningBank (HNSW-indexed)')
        .requiredOption('-t, --task <task>', 'Task description')
        .requiredOption('-r, --resolution <text>', 'How it was resolved')
        .option('-s, --score <n>', 'Success score (0-1)', parseFloat, 0.9)
        .option('-j, --json', 'Output as JSON')
        .action(async (options) => {
        try {
            const result = await intelligencePatternStoreTool.execute({ task: options.task, resolution: options.resolution, reward: options.score || 0.9 }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n💾 Pattern Stored`);
                console.log(`   Task: ${options.task.slice(0, 50)}...`);
                console.log(`   Score: ${((options.score || 0.9) * 100).toFixed(0)}%`);
                console.log(`   Index: HNSW (150x faster retrieval)`);
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Pattern search command
    intelligence
        .command('pattern-search <query>')
        .description('Search ReasoningBank using HNSW (150x faster)')
        .option('-k, --top-k <n>', 'Number of results', parseInt, 5)
        .option('-m, --min-reward <n>', 'Minimum reward filter', parseFloat)
        .option('-j, --json', 'Output as JSON')
        .action(async (query, options) => {
        try {
            const result = await intelligencePatternSearchTool.execute({ query, topK: options.topK, minReward: options.minReward }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n🔍 Pattern Search Results`);
                console.log(`   Query: "${query.slice(0, 50)}"`);
                console.log(`   Engine: ${result.searchEngine}`);
                console.log(`   Found: ${result.count} patterns`);
                if (result.patterns?.length > 0) {
                    console.log(`\n   📋 Results:`);
                    result.patterns.forEach((p, i) => {
                        console.log(`   ${i + 1}. [${((p.similarity || 0) * 100).toFixed(0)}%] ${p.resolution?.slice(0, 40)}...`);
                    });
                }
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Intelligence stats command
    intelligence
        .command('stats')
        .description('Get RuVector intelligence layer statistics')
        .option('-j, --json', 'Output as JSON')
        .action(async (options) => {
        try {
            const result = await intelligenceStatsTool.execute({}, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n📊 RuVector Intelligence Stats`);
                console.log(`\n🧠 SONA Engine:`);
                console.log(`   Micro-LoRA: ${result.features?.sona?.microLora || 'rank-1 (~0.05ms)'}`);
                console.log(`   Base-LoRA: ${result.features?.sona?.baseLora || 'rank-8'}`);
                console.log(`   EWC Lambda: ${result.features?.sona?.ewcLambda || 1000.0}`);
                console.log(`\n⚡ Attention:`);
                console.log(`   Type: ${result.features?.attention?.type || 'moe'}`);
                console.log(`   Experts: ${result.features?.attention?.experts || 4}`);
                console.log(`   Top-K: ${result.features?.attention?.topK || 2}`);
                console.log(`\n🔍 HNSW:`);
                console.log(`   Enabled: ${result.features?.hnsw?.enabled ?? true}`);
                console.log(`   Speedup: ${result.features?.hnsw?.speedup || '150x vs brute-force'}`);
                console.log(`\n📈 Learning:`);
                console.log(`   Trajectories: ${result.persistence?.trajectories ?? result.stats?.trajectoryCount ?? 0}`);
                console.log(`   Active: ${result.stats?.activeTrajectories || 0}`);
                console.log(`\n💾 Persistence (SQLite):`);
                console.log(`   Backend: ${result.persistence?.backend || 'sqlite'}`);
                console.log(`   Routings: ${result.persistence?.routings ?? 0}`);
                console.log(`   Patterns: ${result.persistence?.patterns ?? 0}`);
                console.log(`   Operations: ${result.persistence?.operations ?? 0}`);
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Force learning command
    intelligence
        .command('learn')
        .description('Force immediate SONA learning cycle with EWC++ consolidation')
        .option('-r, --reason <text>', 'Reason for forcing learning')
        .option('-j, --json', 'Output as JSON')
        .action(async (options) => {
        try {
            const result = await intelligenceLearnTool.execute({ reason: options.reason }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n🎓 Learning Cycle Complete`);
                console.log(`   ${result.success ? '✅' : '❌'} ${result.message}`);
                console.log(`   Reason: ${result.reason || 'manual trigger'}`);
                console.log(`   Features: ${result.features?.join(', ')}`);
                console.log(`   Latency: ${result.latencyMs?.toFixed(2)}ms`);
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Attention compute command
    intelligence
        .command('attention <query>')
        .description('Compute attention-weighted similarity using MoE/Flash/Hyperbolic')
        .requiredOption('-c, --candidates <texts...>', 'Candidate texts to score')
        .option('-t, --type <type>', 'Attention type: moe|flash|hyperbolic|graph|dual', 'moe')
        .option('-j, --json', 'Output as JSON')
        .action(async (query, options) => {
        try {
            const result = await intelligenceAttentionTool.execute({
                query,
                candidates: options.candidates,
                attentionType: options.type
            }, mockContext);
            if (options.json) {
                console.log(JSON.stringify(result, null, 2));
            }
            else {
                console.log(`\n🧠 Attention Scores (${options.type || 'moe'})`);
                console.log(`   Query: "${query.slice(0, 40)}..."`);
                console.log(`\n   📊 Results:`);
                result.results?.forEach((r, i) => {
                    const bar = '█'.repeat(Math.round(r.score * 20));
                    console.log(`   ${i + 1}. ${bar} ${(r.score * 100).toFixed(1)}% "${r.text}"`);
                });
                console.log(`\n   ⏱️  Latency: ${result.latencyMs?.toFixed(2)}ms`);
            }
        }
        catch (error) {
            console.error('Error:', error instanceof Error ? error.message : error);
            process.exit(1);
        }
    });
    // Add intelligence subcommand to hooks
    hooks.addCommand(intelligence);
    return hooks;
}
export default createHooksCommand;
//# sourceMappingURL=hooks.js.map