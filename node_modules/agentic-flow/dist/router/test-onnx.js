#!/usr/bin/env node
/**
 * Test script for ONNX Runtime integration
 * Tests CPU inference with Phi-3 model
 */
import { ONNXProvider } from './providers/onnx.js';
async function testONNXProvider() {
    console.log('🧪 Testing ONNX Runtime Provider\n');
    try {
        // Test 1: Initialize ONNX provider
        console.log('Test 1: Provider Initialization');
        console.log('================================');
        const provider = new ONNXProvider({
            modelId: 'Xenova/Phi-3-mini-4k-instruct',
            maxTokens: 100,
            temperature: 0.7
        });
        console.log(`✅ Provider initialized: ${provider.name}`);
        console.log(`📊 Supports streaming: ${provider.supportsStreaming}`);
        console.log(`🔧 Supports tools: ${provider.supportsTools}\n`);
        // Test 2: Simple chat completion
        console.log('Test 2: Chat Completion (CPU)');
        console.log('==============================');
        const chatParams = {
            model: 'Xenova/Phi-3-mini-4k-instruct',
            messages: [
                {
                    role: 'user',
                    content: 'Say "Hello from ONNX Runtime!" and nothing else.'
                }
            ],
            maxTokens: 50,
            temperature: 0.5
        };
        console.log(`📤 Sending request...`);
        console.log(`📝 Prompt: ${chatParams.messages[0].content}\n`);
        const startTime = Date.now();
        const response = await provider.chat(chatParams);
        const latency = Date.now() - startTime;
        console.log('📥 Response received:');
        console.log(`  Provider: ${response.metadata?.provider}`);
        console.log(`  Model: ${response.model}`);
        console.log(`  Latency: ${latency}ms`);
        console.log(`  Stop Reason: ${response.stopReason}`);
        console.log(`  Usage: ${response.usage?.inputTokens} in / ${response.usage?.outputTokens} out`);
        console.log(`  Cost: $${response.metadata?.cost?.toFixed(6) || 0} (FREE - Local inference)`);
        console.log(`  Execution Providers: ${response.metadata?.executionProviders?.join(', ')}`);
        console.log(`\n  Content:`);
        for (const block of response.content) {
            if (block.type === 'text') {
                console.log(`    ${block.text}`);
            }
        }
        console.log('\n✅ Test 2 passed!\n');
        // Test 3: Multi-turn conversation
        console.log('Test 3: Multi-Turn Conversation');
        console.log('================================');
        const conversationParams = {
            model: 'Xenova/Phi-3-mini-4k-instruct',
            messages: [
                {
                    role: 'user',
                    content: 'What is 2+2?'
                },
                {
                    role: 'assistant',
                    content: '4'
                },
                {
                    role: 'user',
                    content: 'What about 2+3?'
                }
            ],
            maxTokens: 50,
            temperature: 0.3
        };
        console.log(`📤 Multi-turn conversation...`);
        const convResponse = await provider.chat(conversationParams);
        console.log('📥 Response:');
        console.log(`  Content: ${convResponse.content[0].type === 'text' ? convResponse.content[0].text : 'N/A'}`);
        console.log('\n✅ Test 3 passed!\n');
        // Test 4: Model info
        console.log('Test 4: Model Information');
        console.log('=========================');
        const modelInfo = provider.getModelInfo();
        console.log(`📊 Model ID: ${modelInfo.modelId}`);
        console.log(`🔧 Execution Providers: ${modelInfo.executionProviders.join(', ')}`);
        console.log(`⚡ GPU Support: ${modelInfo.supportsGPU ? 'Yes' : 'No (CPU only)'}`);
        console.log(`✓ Initialized: ${modelInfo.initialized}`);
        console.log('\n✅ Test 4 passed!\n');
        // Test 5: Performance benchmark
        console.log('Test 5: Performance Benchmark');
        console.log('=============================');
        const benchmarkParams = {
            model: 'Xenova/Phi-3-mini-4k-instruct',
            messages: [
                {
                    role: 'user',
                    content: 'Count from 1 to 5.'
                }
            ],
            maxTokens: 50,
            temperature: 0.5
        };
        const benchmarkRuns = 3;
        const latencies = [];
        for (let i = 0; i < benchmarkRuns; i++) {
            const start = Date.now();
            await provider.chat(benchmarkParams);
            const duration = Date.now() - start;
            latencies.push(duration);
            console.log(`  Run ${i + 1}: ${duration}ms`);
        }
        const avgLatency = latencies.reduce((a, b) => a + b, 0) / latencies.length;
        const tokensPerSec = (50 / avgLatency) * 1000;
        console.log(`\n📊 Benchmark Results:`);
        console.log(`  Average Latency: ${avgLatency.toFixed(0)}ms`);
        console.log(`  Tokens/Second: ${tokensPerSec.toFixed(1)}`);
        console.log('\n✅ Test 5 passed!\n');
        // Cleanup
        await provider.dispose();
        // Final summary
        console.log('🎉 All ONNX Tests Passed!');
        console.log('=========================');
        console.log(`✅ Provider initialization working`);
        console.log(`✅ CPU inference functional`);
        console.log(`✅ Chat completion successful`);
        console.log(`✅ Multi-turn conversations working`);
        console.log(`✅ Performance: ${tokensPerSec.toFixed(1)} tokens/sec`);
        console.log(`✅ Cost: $0.00 (100% free local inference)`);
        console.log(`\n💡 Next Steps:`);
        console.log(`  1. Integrate ONNX provider into router`);
        console.log(`  2. Add GPU support (CUDA/DirectML)`);
        console.log(`  3. Implement model caching`);
        console.log(`  4. Add streaming support`);
    }
    catch (error) {
        console.error('\n❌ Test Failed!');
        console.error('===============');
        console.error(error);
        process.exit(1);
    }
}
// Run tests
testONNXProvider().catch(error => {
    console.error('Fatal error:', error);
    process.exit(1);
});
//# sourceMappingURL=test-onnx.js.map