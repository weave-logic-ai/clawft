#!/usr/bin/env node
/**
 * Test ONNX local inference with Phi-4 model
 */
import { ONNXLocalProvider } from './providers/onnx-local.js';
async function testONNXLocal() {
    console.log('🧪 Testing ONNX Local Inference (Phi-4 CPU)\n');
    try {
        const provider = new ONNXLocalProvider({
            modelPath: './models/phi-4/cpu_and_mobile/cpu-int4-rtn-block-32-acc-level-4/model.onnx',
            executionProviders: ['cpu'],
            maxTokens: 50
        });
        console.log('Test: Simple Inference');
        console.log('======================');
        const response = await provider.chat({
            model: 'phi-4',
            messages: [
                { role: 'user', content: 'What is 2+2?' }
            ],
            maxTokens: 20
        });
        console.log('\n📥 Response:');
        console.log(`  Text: ${response.content[0].type === 'text' ? response.content[0].text : ''}`);
        console.log(`  Latency: ${response.metadata?.latency}ms`);
        console.log(`  Tokens: ${response.usage?.inputTokens} in / ${response.usage?.outputTokens} out`);
        console.log(`  Cost: $${response.metadata?.cost}`);
        console.log(`  Providers: ${response.metadata?.executionProviders?.join(', ')}`);
        console.log('\n✅ Test passed!');
        await provider.dispose();
    }
    catch (error) {
        console.error('\n❌ Test failed:', error);
        process.exit(1);
    }
}
testONNXLocal();
//# sourceMappingURL=test-onnx-local.js.map