/**
 * Plugins System - Load and manage Claude Agent SDK plugins
 *
 * Supports loading plugins from:
 * - Local filesystem
 * - NPM packages
 * - Remote URLs
 * - In-memory definitions
 */
/**
 * Plugin configuration types
 */
export interface LocalPluginConfig {
    type: 'local';
    path: string;
}
export interface NpmPluginConfig {
    type: 'npm';
    package: string;
    version?: string;
}
export interface RemotePluginConfig {
    type: 'remote';
    url: string;
    checksum?: string;
}
export interface InlinePluginConfig {
    type: 'inline';
    name: string;
    tools: PluginTool[];
}
export type PluginConfig = LocalPluginConfig | NpmPluginConfig | RemotePluginConfig | InlinePluginConfig;
/**
 * Plugin tool definition
 */
export interface PluginTool {
    name: string;
    description: string;
    inputSchema: Record<string, any>;
    handler: (input: any) => Promise<any>;
}
/**
 * Loaded plugin instance
 */
export interface LoadedPlugin {
    name: string;
    version: string;
    source: string;
    tools: PluginTool[];
    enabled: boolean;
    loadedAt: number;
}
/**
 * Load a plugin from configuration
 */
export declare function loadPlugin(config: PluginConfig): Promise<LoadedPlugin | null>;
/**
 * Get all loaded plugins
 */
export declare function getLoadedPlugins(): LoadedPlugin[];
/**
 * Get plugin by name
 */
export declare function getPlugin(name: string): LoadedPlugin | null;
/**
 * Enable/disable a plugin
 */
export declare function setPluginEnabled(name: string, enabled: boolean): boolean;
/**
 * Unload a plugin
 */
export declare function unloadPlugin(name: string): boolean;
/**
 * Get all tools from enabled plugins
 */
export declare function getAllPluginTools(): PluginTool[];
/**
 * Execute a plugin tool
 */
export declare function executePluginTool(toolName: string, input: any): Promise<any>;
/**
 * Load plugins from SDK configuration
 */
export declare function loadPluginsFromConfig(configs: PluginConfig[]): Promise<LoadedPlugin[]>;
/**
 * Get plugin configuration for SDK query options
 */
export declare function getPluginsForSdk(): any[];
/**
 * Create an inline plugin helper
 */
export declare function createPlugin(name: string, tools: PluginTool[]): LoadedPlugin;
/**
 * Plugin tool builder for type-safe tool creation
 */
export declare function defineTool<T>(config: {
    name: string;
    description: string;
    inputSchema: Record<string, any>;
    handler: (input: T) => Promise<any>;
}): PluginTool;
//# sourceMappingURL=plugins.d.ts.map