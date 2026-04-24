/**
 * SONA Training CLI Commands
 *
 * Train specialized agents with SONA continuous learning
 */
import { AgentFactory, CodebaseTrainer, AgentTemplates } from '../../services/sona-agent-training.js';
import { ValidationUtils } from '../../services/sona-types.js';
import { readFileSync, writeFileSync, readdirSync, statSync, mkdirSync } from 'fs';
import { join, extname, resolve } from 'path';
export function createSONATrainingCommands(program) {
    const sonaTrain = program
        .command('sona-train')
        .description('Train specialized SONA agents');
    // Create agent
    sonaTrain
        .command('create-agent')
        .description('Create a new specialized agent')
        .requiredOption('-n, --name <name>', 'Agent name')
        .option('-p, --purpose <type>', 'Agent purpose: simple, complex, diverse', 'simple')
        .option('-t, --template <template>', 'Use template: code, chat, data, rag, planner, expert')
        .option('--domain <domain>', 'Domain for expert template')
        .action(async (options) => {
        try {
            const factory = new AgentFactory();
            let config;
            if (options.template) {
                switch (options.template) {
                    case 'code':
                        config = AgentTemplates.codeAssistant();
                        break;
                    case 'chat':
                        config = AgentTemplates.chatBot();
                        break;
                    case 'data':
                        config = AgentTemplates.dataAnalyst();
                        break;
                    case 'rag':
                        config = AgentTemplates.ragAgent();
                        break;
                    case 'planner':
                        config = AgentTemplates.taskPlanner();
                        break;
                    case 'expert':
                        if (!options.domain) {
                            throw new Error('--domain required for expert template');
                        }
                        config = AgentTemplates.domainExpert(options.domain);
                        break;
                    default:
                        throw new Error(`Unknown template: ${options.template}`);
                }
                config.name = options.name;
            }
            else {
                config = {
                    name: options.name,
                    purpose: options.purpose
                };
            }
            const engine = factory.createAgent(options.name, config);
            const stats = factory.getAgentStats(options.name);
            console.log('\n✅ Agent created successfully!');
            console.log(`   Name: ${stats?.name}`);
            console.log(`   Purpose: ${stats?.purpose}`);
            console.log(`   Base LoRA Rank: ${stats?.config.baseLoraRank}`);
            console.log(`   Pattern Clusters: ${stats?.config.patternClusters}`);
            console.log(`   Quality Threshold: ${stats?.config.qualityThreshold}`);
            console.log(`   Route: ${stats?.config.route || 'default'}\n`);
            // Save agent config (with path validation)
            const baseDir = resolve(process.cwd(), '.sona-agents');
            mkdirSync(baseDir, { recursive: true });
            const safePath = ValidationUtils.sanitizePath(join('.sona-agents', `${options.name}.json`), process.cwd());
            writeFileSync(safePath, JSON.stringify(stats, null, 2));
            console.log(`   Config saved: ${safePath}\n`);
        }
        catch (error) {
            console.error(`\n❌ Error creating agent: ${error.message}\n`);
            process.exit(1);
        }
    });
    // Train agent
    sonaTrain
        .command('train')
        .description('Train an agent on examples')
        .requiredOption('-n, --name <name>', 'Agent name')
        .requiredOption('-d, --data <file>', 'Training data file (JSONL)')
        .option('-b, --batch-size <number>', 'Batch size', '100')
        .action(async (options) => {
        try {
            const factory = new AgentFactory();
            // Load agent config (with path validation)
            const safePath = ValidationUtils.sanitizePath(join('.sona-agents', `${options.name}.json`), process.cwd());
            const agentConfig = JSON.parse(readFileSync(safePath, 'utf8'));
            // Recreate agent
            factory.createAgent(options.name, agentConfig.config);
            // Load training data
            const dataContent = readFileSync(options.data, 'utf8');
            const lines = dataContent.split('\n').filter(l => l.trim());
            const examples = lines.map(line => JSON.parse(line));
            console.log(`\n🎓 Training agent: ${options.name}`);
            console.log(`   Examples: ${examples.length}`);
            console.log(`   Batch size: ${options.batchSize}\n`);
            // Train in batches
            const batchSize = parseInt(options.batchSize);
            let totalTrained = 0;
            for (let i = 0; i < examples.length; i += batchSize) {
                const batch = examples.slice(i, i + batchSize);
                const trained = await factory.trainAgent(options.name, batch);
                totalTrained += trained;
                console.log(`   Batch ${Math.floor(i / batchSize) + 1}: ${trained} examples`);
            }
            const stats = factory.getAgentStats(options.name);
            console.log(`\n✅ Training complete!`);
            console.log(`   Total examples: ${totalTrained}`);
            console.log(`   Avg quality: ${stats?.avgQuality.toFixed(3)}`);
            console.log(`   Patterns learned: ${stats?.patterns}\n`);
        }
        catch (error) {
            console.error(`\n❌ Error training agent: ${error.message}\n`);
            process.exit(1);
        }
    });
    // Index codebase
    sonaTrain
        .command('index-codebase')
        .description('Index a codebase for pattern learning')
        .requiredOption('-p, --path <path>', 'Path to codebase')
        .option('-e, --extensions <exts>', 'File extensions (comma-separated)', 'ts,js,py,rs,go')
        .option('--max-files <number>', 'Maximum files to index', '1000')
        .action(async (options) => {
        try {
            const trainer = new CodebaseTrainer();
            console.log(`\n📚 Indexing codebase: ${options.path}\n`);
            // Find code files
            const extensions = options.extensions.split(',').map((e) => e.trim());
            const files = findCodeFiles(options.path, extensions, parseInt(options.maxFiles));
            console.log(`   Found ${files.length} files\n`);
            // Index codebase
            const chunks = await trainer.indexCodebase(files);
            const stats = trainer.getStats();
            console.log(`\n✅ Indexing complete!`);
            console.log(`   Files indexed: ${files.length}`);
            console.log(`   Code chunks: ${chunks}`);
            console.log(`   Patterns: ${stats.totalPatterns || 0}\n`);
            // Save index (with path validation)
            const safePath = ValidationUtils.sanitizePath('.sona-codebase-index.json', process.cwd());
            writeFileSync(safePath, JSON.stringify({
                path: options.path,
                files: files.length,
                chunks,
                stats,
                indexed: new Date().toISOString()
            }, null, 2));
            console.log(`   Index saved: ${safePath}\n`);
        }
        catch (error) {
            console.error(`\n❌ Error indexing codebase: ${error.message}\n`);
            process.exit(1);
        }
    });
    // List agents
    sonaTrain
        .command('list')
        .description('List all trained agents')
        .action(() => {
        try {
            const factory = new AgentFactory();
            // Load all agent configs (with path validation)
            const baseDir = resolve(process.cwd(), '.sona-agents');
            try {
                const files = readdirSync(baseDir);
                const agents = files
                    .filter(f => f.endsWith('.json'))
                    .map(f => {
                    const safePath = ValidationUtils.sanitizePath(join('.sona-agents', f), process.cwd());
                    return JSON.parse(readFileSync(safePath, 'utf8'));
                });
                if (agents.length === 0) {
                    console.log('\n📝 No agents found. Create one with: sona-train create-agent\n');
                    return;
                }
                console.log('\n📋 Trained Agents:\n');
                console.log('  Name                 Purpose    Training  Avg Quality  Patterns');
                console.log('  ' + '─'.repeat(70));
                for (const agent of agents) {
                    console.log(`  ${agent.name.padEnd(20)} ` +
                        `${agent.purpose.padEnd(10)} ` +
                        `${agent.trainingCount.toString().padEnd(9)} ` +
                        `${agent.avgQuality.toFixed(3).padEnd(12)} ` +
                        `${agent.patterns}`);
                }
                console.log('');
            }
            catch (dirError) {
                if (dirError.code === 'ENOENT') {
                    console.log('\n📝 No agents found. Create one with: sona-train create-agent\n');
                }
                else {
                    throw dirError;
                }
            }
        }
        catch (error) {
            console.error(`\n❌ Error listing agents: ${error.message}\n`);
        }
    });
    // Query agent
    sonaTrain
        .command('query')
        .description('Query an agent with pattern matching')
        .requiredOption('-n, --name <name>', 'Agent name')
        .requiredOption('-q, --query <text>', 'Query text')
        .option('-k <number>', 'Number of patterns to retrieve', '5')
        .action(async (options) => {
        try {
            const factory = new AgentFactory();
            // Load agent config (with path validation)
            const safePath = ValidationUtils.sanitizePath(join('.sona-agents', `${options.name}.json`), process.cwd());
            const agentConfig = JSON.parse(readFileSync(safePath, 'utf8'));
            factory.createAgent(options.name, agentConfig.config);
            // Mock embedding (in production, use actual embedding service)
            const queryEmbedding = mockEmbedding(options.query);
            // Find patterns
            const patterns = await factory.findPatterns(options.name, queryEmbedding, parseInt(options.k));
            console.log(`\n🔍 Query: "${options.query}"\n`);
            console.log(`   Found ${patterns.length} similar patterns:\n`);
            for (let i = 0; i < patterns.length; i++) {
                const p = patterns[i];
                console.log(`   ${i + 1}. Quality: ${p.avgQuality?.toFixed(3) || 'N/A'}`);
                console.log(`      Similarity: ${p.similarity?.toFixed(3) || 'N/A'}`);
                console.log(`      Context: ${p.context || 'none'}\n`);
            }
        }
        catch (error) {
            console.error(`\n❌ Error querying agent: ${error.message}\n`);
            process.exit(1);
        }
    });
    return sonaTrain;
}
/**
 * Find code files in directory
 */
function findCodeFiles(dir, extensions, maxFiles) {
    const files = [];
    function scan(currentDir) {
        if (files.length >= maxFiles)
            return;
        const entries = readdirSync(currentDir);
        for (const entry of entries) {
            if (files.length >= maxFiles)
                break;
            const fullPath = join(currentDir, entry);
            const stat = statSync(fullPath);
            if (stat.isDirectory()) {
                if (!entry.startsWith('.') && entry !== 'node_modules') {
                    scan(fullPath);
                }
            }
            else {
                const ext = extname(entry).slice(1);
                if (extensions.includes(ext)) {
                    try {
                        const content = readFileSync(fullPath, 'utf8');
                        files.push({
                            path: fullPath,
                            language: ext,
                            content
                        });
                    }
                    catch (error) {
                        // Skip unreadable files
                    }
                }
            }
        }
    }
    scan(dir);
    return files;
}
/**
 * Mock embedding (replace with actual embedding service)
 */
function mockEmbedding(text) {
    const hash = text.split('').reduce((acc, char) => {
        return ((acc << 5) - acc) + char.charCodeAt(0);
    }, 0);
    const embedding = new Array(3072);
    for (let i = 0; i < 3072; i++) {
        const seed = hash + i;
        embedding[i] = (Math.sin(seed) * 10000) - Math.floor(Math.sin(seed) * 10000);
    }
    return embedding;
}
//# sourceMappingURL=sona-train.js.map