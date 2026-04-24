/**
 * TDD London School Tests for V1toV2Adapter
 *
 * Tests v1.x API translation to v2.0 backend
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { V1toV2Adapter } from '../../../compatibility/V1toV2Adapter';
import type { V1Config } from '../../../compatibility/types';

// Mock AgentDB
const createMockAgentDB = () => ({
  initialize: vi.fn().mockResolvedValue(undefined),
  getController: vi.fn((name: string) => {
    if (name === 'memory') {
      return {
        store: vi.fn().mockResolvedValue(undefined),
        retrieve: vi.fn().mockResolvedValue('test-value'),
        search: vi.fn().mockResolvedValue([
          { id: 'id1', metadata: { key: 'value1' }, score: 0.9 },
          { id: 'id2', metadata: { key: 'value2' }, score: 0.8 }
        ])
      };
    }
    return {};
  }),
  swarms: {
    create: vi.fn().mockResolvedValue({ id: 'swarm-123', topology: 'mesh' }),
    status: vi.fn().mockResolvedValue({
      id: 'swarm-123',
      topology: 'mesh',
      agents: [
        { id: 'agent-1', type: 'coder', status: 'active', metrics: { tasksCompleted: 5 } }
      ],
      health: { status: 'healthy', uptime: 12345 }
    }),
    destroy: vi.fn().mockResolvedValue(undefined)
  },
  agents: {
    spawn: vi.fn().mockResolvedValue({ id: 'agent-456', type: 'coder' })
  },
  tasks: {
    orchestrate: vi.fn().mockResolvedValue({ id: 'task-789', status: 'pending' }),
    status: vi.fn().mockResolvedValue({ id: 'task-789', status: 'completed' }),
    wait: vi.fn().mockResolvedValue({ id: 'task-789', result: 'success' })
  },
  memory: {
    store: vi.fn().mockResolvedValue(undefined),
    retrieve: vi.fn().mockResolvedValue('test-value'),
    vectorSearch: vi.fn().mockResolvedValue([
      { id: 'id1', metadata: { key: 'value1' }, score: 0.9, distance: 0.1 },
      { id: 'id2', metadata: { key: 'value2' }, score: 0.8, distance: 0.2 }
    ])
  }
});

describe('V1toV2Adapter', () => {
  let adapter: V1toV2Adapter;
  let mockAgentDB: ReturnType<typeof createMockAgentDB>;

  beforeEach(() => {
    mockAgentDB = createMockAgentDB();
    adapter = new V1toV2Adapter(mockAgentDB as any);
  });

  describe('initSwarm', () => {
    it('should translate v1 initSwarm to v2 swarms.create', async () => {
      const result = await adapter.initSwarm({ topology: 'mesh' });

      expect(mockAgentDB.swarms.create).toHaveBeenCalledWith(
        expect.objectContaining({ topology: 'mesh' })
      );
      expect(result).toEqual({ id: 'swarm-123', topology: 'mesh' });
    });

    it('should use default topology if not provided', async () => {
      await adapter.initSwarm({});

      expect(mockAgentDB.swarms.create).toHaveBeenCalledWith(
        expect.objectContaining({ topology: 'mesh' })
      );
    });

    it('should pass maxAgents parameter', async () => {
      await adapter.initSwarm({ maxAgents: 10 });

      expect(mockAgentDB.swarms.create).toHaveBeenCalledWith(
        expect.objectContaining({ maxAgents: 10 })
      );
    });

    it('should emit deprecation warning', async () => {
      const warnSpy = vi.spyOn(adapter['warnings'], 'warn');

      await adapter.initSwarm({ topology: 'mesh' });

      expect(warnSpy).toHaveBeenCalledWith(
        'initSwarm',
        expect.objectContaining({
          message: expect.stringContaining('deprecated')
        })
      );
    });
  });

  describe('spawnAgent', () => {
    it('should translate v1 spawnAgent to v2 agents.spawn', async () => {
      const result = await adapter.spawnAgent('coder', { name: 'TestCoder' });

      expect(mockAgentDB.agents.spawn).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'coder', name: 'TestCoder' })
      );
      expect(result).toEqual({ id: 'agent-456', type: 'coder' });
    });

    it('should handle agent spawn without config', async () => {
      await adapter.spawnAgent('researcher');

      expect(mockAgentDB.agents.spawn).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'researcher' })
      );
    });

    it('should emit deprecation warning', async () => {
      const warnSpy = vi.spyOn(adapter['warnings'], 'warn');

      await adapter.spawnAgent('coder');

      expect(warnSpy).toHaveBeenCalledWith('spawnAgent', expect.any(Object));
    });
  });

  describe('orchestrateTask', () => {
    it('should translate v1 orchestrateTask to v2 tasks.orchestrate', async () => {
      const result = await adapter.orchestrateTask('Build a feature', {
        strategy: 'parallel',
        priority: 'high'
      });

      expect(mockAgentDB.tasks.orchestrate).toHaveBeenCalledWith(
        expect.objectContaining({
          description: 'Build a feature',
          strategy: 'parallel',
          priority: 'high'
        })
      );
      expect(result).toEqual({ id: 'task-789', status: 'pending' });
    });

    it('should use default strategy if not provided', async () => {
      await adapter.orchestrateTask('Test task');

      expect(mockAgentDB.tasks.orchestrate).toHaveBeenCalledWith(
        expect.objectContaining({ strategy: 'adaptive' })
      );
    });

    it('should use default priority if not provided', async () => {
      await adapter.orchestrateTask('Test task');

      expect(mockAgentDB.tasks.orchestrate).toHaveBeenCalledWith(
        expect.objectContaining({ priority: 'medium' })
      );
    });
  });

  describe('getMemory', () => {
    it('should translate v1 getMemory to v2 memory.retrieve', async () => {
      const result = await adapter.getMemory('test-key');

      expect(mockAgentDB.memory.retrieve).toHaveBeenCalledWith('test-key');
      expect(result).toBe('test-value');
    });

    it('should emit deprecation warning', async () => {
      const warnSpy = vi.spyOn(adapter['warnings'], 'warn');

      await adapter.getMemory('test-key');

      expect(warnSpy).toHaveBeenCalledWith('getMemory', expect.any(Object));
    });
  });

  describe('setMemory', () => {
    it('should translate v1 setMemory to v2 memory.store', async () => {
      await adapter.setMemory('test-key', 'test-value');

      expect(mockAgentDB.memory.store).toHaveBeenCalledWith('test-key', 'test-value');
    });

    it('should emit deprecation warning', async () => {
      const warnSpy = vi.spyOn(adapter['warnings'], 'warn');

      await adapter.setMemory('test-key', 'test-value');

      expect(warnSpy).toHaveBeenCalledWith('setMemory', expect.any(Object));
    });
  });

  describe('searchMemory', () => {
    it('should translate v1 searchMemory to v2 memory.vectorSearch', async () => {
      const result = await adapter.searchMemory('test query', 5);

      expect(mockAgentDB.memory.vectorSearch).toHaveBeenCalledWith(
        'test query',
        expect.objectContaining({ k: 5 })
      );
    });

    it('should format v2 results to v1 structure', async () => {
      const result = await adapter.searchMemory('test query', 5);

      expect(result).toEqual([
        { key: 'id1', value: { key: 'value1' }, score: 0.9 },
        { key: 'id2', value: { key: 'value2' }, score: 0.8 }
      ]);
    });

    it('should use default limit if not provided', async () => {
      await adapter.searchMemory('test query');

      expect(mockAgentDB.memory.vectorSearch).toHaveBeenCalledWith(
        'test query',
        expect.objectContaining({ k: 10 })
      );
    });
  });

  describe('getSwarmStatus', () => {
    it('should translate v1 getSwarmStatus to v2 swarms.status', async () => {
      const result = await adapter.getSwarmStatus();

      expect(mockAgentDB.swarms.status).toHaveBeenCalled();
    });

    it('should format v2 status to v1 structure', async () => {
      const result = await adapter.getSwarmStatus();

      expect(result).toEqual({
        swarmId: 'swarm-123',
        topology: 'mesh',
        agentCount: 1,
        agents: [
          {
            id: 'agent-1',
            type: 'coder',
            status: 'active',
            tasksCompleted: 5
          }
        ],
        status: 'healthy',
        uptime: 12345
      });
    });
  });

  describe('destroySwarm', () => {
    it('should translate v1 destroySwarm to v2 swarms.destroy', async () => {
      await adapter.destroySwarm();

      expect(mockAgentDB.swarms.destroy).toHaveBeenCalled();
    });
  });

  describe('getTaskStatus', () => {
    it('should translate v1 getTaskStatus to v2 tasks.status', async () => {
      const result = await adapter.getTaskStatus('task-789');

      expect(mockAgentDB.tasks.status).toHaveBeenCalledWith('task-789');
      expect(result).toEqual({ id: 'task-789', status: 'completed' });
    });
  });

  describe('waitForTask', () => {
    it('should translate v1 waitForTask to v2 tasks.wait', async () => {
      const result = await adapter.waitForTask('task-789', 30000);

      expect(mockAgentDB.tasks.wait).toHaveBeenCalledWith('task-789', {
        timeout: 30000
      });
      expect(result).toEqual({ id: 'task-789', result: 'success' });
    });

    it('should handle waitForTask without timeout', async () => {
      await adapter.waitForTask('task-789');

      expect(mockAgentDB.tasks.wait).toHaveBeenCalledWith('task-789', {
        timeout: undefined
      });
    });
  });

  describe('getV2Instance', () => {
    it('should return underlying v2 instance', () => {
      const instance = adapter.getV2Instance();

      expect(instance).toBe(mockAgentDB);
    });
  });

  describe('getWarnings', () => {
    it('should return deprecation warnings', async () => {
      await adapter.initSwarm({ topology: 'mesh' });
      await adapter.spawnAgent('coder');

      const warnings = adapter.getWarnings();

      expect(warnings.length).toBeGreaterThan(0);
    });
  });
});
