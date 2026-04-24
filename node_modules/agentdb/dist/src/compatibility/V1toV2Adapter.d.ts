/**
 * V1toV2Adapter - Translates v1.x API calls to v2.0 backend
 *
 * Provides transparent backwards compatibility by wrapping v2.0
 * AgentDB instance and translating all v1.x method calls.
 */
import type { V1Config } from './types';
/**
 * Adapter that translates v1.x API calls to v2.0 backend
 */
export declare class V1toV2Adapter {
    private v2Instance;
    private warnings;
    constructor(v2Instance: any, config?: V1Config);
    /**
     * Initialize swarm (v1.x API)
     */
    initSwarm(config: {
        topology?: string;
        maxAgents?: number;
        strategy?: string;
    }): Promise<any>;
    /**
     * Spawn an agent (v1.x API)
     */
    spawnAgent(type: string, config?: any): Promise<any>;
    /**
     * Orchestrate a task (v1.x API)
     */
    orchestrateTask(description: string, config?: {
        strategy?: string;
        priority?: string;
        maxAgents?: number;
    }): Promise<any>;
    /**
     * Get memory value (v1.x API)
     */
    getMemory(key: string): Promise<any>;
    /**
     * Set memory value (v1.x API)
     */
    setMemory(key: string, value: any): Promise<void>;
    /**
     * Search memory (v1.x API)
     */
    searchMemory(query: string, limit?: number): Promise<any[]>;
    /**
     * Get swarm status (v1.x API)
     */
    getSwarmStatus(): Promise<any>;
    /**
     * Destroy swarm (v1.x API)
     */
    destroySwarm(): Promise<void>;
    /**
     * Get task status (v1.x API)
     */
    getTaskStatus(taskId: string): Promise<any>;
    /**
     * Wait for task completion (v1.x API)
     */
    waitForTask(taskId: string, timeout?: number): Promise<any>;
    /**
     * Get underlying v2 instance
     */
    getV2Instance(): any;
    /**
     * Get deprecation warnings
     */
    getWarnings(): string[];
    /**
     * Clear deprecation warnings
     */
    clearWarnings(): void;
}
//# sourceMappingURL=V1toV2Adapter.d.ts.map