/**
 * MCP Tools for Billing System
 * Expose billing operations as MCP tools for Claude integration
 */
import { BillingSystem } from '../index.js';
export interface MCPTool {
    name: string;
    description: string;
    inputSchema: any;
    handler: (params: any) => Promise<any>;
}
export declare class BillingMCPTools {
    private billing;
    private tools;
    constructor(billing: BillingSystem);
    private registerTools;
    private addTool;
    /**
     * Get all tools for MCP registration
     */
    getAllTools(): MCPTool[];
    /**
     * Get tool by name
     */
    getTool(name: string): MCPTool | undefined;
    /**
     * Execute a tool
     */
    executeTool(name: string, params: any): Promise<any>;
}
/**
 * Create MCP tools for a billing system
 */
export declare function createBillingMCPTools(billing: BillingSystem): BillingMCPTools;
/**
 * Export tools for fastMCP registration
 */
export declare function registerBillingTools(server: any, billing: BillingSystem): void;
//# sourceMappingURL=tools.d.ts.map