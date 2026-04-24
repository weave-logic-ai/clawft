/**
 * ReasoningBank CLI Commands
 * Handles demo, test, init, benchmark, status, consolidate, list
 */
import { spawn } from 'child_process';
import { join } from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
export async function handleReasoningBankCommand(subcommand) {
    const args = process.argv.slice(4); // Get args after 'reasoningbank <subcommand>'
    try {
        switch (subcommand) {
            case 'demo':
                await runExternalScript('../reasoningbank/demo-comparison.js');
                break;
            case 'test':
                await runExternalScript('../reasoningbank/test-validation.js');
                break;
            case 'init':
                await initDatabase();
                break;
            case 'benchmark':
                await runExternalScript('../reasoningbank/benchmark.js');
                break;
            case 'status':
                await showStatus();
                break;
            case 'consolidate':
                await runConsolidation();
                break;
            case 'list':
                await listMemories(args);
                break;
            case 'help':
            default:
                printHelp();
                break;
        }
    }
    catch (error) {
        console.error(`\n❌ Error: ${error instanceof Error ? error.message : String(error)}\n`);
        process.exit(1);
    }
}
async function runExternalScript(relativePath) {
    const scriptPath = join(__dirname, relativePath);
    return new Promise((resolve, reject) => {
        const child = spawn('node', [scriptPath], {
            stdio: 'inherit',
            cwd: process.cwd()
        });
        child.on('exit', (code) => {
            if (code === 0) {
                resolve();
            }
            else {
                reject(new Error(`Script exited with code ${code}`));
            }
        });
        child.on('error', (error) => {
            reject(error);
        });
    });
}
async function initDatabase() {
    console.log('\n🔧 Initializing ReasoningBank Database');
    console.log('═'.repeat(50));
    const { initialize } = await import('../reasoningbank/index.js');
    console.log('\nInitializing database with migrations...\n');
    await initialize();
    console.log('\n✅ Database initialized successfully!');
    console.log('\nSchema created:');
    console.log('  • patterns (reasoning memories)');
    console.log('  • pattern_embeddings');
    console.log('  • pattern_links');
    console.log('  • task_trajectories');
    console.log('  • matts_runs');
    console.log('  • consolidation_runs');
    console.log('  • metrics_log\n');
}
async function showStatus() {
    console.log('\n📊 ReasoningBank Status');
    console.log('═'.repeat(50));
    const { db } = await import('../reasoningbank/index.js');
    const dbInstance = db.getDb();
    const stats = {
        totalMemories: dbInstance.prepare("SELECT COUNT(*) as count FROM patterns WHERE type = 'reasoning_memory'").get(),
        avgConfidence: dbInstance.prepare("SELECT AVG(confidence) as avg FROM patterns WHERE type = 'reasoning_memory'").get(),
        totalEmbeddings: dbInstance.prepare('SELECT COUNT(*) as count FROM pattern_embeddings').get(),
        totalTrajectories: dbInstance.prepare('SELECT COUNT(*) as count FROM task_trajectories').get()
    };
    console.log(`\n📈 Statistics:`);
    console.log(`  • Total memories: ${stats.totalMemories.count}`);
    console.log(`  • Average confidence: ${stats.avgConfidence.avg?.toFixed(2) || 'N/A'}`);
    console.log(`  • Total embeddings: ${stats.totalEmbeddings.count}`);
    console.log(`  • Total trajectories: ${stats.totalTrajectories.count}\n`);
}
async function runConsolidation() {
    console.log('\n🔄 Running Memory Consolidation');
    console.log('═'.repeat(50));
    console.log('\nDeduplicating and pruning memories...\n');
    const { consolidate } = await import('../reasoningbank/index.js');
    const startTime = Date.now();
    const result = await consolidate();
    const duration = Date.now() - startTime;
    console.log('✅ Consolidation complete!\n');
    console.log(`📊 Results:`);
    console.log(`  • Memories processed: ${result.itemsProcessed}`);
    console.log(`  • Duplicates found: ${result.duplicatesFound}`);
    console.log(`  • Contradictions found: ${result.contradictionsFound}`);
    console.log(`  • Pruned: ${result.itemsPruned}`);
    console.log(`  • Duration: ${result.durationMs}ms\n`);
}
async function listMemories(args) {
    const { db } = await import('../reasoningbank/index.js');
    const dbInstance = db.getDb();
    // Parse arguments
    const sortBy = args.includes('--sort') ? args[args.indexOf('--sort') + 1] : 'created_at';
    const limit = args.includes('--limit') ? parseInt(args[args.indexOf('--limit') + 1], 10) : 10;
    console.log('\n📚 Memory Bank Contents');
    console.log('═'.repeat(50));
    console.log(`\nShowing top ${limit} memories (sorted by ${sortBy})\n`);
    let orderBy = 'created_at DESC';
    if (sortBy === 'confidence') {
        orderBy = 'confidence DESC';
    }
    else if (sortBy === 'usage') {
        orderBy = 'usage_count DESC';
    }
    const memories = dbInstance.prepare(`
    SELECT
      id,
      json_extract(pattern_data, '$.title') as title,
      json_extract(pattern_data, '$.description') as description,
      json_extract(pattern_data, '$.domain') as domain,
      confidence,
      usage_count,
      created_at
    FROM patterns
    WHERE type = 'reasoning_memory'
    ORDER BY ${orderBy}
    LIMIT ?
  `).all(limit);
    if (memories.length === 0) {
        console.log('No memories found. Run `npx agentic-flow reasoningbank demo` to create some!\n');
        return;
    }
    for (let i = 0; i < memories.length; i++) {
        const mem = memories[i];
        console.log(`${i + 1}. ${mem.title}`);
        console.log(`   Confidence: ${parseFloat(mem.confidence).toFixed(2)} | Usage: ${mem.usage_count} | Created: ${mem.created_at}`);
        console.log(`   Domain: ${mem.domain}`);
        console.log(`   ${mem.description}`);
        console.log('');
    }
}
function printHelp() {
    console.log(`
🧠 ReasoningBank - Closed-loop memory system for AI agents

USAGE:
  npx agentic-flow reasoningbank <COMMAND>

COMMANDS:
  demo          Run interactive demo showing learning progression
  test          Run validation test suite
  init          Initialize database schema
  benchmark     Run performance benchmarks
  status        Show memory statistics
  consolidate   Run memory consolidation now
  list          List memories with options
  help          Show this help message

LIST OPTIONS:
  --sort <field>    Sort by: confidence, usage, created_at (default)
  --limit <n>       Show top N memories (default: 10)

EXAMPLES:
  # Run demo to see 0% → 100% success transformation
  npx agentic-flow reasoningbank demo

  # Initialize database
  npx agentic-flow reasoningbank init

  # Run validation tests
  npx agentic-flow reasoningbank test

  # Show current statistics
  npx agentic-flow reasoningbank status

  # Consolidate memories (dedupe + prune)
  npx agentic-flow reasoningbank consolidate

  # List top 10 memories by confidence
  npx agentic-flow reasoningbank list --sort confidence --limit 10

  # List top 5 most used memories
  npx agentic-flow reasoningbank list --sort usage --limit 5

For more information: https://github.com/ruvnet/agentic-flow
  `);
}
//# sourceMappingURL=reasoningbankCommands.js.map