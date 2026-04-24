// MCP server management commands
import { spawn, exec } from 'child_process';
import { promisify } from 'util';
const execAsync = promisify(exec);
const MCP_SERVERS = {
    'claude-flow': {
        name: 'claude-flow',
        command: 'npx',
        args: ['claude-flow@alpha', 'mcp', 'start'],
        description: '101 tools (neural networks, GitHub, workflows, DAA)',
        toolCount: 101
    },
    'flow-nexus': {
        name: 'flow-nexus',
        command: 'npx',
        args: ['flow-nexus@latest', 'mcp', 'start'],
        description: '96 cloud tools (sandboxes, distributed swarms, templates)',
        toolCount: 96
    },
    'agentic-payments': {
        name: 'agentic-payments',
        command: 'npx',
        args: ['-y', 'agentic-payments', 'mcp'],
        description: 'Payment authorization and multi-agent consensus',
        toolCount: 10
    }
};
export async function handleMCPCommand(command, server = 'all') {
    switch (command) {
        case 'start':
            await startMCPServers(server);
            break;
        case 'stop':
            await stopMCPServers(server);
            break;
        case 'status':
            await checkMCPStatus(server);
            break;
        case 'list':
            await listMCPTools(server);
            break;
        default:
            console.error(`❌ Unknown MCP command: ${command}`);
            console.error('Available commands: start, stop, status, list');
            process.exit(1);
    }
}
async function startMCPServers(server) {
    console.log(`\n🚀 Starting MCP Server(s): ${server}\n`);
    const serversToStart = server === 'all'
        ? Object.keys(MCP_SERVERS)
        : [server];
    for (const serverName of serversToStart) {
        const config = MCP_SERVERS[serverName];
        if (!config) {
            console.error(`❌ Unknown server: ${serverName}`);
            continue;
        }
        console.log(`📦 ${config.name}`);
        console.log(`   ${config.description}`);
        console.log(`   Command: ${config.command} ${config.args.join(' ')}`);
        console.log(`   Status: Starting as subprocess...\n`);
        // Start server in background
        const child = spawn(config.command, config.args, {
            detached: true,
            stdio: 'inherit',
            env: {
                ...process.env,
                MCP_AUTO_START: 'true'
            }
        });
        child.unref(); // Allow parent to exit independently
    }
    console.log(`\n✅ MCP server(s) started in background`);
    console.log(`💡 Tip: Use 'npx agentic-flow mcp status' to check server status\n`);
}
async function stopMCPServers(server) {
    console.log(`\n🛑 Stopping MCP Server(s): ${server}\n`);
    const serversToStop = server === 'all'
        ? Object.keys(MCP_SERVERS)
        : [server];
    for (const serverName of serversToStop) {
        const config = MCP_SERVERS[serverName];
        if (!config) {
            console.error(`❌ Unknown server: ${serverName}`);
            continue;
        }
        try {
            // Try to kill processes by name
            const killCommand = process.platform === 'win32'
                ? `taskkill /F /IM node.exe /FI "WINDOWTITLE eq ${serverName}*"`
                : `pkill -f "${serverName}"`;
            await execAsync(killCommand);
            console.log(`✅ Stopped ${config.name}`);
        }
        catch (error) {
            if (error.code === 1) {
                console.log(`⚠️  ${config.name} not running`);
            }
            else {
                console.error(`❌ Error stopping ${config.name}:`, error.message);
            }
        }
    }
    console.log();
}
async function checkMCPStatus(server) {
    console.log(`\n📊 MCP Server Status: ${server}\n`);
    const serversToCheck = server === 'all'
        ? Object.keys(MCP_SERVERS)
        : [server];
    for (const serverName of serversToCheck) {
        const config = MCP_SERVERS[serverName];
        if (!config) {
            console.error(`❌ Unknown server: ${serverName}`);
            continue;
        }
        console.log(`📦 ${config.name}`);
        console.log(`   Description: ${config.description}`);
        console.log(`   Tool Count: ${config.toolCount}`);
        try {
            // Check if process is running
            const checkCommand = process.platform === 'win32'
                ? `tasklist /FI "IMAGENAME eq node.exe" /FO CSV | findstr /C:"${serverName}"`
                : `pgrep -f "${serverName}"`;
            const { stdout } = await execAsync(checkCommand);
            const isRunning = stdout.trim().length > 0;
            console.log(`   Status: ${isRunning ? '✅ Running' : '⚠️  Not running'}`);
        }
        catch (error) {
            console.log(`   Status: ⚠️  Not running`);
        }
        console.log();
    }
}
async function listMCPTools(server) {
    console.log(`\n📋 MCP Tools Available\n`);
    const serversToList = server === 'all'
        ? Object.keys(MCP_SERVERS)
        : [server];
    let totalTools = 0;
    for (const serverName of serversToList) {
        const config = MCP_SERVERS[serverName];
        if (!config) {
            console.error(`❌ Unknown server: ${serverName}`);
            continue;
        }
        console.log(`\n📦 ${config.name.toUpperCase()} (${config.toolCount} tools)`);
        console.log(`   ${config.description}`);
        console.log();
        totalTools += config.toolCount;
        // Show example tools based on server
        const examples = getExampleTools(serverName);
        examples.forEach(tool => {
            console.log(`   • ${tool}`);
        });
    }
    // Add in-SDK tools
    console.log(`\n📦 CLAUDE-FLOW-SDK (6 in-process tools)`);
    console.log(`   Fast, zero-latency tools running in-process`);
    console.log();
    console.log(`   • memory_store - Store values in persistent memory`);
    console.log(`   • memory_retrieve - Retrieve values from memory`);
    console.log(`   • memory_list - List all memory keys`);
    console.log(`   • swarm_init - Initialize agent swarms`);
    console.log(`   • agent_spawn - Spawn specialized agents`);
    console.log(`   • coordination_sync - Synchronize agent coordination`);
    totalTools += 6;
    console.log(`\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
    console.log(`📊 TOTAL: ${totalTools} MCP tools available`);
    console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n`);
    console.log(`💡 Usage: Tools are automatically available to agents`);
    console.log(`   npx agentic-flow --agent coder --task "Your task"\n`);
}
function getExampleTools(serverName) {
    switch (serverName) {
        case 'claude-flow':
            return [
                'swarm_init - Initialize multi-agent swarms',
                'agent_spawn - Create specialized AI agents',
                'task_orchestrate - Orchestrate complex tasks',
                'neural_train - Train neural networks with WASM',
                'github_repo_analyze - Analyze GitHub repositories',
                'workflow_create - Create automated workflows',
                'daa_agent_create - Create autonomous agents',
                'performance_report - Generate performance metrics',
                '... and 93 more tools'
            ];
        case 'flow-nexus':
            return [
                'sandbox_create - Create isolated E2B sandboxes',
                'sandbox_execute - Execute code in sandboxes',
                'swarm_init - Initialize cloud-based swarms',
                'neural_train - Distributed neural training',
                'workflow_create - Event-driven workflows',
                'template_deploy - Deploy project templates',
                'user_login - Authenticate Flow Nexus users',
                'storage_upload - Cloud file storage',
                '... and 88 more tools'
            ];
        case 'agentic-payments':
            return [
                'create_active_mandate - Create payment authorization',
                'sign_mandate - Sign with Ed25519 cryptography',
                'verify_mandate - Verify payment signatures',
                'revoke_mandate - Revoke authorizations',
                'verify_consensus - Multi-agent consensus',
                'generate_agent_identity - Create agent identities',
                '... and 4 more tools'
            ];
        default:
            return [];
    }
}
//# sourceMappingURL=mcpCommands.js.map