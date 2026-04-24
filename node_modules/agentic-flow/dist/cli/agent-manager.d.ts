#!/usr/bin/env node
/**
 * Agent Management CLI - Create, list, and manage custom agents
 * Supports both npm package agents and local .claude/agents
 * Includes conflict detection and deduplication
 */
export declare class AgentManager {
    /**
     * Find all agent files from both package and local directories
     * Deduplicates by preferring local over package
     */
    private findAllAgents;
    /**
     * Recursively scan directory for agent markdown files
     */
    private scanAgentsDirectory;
    /**
     * Parse agent markdown file and extract metadata
     */
    private parseAgentFile;
    /**
     * Get category from file path
     */
    private getCategoryFromPath;
    /**
     * List all agents with deduplication
     */
    list(format?: 'summary' | 'detailed' | 'json'): void;
    /**
     * Create a new agent
     */
    create(options: {
        name?: string;
        description?: string;
        category?: string;
        systemPrompt?: string;
        tools?: string[];
        interactive?: boolean;
    }): Promise<void>;
    /**
     * Generate agent markdown with frontmatter
     */
    private generateAgentMarkdown;
    /**
     * Get information about a specific agent
     */
    info(name: string): void;
    /**
     * Check for conflicts between package and local agents
     */
    checkConflicts(): void;
}
/**
 * CLI command handler
 */
export declare function handleAgentCommand(args: string[]): Promise<void>;
//# sourceMappingURL=agent-manager.d.ts.map