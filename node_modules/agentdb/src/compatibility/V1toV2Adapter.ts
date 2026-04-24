/**
 * V1toV2Adapter - Translates v1.x API calls to v2.0 backend
 *
 * Provides transparent backwards compatibility by wrapping v2.0
 * AgentDB instance and translating all v1.x method calls.
 */

import { DeprecationWarnings } from './DeprecationWarnings';
import type { V1Config } from './types';

/**
 * Adapter that translates v1.x API calls to v2.0 backend
 */
export class V1toV2Adapter {
  private v2Instance: any;
  private warnings: DeprecationWarnings;

  constructor(v2Instance: any, config?: V1Config) {
    this.v2Instance = v2Instance;
    this.warnings = new DeprecationWarnings({
      emitWarnings: config?.deprecationWarnings !== false,
      throwOnDeprecated: config?.strictMode === true,
      severity: 'soft'
    });
  }

  /**
   * Initialize swarm (v1.x API)
   */
  async initSwarm(config: {
    topology?: string;
    maxAgents?: number;
    strategy?: string;
  }): Promise<any> {
    this.warnings.warn('initSwarm', {
      message: 'initSwarm() is deprecated. Use swarms.create() in v2.0',
      migration: 'const swarm = await flow.swarms.create({ topology: "mesh" });',
      documentation: 'https://agentic-flow.dev/migration#init-swarm'
    });

    return await this.v2Instance.swarms.create({
      topology: (config.topology as any) || 'mesh',
      maxAgents: config.maxAgents || 8,
      strategy: (config.strategy as any) || 'auto'
    });
  }

  /**
   * Spawn an agent (v1.x API)
   */
  async spawnAgent(type: string, config?: any): Promise<any> {
    this.warnings.warn('spawnAgent', {
      message: 'spawnAgent() is deprecated. Use agents.spawn() in v2.0',
      migration: 'const agent = await flow.agents.spawn({ type: "coder", ...config });',
      documentation: 'https://agentic-flow.dev/migration#spawn-agent'
    });

    return await this.v2Instance.agents.spawn({
      type,
      ...config
    });
  }

  /**
   * Orchestrate a task (v1.x API)
   */
  async orchestrateTask(
    description: string,
    config?: {
      strategy?: string;
      priority?: string;
      maxAgents?: number;
    }
  ): Promise<any> {
    this.warnings.warn('orchestrateTask', {
      message: 'orchestrateTask() is deprecated. Use tasks.orchestrate() in v2.0',
      migration: 'const result = await flow.tasks.orchestrate({ description, ...config });',
      documentation: 'https://agentic-flow.dev/migration#orchestrate-task'
    });

    return await this.v2Instance.tasks.orchestrate({
      description,
      strategy: (config?.strategy as any) || 'adaptive',
      priority: (config?.priority as any) || 'medium',
      maxAgents: config?.maxAgents
    });
  }

  /**
   * Get memory value (v1.x API)
   */
  async getMemory(key: string): Promise<any> {
    this.warnings.warn('getMemory', {
      message: 'getMemory() is deprecated. Use memory.retrieve() in v2.0',
      migration: 'const data = await flow.memory.retrieve(key);',
      documentation: 'https://agentic-flow.dev/migration#memory'
    });

    return await this.v2Instance.memory.retrieve(key);
  }

  /**
   * Set memory value (v1.x API)
   */
  async setMemory(key: string, value: any): Promise<void> {
    this.warnings.warn('setMemory', {
      message: 'setMemory() is deprecated. Use memory.store() in v2.0',
      migration: 'await flow.memory.store(key, value);',
      documentation: 'https://agentic-flow.dev/migration#memory'
    });

    return await this.v2Instance.memory.store(key, value);
  }

  /**
   * Search memory (v1.x API)
   */
  async searchMemory(query: string, limit?: number): Promise<any[]> {
    this.warnings.warn('searchMemory', {
      message: 'searchMemory() is deprecated. Use memory.vectorSearch() for semantic search',
      migration: 'const results = await flow.memory.vectorSearch(query, { k: limit });',
      documentation: 'https://agentic-flow.dev/migration#memory-search'
    });

    const results = await this.v2Instance.memory.vectorSearch(query, {
      k: limit || 10
    });

    // Format results to match v1 structure
    return results.map((r: any) => ({
      key: r.id,
      value: r.metadata,
      score: r.score
    }));
  }

  /**
   * Get swarm status (v1.x API)
   */
  async getSwarmStatus(): Promise<any> {
    this.warnings.warn('getSwarmStatus', {
      message: 'getSwarmStatus() is deprecated. Use swarms.status() in v2.0',
      migration: 'const status = await flow.swarms.status();',
      documentation: 'https://agentic-flow.dev/migration#swarm-status'
    });

    const v2Status = await this.v2Instance.swarms.status();

    // Translate v2 status format to v1 format
    return {
      swarmId: v2Status.id,
      topology: v2Status.topology,
      agentCount: v2Status.agents.length,
      agents: v2Status.agents.map((agent: any) => ({
        id: agent.id,
        type: agent.type,
        status: agent.status,
        tasksCompleted: agent.metrics?.tasksCompleted || 0
      })),
      status: v2Status.health.status,
      uptime: v2Status.health.uptime
    };
  }

  /**
   * Destroy swarm (v1.x API)
   */
  async destroySwarm(): Promise<void> {
    this.warnings.warn('destroySwarm', {
      message: 'destroySwarm() is deprecated. Use swarms.destroy() in v2.0',
      migration: 'await flow.swarms.destroy();',
      documentation: 'https://agentic-flow.dev/migration#destroy-swarm'
    });

    return await this.v2Instance.swarms.destroy();
  }

  /**
   * Get task status (v1.x API)
   */
  async getTaskStatus(taskId: string): Promise<any> {
    this.warnings.warn('getTaskStatus', {
      message: 'getTaskStatus() is deprecated. Use tasks.status() in v2.0',
      migration: 'const status = await flow.tasks.status(taskId);',
      documentation: 'https://agentic-flow.dev/migration#task-status'
    });

    return await this.v2Instance.tasks.status(taskId);
  }

  /**
   * Wait for task completion (v1.x API)
   */
  async waitForTask(taskId: string, timeout?: number): Promise<any> {
    this.warnings.warn('waitForTask', {
      message: 'waitForTask() is deprecated. Use tasks.wait() in v2.0',
      migration: 'const result = await flow.tasks.wait(taskId, { timeout });',
      documentation: 'https://agentic-flow.dev/migration#wait-task'
    });

    return await this.v2Instance.tasks.wait(taskId, { timeout });
  }

  /**
   * Get underlying v2 instance
   */
  getV2Instance(): any {
    return this.v2Instance;
  }

  /**
   * Get deprecation warnings
   */
  getWarnings(): string[] {
    return this.warnings.getWarnings();
  }

  /**
   * Clear deprecation warnings
   */
  clearWarnings(): void {
    this.warnings.clearWarnings();
  }
}
