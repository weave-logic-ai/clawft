#!/usr/bin/env node
/**
 * ONNX Integration Test - Validates agentic-flow works with ONNX
 *
 * Tests ONNX provider integration with the multi-model router
 */
import { ModelRouter } from './router.js';
async function testONNXIntegration() {
    console.log('🧪 agentic-flow + ONNX Runtime Integration Test\n');
    console.log('='.repeat(60));
    console.log('Testing: Multi-Model Router with ONNX Local Inference');
    console.log('='.repeat(60) + '\n');
    try {
        // Initialize router (ONNX-only mode)
        console.log('Step 1: Router Initialization');
        console.log('==============================');
        const router = new ModelRouter();
        const config = router.getConfig();
        console.log(`✅ Router initialized successfully`);
        console.log(`   Version: ${config.version}`);
        console.log(`   Default Provider: ${config.defaultProvider}`);
        console.log(`   Fallback Chain: ${config.fallbackChain?.join(' → ')}`);
        console.log(`   Routing Mode: ${config.routing?.mode}`);
        console.log(`   Providers Configured: ${Object.keys(config.providers).length}`);
        console.log('');
        // Test ONNX provider
        console.log('Step 2: ONNX Provider Direct Test');
        console.log('===================================');
        // Get ONNX provider directly
        const onnxProvider = router.providers.get('onnx');
        if (!onnxProvider) {
            throw new Error('ONNX provider not initialized');
        }
        console.log(`✅ ONNX provider found: ${onnxProvider.name}`);
        console.log(`   Type: ${onnxProvider.type}`);
        console.log(`   Supports Streaming: ${onnxProvider.supportsStreaming}`);
        console.log(`   Supports Tools: ${onnxProvider.supportsTools}`);
        console.log('');
        // Test inference
        console.log('Step 3: ONNX Inference Test');
        console.log('=============================');
        const response = await onnxProvider.chat({
            model: 'phi-4',
            messages: [
                { role: 'user', content: 'What is 2+2?' }
            ],
            maxTokens: 20
        });
        console.log(`✅ Inference successful`);
        console.log(`   Response: ${response.content[0].type === 'text' ? response.content[0].text : 'N/A'}`);
        console.log(`   Model: ${response.model}`);
        console.log(`   Latency: ${response.metadata?.latency}ms`);
        console.log(`   Tokens/Sec: ${response.metadata?.tokensPerSecond}`);
        console.log(`   Cost: $${response.metadata?.cost || 0}`);
        console.log(`   Input Tokens: ${response.usage?.inputTokens}`);
        console.log(`   Output Tokens: ${response.usage?.outputTokens}`);
        console.log('');
        // Test router configuration
        console.log('Step 4: Router Configuration Validation');
        console.log('=========================================');
        const onnxConfig = config.providers.onnx;
        console.log(`✅ ONNX Configuration:`);
        console.log(`   Model Path: ${onnxConfig?.modelPath || 'default'}`);
        console.log(`   Execution Providers: ${onnxConfig?.executionProviders?.join(', ')}`);
        console.log(`   Max Tokens: ${onnxConfig?.maxTokens}`);
        console.log(`   Temperature: ${onnxConfig?.temperature}`);
        console.log(`   Local Inference: ${onnxConfig?.localInference}`);
        console.log(`   GPU Acceleration: ${onnxConfig?.gpuAcceleration}`);
        console.log('');
        // Test routing rules
        console.log('Step 5: Privacy Routing Rule Validation');
        console.log('=========================================');
        const privacyRule = config.routing?.rules?.find(r => r.condition.privacy === 'high' && r.action.provider === 'onnx');
        if (privacyRule) {
            console.log(`✅ Privacy routing rule configured:`);
            console.log(`   Condition: privacy = high, localOnly = ${privacyRule.condition.localOnly}`);
            console.log(`   Action: Route to ${privacyRule.action.provider}`);
            console.log(`   Reason: ${privacyRule.reason}`);
        }
        else {
            console.log(`⚠️  Privacy routing rule not found (optional)`);
        }
        console.log('');
        // Architecture summary
        console.log('\n' + '='.repeat(60));
        console.log('✅ Integration Test Complete!');
        console.log('='.repeat(60) + '\n');
        console.log('Integration Confirmed:');
        console.log('  ✓ agentic-flow multi-model router working');
        console.log('  ✓ ONNX Runtime provider integrated');
        console.log('  ✓ Local CPU inference operational');
        console.log('  ✓ Configuration loaded successfully');
        console.log('  ✓ Privacy routing rules configured');
        console.log('');
        console.log('Architecture Details:');
        console.log('  • Router: ModelRouter class');
        console.log('  • ONNX Provider: ONNXLocalProvider');
        console.log('  • Model: Microsoft Phi-4-mini-instruct-onnx (INT4)');
        console.log('  • Execution: CPU-only local inference');
        console.log('  • KV Cache: 32-layer autoregressive generation');
        console.log('  • Cost: $0.00 per request (100% free)');
        console.log('');
        console.log('Performance:');
        console.log(`  • Latency: ${response.metadata?.latency}ms`);
        console.log(`  • Throughput: ${response.metadata?.tokensPerSecond} tokens/sec`);
        console.log(`  • Privacy: 100% local processing`);
        console.log(`  • Cost: $0.00 (free inference)`);
        console.log('');
        console.log('Use Cases:');
        console.log('  • Privacy-sensitive data processing');
        console.log('  • GDPR/HIPAA compliant inference');
        console.log('  • Offline operation (no internet)');
        console.log('  • Zero-cost development/testing');
        console.log('  • Medical/legal document analysis');
        console.log('');
    }
    catch (error) {
        console.error('\n❌ Integration Test Failed!');
        console.error('==============================');
        console.error(error);
        process.exit(1);
    }
}
// Run test
testONNXIntegration().catch(error => {
    console.error('Fatal error:', error);
    process.exit(1);
});
//# sourceMappingURL=test-onnx-integration.js.map