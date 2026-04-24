#!/usr/bin/env node
export interface MCPOptions {
    port?: string;
    debug?: boolean;
}
export declare function startStdioServer(options?: MCPOptions): Promise<void>;
export declare function startHttpServer(options?: MCPOptions): Promise<void>;
export declare function listTools(): void;
export declare function showStatus(): void;
export declare function handleMCPCommand(command: string, options?: MCPOptions): void | Promise<void>;
//# sourceMappingURL=mcp.d.ts.map