/**
 * Build Agents Hook - Generate optimized agent configurations
 * Creates YAML agent definitions based on pretrain analysis
 */
import { z } from 'zod';
import * as path from 'path';
import * as fs from 'fs';
import { loadIntelligence } from './shared.js';
// Focus mode configurations
const focusConfigs = {
    quality: {
        description: 'Emphasizes code quality, best practices, and maintainability',
        priorities: ['code-review', 'refactoring', 'documentation', 'testing'],
        temperature: 0.3
    },
    speed: {
        description: 'Optimized for rapid development and iteration',
        priorities: ['implementation', 'prototyping', 'quick-fixes'],
        temperature: 0.7
    },
    security: {
        description: 'Security-first development with vulnerability awareness',
        priorities: ['security-audit', 'input-validation', 'authentication', 'encryption'],
        temperature: 0.2
    },
    testing: {
        description: 'Test-driven development with comprehensive coverage',
        priorities: ['unit-tests', 'integration-tests', 'e2e-tests', 'mocking'],
        temperature: 0.4
    },
    fullstack: {
        description: 'Balanced full-stack development capabilities',
        priorities: ['frontend', 'backend', 'database', 'api-design'],
        temperature: 0.5
    }
};
export const hookBuildAgentsTool = {
    name: 'hook_build_agents',
    description: 'Generate optimized agent configurations from pretrain data',
    parameters: z.object({
        focus: z.enum(['quality', 'speed', 'security', 'testing', 'fullstack'])
            .optional()
            .default('quality')
            .describe('Focus mode for agent generation'),
        output: z.string().optional().default('.claude/agents').describe('Output directory'),
        includePrompts: z.boolean().optional().default(true).describe('Include system prompts'),
        format: z.enum(['yaml', 'json']).optional().default('yaml').describe('Output format')
    }),
    execute: async ({ focus, output, includePrompts, format }, { onProgress }) => {
        const startTime = Date.now();
        const intel = loadIntelligence();
        const focusConfig = focusConfigs[focus];
        onProgress?.({ progress: 0.1, message: 'Analyzing patterns...' });
        // Detect languages from patterns
        const detectedLangs = new Set();
        const detectedFrameworks = new Set();
        for (const state of Object.keys(intel.patterns)) {
            if (state.includes('.rs'))
                detectedLangs.add('rust');
            if (state.includes('.ts') || state.includes('.js'))
                detectedLangs.add('typescript');
            if (state.includes('.tsx') || state.includes('.jsx'))
                detectedFrameworks.add('react');
            if (state.includes('.py'))
                detectedLangs.add('python');
            if (state.includes('.go'))
                detectedLangs.add('go');
            if (state.includes('.vue'))
                detectedFrameworks.add('vue');
            if (state.includes('.sql'))
                detectedFrameworks.add('database');
        }
        onProgress?.({ progress: 0.3, message: 'Generating agent configs...' });
        const agents = [];
        // Language-specific agents
        if (detectedLangs.has('rust')) {
            agents.push({
                name: 'rust-specialist',
                type: 'rust-developer',
                description: 'Rust development specialist for this codebase',
                capabilities: ['cargo', 'unsafe-rust', 'async-rust', 'wasm', 'error-handling'],
                focus: focusConfig.priorities,
                temperature: focusConfig.temperature,
                systemPrompt: includePrompts ? `You are a Rust specialist.
Focus on: memory safety, zero-cost abstractions, idiomatic Rust patterns.
Use cargo conventions, prefer Result over panic, leverage the type system.
${focusConfig.description}` : undefined
            });
        }
        if (detectedLangs.has('typescript')) {
            agents.push({
                name: 'typescript-specialist',
                type: 'typescript-developer',
                description: 'TypeScript development specialist',
                capabilities: ['types', 'generics', 'decorators', 'async-await', 'modules'],
                focus: focusConfig.priorities,
                temperature: focusConfig.temperature,
                systemPrompt: includePrompts ? `You are a TypeScript specialist.
Focus on: strict typing, type inference, generic patterns, module organization.
Prefer type safety over any, use discriminated unions, leverage utility types.
${focusConfig.description}` : undefined
            });
        }
        if (detectedLangs.has('python')) {
            agents.push({
                name: 'python-specialist',
                type: 'python-developer',
                description: 'Python development specialist',
                capabilities: ['typing', 'async', 'testing', 'packaging', 'data-science'],
                focus: focusConfig.priorities,
                temperature: focusConfig.temperature,
                systemPrompt: includePrompts ? `You are a Python specialist.
Focus on: type hints, PEP standards, pythonic idioms, virtual environments.
Use dataclasses, prefer pathlib, leverage context managers.
${focusConfig.description}` : undefined
            });
        }
        if (detectedLangs.has('go')) {
            agents.push({
                name: 'go-specialist',
                type: 'go-developer',
                description: 'Go development specialist',
                capabilities: ['goroutines', 'channels', 'interfaces', 'testing', 'modules'],
                focus: focusConfig.priorities,
                temperature: focusConfig.temperature,
                systemPrompt: includePrompts ? `You are a Go specialist.
Focus on: simplicity, explicit error handling, goroutines, interface composition.
Follow Go conventions, use go fmt, prefer composition over inheritance.
${focusConfig.description}` : undefined
            });
        }
        // Framework-specific agents
        if (detectedFrameworks.has('react')) {
            agents.push({
                name: 'react-specialist',
                type: 'react-developer',
                description: 'React/Next.js development specialist',
                capabilities: ['hooks', 'state-management', 'components', 'ssr', 'testing'],
                focus: focusConfig.priorities,
                temperature: focusConfig.temperature,
                systemPrompt: includePrompts ? `You are a React specialist.
Focus on: functional components, hooks, state management, performance optimization.
Prefer composition, use memo wisely, follow React best practices.
${focusConfig.description}` : undefined
            });
        }
        if (detectedFrameworks.has('database')) {
            agents.push({
                name: 'database-specialist',
                type: 'database-specialist',
                description: 'Database design and optimization specialist',
                capabilities: ['schema-design', 'queries', 'indexing', 'migrations', 'orm'],
                focus: focusConfig.priorities,
                temperature: focusConfig.temperature,
                systemPrompt: includePrompts ? `You are a database specialist.
Focus on: normalized schemas, efficient queries, proper indexing, data integrity.
Consider performance implications, use transactions appropriately.
${focusConfig.description}` : undefined
            });
        }
        // Focus-specific agents
        if (focus === 'testing' || focus === 'quality') {
            agents.push({
                name: 'test-architect',
                type: 'test-engineer',
                description: 'Testing and quality assurance specialist',
                capabilities: ['unit-tests', 'integration-tests', 'mocking', 'coverage', 'tdd'],
                focus: ['testing', 'quality', 'reliability'],
                temperature: focusConfig.temperature,
                systemPrompt: includePrompts ? `You are a testing specialist.
Focus on: comprehensive test coverage, meaningful assertions, test isolation.
Write tests first when possible, mock external dependencies, aim for >80% coverage.
${focusConfig.description}` : undefined
            });
        }
        if (focus === 'security') {
            agents.push({
                name: 'security-auditor',
                type: 'security-specialist',
                description: 'Security audit and hardening specialist',
                capabilities: ['vulnerability-scan', 'auth', 'encryption', 'input-validation', 'owasp'],
                focus: ['security', 'compliance', 'hardening'],
                temperature: focusConfig.temperature,
                systemPrompt: includePrompts ? `You are a security specialist.
Focus on: OWASP top 10, input validation, authentication, authorization, encryption.
Never trust user input, use parameterized queries, implement defense in depth.
${focusConfig.description}` : undefined
            });
        }
        // Add coordinator
        agents.push({
            name: 'project-coordinator',
            type: 'coordinator',
            description: 'Coordinates multi-agent workflows for this project',
            capabilities: ['task-decomposition', 'agent-routing', 'context-management'],
            focus: focusConfig.priorities,
            temperature: focusConfig.temperature
        });
        onProgress?.({ progress: 0.7, message: 'Writing agent files...' });
        // Create output directory
        const outputDir = path.join(process.cwd(), output || '.claude/agents');
        if (!fs.existsSync(outputDir)) {
            fs.mkdirSync(outputDir, { recursive: true });
        }
        // Write agent files
        const savedFiles = [];
        for (const agent of agents) {
            const filename = `${agent.name}.${format}`;
            const filePath = path.join(outputDir, filename);
            let content;
            if (format === 'yaml') {
                content = `# ${agent.description}
name: ${agent.name}
type: ${agent.type}
description: ${agent.description}
capabilities:
${agent.capabilities.map(c => `  - ${c}`).join('\n')}
focus:
${agent.focus.map(f => `  - ${f}`).join('\n')}
temperature: ${agent.temperature}
${agent.systemPrompt ? `systemPrompt: |\n  ${agent.systemPrompt.split('\n').join('\n  ')}` : ''}
`;
            }
            else {
                content = JSON.stringify(agent, null, 2);
            }
            fs.writeFileSync(filePath, content);
            savedFiles.push(filename);
        }
        // Write index file
        const indexPath = path.join(outputDir, `index.${format}`);
        if (format === 'yaml') {
            const indexContent = `# Generated Agent Index
# Focus: ${focus}
# Generated: ${new Date().toISOString()}

agents:
${agents.map(a => `  - ${a.name}`).join('\n')}

detected:
  languages:
${[...detectedLangs].map(l => `    - ${l}`).join('\n') || '    - generic'}
  frameworks:
${[...detectedFrameworks].map(f => `    - ${f}`).join('\n') || '    - none'}
`;
            fs.writeFileSync(indexPath, indexContent);
        }
        else {
            fs.writeFileSync(indexPath, JSON.stringify({
                focus,
                generated: new Date().toISOString(),
                agents: agents.map(a => a.name),
                detected: {
                    languages: [...detectedLangs],
                    frameworks: [...detectedFrameworks]
                }
            }, null, 2));
        }
        savedFiles.push(`index.${format}`);
        onProgress?.({ progress: 1.0, message: 'Agent generation complete!' });
        const latency = Date.now() - startTime;
        return {
            success: true,
            focus,
            agentsGenerated: agents.length,
            agents: agents.map(a => a.name),
            detectedLanguages: [...detectedLangs],
            detectedFrameworks: [...detectedFrameworks],
            savedFiles,
            outputDir: output,
            latencyMs: latency,
            timestamp: new Date().toISOString()
        };
    }
};
//# sourceMappingURL=build-agents.js.map