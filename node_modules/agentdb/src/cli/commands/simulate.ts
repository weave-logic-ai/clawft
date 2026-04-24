/**
 * Main simulate command
 * Entry point for AgentDB latent space simulations
 */

import { Command } from 'commander';
import { HelpFormatter } from '../lib/help-formatter.js';
import { SimulationRunner } from '../lib/simulation-runner.js';
import { ReportGenerator } from '../lib/report-generator.js';
import chalk from 'chalk';

export const simulateCommand = new Command('simulate')
  .description('Run AgentDB latent space simulations')
  .argument('[scenario]', 'Simulation scenario to run (hnsw, attention, clustering, traversal, hypergraph, self-organizing, neural, quantum)')
  .option('--iterations <n>', 'Number of runs (default: 3)', '3')
  .option('--output <path>', 'Report output path (default: ./reports)')
  .option('--format <type>', 'Report format: md, json, html (default: md)', 'md')
  .option('--verbose', 'Detailed output')
  .option('--wizard', 'Interactive wizard mode')
  .option('--custom', 'Custom simulation builder')
  .option('--list', 'List all scenarios')
  .option('--report <id>', 'View simulation report by ID')
  .action(async (scenario, options) => {
    try {
      // Handle special modes
      if (options.list) {
        console.log(HelpFormatter.formatList());
        return;
      }

      if (options.wizard) {
        const { runWizard } = await import('./simulate-wizard.js');
        await runWizard();
        return;
      }

      if (options.custom) {
        const { runCustomBuilder } = await import('./simulate-custom.js');
        await runCustomBuilder();
        return;
      }

      if (options.report) {
        const { viewReport } = await import('./simulate-report.js');
        await viewReport(options.report);
        return;
      }

      // Validate scenario
      const validScenarios = ['hnsw', 'attention', 'clustering', 'traversal', 'hypergraph', 'self-organizing', 'neural', 'quantum'];

      if (!scenario) {
        console.log(HelpFormatter.formatTopLevel());
        return;
      }

      if (!validScenarios.includes(scenario)) {
        console.log(chalk.red(`\n❌ Unknown scenario: ${scenario}\n`));
        console.log(HelpFormatter.formatList());
        process.exit(1);
      }

      // Parse iterations
      const iterations = parseInt(options.iterations, 10);
      if (isNaN(iterations) || iterations < 1 || iterations > 100) {
        console.log(chalk.red('\n❌ Iterations must be between 1 and 100\n'));
        process.exit(1);
      }

      // Validate format
      const validFormats = ['md', 'json', 'html'];
      if (!validFormats.includes(options.format)) {
        console.log(chalk.red(`\n❌ Invalid format: ${options.format}. Must be one of: ${validFormats.join(', ')}\n`));
        process.exit(1);
      }

      // Run simulation
      console.log(chalk.cyan.bold('\n╔══════════════════════════════════════════════════════════════╗'));
      console.log(chalk.cyan.bold('║  AgentDB Latent Space Simulation                           ║'));
      console.log(chalk.cyan.bold('╚══════════════════════════════════════════════════════════════╝\n'));

      const runner = new SimulationRunner();
      const config = {
        useOptimal: true, // Default to optimal configuration
      };

      const report = await runner.runScenario(scenario, config, iterations);

      // Generate and save report
      const generator = new ReportGenerator();
      const outputPath = options.output || './reports';
      const savedPath = await generator.saveReport(report, outputPath, options.format);

      console.log(chalk.green(`\n✅ Report saved to: ${savedPath}\n`));

      // Show summary
      console.log(chalk.bold('Summary:'));
      console.log(`  Success Rate: ${(report.summary.successRate * 100).toFixed(1)}%`);
      console.log(`  Coherence: ${(report.coherenceScore * 100).toFixed(1)}%`);
      console.log(`  Avg Latency: ${report.summary.avgLatencyUs.toFixed(2)}μs`);
      console.log(`  Avg Recall@10: ${(report.summary.avgRecall * 100).toFixed(1)}%`);
      console.log('');
    } catch (error: any) {
      console.error(chalk.red(`\n❌ Error: ${error.message}\n`));
      if (options.verbose && error.stack) {
        console.error(error.stack);
      }
      process.exit(1);
    }
  });

// Add custom help
simulateCommand.on('--help', () => {
  console.log(HelpFormatter.formatTopLevel());
});
