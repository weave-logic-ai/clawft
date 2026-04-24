/**
 * TDD London School Tests for MigrationUtilities
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { MigrationUtilities } from '../../../compatibility/MigrationUtilities';
import type { MigrationReport, ValidationResult, V2Config } from '../../../compatibility/types';

describe('MigrationUtilities', () => {
  describe('analyzeCode', () => {
    it('should detect v1 API usage', () => {
      const code = `
        const flow = new AgenticFlow();
        await flow.initSwarm({ topology: 'mesh' });
        const agent = await flow.spawnAgent('coder');
      `;

      const report = MigrationUtilities.analyzeCode(code);

      expect(report.v1APIsFound).toContainEqual(
        expect.objectContaining({ api: 'initSwarm' })
      );
      expect(report.v1APIsFound).toContainEqual(
        expect.objectContaining({ api: 'spawnAgent' })
      );
    });

    it('should count occurrences correctly', () => {
      const code = `
        await flow.getMemory('key1');
        await flow.getMemory('key2');
        await flow.getMemory('key3');
      `;

      const report = MigrationUtilities.analyzeCode(code);

      const getMemoryAPI = report.v1APIsFound.find(api => api.api === 'getMemory');
      expect(getMemoryAPI?.occurrences).toBe(3);
    });

    it('should suggest migrations for found APIs', () => {
      const code = `
        await flow.initSwarm({ topology: 'mesh' });
      `;

      const report = MigrationUtilities.analyzeCode(code);

      expect(report.suggestedMigrations).toContainEqual(
        expect.objectContaining({
          from: 'initSwarm',
          to: 'swarms.create'
        })
      );
    });

    it('should estimate effort as "none" for no v1 APIs', () => {
      const code = `
        const flow = new AgenticFlowV2();
        await flow.swarms.create({ topology: 'mesh' });
      `;

      const report = MigrationUtilities.analyzeCode(code);

      expect(report.estimatedEffort).toBe('none');
      expect(report.v1APIsFound).toHaveLength(0);
    });

    it('should estimate effort as "low" for <10 APIs', () => {
      const code = `
        await flow.initSwarm({ topology: 'mesh' });
        await flow.spawnAgent('coder');
      `;

      const report = MigrationUtilities.analyzeCode(code);

      expect(report.estimatedEffort).toBe('low');
    });

    it('should estimate effort as "medium" for 10-50 APIs', () => {
      let code = '';
      for (let i = 0; i < 15; i++) {
        code += `await flow.getMemory('key${i}');\n`;
      }

      const report = MigrationUtilities.analyzeCode(code);

      expect(report.estimatedEffort).toBe('medium');
    });

    it('should estimate effort as "high" for >50 APIs', () => {
      let code = '';
      for (let i = 0; i < 60; i++) {
        code += `await flow.getMemory('key${i}');\n`;
      }

      const report = MigrationUtilities.analyzeCode(code);

      expect(report.estimatedEffort).toBe('high');
    });

    it('should mark all migrations as automatic', () => {
      const code = `
        await flow.initSwarm({ topology: 'mesh' });
        await flow.spawnAgent('coder');
      `;

      const report = MigrationUtilities.analyzeCode(code);

      expect(report.suggestedMigrations.every(m => m.automatic)).toBe(true);
    });
  });

  describe('generateMigrationScript', () => {
    it('should replace v1 imports with v2 imports', () => {
      const code = `import { AgenticFlow } from 'agentic-flow';`;

      const migrated = MigrationUtilities.generateMigrationScript(code);

      expect(migrated).toContain('AgenticFlowV2');
      expect(migrated).not.toContain('AgenticFlow }');
    });

    it('should replace new AgenticFlow() with new AgenticFlowV2()', () => {
      const code = `const flow = new AgenticFlow({ topology: 'mesh' });`;

      const migrated = MigrationUtilities.generateMigrationScript(code);

      expect(migrated).toContain('new AgenticFlowV2');
      expect(migrated).not.toMatch(/new\s+AgenticFlow[^V]/); // Ensure it's not AgenticFlow (without V2)
    });

    it('should replace initSwarm with swarms.create', () => {
      const code = `await flow.initSwarm({ topology: 'mesh' });`;

      const migrated = MigrationUtilities.generateMigrationScript(code);

      expect(migrated).toContain('flow.swarms.create');
      expect(migrated).not.toContain('initSwarm');
    });

    it('should replace spawnAgent with agents.spawn', () => {
      const code = `await flow.spawnAgent('coder', { name: 'TestCoder' });`;

      const migrated = MigrationUtilities.generateMigrationScript(code);

      expect(migrated).toContain('flow.agents.spawn({ type:');
    });

    it('should replace getMemory with memory.retrieve', () => {
      const code = `const value = await flow.getMemory('key');`;

      const migrated = MigrationUtilities.generateMigrationScript(code);

      expect(migrated).toContain('flow.memory.retrieve');
      expect(migrated).not.toContain('getMemory');
    });

    it('should replace setMemory with memory.store', () => {
      const code = `await flow.setMemory('key', 'value');`;

      const migrated = MigrationUtilities.generateMigrationScript(code);

      expect(migrated).toContain('flow.memory.store');
      expect(migrated).not.toContain('setMemory');
    });

    it('should handle complete file migration', () => {
      const code = `
        import { AgenticFlow } from 'agentic-flow';

        const flow = new AgenticFlow({
          memoryPath: './db',
          topology: 'mesh'
        });

        await flow.initSwarm({ topology: 'mesh' });
        const agent = await flow.spawnAgent('coder');
        await flow.setMemory('key', 'value');
        const value = await flow.getMemory('key');
      `;

      const migrated = MigrationUtilities.generateMigrationScript(code);

      expect(migrated).toContain('AgenticFlowV2');
      expect(migrated).toContain('swarms.create');
      expect(migrated).toContain('agents.spawn');
      expect(migrated).toContain('memory.store');
      expect(migrated).toContain('memory.retrieve');
    });
  });

  describe('validateMigratedConfig', () => {
    it('should validate valid v2 config', () => {
      const config: V2Config = {
        backend: 'agentdb',
        version: '2.0',
        memory: {
          path: './db',
          backend: 'ruvector'
        },
        swarm: {
          topology: 'mesh',
          maxAgents: 8,
          strategy: 'auto'
        },
        routing: {
          provider: 'anthropic',
          model: 'claude-3-opus',
          optimization: 'quality'
        }
      };

      const result = MigrationUtilities.validateMigratedConfig(config);

      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should detect missing backend field', () => {
      const config = {
        version: '2.0',
        memory: { path: './db', backend: 'ruvector' as const }
      } as any;

      const result = MigrationUtilities.validateMigratedConfig(config);

      expect(result.valid).toBe(false);
      expect(result.errors).toContainEqual(
        expect.stringContaining('backend is required')
      );
    });

    it('should detect missing memory.path', () => {
      const config = {
        backend: 'agentdb',
        version: '2.0',
        memory: { backend: 'ruvector' as const }
      } as any;

      const result = MigrationUtilities.validateMigratedConfig(config);

      expect(result.valid).toBe(false);
      expect(result.errors).toContainEqual(
        expect.stringContaining('memory.path is required')
      );
    });

    it('should warn about missing HNSW optimization', () => {
      const config: V2Config = {
        backend: 'agentdb',
        version: '2.0',
        memory: {
          path: './db',
          backend: 'ruvector'
          // enableHNSW not set
        },
        swarm: {
          topology: 'mesh',
          maxAgents: 8,
          strategy: 'auto'
        },
        routing: {
          provider: 'anthropic',
          model: 'claude-3-opus',
          optimization: 'quality'
        }
      };

      const result = MigrationUtilities.validateMigratedConfig(config);

      expect(result.valid).toBe(true);
      expect(result.warnings).toContainEqual(
        expect.stringContaining('HNSW')
      );
    });

    it('should warn about missing quantization', () => {
      const config: V2Config = {
        backend: 'agentdb',
        version: '2.0',
        memory: {
          path: './db',
          backend: 'ruvector',
          enableHNSW: true
          // enableQuantization not set
        },
        swarm: {
          topology: 'mesh',
          maxAgents: 8,
          strategy: 'auto'
        },
        routing: {
          provider: 'anthropic',
          model: 'claude-3-opus',
          optimization: 'quality'
        }
      };

      const result = MigrationUtilities.validateMigratedConfig(config);

      expect(result.valid).toBe(true);
      expect(result.warnings).toContainEqual(
        expect.stringContaining('quantization')
      );
    });
  });

  describe('convertV1ConfigToV2', () => {
    it('should convert v1 config to v2 config', () => {
      const v1Config = {
        memoryPath: './swarm-memory.db',
        topology: 'mesh',
        maxAgents: 10,
        provider: 'anthropic',
        model: 'claude-3-opus'
      };

      const v2Config = MigrationUtilities.convertV1ConfigToV2(v1Config);

      expect(v2Config.backend).toBe('agentdb');
      expect(v2Config.version).toBe('2.0');
      expect(v2Config.memory.path).toBe('./swarm-memory.db');
      expect(v2Config.swarm.topology).toBe('mesh');
      expect(v2Config.swarm.maxAgents).toBe(10);
      expect(v2Config.routing.provider).toBe('anthropic');
      expect(v2Config.routing.model).toBe('claude-3-opus');
    });

    it('should use defaults for missing fields', () => {
      const v1Config = {};

      const v2Config = MigrationUtilities.convertV1ConfigToV2(v1Config);

      expect(v2Config.memory.path).toBe(':memory:');
      expect(v2Config.swarm.topology).toBe('mesh');
      expect(v2Config.swarm.maxAgents).toBe(8);
    });

    it('should enable optimizations by default', () => {
      const v1Config = {};

      const v2Config = MigrationUtilities.convertV1ConfigToV2(v1Config);

      expect(v2Config.memory.enableHNSW).toBe(true);
      expect(v2Config.memory.enableQuantization).toBe(true);
    });
  });
});
