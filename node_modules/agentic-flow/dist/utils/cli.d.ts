export interface CliOptions {
    mode: 'agent' | 'parallel' | 'list' | 'mcp' | 'mcp-manager' | 'config' | 'agent-manager' | 'proxy' | 'quic' | 'claude-code' | 'reasoningbank' | 'federation';
    agent?: string;
    task?: string;
    model?: string;
    provider?: string;
    anthropicApiKey?: string;
    openrouterApiKey?: string;
    stream?: boolean;
    temperature?: number;
    maxTokens?: number;
    agentsDir?: string;
    outputFormat?: 'text' | 'json' | 'markdown';
    verbose?: boolean;
    timeout?: number;
    retryOnError?: boolean;
    optimize?: boolean;
    optimizePriority?: 'quality' | 'balanced' | 'cost' | 'speed' | 'privacy';
    maxCost?: number;
    claudeCode?: boolean;
    agentBooster?: boolean;
    boosterThreshold?: number;
    help?: boolean;
    version?: boolean;
    mcpCommand?: string;
    mcpServer?: string;
}
export declare function parseArgs(): CliOptions;
export declare function printHelp(): void;
export declare function validateOptions(options: CliOptions): string | null;
//# sourceMappingURL=cli.d.ts.map