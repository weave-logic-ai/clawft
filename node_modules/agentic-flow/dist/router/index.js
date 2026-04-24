/**
 * Router module - Multi-model routing for agentic-flow
 *
 * Provides intelligent routing between LLM providers:
 * - Anthropic
 * - OpenRouter
 * - Gemini
 * - ONNX Local
 */
// Main router class
export { ModelRouter } from './router.js';
// Model mappings
export { CLAUDE_MODELS, mapModelId, getModelName, listModels } from './model-mapping.js';
// Providers
export { OpenRouterProvider } from './providers/openrouter.js';
export { AnthropicProvider } from './providers/anthropic.js';
export { GeminiProvider } from './providers/gemini.js';
export { ONNXLocalProvider } from './providers/onnx-local.js';
//# sourceMappingURL=index.js.map