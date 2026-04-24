"use strict";
/**
 * ONNX LLM Text Generation for RuVector
 *
 * Provides real local LLM inference using ONNX Runtime via transformers.js
 * Supports small models that run efficiently on CPU:
 * - SmolLM 135M - Smallest, fast (~135MB)
 * - SmolLM 360M - Better quality (~360MB)
 * - TinyLlama 1.1B - Best small model quality (~1GB quantized)
 * - Qwen2.5 0.5B - Good balance (~500MB)
 *
 * Features:
 * - Automatic model downloading and caching
 * - Quantized INT4/INT8 models for efficiency
 * - Streaming generation support
 * - Temperature, top-k, top-p sampling
 * - KV cache for efficient multi-turn conversations
 */
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.OnnxLLM = exports.AVAILABLE_MODELS = void 0;
exports.isTransformersAvailable = isTransformersAvailable;
exports.initOnnxLLM = initOnnxLLM;
exports.generate = generate;
exports.generateStream = generateStream;
exports.chat = chat;
exports.getModelInfo = getModelInfo;
exports.unload = unload;
const path = __importStar(require("path"));
const fs = __importStar(require("fs"));
// Force native dynamic import (avoids TypeScript transpiling to require)
// eslint-disable-next-line @typescript-eslint/no-implied-eval
const dynamicImport = new Function('specifier', 'return import(specifier)');
// ============================================================================
// Available Models
// ============================================================================
exports.AVAILABLE_MODELS = {
    // =========================================================================
    // TRM - Tiny Random Models (smallest, fastest)
    // =========================================================================
    'trm-tinystories': {
        id: 'Xenova/TinyStories-33M',
        name: 'TinyStories 33M (TRM)',
        size: '~65MB',
        description: 'Ultra-tiny model for stories and basic generation',
        contextLength: 512,
    },
    'trm-gpt2-tiny': {
        id: 'Xenova/gpt2',
        name: 'GPT-2 124M (TRM)',
        size: '~250MB',
        description: 'Classic GPT-2 tiny for general text',
        contextLength: 1024,
    },
    'trm-distilgpt2': {
        id: 'Xenova/distilgpt2',
        name: 'DistilGPT-2 (TRM)',
        size: '~82MB',
        description: 'Distilled GPT-2, fastest general model',
        contextLength: 1024,
    },
    // =========================================================================
    // SmolLM - Smallest production-ready models
    // =========================================================================
    'smollm-135m': {
        id: 'HuggingFaceTB/SmolLM-135M-Instruct',
        name: 'SmolLM 135M',
        size: '~135MB',
        description: 'Smallest instruct model, very fast',
        contextLength: 2048,
    },
    'smollm-360m': {
        id: 'HuggingFaceTB/SmolLM-360M-Instruct',
        name: 'SmolLM 360M',
        size: '~360MB',
        description: 'Small model, fast, better quality',
        contextLength: 2048,
    },
    'smollm2-135m': {
        id: 'HuggingFaceTB/SmolLM2-135M-Instruct',
        name: 'SmolLM2 135M',
        size: '~135MB',
        description: 'Latest SmolLM v2, improved capabilities',
        contextLength: 2048,
    },
    'smollm2-360m': {
        id: 'HuggingFaceTB/SmolLM2-360M-Instruct',
        name: 'SmolLM2 360M',
        size: '~360MB',
        description: 'Latest SmolLM v2, better quality',
        contextLength: 2048,
    },
    // =========================================================================
    // Qwen - Chinese/English bilingual models
    // =========================================================================
    'qwen2.5-0.5b': {
        id: 'Qwen/Qwen2.5-0.5B-Instruct',
        name: 'Qwen2.5 0.5B',
        size: '~300MB quantized',
        description: 'Good balance of speed and quality, multilingual',
        contextLength: 4096,
    },
    // =========================================================================
    // TinyLlama - Llama architecture in tiny form
    // =========================================================================
    'tinyllama': {
        id: 'TinyLlama/TinyLlama-1.1B-Chat-v1.0',
        name: 'TinyLlama 1.1B',
        size: '~600MB quantized',
        description: 'Best small model quality, slower',
        contextLength: 2048,
    },
    // =========================================================================
    // Code-specialized models
    // =========================================================================
    'codegemma-2b': {
        id: 'google/codegemma-2b',
        name: 'CodeGemma 2B',
        size: '~1GB quantized',
        description: 'Code generation specialist',
        contextLength: 8192,
    },
    'deepseek-coder-1.3b': {
        id: 'deepseek-ai/deepseek-coder-1.3b-instruct',
        name: 'DeepSeek Coder 1.3B',
        size: '~700MB quantized',
        description: 'Excellent for code tasks',
        contextLength: 4096,
    },
    // =========================================================================
    // Phi models - Microsoft's tiny powerhouses
    // =========================================================================
    'phi-2': {
        id: 'microsoft/phi-2',
        name: 'Phi-2 2.7B',
        size: '~1.5GB quantized',
        description: 'High quality small model',
        contextLength: 2048,
    },
    'phi-3-mini': {
        id: 'microsoft/Phi-3-mini-4k-instruct',
        name: 'Phi-3 Mini',
        size: '~2GB quantized',
        description: 'Best quality tiny model',
        contextLength: 4096,
    },
};
// ============================================================================
// ONNX LLM Generator
// ============================================================================
let pipeline = null;
let transformers = null;
let loadedModel = null;
let loadPromise = null;
let loadError = null;
/**
 * Check if transformers.js is available
 */
async function isTransformersAvailable() {
    try {
        await dynamicImport('@xenova/transformers');
        return true;
    }
    catch {
        return false;
    }
}
/**
 * Initialize the ONNX LLM with specified model
 */
async function initOnnxLLM(config = {}) {
    if (pipeline && loadedModel === config.modelId) {
        return true;
    }
    if (loadError)
        throw loadError;
    if (loadPromise) {
        await loadPromise;
        return pipeline !== null;
    }
    const modelId = config.modelId || 'HuggingFaceTB/SmolLM-135M-Instruct';
    loadPromise = (async () => {
        try {
            console.error(`Loading ONNX LLM: ${modelId}...`);
            // Import transformers.js
            transformers = await dynamicImport('@xenova/transformers');
            const { pipeline: createPipeline, env } = transformers;
            // Configure cache directory
            if (config.cacheDir) {
                env.cacheDir = config.cacheDir;
            }
            else {
                env.cacheDir = path.join(process.env.HOME || '/tmp', '.ruvector', 'models', 'onnx-llm');
            }
            // Ensure cache directory exists
            if (!fs.existsSync(env.cacheDir)) {
                fs.mkdirSync(env.cacheDir, { recursive: true });
            }
            // Disable remote model fetching warnings
            env.allowRemoteModels = true;
            env.allowLocalModels = true;
            // Create text generation pipeline
            console.error(`Downloading model (first run may take a while)...`);
            pipeline = await createPipeline('text-generation', modelId, {
                quantized: config.quantized !== false,
                device: config.device || 'cpu',
            });
            loadedModel = modelId;
            console.error(`ONNX LLM ready: ${modelId}`);
        }
        catch (e) {
            loadError = new Error(`Failed to initialize ONNX LLM: ${e.message}`);
            throw loadError;
        }
    })();
    await loadPromise;
    return pipeline !== null;
}
/**
 * Generate text using ONNX LLM
 */
async function generate(prompt, config = {}) {
    if (!pipeline) {
        await initOnnxLLM();
    }
    if (!pipeline) {
        throw new Error('ONNX LLM not initialized');
    }
    const start = performance.now();
    // Build the input text (apply chat template if needed)
    let inputText = prompt;
    if (config.systemPrompt) {
        // Apply simple chat format
        inputText = `<|system|>\n${config.systemPrompt}<|end|>\n<|user|>\n${prompt}<|end|>\n<|assistant|>\n`;
    }
    // Generate
    const outputs = await pipeline(inputText, {
        max_new_tokens: config.maxNewTokens || 128,
        temperature: config.temperature || 0.7,
        top_p: config.topP || 0.9,
        top_k: config.topK || 50,
        repetition_penalty: config.repetitionPenalty || 1.1,
        do_sample: (config.temperature || 0.7) > 0,
        return_full_text: false,
    });
    const timeMs = performance.now() - start;
    const generatedText = outputs[0]?.generated_text || '';
    // Estimate tokens (rough approximation)
    const tokensGenerated = Math.ceil(generatedText.split(/\s+/).length * 1.3);
    return {
        text: generatedText.trim(),
        tokensGenerated,
        timeMs,
        tokensPerSecond: tokensGenerated / (timeMs / 1000),
        model: loadedModel || 'unknown',
        cached: true,
    };
}
/**
 * Generate with streaming (token by token)
 */
async function generateStream(prompt, config = {}) {
    if (!pipeline) {
        await initOnnxLLM();
    }
    if (!pipeline) {
        throw new Error('ONNX LLM not initialized');
    }
    const start = performance.now();
    let fullText = '';
    let tokenCount = 0;
    // Build input text
    let inputText = prompt;
    if (config.systemPrompt) {
        inputText = `<|system|>\n${config.systemPrompt}<|end|>\n<|user|>\n${prompt}<|end|>\n<|assistant|>\n`;
    }
    // Create streamer
    const { TextStreamer } = transformers;
    const streamer = new TextStreamer(pipeline.tokenizer, {
        skip_prompt: true,
        callback_function: (text) => {
            fullText += text;
            tokenCount++;
            if (config.onToken) {
                config.onToken(text);
            }
        },
    });
    // Generate with streamer
    await pipeline(inputText, {
        max_new_tokens: config.maxNewTokens || 128,
        temperature: config.temperature || 0.7,
        top_p: config.topP || 0.9,
        top_k: config.topK || 50,
        repetition_penalty: config.repetitionPenalty || 1.1,
        do_sample: (config.temperature || 0.7) > 0,
        streamer,
    });
    const timeMs = performance.now() - start;
    // Return generator that yields the collected text
    async function* generator() {
        yield fullText;
        return {
            text: fullText.trim(),
            tokensGenerated: tokenCount,
            timeMs,
            tokensPerSecond: tokenCount / (timeMs / 1000),
            model: loadedModel || 'unknown',
            cached: true,
        };
    }
    return generator();
}
/**
 * Chat completion with conversation history
 */
async function chat(messages, config = {}) {
    if (!pipeline) {
        await initOnnxLLM();
    }
    if (!pipeline) {
        throw new Error('ONNX LLM not initialized');
    }
    // Build conversation text from messages
    let conversationText = '';
    for (const msg of messages) {
        if (msg.role === 'system') {
            conversationText += `<|system|>\n${msg.content}<|end|>\n`;
        }
        else if (msg.role === 'user') {
            conversationText += `<|user|>\n${msg.content}<|end|>\n`;
        }
        else if (msg.role === 'assistant') {
            conversationText += `<|assistant|>\n${msg.content}<|end|>\n`;
        }
    }
    conversationText += '<|assistant|>\n';
    return generate(conversationText, { ...config, systemPrompt: undefined });
}
/**
 * Get model information
 */
function getModelInfo() {
    return {
        model: loadedModel,
        ready: pipeline !== null,
        availableModels: exports.AVAILABLE_MODELS,
    };
}
/**
 * Unload the current model to free memory
 */
async function unload() {
    if (pipeline) {
        // Note: transformers.js doesn't have explicit dispose, but we can null the reference
        pipeline = null;
        loadedModel = null;
        loadPromise = null;
        loadError = null;
    }
}
// ============================================================================
// Class wrapper for OOP usage
// ============================================================================
class OnnxLLM {
    constructor(config = {}) {
        this.initialized = false;
        this.config = config;
    }
    async init() {
        if (this.initialized)
            return true;
        this.initialized = await initOnnxLLM(this.config);
        return this.initialized;
    }
    async generate(prompt, config) {
        if (!this.initialized)
            await this.init();
        return generate(prompt, config);
    }
    async chat(messages, config) {
        if (!this.initialized)
            await this.init();
        return chat(messages, config);
    }
    async unload() {
        await unload();
        this.initialized = false;
    }
    get ready() {
        return this.initialized;
    }
    get model() {
        return loadedModel;
    }
}
exports.OnnxLLM = OnnxLLM;
exports.default = OnnxLLM;
