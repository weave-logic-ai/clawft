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

import { attentionCommand } from './commands/attention.js';

export async function handleAttentionCommand(args: string[]): Promise<void> {
  // Dynamically import commander to avoid circular dependencies
  const { Command } = await import('commander');

  // Create a program for attention commands
  const program = new Command('attention');

  // Add the attention command with all its subcommands
  program.addCommand(attentionCommand);

  // Parse the command line arguments
  // We need to prepend 'node' and 'agentdb' to make commander happy
  await program.parseAsync(['node', 'agentdb-attention', ...args], { from: 'user' });
}

/**
 * Help text for attention commands (to be added to main printHelp())
 */
export const attentionHelpText = `
${'\x1b[1m'}ATTENTION MECHANISM COMMANDS:${'\x1b[0m'}
  ${'\x1b[36m'}agentdb attention init${'\x1b[0m'} [--mechanism <type>] [--force]
    Initialize attention configuration
    Options:
      --mechanism <type>  Attention mechanism (flash, hyperbolic, sparse, linear, performer)
      --force             Force overwrite existing configuration
      --json              Output as JSON
    Example: agentdb attention init --mechanism flash

  ${'\x1b[36m'}agentdb attention compute${'\x1b[0m'} --mechanism <type> --query <text> --keys-file <path>
    Compute attention for query-key-value triplets
    Options:
      --mechanism <type>  Attention mechanism (flash, hyperbolic, sparse, linear, performer)
      --query <text>      Query text or vector
      --keys-file <path>  Path to keys JSON file
      --values-file <path> Path to values JSON file (optional)
      --heads <n>         Number of attention heads (default: 8)
      --dimension <n>     Attention dimension (default: 384)
      --output <path>     Output file path
      --json              Output as JSON
    Example: agentdb attention compute --mechanism flash --query "search query" --keys-file keys.json

  ${'\x1b[36m'}agentdb attention benchmark${'\x1b[0m'} [--mechanism <type>] [--all] [--iterations <n>]
    Benchmark attention mechanism performance
    Options:
      --mechanism <type>  Specific mechanism to benchmark
      --all               Benchmark all mechanisms
      --iterations <n>    Number of iterations (default: 100)
      --output <path>     Output file path for results
      --json              Output as JSON
      --verbose           Verbose output
    Example: agentdb attention benchmark --all --iterations 100 --output results.json

  ${'\x1b[36m'}agentdb attention optimize${'\x1b[0m'} --mechanism <type> [--curvature <n>] [--sparsity <n>]
    Optimize attention mechanism parameters
    Options:
      --mechanism <type>  Attention mechanism (hyperbolic, sparse)
      --curvature <n>     Hyperbolic curvature (default: -1.0)
      --sparsity <n>      Sparsity ratio 0-1 (default: 0.9)
      --output <path>     Output file path for optimized config
      --json              Output as JSON
    Example: agentdb attention optimize --mechanism hyperbolic --curvature -1.0
`;

/**
 * Examples for attention commands (to be added to main printHelp())
 */
export const attentionExamples = `
  # Attention Mechanisms: Configure and benchmark
  agentdb attention init --mechanism flash
  agentdb attention compute --mechanism flash --query "search query" --keys-file keys.json
  agentdb attention benchmark --all --iterations 100 --output benchmark.json
  agentdb attention optimize --mechanism hyperbolic --curvature -1.0 --output optimized.json
`;
