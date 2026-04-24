/**
 * Plugins System - Load and manage Claude Agent SDK plugins
 *
 * Supports loading plugins from:
 * - Local filesystem
 * - NPM packages
 * - Remote URLs
 * - In-memory definitions
 */
import { logger } from "../utils/logger.js";
import { existsSync, readFileSync } from "fs";
import { join, resolve } from "path";
// Plugin registry
const loadedPlugins = new Map();
/**
 * Load a plugin from configuration
 */
export async function loadPlugin(config) {
    try {
        switch (config.type) {
            case 'local':
                return await loadLocalPlugin(config);
            case 'npm':
                return await loadNpmPlugin(config);
            case 'remote':
                return await loadRemotePlugin(config);
            case 'inline':
                return loadInlinePlugin(config);
            default:
                logger.error('Unknown plugin type', { config });
                return null;
        }
    }
    catch (error) {
        logger.error('Failed to load plugin', { config, error: error.message });
        return null;
    }
}
/**
 * Load plugin from local filesystem
 */
async function loadLocalPlugin(config) {
    const pluginPath = resolve(config.path);
    if (!existsSync(pluginPath)) {
        logger.error('Plugin path does not exist', { path: pluginPath });
        return null;
    }
    // Look for package.json or plugin.json
    const packageJsonPath = join(pluginPath, 'package.json');
    const pluginJsonPath = join(pluginPath, 'plugin.json');
    let metadata = { name: 'unknown', version: '0.0.0' };
    if (existsSync(packageJsonPath)) {
        metadata = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
    }
    else if (existsSync(pluginJsonPath)) {
        metadata = JSON.parse(readFileSync(pluginJsonPath, 'utf-8'));
    }
    // Try to load the plugin module
    const mainPath = join(pluginPath, metadata.main || 'index.js');
    if (!existsSync(mainPath)) {
        logger.error('Plugin main file not found', { path: mainPath });
        return null;
    }
    const module = await import(mainPath);
    const tools = module.tools || module.default?.tools || [];
    const plugin = {
        name: metadata.name,
        version: metadata.version,
        source: `local:${pluginPath}`,
        tools,
        enabled: true,
        loadedAt: Date.now()
    };
    loadedPlugins.set(plugin.name, plugin);
    logger.info('Local plugin loaded', { name: plugin.name, tools: tools.length });
    return plugin;
}
/**
 * Load plugin from NPM package
 */
async function loadNpmPlugin(config) {
    try {
        const module = await import(config.package);
        const metadata = module.default?.metadata || { name: config.package, version: config.version || '0.0.0' };
        const tools = module.tools || module.default?.tools || [];
        const plugin = {
            name: metadata.name || config.package,
            version: metadata.version || config.version || '0.0.0',
            source: `npm:${config.package}`,
            tools,
            enabled: true,
            loadedAt: Date.now()
        };
        loadedPlugins.set(plugin.name, plugin);
        logger.info('NPM plugin loaded', { name: plugin.name, tools: tools.length });
        return plugin;
    }
    catch (error) {
        logger.error('Failed to load NPM plugin', { package: config.package, error: error.message });
        return null;
    }
}
/**
 * Load plugin from remote URL
 */
async function loadRemotePlugin(config) {
    try {
        const response = await fetch(config.url);
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        const content = await response.text();
        // Verify checksum if provided
        if (config.checksum) {
            const hash = await computeHash(content);
            if (hash !== config.checksum) {
                throw new Error('Checksum mismatch - plugin may be compromised');
            }
        }
        // Parse plugin definition (JSON format)
        const pluginDef = JSON.parse(content);
        const plugin = {
            name: pluginDef.name || 'remote-plugin',
            version: pluginDef.version || '0.0.0',
            source: `remote:${config.url}`,
            tools: pluginDef.tools || [],
            enabled: true,
            loadedAt: Date.now()
        };
        loadedPlugins.set(plugin.name, plugin);
        logger.info('Remote plugin loaded', { name: plugin.name, url: config.url });
        return plugin;
    }
    catch (error) {
        logger.error('Failed to load remote plugin', { url: config.url, error: error.message });
        return null;
    }
}
/**
 * Load inline plugin from configuration
 */
function loadInlinePlugin(config) {
    const plugin = {
        name: config.name,
        version: '1.0.0',
        source: 'inline',
        tools: config.tools,
        enabled: true,
        loadedAt: Date.now()
    };
    loadedPlugins.set(plugin.name, plugin);
    logger.info('Inline plugin loaded', { name: plugin.name, tools: config.tools.length });
    return plugin;
}
/**
 * Compute SHA-256 hash for checksum verification
 */
async function computeHash(content) {
    const encoder = new TextEncoder();
    const data = encoder.encode(content);
    const hashBuffer = await crypto.subtle.digest('SHA-256', data);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
}
/**
 * Get all loaded plugins
 */
export function getLoadedPlugins() {
    return Array.from(loadedPlugins.values());
}
/**
 * Get plugin by name
 */
export function getPlugin(name) {
    return loadedPlugins.get(name) || null;
}
/**
 * Enable/disable a plugin
 */
export function setPluginEnabled(name, enabled) {
    const plugin = loadedPlugins.get(name);
    if (!plugin)
        return false;
    plugin.enabled = enabled;
    logger.info('Plugin state changed', { name, enabled });
    return true;
}
/**
 * Unload a plugin
 */
export function unloadPlugin(name) {
    const existed = loadedPlugins.delete(name);
    if (existed) {
        logger.info('Plugin unloaded', { name });
    }
    return existed;
}
/**
 * Get all tools from enabled plugins
 */
export function getAllPluginTools() {
    const tools = [];
    for (const plugin of loadedPlugins.values()) {
        if (plugin.enabled) {
            tools.push(...plugin.tools);
        }
    }
    return tools;
}
/**
 * Execute a plugin tool
 */
export async function executePluginTool(toolName, input) {
    for (const plugin of loadedPlugins.values()) {
        if (!plugin.enabled)
            continue;
        const tool = plugin.tools.find(t => t.name === toolName);
        if (tool) {
            logger.info('Executing plugin tool', { plugin: plugin.name, tool: toolName });
            return tool.handler(input);
        }
    }
    throw new Error(`Plugin tool not found: ${toolName}`);
}
/**
 * Load plugins from SDK configuration
 */
export async function loadPluginsFromConfig(configs) {
    const loaded = [];
    for (const config of configs) {
        const plugin = await loadPlugin(config);
        if (plugin) {
            loaded.push(plugin);
        }
    }
    logger.info('Plugins loaded from config', { total: loaded.length });
    return loaded;
}
/**
 * Get plugin configuration for SDK query options
 */
export function getPluginsForSdk() {
    const plugins = [];
    for (const plugin of loadedPlugins.values()) {
        if (!plugin.enabled)
            continue;
        if (plugin.source.startsWith('local:')) {
            plugins.push({
                type: 'local',
                path: plugin.source.replace('local:', '')
            });
        }
        else if (plugin.source.startsWith('npm:')) {
            plugins.push({
                type: 'npm',
                package: plugin.source.replace('npm:', '')
            });
        }
    }
    return plugins;
}
/**
 * Create an inline plugin helper
 */
export function createPlugin(name, tools) {
    return loadInlinePlugin({ type: 'inline', name, tools });
}
/**
 * Plugin tool builder for type-safe tool creation
 */
export function defineTool(config) {
    return {
        name: config.name,
        description: config.description,
        inputSchema: config.inputSchema,
        handler: config.handler
    };
}
//# sourceMappingURL=plugins.js.map