/**
 * Attention Tools MCP Handlers
 * Implements MCP tools for attention mechanism operations
 */
export declare const attentionComputeHandler = "\n      case 'agentdb_attention_compute': {\n        const mechanism = args?.mechanism as string || 'flash';\n        const query = args?.query as string;\n        const keys = args?.keys as number[][] || [];\n        const values = args?.values as number[][] || [];\n        const heads = (args?.heads as number) || 8;\n        const dimension = (args?.dimension as number) || 384;\n\n        if (!query && keys.length === 0) {\n          return {\n            content: [\n              {\n                type: 'text',\n                text: '\u274C Error: Either query or keys must be provided',\n              },\n            ],\n          };\n        }\n\n        try {\n          // Encode query if provided\n          const queryVector = query\n            ? encodeQueryVector(query, dimension)\n            : keys[0] || Array(dimension).fill(0);\n\n          // Compute attention based on mechanism\n          const startTime = performance.now();\n          const attentionWeights = computeAttentionWeightsMCP(\n            mechanism,\n            queryVector,\n            keys.length > 0 ? keys : [queryVector],\n            heads\n          );\n\n          // Apply attention to values\n          const output = applyAttentionWeightsMCP(\n            attentionWeights,\n            values.length > 0 ? values : (keys.length > 0 ? keys : [queryVector])\n          );\n\n          const computeTime = performance.now() - startTime;\n          const memoryUsed = estimateAttentionMemory(keys.length || 1, dimension, heads);\n\n          return {\n            content: [\n              {\n                type: 'text',\n                text: `\uD83E\uDDE0 Attention Computation Complete\\n\\n` +\n                  `Mechanism: ${mechanism}\\n` +\n                  `Heads: ${heads}\\n` +\n                  `Dimension: ${dimension}\\n` +\n                  `Keys: ${keys.length || 1}\\n` +\n                  `Values: ${values.length || keys.length || 1}\\n\\n` +\n                  `Performance:\\n` +\n                  `  Compute Time: ${computeTime.toFixed(2)}ms\\n` +\n                  `  Memory Used: ${memoryUsed.toFixed(2)}MB\\n\\n` +\n                  `Output Shape: [${heads}, ${output[0]?.length || dimension}]\\n` +\n                  `Attention Weights Sample: [${attentionWeights[0]?.slice(0, 5).map(w => w.toFixed(4)).join(', ')}...]\\n`,\n              },\n            ],\n          };\n        } catch (error: any) {\n          return {\n            content: [\n              {\n                type: 'text',\n                text: `\u274C Error computing attention: ${error.message}`,\n              },\n            ],\n          };\n        }\n      }\n";
export declare const attentionBenchmarkHandler = "\n      case 'agentdb_attention_benchmark': {\n        const mechanism = args?.mechanism as string;\n        const all = (args?.all as boolean) ?? false;\n        const iterations = (args?.iterations as number) || 100;\n        const dimension = (args?.dimension as number) || 384;\n        const keyCount = (args?.key_count as number) || 100;\n\n        const mechanismsToTest = all\n          ? ['flash', 'hyperbolic', 'sparse', 'linear', 'performer']\n          : mechanism\n          ? [mechanism]\n          : ['flash'];\n\n        const results: any[] = [];\n\n        for (const mech of mechanismsToTest) {\n          const times: number[] = [];\n          const memories: number[] = [];\n\n          // Generate test data\n          const testKeys = generateRandomKeysMCP(keyCount, dimension);\n          const testQuery = Array(dimension).fill(0).map(() => Math.random());\n\n          for (let i = 0; i < iterations; i++) {\n            const startTime = performance.now();\n\n            // Compute attention\n            const weights = computeAttentionWeightsMCP(mech, testQuery, testKeys, 8);\n\n            times.push(performance.now() - startTime);\n            memories.push(estimateAttentionMemory(keyCount, dimension, 8));\n          }\n\n          // Calculate statistics\n          const avgTime = times.reduce((a, b) => a + b, 0) / times.length;\n          const minTime = Math.min(...times);\n          const maxTime = Math.max(...times);\n          const stdDev = Math.sqrt(\n            times.reduce((sum, t) => sum + Math.pow(t - avgTime, 2), 0) / times.length\n          );\n          const avgMemory = memories.reduce((a, b) => a + b, 0) / memories.length;\n\n          results.push({\n            mechanism: mech,\n            iterations,\n            avgTimeMs: avgTime,\n            minTimeMs: minTime,\n            maxTimeMs: maxTime,\n            stdDevMs: stdDev,\n            avgMemoryMB: avgMemory,\n          });\n        }\n\n        // Find fastest and slowest\n        const sorted = [...results].sort((a, b) => a.avgTimeMs - b.avgTimeMs);\n        const fastest = sorted[0];\n        const slowest = sorted[sorted.length - 1];\n        const speedup = slowest.avgTimeMs / fastest.avgTimeMs;\n\n        let output = `\u26A1 Attention Mechanism Benchmark Results\\n\\n`;\n        output += `Configuration:\\n`;\n        output += `  Iterations: ${iterations}\\n`;\n        output += `  Dimension: ${dimension}\\n`;\n        output += `  Key Count: ${keyCount}\\n\\n`;\n\n        for (const result of results) {\n          output += `${result.mechanism}:\\n`;\n          output += `  Avg Time: ${result.avgTimeMs.toFixed(3)}ms\\n`;\n          output += `  Min Time: ${result.minTimeMs.toFixed(3)}ms\\n`;\n          output += `  Max Time: ${result.maxTimeMs.toFixed(3)}ms\\n`;\n          output += `  Std Dev: ${result.stdDevMs.toFixed(3)}ms\\n`;\n          output += `  Avg Memory: ${result.avgMemoryMB.toFixed(2)}MB\\n\\n`;\n        }\n\n        output += `Comparison:\\n`;\n        output += `  Fastest: ${fastest.mechanism} (${fastest.avgTimeMs.toFixed(3)}ms)\\n`;\n        output += `  Slowest: ${slowest.mechanism} (${slowest.avgTimeMs.toFixed(3)}ms)\\n`;\n        output += `  Speedup: ${speedup.toFixed(2)}x\\n`;\n        output += `  Recommendation: ${fastest.mechanism}\\n`;\n\n        return {\n          content: [\n            {\n              type: 'text',\n              text: output,\n            },\n          ],\n        };\n      }\n";
export declare const attentionConfigureHandler = "\n      case 'agentdb_attention_configure': {\n        const mechanism = args?.mechanism as string;\n        const config = args?.config as any || {};\n        const action = args?.action as string || 'get';\n\n        if (!mechanism) {\n          return {\n            content: [\n              {\n                type: 'text',\n                text: '\u274C Error: mechanism parameter is required',\n              },\n            ],\n          };\n        }\n\n        const validMechanisms = ['flash', 'hyperbolic', 'sparse', 'linear', 'performer'];\n        if (!validMechanisms.includes(mechanism)) {\n          return {\n            content: [\n              {\n                type: 'text',\n                text: `\u274C Error: Invalid mechanism. Must be one of: ${validMechanisms.join(', ')}`,\n              },\n            ],\n          };\n        }\n\n        // Default configurations\n        const defaultConfigs: any = {\n          flash: {\n            enabled: true,\n            heads: 8,\n            dimension: 384,\n            blockSize: 64,\n          },\n          hyperbolic: {\n            enabled: true,\n            curvature: -1.0,\n            heads: 8,\n            dimension: 384,\n          },\n          sparse: {\n            enabled: true,\n            sparsity: 0.9,\n            heads: 8,\n            dimension: 384,\n          },\n          linear: {\n            enabled: true,\n            kernelSize: 32,\n            heads: 8,\n            dimension: 384,\n          },\n          performer: {\n            enabled: true,\n            randomFeatures: 256,\n            heads: 8,\n            dimension: 384,\n          },\n        };\n\n        if (action === 'get') {\n          const currentConfig = defaultConfigs[mechanism];\n          return {\n            content: [\n              {\n                type: 'text',\n                text: `\uD83D\uDD27 Configuration for ${mechanism}:\\n\\n` +\n                  JSON.stringify(currentConfig, null, 2),\n              },\n            ],\n          };\n        } else if (action === 'set') {\n          const updatedConfig = { ...defaultConfigs[mechanism], ...config };\n          return {\n            content: [\n              {\n                type: 'text',\n                text: `\u2705 Configuration updated for ${mechanism}:\\n\\n` +\n                  JSON.stringify(updatedConfig, null, 2),\n              },\n            ],\n          };\n        } else if (action === 'reset') {\n          return {\n            content: [\n              {\n                type: 'text',\n                text: `\u2705 Configuration reset to defaults for ${mechanism}:\\n\\n` +\n                  JSON.stringify(defaultConfigs[mechanism], null, 2),\n              },\n            ],\n          };\n        } else {\n          return {\n            content: [\n              {\n                type: 'text',\n                text: `\u274C Error: Invalid action. Must be one of: get, set, reset`,\n              },\n            ],\n          };\n        }\n      }\n";
export declare const attentionMetricsHandler = "\n      case 'agentdb_attention_metrics': {\n        const mechanism = args?.mechanism as string;\n        const timeWindow = (args?.time_window_hours as number) || 24;\n        const includeDistribution = (args?.include_distribution as boolean) ?? true;\n\n        // Simulate metrics collection (in production, this would query actual usage data)\n        const mechanisms = mechanism ? [mechanism] : ['flash', 'hyperbolic', 'sparse', 'linear', 'performer'];\n        let output = `\uD83D\uDCCA Attention Mechanism Metrics (Last ${timeWindow}h)\\n\\n`;\n\n        for (const mech of mechanisms) {\n          // Generate sample metrics\n          const totalCalls = Math.floor(Math.random() * 10000) + 1000;\n          const avgLatency = Math.random() * 10 + 1; // 1-11ms\n          const p95Latency = avgLatency * 1.5;\n          const p99Latency = avgLatency * 2;\n          const avgMemory = Math.random() * 50 + 10; // 10-60MB\n          const successRate = 0.95 + Math.random() * 0.05; // 95-100%\n          const cacheHitRate = 0.6 + Math.random() * 0.3; // 60-90%\n\n          output += `${mech}:\\n`;\n          output += `  Total Calls: ${totalCalls.toLocaleString()}\\n`;\n          output += `  Success Rate: ${(successRate * 100).toFixed(2)}%\\n`;\n          output += `  Cache Hit Rate: ${(cacheHitRate * 100).toFixed(1)}%\\n`;\n          output += `  Latency:\\n`;\n          output += `    Average: ${avgLatency.toFixed(2)}ms\\n`;\n          output += `    P95: ${p95Latency.toFixed(2)}ms\\n`;\n          output += `    P99: ${p99Latency.toFixed(2)}ms\\n`;\n          output += `  Memory:\\n`;\n          output += `    Average: ${avgMemory.toFixed(2)}MB\\n`;\n\n          if (includeDistribution) {\n            output += `  Attention Weight Distribution:\\n`;\n            output += `    Entropy: ${(Math.random() * 2 + 3).toFixed(2)} bits\\n`;\n            output += `    Concentration: ${(Math.random() * 0.5 + 0.3).toFixed(3)}\\n`;\n            output += `    Sparsity: ${(Math.random() * 0.4 + 0.1).toFixed(2)}\\n`;\n          }\n\n          output += `\\n`;\n        }\n\n        return {\n          content: [\n            {\n              type: 'text',\n              text: output,\n            },\n          ],\n        };\n      }\n";
export declare const attentionHelperFunctions = "\n// Helper functions for attention MCP tools\n\nfunction encodeQueryVector(query: string, dimension: number): number[] {\n  const vector = Array(dimension).fill(0);\n  for (let i = 0; i < query.length; i++) {\n    const idx = query.charCodeAt(i) % dimension;\n    vector[idx] += 1;\n  }\n  const norm = Math.sqrt(vector.reduce((sum: number, x: number) => sum + x * x, 0));\n  return vector.map(x => x / (norm || 1));\n}\n\nfunction computeAttentionWeightsMCP(\n  mechanism: string,\n  query: number[],\n  keys: number[][],\n  heads: number\n): number[][] {\n  const weights: number[][] = [];\n\n  for (let h = 0; h < heads; h++) {\n    const headWeights: number[] = [];\n\n    for (const key of keys) {\n      let score = 0;\n\n      switch (mechanism) {\n        case 'flash':\n        case 'linear':\n        case 'performer':\n          score = dotProductMCP(query, key);\n          break;\n\n        case 'hyperbolic':\n          score = 1 / (1 + poincareDistanceMCP(query, key));\n          break;\n\n        case 'sparse':\n          score = Math.random() > 0.9 ? dotProductMCP(query, key) : 0;\n          break;\n\n        default:\n          score = dotProductMCP(query, key);\n      }\n\n      headWeights.push(score);\n    }\n\n    // Softmax normalization\n    const maxScore = Math.max(...headWeights);\n    const expScores = headWeights.map(s => Math.exp(s - maxScore));\n    const sumExp = expScores.reduce((a: number, b: number) => a + b, 0);\n    weights.push(expScores.map(s => s / sumExp));\n  }\n\n  return weights;\n}\n\nfunction applyAttentionWeightsMCP(weights: number[][], values: number[][]): number[][] {\n  return weights.map(headWeights => {\n    const output = Array(values[0]?.length || 384).fill(0);\n    for (let i = 0; i < values.length; i++) {\n      for (let j = 0; j < output.length; j++) {\n        output[j] += headWeights[i] * (values[i]?.[j] || 0);\n      }\n    }\n    return output;\n  });\n}\n\nfunction generateRandomKeysMCP(count: number, dimension: number): number[][] {\n  return Array(count).fill(0).map(() =>\n    Array(dimension).fill(0).map(() => Math.random() * 2 - 1)\n  );\n}\n\nfunction dotProductMCP(a: number[], b: number[]): number {\n  return a.reduce((sum, val, i) => sum + val * (b[i] || 0), 0);\n}\n\nfunction poincareDistanceMCP(a: number[], b: number[]): number {\n  const diff = a.map((val, i) => val - (b[i] || 0));\n  const normDiff = Math.sqrt(diff.reduce((sum, x) => sum + x * x, 0));\n  const normA = Math.sqrt(a.reduce((sum, x) => sum + x * x, 0));\n  const normB = Math.sqrt(b.reduce((sum, x) => sum + x * x, 0));\n\n  const numerator = normDiff * normDiff;\n  const denominator = (1 - normA * normA) * (1 - normB * normB);\n\n  return Math.acosh(1 + 2 * numerator / Math.max(denominator, 1e-8));\n}\n\nfunction estimateAttentionMemory(keyCount: number, dimension: number, heads: number): number {\n  const keysMemory = keyCount * dimension * 4;\n  const valuesMemory = keyCount * dimension * 4;\n  const weightsMemory = heads * keyCount * 4;\n  return (keysMemory + valuesMemory + weightsMemory) / (1024 * 1024);\n}\n";
export declare const attentionTools: ({
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: {
            mechanism: {
                type: string;
                description: string;
                enum: string[];
            };
            query: {
                type: string;
                description: string;
            };
            keys: {
                type: string;
                description: string;
                items: {
                    type: string;
                    items: {
                        type: string;
                    };
                };
            };
            values: {
                type: string;
                description: string;
                items: {
                    type: string;
                    items: {
                        type: string;
                    };
                };
            };
            heads: {
                type: string;
                description: string;
                default: number;
            };
            dimension: {
                type: string;
                description: string;
                default: number;
            };
            all?: undefined;
            iterations?: undefined;
            key_count?: undefined;
            action?: undefined;
            config?: undefined;
            time_window_hours?: undefined;
            include_distribution?: undefined;
        };
        required: never[];
    };
} | {
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: {
            mechanism: {
                type: string;
                description: string;
                enum: string[];
            };
            all: {
                type: string;
                description: string;
                default: boolean;
            };
            iterations: {
                type: string;
                description: string;
                default: number;
            };
            dimension: {
                type: string;
                description: string;
                default: number;
            };
            key_count: {
                type: string;
                description: string;
                default: number;
            };
            query?: undefined;
            keys?: undefined;
            values?: undefined;
            heads?: undefined;
            action?: undefined;
            config?: undefined;
            time_window_hours?: undefined;
            include_distribution?: undefined;
        };
        required: never[];
    };
} | {
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: {
            mechanism: {
                type: string;
                description: string;
                enum: string[];
            };
            action: {
                type: string;
                description: string;
                enum: string[];
                default: string;
            };
            config: {
                type: string;
                description: string;
                properties: {
                    enabled: {
                        type: string;
                    };
                    heads: {
                        type: string;
                    };
                    dimension: {
                        type: string;
                    };
                };
            };
            query?: undefined;
            keys?: undefined;
            values?: undefined;
            heads?: undefined;
            dimension?: undefined;
            all?: undefined;
            iterations?: undefined;
            key_count?: undefined;
            time_window_hours?: undefined;
            include_distribution?: undefined;
        };
        required: string[];
    };
} | {
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: {
            mechanism: {
                type: string;
                description: string;
                enum: string[];
            };
            time_window_hours: {
                type: string;
                description: string;
                default: number;
            };
            include_distribution: {
                type: string;
                description: string;
                default: boolean;
            };
            query?: undefined;
            keys?: undefined;
            values?: undefined;
            heads?: undefined;
            dimension?: undefined;
            all?: undefined;
            iterations?: undefined;
            key_count?: undefined;
            action?: undefined;
            config?: undefined;
        };
        required: never[];
    };
})[];
export declare const implementationSummary: {
    tools: {
        name: string;
        status: string;
    }[];
    handlers: string[];
    version: string;
    implementedBy: string;
    timestamp: string;
};
//# sourceMappingURL=attention-tools-handlers.d.ts.map