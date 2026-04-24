/**
 * Example: Long-Running Agent with Provider Fallback
 *
 * This example demonstrates:
 * - Automatic provider fallback
 * - Cost optimization
 * - Health monitoring
 * - Checkpointing for crash recovery
 * - Budget constraints
 */
import { LongRunningAgent } from '../core/long-running-agent.js';
async function main() {
    console.log('🚀 Starting Long-Running Agent with Provider Fallback\n');
    // Configure providers with priorities and costs
    const providers = [
        {
            name: 'gemini',
            apiKey: process.env.GOOGLE_GEMINI_API_KEY,
            priority: 1, // Try Gemini first (fastest, cheapest)
            maxRetries: 3,
            timeout: 30000,
            costPerToken: 0.00015, // $0.15 per 1M tokens
            enabled: !!process.env.GOOGLE_GEMINI_API_KEY,
            healthCheckInterval: 60000 // Check every minute
        },
        {
            name: 'anthropic',
            apiKey: process.env.ANTHROPIC_API_KEY,
            priority: 2, // Fallback to Claude (higher quality)
            maxRetries: 3,
            timeout: 60000,
            costPerToken: 0.003, // $3 per 1M tokens (Sonnet)
            enabled: !!process.env.ANTHROPIC_API_KEY,
            healthCheckInterval: 60000
        },
        {
            name: 'openrouter',
            apiKey: process.env.OPENROUTER_API_KEY,
            priority: 3, // Fallback to OpenRouter
            maxRetries: 3,
            timeout: 60000,
            costPerToken: 0.001, // Varies by model
            enabled: !!process.env.OPENROUTER_API_KEY,
            healthCheckInterval: 60000
        },
        {
            name: 'onnx',
            priority: 4, // Last resort (local, free but slower)
            maxRetries: 2,
            timeout: 120000,
            costPerToken: 0, // FREE
            enabled: true,
            healthCheckInterval: 300000 // Check every 5 minutes
        }
    ];
    // Create agent with cost and performance optimization
    const agent = new LongRunningAgent({
        agentName: 'research-and-code-agent',
        providers,
        fallbackStrategy: {
            type: 'cost-optimized', // Prefer cheaper providers
            maxFailures: 3, // Open circuit breaker after 3 failures
            recoveryTime: 60000, // Try recovery after 1 minute
            retryBackoff: 'exponential',
            costThreshold: 0.50, // Max $0.50 per request
            latencyThreshold: 30000 // Max 30s per request
        },
        checkpointInterval: 30000, // Save state every 30 seconds
        maxRuntime: 3600000, // Max 1 hour
        costBudget: 5.00 // Max $5 total spend
    });
    await agent.start();
    try {
        // Simulate long-running workflow with different task complexities
        console.log('\n📋 Task 1: Simple Code Generation (Gemini optimal)\n');
        const task1 = await agent.executeTask({
            name: 'generate-hello-world',
            complexity: 'simple',
            estimatedTokens: 200,
            execute: async (provider) => {
                console.log(`  Using provider: ${provider}`);
                // Simulated API call
                await new Promise(resolve => setTimeout(resolve, 1000));
                return { code: 'console.log("Hello World");', provider };
            }
        });
        console.log(`  ✅ Result:`, task1);
        console.log('\n📋 Task 2: Complex Architecture Design (Claude optimal)\n');
        const task2 = await agent.executeTask({
            name: 'design-microservices-architecture',
            complexity: 'complex',
            estimatedTokens: 5000,
            execute: async (provider) => {
                console.log(`  Using provider: ${provider}`);
                await new Promise(resolve => setTimeout(resolve, 2000));
                return {
                    architecture: 'Event-driven microservices with CQRS',
                    provider
                };
            }
        });
        console.log(`  ✅ Result:`, task2);
        console.log('\n📋 Task 3: Medium Refactoring (Auto-optimized)\n');
        const task3 = await agent.executeTask({
            name: 'refactor-legacy-code',
            complexity: 'medium',
            estimatedTokens: 1500,
            execute: async (provider) => {
                console.log(`  Using provider: ${provider}`);
                await new Promise(resolve => setTimeout(resolve, 1500));
                return {
                    refactored: true,
                    improvements: ['Better naming', 'Modular design'],
                    provider
                };
            }
        });
        console.log(`  ✅ Result:`, task3);
        // Simulate provider failure for demonstration
        console.log('\n📋 Task 4: Testing Fallback (Simulated Failure)\n');
        let failureCount = 0;
        const task4 = await agent.executeTask({
            name: 'test-fallback',
            complexity: 'simple',
            estimatedTokens: 300,
            execute: async (provider) => {
                console.log(`  Attempting with provider: ${provider}`);
                // Simulate failure on first two attempts
                if (failureCount < 2) {
                    failureCount++;
                    throw new Error('Simulated rate limit error');
                }
                await new Promise(resolve => setTimeout(resolve, 1000));
                return {
                    message: 'Success after fallback!',
                    provider,
                    attempts: failureCount + 1
                };
            }
        });
        console.log(`  ✅ Result:`, task4);
        // Get final status
        console.log('\n📊 Final Agent Status:\n');
        const status = agent.getStatus();
        console.log(JSON.stringify(status, null, 2));
        console.log('\n💰 Cost Summary:\n');
        const metrics = agent.getMetrics();
        console.log('Total Cost:', `$${metrics.costs.total.toFixed(4)}`);
        console.log('Total Tokens:', metrics.costs.totalTokens.toLocaleString());
        console.log('\nBy Provider:');
        for (const [provider, cost] of Object.entries(metrics.costs.byProvider)) {
            console.log(`  ${provider}: $${cost.toFixed(4)}`);
        }
        console.log('\n📈 Provider Health:\n');
        for (const health of metrics.health) {
            console.log(`${health.provider}:`);
            console.log(`  Healthy: ${health.isHealthy}`);
            console.log(`  Success Rate: ${(health.successRate * 100).toFixed(1)}%`);
            console.log(`  Avg Latency: ${health.averageLatency.toFixed(0)}ms`);
            console.log(`  Circuit Breaker: ${health.circuitBreakerOpen ? 'OPEN' : 'CLOSED'}`);
            console.log('');
        }
    }
    catch (error) {
        console.error('❌ Agent execution failed:', error);
    }
    finally {
        await agent.stop();
        console.log('\n✅ Agent stopped successfully\n');
    }
}
// Run example
if (import.meta.url === `file://${process.argv[1]}`) {
    main().catch(console.error);
}
export { main as runProviderFallbackExample };
//# sourceMappingURL=use-provider-fallback.js.map