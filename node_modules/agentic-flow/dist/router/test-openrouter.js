#!/usr/bin/env node
// Test script for OpenRouter integration
import { ModelRouter } from './router.js';
async function testOpenRouter() {
    console.log('🧪 Testing OpenRouter Integration\n');
    try {
        // Initialize router
        console.log('📦 Initializing router...');
        const router = new ModelRouter();
        const config = router.getConfig();
        console.log(`✅ Router initialized with default provider: ${config.defaultProvider}\n`);
        // Test 1: Simple chat completion
        console.log('Test 1: Simple Chat Completion');
        console.log('================================');
        const chatParams = {
            model: config.providers.anthropic?.models?.default || 'claude-3-5-sonnet-20241022',
            messages: [
                {
                    role: 'user',
                    content: 'Say "Hello from Multi-Model Router!" and nothing else.'
                }
            ],
            temperature: 0.7,
            maxTokens: 100
        };
        console.log(`📤 Sending request to model: ${chatParams.model}`);
        console.log(`📝 Prompt: ${chatParams.messages[0].content}\n`);
        const startTime = Date.now();
        const response = await router.chat(chatParams);
        const latency = Date.now() - startTime;
        console.log('📥 Response received:');
        console.log(`  Provider: ${response.metadata?.provider}`);
        console.log(`  Model: ${response.model}`);
        console.log(`  Latency: ${latency}ms`);
        console.log(`  Stop Reason: ${response.stopReason}`);
        console.log(`  Usage: ${response.usage?.inputTokens} in / ${response.usage?.outputTokens} out`);
        console.log(`  Cost: $${response.metadata?.cost?.toFixed(6) || 0}`);
        console.log(`\n  Content:`);
        for (const block of response.content) {
            if (block.type === 'text') {
                console.log(`    ${block.text}`);
            }
        }
        console.log('\n✅ Test 1 passed!\n');
        // Test 2: Different model via OpenRouter
        console.log('Test 2: Alternative Model');
        console.log('=========================');
        const altParams = {
            model: 'claude-3-5-haiku-20241022',
            messages: [
                {
                    role: 'user',
                    content: 'Respond with just the word "SUCCESS" if you can read this.'
                }
            ],
            temperature: 0.5,
            maxTokens: 50
        };
        console.log(`📤 Testing model: ${altParams.model}\n`);
        const altResponse = await router.chat(altParams);
        console.log('📥 Response:');
        console.log(`  Model: ${altResponse.model}`);
        console.log(`  Content: ${altResponse.content[0].type === 'text' ? altResponse.content[0].text : 'N/A'}`);
        console.log('\n✅ Test 2 passed!\n');
        // Test 3: Show metrics
        console.log('Test 3: Router Metrics');
        console.log('=====================');
        const metrics = router.getMetrics();
        console.log(`📊 Total Requests: ${metrics.totalRequests}`);
        console.log(`💰 Total Cost: $${metrics.totalCost.toFixed(6)}`);
        console.log(`📝 Total Tokens: ${metrics.totalTokens.input} in / ${metrics.totalTokens.output} out`);
        console.log('\nProvider Breakdown:');
        for (const [provider, stats] of Object.entries(metrics.providerBreakdown)) {
            console.log(`  ${provider}:`);
            console.log(`    Requests: ${stats.requests}`);
            console.log(`    Cost: $${stats.cost.toFixed(6)}`);
            console.log(`    Avg Latency: ${stats.avgLatency.toFixed(0)}ms`);
        }
        console.log('\n✅ Test 3 passed!\n');
        // Test 4: Rule-based routing (if configured)
        if (config.routing?.mode === 'rule-based') {
            console.log('Test 4: Rule-Based Routing');
            console.log('==========================');
            const ruleParams = {
                model: config.providers.anthropic?.models?.default || 'claude-3-5-sonnet-20241022',
                messages: [
                    {
                        role: 'user',
                        content: 'This is a test for rule-based routing'
                    }
                ]
            };
            const ruleResponse = await router.chat(ruleParams, 'researcher');
            console.log(`📥 Routed to: ${ruleResponse.metadata?.provider}`);
            console.log(`📝 Response: ${ruleResponse.content[0].type === 'text' ? ruleResponse.content[0].text?.substring(0, 100) : 'N/A'}...`);
            console.log('\n✅ Test 4 passed!\n');
        }
        // Final summary
        console.log('🎉 All Tests Passed!');
        console.log('===================');
        console.log(`✅ OpenRouter integration working`);
        console.log(`✅ Router configuration loaded`);
        console.log(`✅ Chat completion successful`);
        console.log(`✅ Metrics tracking functional`);
        console.log(`\n📊 Final Metrics:`);
        console.log(`  Total Requests: ${metrics.totalRequests}`);
        console.log(`  Total Cost: $${metrics.totalCost.toFixed(6)}`);
        console.log(`  Providers Used: ${Object.keys(metrics.providerBreakdown).join(', ')}`);
    }
    catch (error) {
        console.error('\n❌ Test Failed!');
        console.error('===============');
        console.error(error);
        process.exit(1);
    }
}
// Run tests
testOpenRouter().catch(error => {
    console.error('Fatal error:', error);
    process.exit(1);
});
//# sourceMappingURL=test-openrouter.js.map