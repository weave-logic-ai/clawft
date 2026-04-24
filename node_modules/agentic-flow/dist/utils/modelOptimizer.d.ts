/**
 * Smart Model Optimizer - Automatically selects the best model for each agent and task
 * Balances performance vs cost based on agent requirements
 */
export interface ModelRecommendation {
    provider: string;
    model: string;
    modelName: string;
    reasoning: string;
    cost_per_1m_input: number;
    cost_per_1m_output: number;
    quality_score: number;
    speed_score: number;
    cost_score: number;
    overall_score: number;
    tier: 'flagship' | 'cost-effective' | 'balanced' | 'budget' | 'local';
}
export interface OptimizationCriteria {
    agent: string;
    task: string;
    priority?: 'quality' | 'balanced' | 'cost' | 'speed' | 'privacy';
    maxCostPerTask?: number;
    requiresReasoning?: boolean;
    requiresMultimodal?: boolean;
    requiresTools?: boolean;
    taskComplexity?: 'simple' | 'moderate' | 'complex' | 'expert';
}
declare const MODEL_DATABASE: {
    'claude-sonnet-4-5': {
        provider: string;
        model: string;
        modelName: string;
        cost_per_1m_input: number;
        cost_per_1m_output: number;
        quality_score: number;
        speed_score: number;
        cost_score: number;
        tier: "flagship";
        supports_tools: boolean;
        strengths: string[];
        weaknesses: string[];
        bestFor: string[];
    };
    'gpt-4o': {
        provider: string;
        model: string;
        modelName: string;
        cost_per_1m_input: number;
        cost_per_1m_output: number;
        quality_score: number;
        speed_score: number;
        cost_score: number;
        tier: "flagship";
        supports_tools: boolean;
        strengths: string[];
        weaknesses: string[];
        bestFor: string[];
    };
    'gemini-2-5-pro': {
        provider: string;
        model: string;
        modelName: string;
        cost_per_1m_input: number;
        cost_per_1m_output: number;
        quality_score: number;
        speed_score: number;
        cost_score: number;
        tier: "flagship";
        supports_tools: boolean;
        strengths: string[];
        weaknesses: string[];
        bestFor: string[];
    };
    'deepseek-r1': {
        provider: string;
        model: string;
        modelName: string;
        cost_per_1m_input: number;
        cost_per_1m_output: number;
        quality_score: number;
        speed_score: number;
        cost_score: number;
        tier: "cost-effective";
        supports_tools: boolean;
        strengths: string[];
        weaknesses: string[];
        bestFor: string[];
    };
    'deepseek-chat-v3': {
        provider: string;
        model: string;
        modelName: string;
        cost_per_1m_input: number;
        cost_per_1m_output: number;
        quality_score: number;
        speed_score: number;
        cost_score: number;
        tier: "cost-effective";
        supports_tools: boolean;
        strengths: string[];
        weaknesses: string[];
        bestFor: string[];
    };
    'gemini-2-5-flash': {
        provider: string;
        model: string;
        modelName: string;
        cost_per_1m_input: number;
        cost_per_1m_output: number;
        quality_score: number;
        speed_score: number;
        cost_score: number;
        tier: "balanced";
        supports_tools: boolean;
        strengths: string[];
        weaknesses: string[];
        bestFor: string[];
    };
    'llama-3-3-8b': {
        provider: string;
        model: string;
        modelName: string;
        cost_per_1m_input: number;
        cost_per_1m_output: number;
        quality_score: number;
        speed_score: number;
        cost_score: number;
        tier: "balanced";
        supports_tools: boolean;
        strengths: string[];
        weaknesses: string[];
        bestFor: string[];
    };
    'qwen-2-5-72b': {
        provider: string;
        model: string;
        modelName: string;
        cost_per_1m_input: number;
        cost_per_1m_output: number;
        quality_score: number;
        speed_score: number;
        cost_score: number;
        tier: "balanced";
        supports_tools: boolean;
        strengths: string[];
        weaknesses: string[];
        bestFor: string[];
    };
    'llama-3-1-8b': {
        provider: string;
        model: string;
        modelName: string;
        cost_per_1m_input: number;
        cost_per_1m_output: number;
        quality_score: number;
        speed_score: number;
        cost_score: number;
        tier: "budget";
        supports_tools: boolean;
        strengths: string[];
        weaknesses: string[];
        bestFor: string[];
    };
    'onnx-phi-4': {
        provider: string;
        model: string;
        modelName: string;
        cost_per_1m_input: number;
        cost_per_1m_output: number;
        quality_score: number;
        speed_score: number;
        cost_score: number;
        tier: "local";
        supports_tools: boolean;
        strengths: string[];
        weaknesses: string[];
        bestFor: string[];
    };
};
export declare class ModelOptimizer {
    /**
     * Optimize model selection based on agent, task, and priorities
     */
    static optimize(criteria: OptimizationCriteria): ModelRecommendation;
    /**
     * Infer task complexity from task description
     */
    private static inferComplexity;
    /**
     * Estimate cost for a task (rough approximation)
     */
    private static estimateCost;
    /**
     * Generate human-readable reasoning for model selection
     */
    private static generateReasoning;
    /**
     * Get all available models with their characteristics
     */
    static getAvailableModels(): typeof MODEL_DATABASE;
    /**
     * Display optimization recommendations in console
     */
    static displayRecommendation(recommendation: ModelRecommendation): void;
}
export {};
//# sourceMappingURL=modelOptimizer.d.ts.map