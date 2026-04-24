/**
 * Interactive wizard for simulation configuration
 * Uses inquirer.js for beautiful prompts
 */

import inquirer from 'inquirer';
import { SimulationRunner } from '../lib/simulation-runner.js';
import { ReportGenerator } from '../lib/report-generator.js';
import { ConfigValidator, type SimulationConfig } from '../lib/config-validator.js';
import chalk from 'chalk';

export async function runWizard(): Promise<void> {
  console.log(chalk.cyan.bold('\n🧙 AgentDB Simulation Wizard\n'));

  // Step 1: Choose mode
  const { mode } = await inquirer.prompt([
    {
      type: 'list',
      name: 'mode',
      message: 'What would you like to do?',
      choices: [
        { name: '🎯 Run validated scenario (recommended)', value: 'scenario' },
        { name: '🔧 Build custom simulation', value: 'custom' },
        { name: '📊 View past reports', value: 'reports' },
        { name: '❌ Exit', value: 'exit' },
      ],
    },
  ]);

  if (mode === 'exit') {
    console.log(chalk.gray('\n👋 Goodbye!\n'));
    return;
  }

  if (mode === 'reports') {
    const { viewReport } = await import('./simulate-report.js');
    await viewReport();
    return;
  }

  if (mode === 'custom') {
    await customWizard();
    return;
  }

  await scenarioWizard();
}

async function scenarioWizard(): Promise<void> {
  console.log(chalk.bold('\n📋 Scenario Selection\n'));

  // Step 2: Select scenario
  const { scenario } = await inquirer.prompt([
    {
      type: 'list',
      name: 'scenario',
      message: 'Choose a simulation scenario:',
      choices: [
        {
          name: '⚡ HNSW Exploration (8.2x speedup)',
          value: 'hnsw',
          short: 'HNSW',
        },
        {
          name: '🧠 Attention Analysis (12.4% improvement)',
          value: 'attention',
          short: 'Attention',
        },
        {
          name: '🎯 Traversal Optimization (96.8% recall)',
          value: 'traversal',
          short: 'Traversal',
        },
        {
          name: '🔄 Self-Organizing (97.9% uptime)',
          value: 'self-organizing',
          short: 'Self-Organizing',
        },
        {
          name: '🚀 Neural Augmentation (29.4% improvement)',
          value: 'neural',
          short: 'Neural',
        },
        {
          name: '🔗 Hypergraph Exploration (3.7x compression)',
          value: 'hypergraph',
          short: 'Hypergraph',
        },
        {
          name: '📊 Clustering Analysis (Q=0.758)',
          value: 'clustering',
          short: 'Clustering',
        },
        {
          name: '🔮 Quantum-Hybrid (Theoretical)',
          value: 'quantum',
          short: 'Quantum',
        },
      ],
    },
  ]);

  // Step 3: Configuration
  const config = await inquirer.prompt([
    {
      type: 'number',
      name: 'nodes',
      message: 'Number of nodes:',
      default: 100000,
      validate: (value: number) => {
        if (value < 1000 || value > 10000000) {
          return 'Node count must be between 1,000 and 10,000,000';
        }
        return true;
      },
    },
    {
      type: 'number',
      name: 'dimensions',
      message: 'Vector dimensions:',
      default: 384,
      validate: (value: number) => {
        if (value < 64 || value > 2048) {
          return 'Dimensions must be between 64 and 2048';
        }
        return true;
      },
    },
    {
      type: 'number',
      name: 'iterations',
      message: 'Number of runs (for coherence analysis):',
      default: 3,
      validate: (value: number) => {
        if (value < 1 || value > 100) {
          return 'Iterations must be between 1 and 100';
        }
        return true;
      },
    },
    {
      type: 'confirm',
      name: 'useOptimal',
      message: 'Use optimal validated configuration?',
      default: true,
    },
    {
      type: 'list',
      name: 'format',
      message: 'Report format:',
      choices: [
        { name: 'Markdown (readable)', value: 'md' },
        { name: 'JSON (data)', value: 'json' },
        { name: 'HTML (web)', value: 'html' },
      ],
      default: 'md',
    },
  ]);

  // Merge with optimal config if requested
  const finalConfig: SimulationConfig = config.useOptimal
    ? { ...ConfigValidator.getOptimalConfig(scenario), ...config }
    : { ...config };

  // Step 4: Confirmation
  console.log(chalk.bold('\n📋 Simulation Configuration:'));
  console.log(`   Scenario: ${chalk.cyan(scenario)}`);
  console.log(`   Nodes: ${chalk.cyan(finalConfig.nodes?.toLocaleString())}`);
  console.log(`   Dimensions: ${chalk.cyan(finalConfig.dimensions)}`);
  console.log(`   Iterations: ${chalk.cyan(finalConfig.iterations)}`);
  console.log(`   Format: ${chalk.cyan(config.format)}`);
  if (config.useOptimal) {
    console.log(chalk.green('   ✅ Using optimal validated parameters'));
  }

  const { confirm } = await inquirer.prompt([
    {
      type: 'confirm',
      name: 'confirm',
      message: 'Start simulation?',
      default: true,
    },
  ]);

  if (!confirm) {
    console.log(chalk.yellow('\n❌ Simulation cancelled\n'));
    return;
  }

  // Run simulation
  try {
    const runner = new SimulationRunner();
    const report = await runner.runScenario(scenario, finalConfig, finalConfig.iterations || 3);

    // Generate and save report
    const generator = new ReportGenerator();
    const outputPath = './reports';
    const savedPath = await generator.saveReport(report, outputPath, config.format);

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
    throw error;
  }
}

async function customWizard(): Promise<void> {
  console.log(chalk.bold('\n🔧 Custom Simulation Builder\n'));

  const components = await inquirer.prompt([
    {
      type: 'list',
      name: 'backend',
      message: '1/6 Choose vector backend:',
      choices: [
        { name: '🚀 RuVector (8.2x speedup) [OPTIMAL]', value: 'ruvector' },
        { name: '📦 hnswlib (baseline)', value: 'hnswlib' },
        { name: '🔬 FAISS', value: 'faiss' },
      ],
      default: 'ruvector',
    },
    {
      type: 'list',
      name: 'attentionHeads',
      message: '2/6 Attention mechanism:',
      choices: [
        { name: '🧠 8-head attention (+12.4%) [OPTIMAL]', value: 8 },
        { name: '4-head attention', value: 4 },
        { name: '16-head attention', value: 16 },
        { name: 'No attention', value: 0 },
      ],
      default: 8,
    },
    {
      type: 'list',
      name: 'searchStrategy',
      message: '3/6 Search strategy:',
      choices: [
        { name: '🎯 Beam-5 + Dynamic-k (96.8% recall) [OPTIMAL]', value: 'beam-dynamic' },
        { name: 'Greedy (baseline)', value: 'greedy' },
        { name: 'A* search', value: 'astar' },
        { name: 'Dynamic-k only', value: 'dynamic-k' },
      ],
      default: 'beam-dynamic',
    },
    {
      type: 'list',
      name: 'clustering',
      message: '4/6 Clustering algorithm:',
      choices: [
        { name: '🎯 Louvain (Q=0.758) [OPTIMAL]', value: 'louvain' },
        { name: 'Spectral', value: 'spectral' },
        { name: 'Hierarchical', value: 'hierarchical' },
        { name: 'None', value: 'none' },
      ],
      default: 'louvain',
    },
    {
      type: 'list',
      name: 'selfHealing',
      message: '5/6 Self-healing mode:',
      choices: [
        { name: '🛡️ MPC (97.9% uptime) [OPTIMAL]', value: 'mpc' },
        { name: 'Reactive', value: 'reactive' },
        { name: 'None', value: 'none' },
      ],
      default: 'mpc',
    },
    {
      type: 'checkbox',
      name: 'neuralFeatures',
      message: '6/6 Neural augmentation features:',
      choices: [
        { name: 'GNN edge selection (-18% memory)', value: 'gnn-edges', checked: true },
        { name: 'RL navigation (-26% hops)', value: 'rl-nav', checked: true },
        { name: 'Joint optimization (+9.1%)', value: 'joint-opt', checked: true },
        { name: 'Full neural pipeline (+29.4%)', value: 'full-pipeline', checked: true },
      ],
    },
  ]);

  // Additional configuration
  const additionalConfig = await inquirer.prompt([
    {
      type: 'number',
      name: 'nodes',
      message: 'Number of nodes:',
      default: 100000,
    },
    {
      type: 'number',
      name: 'dimensions',
      message: 'Vector dimensions:',
      default: 384,
    },
    {
      type: 'number',
      name: 'iterations',
      message: 'Number of iterations:',
      default: 3,
    },
    {
      type: 'list',
      name: 'format',
      message: 'Report format:',
      choices: [
        { name: 'Markdown', value: 'md' },
        { name: 'JSON', value: 'json' },
        { name: 'HTML', value: 'html' },
      ],
      default: 'md',
    },
  ]);

  const finalConfig: SimulationConfig = { ...components, ...additionalConfig };

  // Validate configuration
  const validation = ConfigValidator.validate(finalConfig);
  if (!validation.valid) {
    console.log(chalk.red('\n❌ Invalid configuration:'));
    validation.errors.forEach((err) => console.log(`   - ${err}`));
    console.log('');
    return;
  }

  if (validation.warnings.length > 0) {
    console.log(chalk.yellow('\n⚠️  Configuration warnings:'));
    validation.warnings.forEach((warn) => console.log(`   - ${warn}`));
    console.log('');
  }

  // Show configuration
  console.log(chalk.bold('\n📋 Custom Simulation Configuration:'));
  console.log(`   Backend: ${chalk.cyan(finalConfig.backend)}`);
  console.log(`   Attention: ${chalk.cyan(finalConfig.attentionHeads)}-head`);
  console.log(`   Search: ${chalk.cyan(finalConfig.searchStrategy)}`);
  console.log(`   Clustering: ${chalk.cyan(finalConfig.clustering)}`);
  console.log(`   Self-healing: ${chalk.cyan(finalConfig.selfHealing)}`);
  console.log(`   Neural features: ${chalk.cyan(finalConfig.neuralFeatures?.length || 0)} enabled`);
  console.log('');

  const { confirm } = await inquirer.prompt([
    {
      type: 'confirm',
      name: 'confirm',
      message: 'Run custom simulation?',
      default: true,
    },
  ]);

  if (!confirm) {
    console.log(chalk.yellow('\n❌ Simulation cancelled\n'));
    return;
  }

  // Run simulation with custom scenario
  try {
    const runner = new SimulationRunner();
    const report = await runner.runScenario('custom', finalConfig, finalConfig.iterations);

    // Generate and save report
    const generator = new ReportGenerator();
    const outputPath = './reports';
    const savedPath = await generator.saveReport(report, outputPath, additionalConfig.format);

    console.log(chalk.green(`\n✅ Report saved to: ${savedPath}\n`));
  } catch (error: any) {
    console.error(chalk.red(`\n❌ Error: ${error.message}\n`));
    throw error;
  }
}
