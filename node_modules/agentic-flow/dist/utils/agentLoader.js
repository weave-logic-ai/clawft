// Agent loader for .claude/agents integration
import { readFileSync, readdirSync, statSync, existsSync } from 'fs';
import { join, extname, dirname } from 'path';
import { fileURLToPath } from 'url';
import { logger } from './logger.js';
// Get the package root directory
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const packageRoot = join(__dirname, '../..');
const defaultAgentsDir = join(packageRoot, '.claude/agents');
/**
 * Parse agent markdown file with frontmatter
 */
function parseAgentFile(filePath) {
    try {
        const content = readFileSync(filePath, 'utf-8');
        // Check for frontmatter
        const frontmatterMatch = content.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/);
        if (!frontmatterMatch) {
            logger.warn('Agent file missing frontmatter', { filePath });
            return null;
        }
        const [, frontmatter, systemPrompt] = frontmatterMatch;
        // Parse YAML-like frontmatter
        const meta = {};
        frontmatter.split('\n').forEach(line => {
            const match = line.match(/^(\w+):\s*(.+)$/);
            if (match) {
                const [, key, value] = match;
                // Remove quotes if present
                const cleanValue = value.replace(/^["']|["']$/g, '');
                if (key === 'tools') {
                    meta[key] = cleanValue.split(',').map(t => t.trim());
                }
                else {
                    meta[key] = cleanValue;
                }
            }
        });
        if (!meta.name || !meta.description) {
            logger.warn('Agent file missing required metadata', { filePath });
            return null;
        }
        return {
            name: meta.name,
            description: meta.description,
            systemPrompt: systemPrompt.trim(),
            color: meta.color,
            tools: meta.tools,
            filePath
        };
    }
    catch (error) {
        logger.error('Failed to parse agent file', { filePath, error });
        return null;
    }
}
/**
 * Recursively find all agent definition files
 */
function findAgentFiles(dir) {
    const files = [];
    try {
        const entries = readdirSync(dir);
        for (const entry of entries) {
            const fullPath = join(dir, entry);
            const stat = statSync(fullPath);
            if (stat.isDirectory()) {
                files.push(...findAgentFiles(fullPath));
            }
            else if (extname(entry) === '.md' && entry !== 'README.md') {
                files.push(fullPath);
            }
        }
    }
    catch (error) {
        logger.warn('Failed to read directory', { dir, error });
    }
    return files;
}
/**
 * Load all agents from .claude/agents directory with deduplication
 * Local agents (.claude/agents in CWD) override package agents
 */
export function loadAgents(agentsDir) {
    const agents = new Map();
    const agentsByRelativePath = new Map();
    // If explicit directory is provided, use only that
    if (agentsDir) {
        logger.info('Loading agents from explicit directory', { agentsDir });
        const agentFiles = findAgentFiles(agentsDir);
        for (const filePath of agentFiles) {
            const agent = parseAgentFile(filePath);
            if (agent) {
                agents.set(agent.name, agent);
            }
        }
        return agents;
    }
    // Otherwise, load from both package and local with deduplication
    const localAgentsDir = join(process.cwd(), '.claude/agents');
    // 1. Load package agents first (if they exist)
    if (existsSync(defaultAgentsDir)) {
        logger.info('Loading package agents', { agentsDir: defaultAgentsDir });
        const packageFiles = findAgentFiles(defaultAgentsDir);
        logger.debug('Found package agent files', { count: packageFiles.length });
        for (const filePath of packageFiles) {
            const agent = parseAgentFile(filePath);
            if (agent) {
                const relativePath = filePath.substring(defaultAgentsDir.length + 1);
                agentsByRelativePath.set(relativePath, agent);
                agents.set(agent.name, agent);
                logger.debug('Loaded package agent', { name: agent.name, path: relativePath });
            }
        }
    }
    // 2. Load local agents (override package agents with same relative path)
    if (existsSync(localAgentsDir) && localAgentsDir !== defaultAgentsDir) {
        logger.info('Loading local agents', { agentsDir: localAgentsDir });
        const localFiles = findAgentFiles(localAgentsDir);
        logger.debug('Found local agent files', { count: localFiles.length });
        for (const filePath of localFiles) {
            const agent = parseAgentFile(filePath);
            if (agent) {
                const relativePath = filePath.substring(localAgentsDir.length + 1);
                const existingAgent = agentsByRelativePath.get(relativePath);
                if (existingAgent) {
                    logger.info('Local agent overrides package agent', {
                        name: agent.name,
                        path: relativePath
                    });
                    // Remove old agent
                    agents.delete(existingAgent.name);
                }
                agentsByRelativePath.set(relativePath, agent);
                agents.set(agent.name, agent);
                logger.debug('Loaded local agent', { name: agent.name, path: relativePath });
            }
        }
    }
    logger.info('Agents loaded successfully', {
        total: agents.size,
        package: existsSync(defaultAgentsDir) ? findAgentFiles(defaultAgentsDir).length : 0,
        local: existsSync(localAgentsDir) ? findAgentFiles(localAgentsDir).length : 0
    });
    return agents;
}
/**
 * Get a specific agent by name
 */
export function getAgent(name, agentsDir) {
    const agents = loadAgents(agentsDir);
    return agents.get(name);
}
/**
 * List all available agents
 */
export function listAgents(agentsDir) {
    const agents = loadAgents(agentsDir);
    return Array.from(agents.values());
}
//# sourceMappingURL=agentLoader.js.map