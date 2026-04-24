export declare const enabledTools: readonly ["Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebFetch", "WebSearch"];
export declare const toolConfig: {
    enableAllTools: boolean;
    mcpServers: {
        'claude-flow': {
            command: string;
            args: string[];
            env: {
                CLAUDE_FLOW_MEMORY_ENABLED: string;
                CLAUDE_FLOW_COORDINATION_ENABLED: string;
            };
        };
    };
    permissionMode: "default";
    additionalTools: string[];
};
//# sourceMappingURL=tools.d.ts.map