/**
 * MigrationUtilities - Tools for migrating from v1.x to v2.0
 *
 * Provides code analysis, automatic migration, and validation
 * utilities to help users upgrade from v1.x to v2.0.
 */

import type {
  MigrationReport,
  ValidationResult,
  V1Config,
  V2Config
} from './types';

export class MigrationUtilities {
  /**
   * Analyze v1 code for migration opportunities
   */
  static analyzeCode(code: string): MigrationReport {
    const report: MigrationReport = {
      v1APIsFound: [],
      suggestedMigrations: [],
      estimatedEffort: 'low'
    };

    // Detect v1 API usage patterns
    const v1Patterns = [
      { pattern: /\.initSwarm\(/g, api: 'initSwarm', replacement: 'swarms.create' },
      { pattern: /\.spawnAgent\(/g, api: 'spawnAgent', replacement: 'agents.spawn' },
      { pattern: /\.orchestrateTask\(/g, api: 'orchestrateTask', replacement: 'tasks.orchestrate' },
      { pattern: /\.getMemory\(/g, api: 'getMemory', replacement: 'memory.retrieve' },
      { pattern: /\.setMemory\(/g, api: 'setMemory', replacement: 'memory.store' },
      { pattern: /\.searchMemory\(/g, api: 'searchMemory', replacement: 'memory.vectorSearch' },
      { pattern: /\.getSwarmStatus\(/g, api: 'getSwarmStatus', replacement: 'swarms.status' },
      { pattern: /\.destroySwarm\(/g, api: 'destroySwarm', replacement: 'swarms.destroy' },
      { pattern: /\.getTaskStatus\(/g, api: 'getTaskStatus', replacement: 'tasks.status' },
      { pattern: /\.waitForTask\(/g, api: 'waitForTask', replacement: 'tasks.wait' }
    ];

    for (const { pattern, api, replacement } of v1Patterns) {
      const matches = code.match(pattern);
      if (matches) {
        report.v1APIsFound.push({
          api,
          occurrences: matches.length,
          replacement
        });

        report.suggestedMigrations.push({
          from: api,
          to: replacement,
          difficulty: 'easy',
          automatic: true
        });
      }
    }

    // Estimate effort
    const totalAPIs = report.v1APIsFound.reduce((sum, api) => sum + api.occurrences, 0);
    if (totalAPIs === 0) {
      report.estimatedEffort = 'none';
    } else if (totalAPIs < 10) {
      report.estimatedEffort = 'low';
    } else if (totalAPIs < 50) {
      report.estimatedEffort = 'medium';
    } else {
      report.estimatedEffort = 'high';
    }

    return report;
  }

  /**
   * Generate migration script (automatic code transformation)
   */
  static generateMigrationScript(code: string): string {
    let migratedCode = code;

    // Replace v1 imports
    migratedCode = migratedCode.replace(
      /import\s+{\s*AgenticFlow\s*}\s+from\s+['"]agentic-flow['"]/g,
      'import { AgenticFlowV2 } from "agentic-flow"'
    );

    // Replace constructor calls - use word boundary to match complete word only
    migratedCode = migratedCode.replace(
      /new\s+AgenticFlow\b/g,
      'new AgenticFlowV2'
    );

    // Replace method calls
    const replacements = [
      { from: '.initSwarm(', to: '.swarms.create(' },
      { from: '.spawnAgent(', to: '.agents.spawn({ type: ' },
      { from: '.orchestrateTask(', to: '.tasks.orchestrate({ description: ' },
      { from: '.getMemory(', to: '.memory.retrieve(' },
      { from: '.setMemory(', to: '.memory.store(' },
      { from: '.searchMemory(', to: '.memory.vectorSearch(' },
      { from: '.getSwarmStatus(', to: '.swarms.status(' },
      { from: '.destroySwarm(', to: '.swarms.destroy(' },
      { from: '.getTaskStatus(', to: '.tasks.status(' },
      { from: '.waitForTask(', to: '.tasks.wait(' }
    ];

    for (const { from, to } of replacements) {
      migratedCode = migratedCode.split(from).join(to);
    }

    return migratedCode;
  }

  /**
   * Validate migrated v2 config
   */
  static validateMigratedConfig(v2Config: V2Config): ValidationResult {
    const errors: string[] = [];
    const warnings: string[] = [];

    // Required fields
    if (!v2Config.backend) {
      errors.push('backend is required in v2.0 config');
    }

    if (!v2Config.memory?.path) {
      errors.push('memory.path is required in v2.0 config');
    }

    // Warnings for missing optimizations
    if (!v2Config.memory?.enableHNSW) {
      warnings.push('Consider enabling HNSW indexing for 150x faster search');
    }

    if (!v2Config.memory?.enableQuantization) {
      warnings.push('Consider enabling quantization for 4x memory reduction');
    }

    return {
      valid: errors.length === 0,
      errors,
      warnings
    };
  }

  /**
   * Convert v1 config to v2 config
   */
  static convertV1ConfigToV2(v1Config: V1Config): V2Config {
    return {
      backend: 'agentdb',
      version: '2.0',

      memory: {
        path: v1Config.memoryPath || ':memory:',
        backend: 'ruvector',
        enableHNSW: true,
        enableQuantization: true,
        cacheSize: 1000,
        batchSize: 100
      },

      swarm: {
        topology: v1Config.topology || 'mesh',
        maxAgents: v1Config.maxAgents || 8,
        strategy: 'auto'
      },

      routing: {
        provider: v1Config.provider || 'anthropic',
        model: v1Config.model || 'claude-3-opus',
        optimization: 'balanced'
      },

      intelligence: {
        attentionMechanisms: ['hyperbolic'],
        gnnLearning: true,
        causalReasoning: true,
        reasoningBank: true
      },

      compatibility: {
        v1Mode: false,
        preserveV1Behavior: false,
        emitWarnings: true
      }
    };
  }

  /**
   * Generate migration guide
   */
  static generateMigrationGuide(report: MigrationReport): string {
    const lines: string[] = [
      '# Migration Guide: v1.x → v2.0',
      '',
      `## Summary`,
      `- APIs to migrate: ${report.v1APIsFound.length}`,
      `- Total occurrences: ${report.v1APIsFound.reduce((sum, api) => sum + api.occurrences, 0)}`,
      `- Estimated effort: ${report.estimatedEffort}`,
      '',
      '## API Migrations',
      ''
    ];

    for (const api of report.v1APIsFound) {
      lines.push(`### ${api.api} → ${api.replacement}`);
      lines.push(`- Occurrences: ${api.occurrences}`);
      lines.push(`- Automatic migration: ✅`);
      lines.push('');
    }

    lines.push('## Next Steps');
    lines.push('');
    lines.push('1. Run `MigrationUtilities.generateMigrationScript()` to auto-migrate code');
    lines.push('2. Update config to v2.0 format');
    lines.push('3. Test migrated code thoroughly');
    lines.push('4. Enable v2.0 optimizations (HNSW, quantization)');
    lines.push('');

    return lines.join('\n');
  }
}
