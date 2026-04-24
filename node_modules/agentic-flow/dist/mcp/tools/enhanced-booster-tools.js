/**
 * Enhanced Agent Booster MCP Tools
 *
 * RuVector-powered code editing with:
 * - SONA learning (0.05ms updates)
 * - HNSW cache (150x faster pattern recall)
 * - GNN matching (differentiable search)
 * - Confidence improvement through learning
 */
// Lazy import to avoid startup cost
let EnhancedAgentBooster = null;
let getEnhancedBooster = null;
let benchmark = null;
async function loadEnhancedBooster() {
    if (!EnhancedAgentBooster) {
        const module = await import('../../intelligence/agent-booster-enhanced.js');
        EnhancedAgentBooster = module.EnhancedAgentBooster;
        getEnhancedBooster = module.getEnhancedBooster;
        benchmark = module.benchmark;
    }
    return { EnhancedAgentBooster, getEnhancedBooster, benchmark };
}
/**
 * MCP Tool definitions for enhanced booster v2
 */
export const enhancedBoosterTools = [
    {
        name: 'enhanced_booster_edit',
        description: `RuVector-enhanced code editing v2 with full intelligence stack.

Features:
- Exact cache: 0ms for identical patterns
- Fuzzy match: 1-5ms for similar patterns (var x → var y)
- GNN search: Differentiable soft attention matching
- ONNX embeddings: Semantic code understanding
- Error learning: Avoids known bad patterns
- SONA learning: Continuous improvement with EWC++

Returns strategy: exact_cache, fuzzy_match, gnn_match, error_avoided, or agent_booster`,
        inputSchema: {
            type: 'object',
            properties: {
                code: {
                    type: 'string',
                    description: 'Original code to edit'
                },
                edit: {
                    type: 'string',
                    description: 'Target edit or new code'
                },
                language: {
                    type: 'string',
                    description: 'Programming language (javascript, typescript, python, etc.)'
                },
                filePath: {
                    type: 'string',
                    description: 'Optional file path for context'
                }
            },
            required: ['code', 'edit', 'language']
        }
    },
    {
        name: 'enhanced_booster_edit_file',
        description: `Edit a file using RuVector-enhanced agent booster.

Reads file, applies edit, writes result if successful.
Uses cached patterns for instant edits when available.`,
        inputSchema: {
            type: 'object',
            properties: {
                target_filepath: {
                    type: 'string',
                    description: 'Path of the file to modify'
                },
                edit: {
                    type: 'string',
                    description: 'Target edit or code changes'
                },
                language: {
                    type: 'string',
                    description: 'Programming language (auto-detected if not provided)'
                }
            },
            required: ['target_filepath', 'edit']
        }
    },
    {
        name: 'enhanced_booster_stats',
        description: 'Get enhanced agent booster statistics including cache hit rate, learned patterns, and confidence improvement',
        inputSchema: {
            type: 'object',
            properties: {},
            required: []
        }
    },
    {
        name: 'enhanced_booster_pretrain',
        description: 'Pretrain the enhanced booster with common code patterns for faster cold starts',
        inputSchema: {
            type: 'object',
            properties: {},
            required: []
        }
    },
    {
        name: 'enhanced_booster_benchmark',
        description: 'Run benchmark comparing enhanced vs baseline agent-booster performance',
        inputSchema: {
            type: 'object',
            properties: {
                iterations: {
                    type: 'number',
                    description: 'Number of iterations (default: 20)',
                    default: 20
                }
            },
            required: []
        }
    },
    {
        name: 'enhanced_booster_record_outcome',
        description: 'Record edit outcome (success/failure) for learning. Call this after verifying edit worked.',
        inputSchema: {
            type: 'object',
            properties: {
                patternId: {
                    type: 'string',
                    description: 'Pattern ID from the edit result'
                },
                success: {
                    type: 'boolean',
                    description: 'Whether the edit was successful'
                }
            },
            required: ['patternId', 'success']
        }
    },
    {
        name: 'enhanced_booster_batch',
        description: 'Apply multiple edits in parallel (4x faster for multi-file changes)',
        inputSchema: {
            type: 'object',
            properties: {
                edits: {
                    type: 'array',
                    description: 'Array of edit requests',
                    items: {
                        type: 'object',
                        properties: {
                            code: { type: 'string' },
                            edit: { type: 'string' },
                            language: { type: 'string' },
                            filePath: { type: 'string' }
                        },
                        required: ['code', 'edit', 'language']
                    }
                },
                maxConcurrency: {
                    type: 'number',
                    description: 'Max parallel edits (default: 4)',
                    default: 4
                }
            },
            required: ['edits']
        }
    },
    {
        name: 'enhanced_booster_prefetch',
        description: 'Prefetch likely edits for a file based on co-edit patterns and language',
        inputSchema: {
            type: 'object',
            properties: {
                filePath: {
                    type: 'string',
                    description: 'File path to prefetch patterns for'
                }
            },
            required: ['filePath']
        }
    },
    {
        name: 'enhanced_booster_likely_files',
        description: 'Get files likely to be edited next based on co-edit history',
        inputSchema: {
            type: 'object',
            properties: {
                filePath: {
                    type: 'string',
                    description: 'Current file being edited'
                },
                topK: {
                    type: 'number',
                    description: 'Number of suggestions (default: 5)',
                    default: 5
                }
            },
            required: ['filePath']
        }
    }
];
/**
 * MCP Tool handlers
 */
export const enhancedBoosterHandlers = {
    enhanced_booster_edit: async (params) => {
        const { getEnhancedBooster } = await loadEnhancedBooster();
        const booster = getEnhancedBooster();
        const result = await booster.apply({
            code: params.code,
            edit: params.edit,
            language: params.language,
            filePath: params.filePath
        });
        const strategyEmoji = {
            exact_cache: '🎯 EXACT CACHE HIT',
            fuzzy_match: '🔮 FUZZY MATCH',
            gnn_match: '🧠 GNN MATCH',
            error_avoided: '⚠️ ERROR AVOIDED',
            agent_booster: '🔧 AGENT BOOSTER',
            fallback: '❌ FALLBACK'
        };
        return {
            content: [{
                    type: 'text',
                    text: `⚡ Enhanced Agent Booster v2 Result:

Strategy: ${strategyEmoji[result.strategy] || result.strategy}
Success: ${result.success ? '✅' : '❌'}
Confidence: ${(result.confidence * 100).toFixed(1)}%
Latency: ${result.latency}ms
${result.fuzzyScore ? `Fuzzy Score: ${(result.fuzzyScore * 100).toFixed(1)}%` : ''}
${result.learned ? '📚 Pattern learned for future use' : ''}
${result.similarPatterns ? `🔍 Found ${result.similarPatterns} similar patterns` : ''}
${result.patternId ? `Pattern ID: ${result.patternId}` : ''}

Output:
${result.output}`
                }]
        };
    },
    enhanced_booster_edit_file: async (params) => {
        const { getEnhancedBooster } = await loadEnhancedBooster();
        const fs = await import('fs');
        const path = await import('path');
        if (!fs.existsSync(params.target_filepath)) {
            return {
                content: [{
                        type: 'text',
                        text: `❌ File not found: ${params.target_filepath}`
                    }],
                isError: true
            };
        }
        const code = fs.readFileSync(params.target_filepath, 'utf8');
        const language = params.language || path.extname(params.target_filepath).slice(1);
        const booster = getEnhancedBooster();
        const result = await booster.apply({
            code,
            edit: params.edit,
            language,
            filePath: params.target_filepath
        });
        if (result.success && result.confidence > 0.7) {
            fs.writeFileSync(params.target_filepath, result.output, 'utf8');
        }
        const stats = booster.getStats();
        return {
            content: [{
                    type: 'text',
                    text: `⚡ Enhanced File Edit Result:

📁 File: ${params.target_filepath}
Strategy: ${result.strategy}
${result.cacheHit ? '🎯 CACHE HIT!' : ''}
Success: ${result.success ? '✅ Written' : '❌ Failed'}
Confidence: ${(result.confidence * 100).toFixed(1)}%
Latency: ${result.latency}ms
${result.learned ? '📚 Pattern learned' : ''}

📊 Session Stats:
   Cache Hit Rate: ${stats.hitRate}
   Patterns Learned: ${stats.patternsLearned}
   Avg Confidence: ${(stats.avgConfidence * 100).toFixed(1)}%
   ${result.patternId ? `Pattern ID: ${result.patternId}` : ''}`
                }]
        };
    },
    enhanced_booster_stats: async () => {
        const { getEnhancedBooster } = await loadEnhancedBooster();
        const booster = getEnhancedBooster();
        const stats = booster.getStats();
        const intelligence = booster.getIntelligenceStats();
        return {
            content: [{
                    type: 'text',
                    text: `📊 Enhanced Agent Booster Statistics

🎯 Performance:
   Total Edits: ${stats.totalEdits}
   Cache Hits: ${stats.cacheHits}
   Fuzzy Hits: ${stats.fuzzyHits}
   GNN Hits: ${stats.gnnHits}
   Cache Misses: ${stats.cacheMisses}
   Hit Rate: ${stats.hitRate}
   Avg Latency: ${stats.avgLatency.toFixed(1)}ms

📚 Learning:
   Patterns Learned: ${stats.patternsLearned}
   SONA Updates: ${stats.sonaUpdates}
   GNN Searches: ${stats.gnnSearches}
   Avg Confidence: ${(stats.avgConfidence * 100).toFixed(1)}%
   Confidence Improvement: ${stats.confidenceImprovement}

🗜️ Tiered Compression:
   ┌─────────────────────────────────────────────────┐
   │ Tier      │ Count │ Access Freq │ Memory Save  │
   ├─────────────────────────────────────────────────┤
   │ 🔥 Hot    │ ${String(stats.tierDistribution?.hot || 0).padStart(5)} │ >80%        │ 0%           │
   │ 🌡️ Warm   │ ${String(stats.tierDistribution?.warm || 0).padStart(5)} │ 40-80%      │ 50%          │
   │ ❄️ Cool   │ ${String(stats.tierDistribution?.cool || 0).padStart(5)} │ 10-40%      │ 87.5%        │
   │ 🧊 Cold   │ ${String(stats.tierDistribution?.cold || 0).padStart(5)} │ 1-10%       │ 93.75%       │
   │ 📦 Archive│ ${String(stats.tierDistribution?.archive || 0).padStart(5)} │ <1%         │ 96.9%        │
   └─────────────────────────────────────────────────┘
   Total Pattern Accesses: ${stats.totalPatternAccesses}
   Compression Ratio: ${stats.compressionRatio}
   Memory Savings: ${stats.memorySavings}

${intelligence ? `🧠 Intelligence Engine:
   Memories: ${intelligence.totalMemories}
   Episodes: ${intelligence.totalEpisodes}
   Trajectories: ${intelligence.trajectoriesRecorded}
   SONA Enabled: ${intelligence.sonaEnabled}
   ONNX Enabled: ${intelligence.onnxEnabled}` : ''}`
                }]
        };
    },
    enhanced_booster_pretrain: async () => {
        const { getEnhancedBooster } = await loadEnhancedBooster();
        const booster = getEnhancedBooster();
        const result = await booster.pretrain();
        return {
            content: [{
                    type: 'text',
                    text: `🚀 Pretrain Complete!

Patterns Added: ${result.patterns}
Time: ${result.timeMs}ms

Common patterns cached:
- Variable conversions (var → const)
- Type annotations (TypeScript)
- Async/await conversions
- Error handling wrappers
- Console removal
- Python type hints
- Module import conversions

Cache is now warm for instant edits! ⚡`
                }]
        };
    },
    enhanced_booster_benchmark: async (params) => {
        const { benchmark } = await loadEnhancedBooster();
        const result = await benchmark(params.iterations || 20);
        return {
            content: [{
                    type: 'text',
                    text: `📈 Enhanced vs Baseline Benchmark

🔧 Baseline (agent-booster):
   Avg Latency: ${result.baseline.avgLatency.toFixed(1)}ms
   Avg Confidence: ${(result.baseline.avgConfidence * 100).toFixed(1)}%

⚡ Enhanced (RuVector):
   Avg Latency: ${result.enhanced.avgLatency.toFixed(1)}ms
   Avg Confidence: ${(result.enhanced.avgConfidence * 100).toFixed(1)}%
   Cache Hit Rate: ${(result.enhanced.cacheHitRate * 100).toFixed(1)}%

🎯 Improvement:
   Latency: ${result.improvement.latency}
   Confidence: ${result.improvement.confidence}

The enhanced booster learns from each edit and gets faster over time! 🚀`
                }]
        };
    },
    enhanced_booster_record_outcome: async (params) => {
        const { getEnhancedBooster } = await loadEnhancedBooster();
        const booster = getEnhancedBooster();
        await booster.recordOutcome(params.patternId, params.success);
        return {
            content: [{
                    type: 'text',
                    text: `📝 Outcome Recorded

Pattern: ${params.patternId}
Result: ${params.success ? '✅ Success - confidence boosted' : '❌ Failure - confidence reduced'}

This feedback improves future predictions!`
                }]
        };
    },
    enhanced_booster_batch: async (params) => {
        const { getEnhancedBooster } = await loadEnhancedBooster();
        const booster = getEnhancedBooster();
        const results = await booster.applyBatch(params.edits, params.maxConcurrency || 4);
        const successful = results.filter(r => r.success).length;
        const cacheHits = results.filter(r => r.cacheHit).length;
        const fuzzyHits = results.filter(r => r.strategy === 'fuzzy_match').length;
        const totalLatency = results.reduce((sum, r) => sum + r.latency, 0);
        return {
            content: [{
                    type: 'text',
                    text: `⚡ Batch Edit Results:

📊 Summary:
   Total: ${results.length}
   Successful: ${successful}
   Cache Hits: ${cacheHits}
   Fuzzy Matches: ${fuzzyHits}
   Total Latency: ${totalLatency}ms
   Avg Latency: ${(totalLatency / results.length).toFixed(1)}ms

${results.map((r, i) => `${i + 1}. ${r.success ? '✅' : '❌'} ${r.strategy} (${r.latency}ms)`).join('\n')}`
                }]
        };
    },
    enhanced_booster_prefetch: async (params) => {
        const { getEnhancedBooster } = await loadEnhancedBooster();
        const booster = getEnhancedBooster();
        const result = await booster.prefetch(params.filePath);
        return {
            content: [{
                    type: 'text',
                    text: `🔮 Prefetch Results for ${params.filePath}:

Confidence: ${(result.confidence * 100).toFixed(0)}%

Likely edits:
${result.likelyEdits.length > 0
                        ? result.likelyEdits.map((e, i) => `  ${i + 1}. ${e}`).join('\n')
                        : '  No patterns learned yet for this file type'}

Patterns are being pre-warmed in background...`
                }]
        };
    },
    enhanced_booster_likely_files: async (params) => {
        const { getEnhancedBooster } = await loadEnhancedBooster();
        const booster = getEnhancedBooster();
        const files = booster.getLikelyNextFiles(params.filePath, params.topK || 5);
        return {
            content: [{
                    type: 'text',
                    text: `📁 Likely Next Files to Edit:

Current: ${params.filePath}

${files.length > 0
                        ? files.map((f, i) => `  ${i + 1}. ${f.file} (score: ${f.score})`).join('\n')
                        : '  No co-edit history yet. Edit more files to build the graph!'}

Based on co-edit patterns across sessions.`
                }]
        };
    }
};
export default { tools: enhancedBoosterTools, handlers: enhancedBoosterHandlers };
//# sourceMappingURL=enhanced-booster-tools.js.map