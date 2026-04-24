/**
 * Integration Tests for Backwards Compatibility Layer
 *
 * Tests that v1.x code runs unchanged on v2.0 backend
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { V1toV2Adapter } from '../../../compatibility/V1toV2Adapter';
import { VersionDetector } from '../../../compatibility/VersionDetector';
import { DeprecationWarnings } from '../../../compatibility/DeprecationWarnings';
import { MigrationUtilities } from '../../../compatibility/MigrationUtilities';

// Mock v2 AgentDB instance
const createMockV2Instance = () => ({
  swarms: {
    create: vi.fn().mockResolvedValue({ id: 'swarm-1', topology: 'mesh' }),
    status: vi.fn().mockResolvedValue({
      id: 'swarm-1',
      topology: 'mesh',
      agents: [],
      health: { status: 'healthy', uptime: 1000 }
    }),
    destroy: vi.fn().mockResolvedValue(undefined)
  },
  agents: {
    spawn: vi.fn().mockResolvedValue({ id: 'agent-1', type: 'coder' })
  },
  tasks: {
    orchestrate: vi.fn().mockResolvedValue({ id: 'task-1', status: 'pending' }),
    status: vi.fn().mockResolvedValue({ id: 'task-1', status: 'completed' }),
    wait: vi.fn().mockResolvedValue({ id: 'task-1', result: 'success' })
  },
  memory: {
    store: vi.fn().mockResolvedValue(undefined),
    retrieve: vi.fn().mockResolvedValue('test-value'),
    vectorSearch: vi.fn().mockResolvedValue([
      { id: 'id1', metadata: { key: 'value1' }, score: 0.9 }
    ])
  }
});

describe('Backwards Compatibility Integration', () => {
  describe('Complete v1.x workflow on v2.0 backend', () => {
    it('should support full v1.x lifecycle', async () => {
      const mockV2 = createMockV2Instance();
      const adapter = new V1toV2Adapter(mockV2);

      // v1.x initialization
      const swarm = await adapter.initSwarm({ topology: 'mesh', maxAgents: 8 });
      expect(swarm.id).toBe('swarm-1');
      expect(mockV2.swarms.create).toHaveBeenCalled();

      // v1.x agent spawning
      const agent = await adapter.spawnAgent('coder', { name: 'TestCoder' });
      expect(agent.id).toBe('agent-1');
      expect(mockV2.agents.spawn).toHaveBeenCalled();

      // v1.x task orchestration
      const task = await adapter.orchestrateTask('Build feature', {
        strategy: 'parallel',
        priority: 'high'
      });
      expect(task.id).toBe('task-1');
      expect(mockV2.tasks.orchestrate).toHaveBeenCalled();

      // v1.x memory operations
      await adapter.setMemory('key', 'value');
      expect(mockV2.memory.store).toHaveBeenCalledWith('key', 'value');

      const value = await adapter.getMemory('key');
      expect(value).toBe('test-value');
      expect(mockV2.memory.retrieve).toHaveBeenCalledWith('key');

      // v1.x memory search
      const results = await adapter.searchMemory('test query', 10);
      expect(results).toHaveLength(1);
      expect(results[0].key).toBe('id1');
      expect(mockV2.memory.vectorSearch).toHaveBeenCalled();

      // v1.x status check
      const status = await adapter.getSwarmStatus();
      expect(status.swarmId).toBe('swarm-1');
      expect(mockV2.swarms.status).toHaveBeenCalled();

      // v1.x cleanup
      await adapter.destroySwarm();
      expect(mockV2.swarms.destroy).toHaveBeenCalled();
    });

    it('should emit deprecation warnings for all v1 APIs', async () => {
      const mockV2 = createMockV2Instance();
      const adapter = new V1toV2Adapter(mockV2);

      await adapter.initSwarm({ topology: 'mesh' });
      await adapter.spawnAgent('coder');
      await adapter.orchestrateTask('task');
      await adapter.setMemory('key', 'value');
      await adapter.getMemory('key');
      await adapter.searchMemory('query');
      await adapter.getSwarmStatus();
      await adapter.destroySwarm();

      const warnings = adapter.getWarnings();
      expect(warnings.length).toBeGreaterThanOrEqual(8);
    });
  });

  describe('Version detection integration', () => {
    it('should detect v1.x from simple config', () => {
      const config = {
        memoryPath: './db',
        topology: 'mesh'
      };

      const result = VersionDetector.detect(config, 'AgenticFlow');
      expect(result.version).toBe('1.x');
    });

    it('should detect v2.0 from structured config', () => {
      const config = {
        backend: 'agentdb',
        memory: {
          path: './db',
          backend: 'ruvector' as const,
          enableHNSW: true
        }
      };

      const result = VersionDetector.detect(config, 'AgenticFlow');
      expect(result.version).toBe('2.0');
    });

    it('should provide migration path for each v1 API', () => {
      const v1APIs = [
        'initSwarm',
        'spawnAgent',
        'orchestrateTask',
        'getMemory',
        'setMemory',
        'searchMemory',
        'getSwarmStatus',
        'destroySwarm'
      ];

      for (const api of v1APIs) {
        expect(VersionDetector.isV1API(api)).toBe(true);
        const v2API = VersionDetector.getAPIMapping(api);
        expect(v2API).toBeDefined();
        expect(v2API).toContain('.');
      }
    });
  });

  describe('Migration utilities integration', () => {
    it('should analyze complete v1 codebase', () => {
      const v1Code = `
        import { AgenticFlow } from 'agentic-flow';

        async function main() {
          const flow = new AgenticFlow({
            memoryPath: './swarm.db',
            topology: 'mesh'
          });

          await flow.initSwarm({ topology: 'mesh' });
          const agent = await flow.spawnAgent('coder');
          const task = await flow.orchestrateTask('Build feature');
          await flow.setMemory('status', 'active');
          const status = await flow.getMemory('status');
          const results = await flow.searchMemory('feature');
          const swarmStatus = await flow.getSwarmStatus();
          await flow.destroySwarm();
        }
      `;

      const report = MigrationUtilities.analyzeCode(v1Code);

      expect(report.v1APIsFound.length).toBeGreaterThan(0);
      expect(report.suggestedMigrations.length).toBeGreaterThan(0);
      expect(report.estimatedEffort).toBe('low');
    });

    it('should generate valid v2 code', () => {
      const v1Code = `
        import { AgenticFlow } from 'agentic-flow';
        const flow = new AgenticFlow();
        await flow.initSwarm({ topology: 'mesh' });
        await flow.spawnAgent('coder');
      `;

      const v2Code = MigrationUtilities.generateMigrationScript(v1Code);

      expect(v2Code).toContain('AgenticFlowV2');
      expect(v2Code).toContain('swarms.create');
      expect(v2Code).toContain('agents.spawn');
      expect(v2Code).not.toContain('initSwarm');
      expect(v2Code).not.toContain('spawnAgent');
    });

    it('should validate v2 config correctly', () => {
      const validConfig = {
        backend: 'agentdb' as const,
        version: '2.0' as const,
        memory: {
          path: './db',
          backend: 'ruvector' as const,
          enableHNSW: true,
          enableQuantization: true
        },
        swarm: {
          topology: 'mesh' as const,
          maxAgents: 8,
          strategy: 'auto' as const
        },
        routing: {
          provider: 'anthropic',
          model: 'claude-3-opus',
          optimization: 'quality' as const
        }
      };

      const result = MigrationUtilities.validateMigratedConfig(validConfig);

      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should convert v1 config to v2 config', () => {
      const v1Config = {
        memoryPath: './swarm.db',
        topology: 'mesh' as const,
        maxAgents: 10,
        provider: 'anthropic',
        model: 'claude-3-opus'
      };

      const v2Config = MigrationUtilities.convertV1ConfigToV2(v1Config);

      expect(v2Config.backend).toBe('agentdb');
      expect(v2Config.version).toBe('2.0');
      expect(v2Config.memory.path).toBe('./swarm.db');
      expect(v2Config.memory.backend).toBe('ruvector');
      expect(v2Config.memory.enableHNSW).toBe(true);
      expect(v2Config.swarm.topology).toBe('mesh');
      expect(v2Config.swarm.maxAgents).toBe(10);
      expect(v2Config.routing.provider).toBe('anthropic');
      expect(v2Config.routing.model).toBe('claude-3-opus');
    });
  });

  describe('Deprecation warnings integration', () => {
    it('should support different severity levels', () => {
      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

      // Silent mode
      const silent = new DeprecationWarnings({ severity: 'silent' });
      silent.warn('test', {
        message: 'Test',
        migration: 'Test',
        documentation: 'Test'
      });
      expect(consoleSpy).not.toHaveBeenCalled();

      // Soft mode
      const soft = new DeprecationWarnings({ severity: 'soft' });
      soft.warn('test', {
        message: 'Test',
        migration: 'Test',
        documentation: 'Test'
      });
      expect(consoleSpy).toHaveBeenCalled();

      consoleSpy.mockRestore();
    });

    it('should track warning history', () => {
      const warnings = new DeprecationWarnings();

      warnings.warn('api1', {
        message: 'Test 1',
        migration: 'Migration 1',
        documentation: 'Doc 1'
      });
      warnings.warn('api2', {
        message: 'Test 2',
        migration: 'Migration 2',
        documentation: 'Doc 2'
      });

      expect(warnings.getWarningCount()).toBe(2);
      expect(warnings.hasWarnings()).toBe(true);

      warnings.clearWarnings();
      expect(warnings.getWarningCount()).toBe(0);
      expect(warnings.hasWarnings()).toBe(false);
    });
  });

  describe('Performance', () => {
    it('should have minimal adapter overhead (<1ms)', async () => {
      const mockV2 = createMockV2Instance();
      const adapter = new V1toV2Adapter(mockV2);

      const start = performance.now();
      await adapter.initSwarm({ topology: 'mesh' });
      const end = performance.now();

      const adapterOverhead = end - start;
      expect(adapterOverhead).toBeLessThan(1); // <1ms overhead
    });

    it('should handle high-volume API calls efficiently', async () => {
      const mockV2 = createMockV2Instance();
      const adapter = new V1toV2Adapter(mockV2);

      const start = performance.now();
      const promises = [];
      for (let i = 0; i < 100; i++) {
        promises.push(adapter.setMemory(`key${i}`, `value${i}`));
      }
      await Promise.all(promises);
      const end = performance.now();

      const avgTime = (end - start) / 100;
      expect(avgTime).toBeLessThan(1); // <1ms per call
    });
  });
});
