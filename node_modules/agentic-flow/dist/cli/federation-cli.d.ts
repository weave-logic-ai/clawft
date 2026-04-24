#!/usr/bin/env node
/**
 * Federation Hub CLI - Manage ephemeral agent federation
 * Supports hub server, agent lifecycle, stats, and monitoring
 */
export interface FederationHubConfig {
    port?: number;
    dbPath?: string;
    maxAgents?: number;
    syncInterval?: number;
    verbose?: boolean;
}
export interface AgentConfig {
    agentId?: string;
    tenantId?: string;
    lifetime?: number;
    hubEndpoint?: string;
    agentType?: string;
}
/**
 * Federation Hub CLI Manager
 */
export declare class FederationCLI {
    private hubProcess;
    /**
     * Start federation hub server
     */
    startHub(config?: FederationHubConfig): Promise<void>;
    /**
     * Spawn ephemeral agent
     */
    spawnAgent(config?: AgentConfig): Promise<void>;
    /**
     * Show hub statistics
     */
    stats(hubEndpoint?: string): Promise<void>;
    /**
     * Show federation status
     */
    status(): Promise<void>;
    /**
     * Run multi-agent collaboration test
     */
    testCollaboration(): Promise<void>;
    /**
     * Print help message
     */
    printHelp(): void;
}
/**
 * CLI command handler
 */
export declare function handleFederationCommand(args: string[]): Promise<void>;
//# sourceMappingURL=federation-cli.d.ts.map