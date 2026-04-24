/**
 * Attention CLI Integration Helper
 * This file provides the integration code for adding attention commands to agentdb-cli.ts
 *
 * To integrate, add the following to agentdb-cli.ts:
 *
 * 1. Import (near top with other command imports):
 *    import { handleAttentionCommand } from './attention-cli-integration.js';
 *
 * 2. In main() function (after stats command):
 *    if (command === 'attention') {
 *      await handleAttentionCommand(args.slice(1));
 *      return;
 *    }
 */
export declare function handleAttentionCommand(args: string[]): Promise<void>;
/**
 * Help text for attention commands (to be added to main printHelp())
 */
export declare const attentionHelpText = "\n\u001B[1mATTENTION MECHANISM COMMANDS:\u001B[0m\n  \u001B[36magentdb attention init\u001B[0m [--mechanism <type>] [--force]\n    Initialize attention configuration\n    Options:\n      --mechanism <type>  Attention mechanism (flash, hyperbolic, sparse, linear, performer)\n      --force             Force overwrite existing configuration\n      --json              Output as JSON\n    Example: agentdb attention init --mechanism flash\n\n  \u001B[36magentdb attention compute\u001B[0m --mechanism <type> --query <text> --keys-file <path>\n    Compute attention for query-key-value triplets\n    Options:\n      --mechanism <type>  Attention mechanism (flash, hyperbolic, sparse, linear, performer)\n      --query <text>      Query text or vector\n      --keys-file <path>  Path to keys JSON file\n      --values-file <path> Path to values JSON file (optional)\n      --heads <n>         Number of attention heads (default: 8)\n      --dimension <n>     Attention dimension (default: 384)\n      --output <path>     Output file path\n      --json              Output as JSON\n    Example: agentdb attention compute --mechanism flash --query \"search query\" --keys-file keys.json\n\n  \u001B[36magentdb attention benchmark\u001B[0m [--mechanism <type>] [--all] [--iterations <n>]\n    Benchmark attention mechanism performance\n    Options:\n      --mechanism <type>  Specific mechanism to benchmark\n      --all               Benchmark all mechanisms\n      --iterations <n>    Number of iterations (default: 100)\n      --output <path>     Output file path for results\n      --json              Output as JSON\n      --verbose           Verbose output\n    Example: agentdb attention benchmark --all --iterations 100 --output results.json\n\n  \u001B[36magentdb attention optimize\u001B[0m --mechanism <type> [--curvature <n>] [--sparsity <n>]\n    Optimize attention mechanism parameters\n    Options:\n      --mechanism <type>  Attention mechanism (hyperbolic, sparse)\n      --curvature <n>     Hyperbolic curvature (default: -1.0)\n      --sparsity <n>      Sparsity ratio 0-1 (default: 0.9)\n      --output <path>     Output file path for optimized config\n      --json              Output as JSON\n    Example: agentdb attention optimize --mechanism hyperbolic --curvature -1.0\n";
/**
 * Examples for attention commands (to be added to main printHelp())
 */
export declare const attentionExamples = "\n  # Attention Mechanisms: Configure and benchmark\n  agentdb attention init --mechanism flash\n  agentdb attention compute --mechanism flash --query \"search query\" --keys-file keys.json\n  agentdb attention benchmark --all --iterations 100 --output benchmark.json\n  agentdb attention optimize --mechanism hyperbolic --curvature -1.0 --output optimized.json\n";
//# sourceMappingURL=attention-cli-integration.d.ts.map