#!/usr/bin/env node
/**
 * Integration test for ReasoningBank
 * Tests the full workflow: retrieve → judge → distill → consolidate
 */
import { initialize, runTask } from './index.js';
console.log('🧪 ReasoningBank Integration Test\n');
async function main() {
    // 1. Initialize
    console.log('1️⃣ Initializing ReasoningBank...');
    await initialize();
    console.log('   ✅ Initialization complete\n');
    // 2. Test full task execution
    console.log('2️⃣ Testing full task execution...');
    const result = await runTask({
        taskId: 'test-task-001',
        agentId: 'test-agent',
        query: 'Test login to admin panel with CSRF validation',
        domain: 'test.web',
        executeFn: async (memories) => {
            console.log(`   📝 Received ${memories.length} memories for task execution`);
            // Simulate a successful task trajectory
            const trajectory = {
                steps: [
                    { action: 'navigate', url: 'https://admin.test.com/login' },
                    { action: 'extract_csrf', location: 'meta[name=csrf-token]', success: true },
                    { action: 'fill_form', fields: { username: 'admin', password: '***', csrf: '[TOKEN]' } },
                    { action: 'submit', status: 200 },
                    { action: 'verify_redirect', url: '/dashboard', success: true },
                    { action: 'complete', message: 'Login successful' }
                ],
                metadata: {
                    duration_ms: 1234,
                    steps_count: 6
                }
            };
            return trajectory;
        }
    });
    console.log(`   ✅ Task complete:`);
    console.log(`      - Verdict: ${result.verdict.label} (${result.verdict.confidence})`);
    console.log(`      - Used memories: ${result.usedMemories.length}`);
    console.log(`      - New memories: ${result.newMemories.length}`);
    console.log(`      - Consolidated: ${result.consolidated ? 'Yes' : 'No'}\n`);
    // 3. Test retrieval
    console.log('3️⃣ Testing memory retrieval...');
    const { retrieveMemories } = await import('./core/retrieve.js');
    const memories = await retrieveMemories('How to handle CSRF tokens in login forms?', {
        domain: 'test.web',
        k: 3
    });
    console.log(`   ✅ Retrieved ${memories.length} memories`);
    if (memories.length > 0) {
        console.log(`      - Top memory: "${memories[0].title}"`);
        console.log(`      - Score: ${memories[0].score.toFixed(3)}`);
        console.log(`      - Similarity: ${(memories[0].components.similarity * 100).toFixed(1)}%\n`);
    }
    // 4. Test MaTTS (if enabled)
    const config = await import('./utils/config.js').then(m => m.loadConfig());
    if (config.features?.enable_matts_parallel) {
        console.log('4️⃣ Testing MaTTS parallel mode...');
        const { mattsParallel } = await import('./core/matts.js');
        const mattsResult = await mattsParallel(async () => ({
            steps: [
                { action: 'test', result: 'success' }
            ],
            metadata: {}
        }), 'Test MaTTS execution', { k: 3, taskId: 'matts-test', domain: 'test.matts' });
        console.log(`   ✅ MaTTS complete:`);
        console.log(`      - Trajectories: ${mattsResult.trajectories.length}`);
        console.log(`      - Success rate: ${(mattsResult.successRate * 100).toFixed(1)}%`);
        console.log(`      - Aggregated memories: ${mattsResult.aggregatedMemories.length}`);
        console.log(`      - Duration: ${mattsResult.duration}ms\n`);
    }
    // 5. Database statistics
    console.log('5️⃣ Database statistics...');
    const { getDb } = await import('./db/queries.js');
    const db = getDb();
    const totalMemories = db.prepare("SELECT COUNT(*) as count FROM patterns WHERE type = 'reasoning_memory'").get();
    const totalEmbeddings = db.prepare('SELECT COUNT(*) as count FROM pattern_embeddings').get();
    console.log(`   ✅ Database status:`);
    console.log(`      - Total memories: ${totalMemories.count}`);
    console.log(`      - Total embeddings: ${totalEmbeddings.count}\n`);
    console.log('✅ Integration test complete!\n');
    process.exit(0);
}
main().catch(error => {
    console.error('❌ Integration test failed:', error);
    process.exit(1);
});
//# sourceMappingURL=test-integration.js.map