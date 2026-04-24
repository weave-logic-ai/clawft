/**
 * Smart Model Optimizer - Automatically selects the best model for each agent and task
 * Balances performance vs cost based on agent requirements
 */
import { logger } from './logger.js';
// Model database with performance characteristics
const MODEL_DATABASE = {
    // Tier 1: Flagship Models
    'claude-sonnet-4-5': {
        provider: 'anthropic',
        model: 'claude-sonnet-4-5-20250929',
        modelName: 'Claude Sonnet 4.5',
        cost_per_1m_input: 3.00,
        cost_per_1m_output: 15.00,
        quality_score: 95,
        speed_score: 85,
        cost_score: 20,
        tier: 'flagship',
        supports_tools: true,
        strengths: ['reasoning', 'coding', 'analysis', 'production'],
        weaknesses: ['cost'],
        bestFor: ['coder', 'reviewer', 'architecture', 'planner', 'production-validator']
    },
    'gpt-4o': {
        provider: 'openrouter',
        model: 'openai/gpt-4o',
        modelName: 'GPT-4o',
        cost_per_1m_input: 2.50,
        cost_per_1m_output: 10.00,
        quality_score: 88,
        speed_score: 90,
        cost_score: 30,
        tier: 'flagship',
        supports_tools: true,
        strengths: ['multimodal', 'speed', 'general-purpose', 'vision'],
        weaknesses: ['cost'],
        bestFor: ['researcher', 'analyst', 'multimodal-tasks']
    },
    'gemini-2-5-pro': {
        provider: 'openrouter',
        model: 'google/gemini-2.5-pro',
        modelName: 'Gemini 2.5 Pro',
        cost_per_1m_input: 1.25,
        cost_per_1m_output: 5.00,
        quality_score: 90,
        speed_score: 75,
        cost_score: 50,
        tier: 'flagship',
        supports_tools: true,
        strengths: ['reasoning', 'large-context', 'math', 'analysis'],
        weaknesses: ['speed'],
        bestFor: ['planner', 'architecture', 'researcher', 'code-analyzer']
    },
    // Tier 2: Cost-Effective Champions
    'deepseek-r1': {
        provider: 'openrouter',
        model: 'deepseek/deepseek-r1-0528:free',
        modelName: 'DeepSeek R1',
        cost_per_1m_input: 0.00,
        cost_per_1m_output: 0.00,
        quality_score: 90,
        speed_score: 80,
        cost_score: 100,
        tier: 'cost-effective',
        supports_tools: false, // DeepSeek R1 does NOT support tool/function calling
        strengths: ['reasoning', 'coding', 'math', 'value', 'free'],
        weaknesses: ['newer-model', 'no-tool-use'],
        bestFor: ['coder', 'pseudocode', 'specification', 'refinement', 'tester']
    },
    'deepseek-chat-v3': {
        provider: 'openrouter',
        model: 'deepseek/deepseek-chat-v3.1:free',
        modelName: 'DeepSeek Chat V3.1',
        cost_per_1m_input: 0.00,
        cost_per_1m_output: 0.00,
        quality_score: 82,
        speed_score: 90,
        cost_score: 100,
        tier: 'cost-effective',
        supports_tools: true,
        strengths: ['cost', 'speed', 'coding', 'development', 'free'],
        weaknesses: ['complex-reasoning'],
        bestFor: ['coder', 'reviewer', 'tester', 'backend-dev', 'cicd-engineer']
    },
    // Tier 3: Balanced Performance
    'gemini-2-5-flash': {
        provider: 'openrouter',
        model: 'google/gemini-2.5-flash',
        modelName: 'Gemini 2.5 Flash',
        cost_per_1m_input: 0.075,
        cost_per_1m_output: 0.30,
        quality_score: 78,
        speed_score: 98,
        cost_score: 98,
        tier: 'balanced',
        supports_tools: true,
        strengths: ['speed', 'cost', 'interactive'],
        weaknesses: ['quality'],
        bestFor: ['researcher', 'planner', 'smart-agent']
    },
    'llama-3-3-8b': {
        provider: 'openrouter',
        model: 'meta-llama/llama-3.3-8b-instruct:free',
        modelName: 'Llama 3.3 8B',
        cost_per_1m_input: 0.00,
        cost_per_1m_output: 0.00,
        quality_score: 72,
        speed_score: 95,
        cost_score: 100,
        tier: 'balanced',
        supports_tools: true,
        strengths: ['open-source', 'versatile', 'coding', 'free', 'fast'],
        weaknesses: ['smaller-model'],
        bestFor: ['coder', 'reviewer', 'base-template-generator', 'tester']
    },
    'qwen-2-5-72b': {
        provider: 'openrouter',
        model: 'qwen/qwen-2.5-72b-instruct',
        modelName: 'Qwen 2.5 72B',
        cost_per_1m_input: 0.35,
        cost_per_1m_output: 0.40,
        quality_score: 81,
        speed_score: 85,
        cost_score: 90,
        tier: 'balanced',
        supports_tools: true,
        strengths: ['multilingual', 'coding', 'reasoning'],
        weaknesses: ['english-optimized'],
        bestFor: ['researcher', 'coder', 'multilingual-tasks']
    },
    // Tier 4: Budget Options
    'llama-3-1-8b': {
        provider: 'openrouter',
        model: 'meta-llama/llama-3.1-8b-instruct',
        modelName: 'Llama 3.1 8B',
        cost_per_1m_input: 0.06,
        cost_per_1m_output: 0.06,
        quality_score: 65,
        speed_score: 95,
        cost_score: 99,
        tier: 'budget',
        supports_tools: true,
        strengths: ['ultra-low-cost', 'speed'],
        weaknesses: ['quality', 'complex-tasks'],
        bestFor: ['simple-tasks', 'testing']
    },
    // Tier 5: Local/Privacy
    'onnx-phi-4': {
        provider: 'onnx',
        model: 'phi-4-mini',
        modelName: 'ONNX Phi-4 Mini',
        cost_per_1m_input: 0.00,
        cost_per_1m_output: 0.00,
        quality_score: 58,
        speed_score: 30,
        cost_score: 100,
        tier: 'local',
        supports_tools: false,
        strengths: ['privacy', 'offline', 'zero-cost'],
        weaknesses: ['quality', 'speed'],
        bestFor: ['privacy-tasks', 'offline-tasks']
    }
};
// Agent complexity and quality requirements
const AGENT_REQUIREMENTS = {
    // High-quality code generation
    'coder': { minQuality: 85, complexity: 'complex', needsReasoning: true },
    'sparc-coder': { minQuality: 85, complexity: 'complex', needsReasoning: true },
    'backend-dev': { minQuality: 80, complexity: 'complex', needsReasoning: true },
    // Architecture and design
    'architecture': { minQuality: 90, complexity: 'expert', needsReasoning: true },
    'system-architect': { minQuality: 90, complexity: 'expert', needsReasoning: true },
    'planner': { minQuality: 85, complexity: 'complex', needsReasoning: true },
    // Code review and analysis
    'reviewer': { minQuality: 85, complexity: 'complex', needsReasoning: true },
    'code-analyzer': { minQuality: 80, complexity: 'complex', needsReasoning: true },
    'production-validator': { minQuality: 90, complexity: 'expert', needsReasoning: true },
    // Testing
    'tester': { minQuality: 75, complexity: 'moderate', needsReasoning: false },
    'tdd-london-swarm': { minQuality: 80, complexity: 'complex', needsReasoning: true },
    // Research and analysis
    'researcher': { minQuality: 75, complexity: 'moderate', needsReasoning: true },
    'analyst': { minQuality: 80, complexity: 'complex', needsReasoning: true },
    // SPARC phases
    'specification': { minQuality: 85, complexity: 'complex', needsReasoning: true },
    'pseudocode': { minQuality: 80, complexity: 'complex', needsReasoning: true },
    'refinement': { minQuality: 85, complexity: 'complex', needsReasoning: true },
    // DevOps and automation
    'cicd-engineer': { minQuality: 75, complexity: 'moderate', needsReasoning: false },
    'smart-agent': { minQuality: 70, complexity: 'moderate', needsReasoning: false },
    // Documentation
    'api-docs': { minQuality: 70, complexity: 'moderate', needsReasoning: false },
    'base-template-generator': { minQuality: 70, complexity: 'simple', needsReasoning: false },
    // Default for unknown agents
    'default': { minQuality: 75, complexity: 'moderate', needsReasoning: true }
};
export class ModelOptimizer {
    /**
     * Optimize model selection based on agent, task, and priorities
     */
    static optimize(criteria) {
        logger.info('Optimizing model selection', criteria);
        // Get agent requirements
        const agentKey = criteria.agent.toLowerCase();
        const agentReqs = AGENT_REQUIREMENTS[agentKey] || AGENT_REQUIREMENTS['default'];
        // Determine task complexity from task description if not provided
        const taskComplexity = criteria.taskComplexity || this.inferComplexity(criteria.task);
        // Set default priority to balanced if not specified
        const priority = criteria.priority || 'balanced';
        // Filter models that support tools if required
        let availableModels = Object.entries(MODEL_DATABASE);
        if (criteria.requiresTools) {
            availableModels = availableModels.filter(([key, model]) => model.supports_tools !== false);
            logger.info(`Filtered to ${availableModels.length} models with tool support`);
        }
        // Score all models
        const scoredModels = availableModels.map(([key, model]) => {
            // Calculate overall score based on priority
            let overall_score;
            switch (priority) {
                case 'quality':
                    overall_score = model.quality_score * 0.7 + model.speed_score * 0.2 + model.cost_score * 0.1;
                    break;
                case 'cost':
                    overall_score = model.cost_score * 0.7 + model.quality_score * 0.2 + model.speed_score * 0.1;
                    break;
                case 'speed':
                    overall_score = model.speed_score * 0.7 + model.quality_score * 0.2 + model.cost_score * 0.1;
                    break;
                case 'privacy':
                    // Heavily favor local models for privacy
                    overall_score = model.tier === 'local' ? 100 : model.cost_score * 0.5 + model.quality_score * 0.5;
                    break;
                case 'balanced':
                default:
                    overall_score = model.quality_score * 0.4 + model.cost_score * 0.4 + model.speed_score * 0.2;
                    break;
            }
            // Apply agent-specific bonuses
            if (model.bestFor.includes(criteria.agent.toLowerCase())) {
                overall_score += 10;
            }
            // Apply quality threshold
            if (model.quality_score < agentReqs.minQuality) {
                overall_score *= 0.5; // Penalize models below quality threshold
            }
            // Apply complexity matching
            if (taskComplexity === 'expert' && model.tier !== 'flagship') {
                overall_score *= 0.7;
            }
            else if (taskComplexity === 'simple' && model.tier === 'flagship') {
                overall_score *= 0.8; // Don't waste flagship models on simple tasks unless quality priority
            }
            // Apply cost cap if specified
            if (criteria.maxCostPerTask) {
                const estimatedCost = this.estimateCost(model, criteria.task);
                if (estimatedCost > criteria.maxCostPerTask) {
                    overall_score *= 0.3; // Heavy penalty for exceeding budget
                }
            }
            return {
                key,
                ...model,
                overall_score
            };
        });
        // Sort by overall score
        scoredModels.sort((a, b) => b.overall_score - a.overall_score);
        // Get top recommendation
        const top = scoredModels[0];
        // Generate reasoning
        const reasoning = this.generateReasoning(top, criteria, agentReqs, taskComplexity, priority);
        const recommendation = {
            provider: top.provider,
            model: top.model,
            modelName: top.modelName,
            cost_per_1m_input: top.cost_per_1m_input,
            cost_per_1m_output: top.cost_per_1m_output,
            quality_score: top.quality_score,
            speed_score: top.speed_score,
            cost_score: top.cost_score,
            overall_score: top.overall_score,
            tier: top.tier,
            reasoning
        };
        logger.info('Model optimization complete', {
            selected: recommendation.modelName,
            score: recommendation.overall_score
        });
        return recommendation;
    }
    /**
     * Infer task complexity from task description
     */
    static inferComplexity(task) {
        const lowerTask = task.toLowerCase();
        // Expert-level indicators
        if (lowerTask.includes('architecture') ||
            lowerTask.includes('design system') ||
            lowerTask.includes('production') ||
            lowerTask.includes('enterprise') ||
            lowerTask.includes('scale') ||
            lowerTask.includes('distributed')) {
            return 'expert';
        }
        // Complex indicators
        if (lowerTask.includes('implement') ||
            lowerTask.includes('create') ||
            lowerTask.includes('build') ||
            lowerTask.includes('develop') ||
            lowerTask.includes('integrate') ||
            lowerTask.includes('api') ||
            lowerTask.includes('database')) {
            return 'complex';
        }
        // Simple indicators
        if (lowerTask.includes('simple') ||
            lowerTask.includes('basic') ||
            lowerTask.includes('hello world') ||
            lowerTask.includes('example') ||
            lowerTask.includes('template')) {
            return 'simple';
        }
        // Default to moderate
        return 'moderate';
    }
    /**
     * Estimate cost for a task (rough approximation)
     */
    static estimateCost(model, task) {
        // Rough estimate: task length + expected output
        const inputTokens = Math.ceil(task.length / 4);
        const outputTokens = 1000; // Assume 1K token output
        const inputCost = (inputTokens / 1000000) * model.cost_per_1m_input;
        const outputCost = (outputTokens / 1000000) * model.cost_per_1m_output;
        return inputCost + outputCost;
    }
    /**
     * Generate human-readable reasoning for model selection
     */
    static generateReasoning(model, criteria, agentReqs, taskComplexity, priority) {
        const reasons = [];
        // Priority-based reasoning
        switch (priority) {
            case 'quality':
                reasons.push(`Selected for highest quality (${model.quality_score}/100)`);
                break;
            case 'cost':
                reasons.push(`Selected for best cost efficiency (${model.cost_score}/100)`);
                break;
            case 'speed':
                reasons.push(`Selected for fastest response (${model.speed_score}/100)`);
                break;
            case 'privacy':
                if (model.tier === 'local') {
                    reasons.push('Selected for 100% privacy (runs locally)');
                }
                else {
                    reasons.push('Best available option for privacy concerns');
                }
                break;
            case 'balanced':
                reasons.push(`Balanced selection (overall: ${Math.round(model.overall_score)}/100)`);
                break;
        }
        // Agent-specific reasoning
        if (model.bestFor.includes(criteria.agent.toLowerCase())) {
            reasons.push(`Optimized for ${criteria.agent} agent tasks`);
        }
        // Complexity matching
        if (taskComplexity === 'expert' && model.tier === 'flagship') {
            reasons.push('Flagship model for expert-level complexity');
        }
        else if (taskComplexity === 'simple' && model.tier !== 'flagship') {
            reasons.push('Cost-effective for simple tasks');
        }
        // Cost information
        const estCost = this.estimateCost(model, criteria.task);
        reasons.push(`Estimated cost: $${estCost.toFixed(6)} per task`);
        // Tier information
        reasons.push(`Tier: ${model.tier}`);
        return reasons.join('. ');
    }
    /**
     * Get all available models with their characteristics
     */
    static getAvailableModels() {
        return MODEL_DATABASE;
    }
    /**
     * Display optimization recommendations in console
     */
    static displayRecommendation(recommendation) {
        console.log('\n🎯 Optimized Model Selection');
        console.log('═'.repeat(60));
        console.log(`Model: ${recommendation.modelName}`);
        console.log(`Provider: ${recommendation.provider}`);
        console.log(`Tier: ${recommendation.tier}`);
        console.log('');
        console.log('Scores:');
        console.log(`  Quality:  ${recommendation.quality_score}/100`);
        console.log(`  Speed:    ${recommendation.speed_score}/100`);
        console.log(`  Cost:     ${recommendation.cost_score}/100`);
        console.log(`  Overall:  ${Math.round(recommendation.overall_score)}/100`);
        console.log('');
        console.log('Cost: $' + recommendation.cost_per_1m_input.toFixed(2) + '/1M input, ' +
            '$' + recommendation.cost_per_1m_output.toFixed(2) + '/1M output');
        console.log('');
        console.log('Reasoning:');
        console.log(`  ${recommendation.reasoning}`);
        console.log('═'.repeat(60));
        console.log('');
    }
}
//# sourceMappingURL=modelOptimizer.js.map