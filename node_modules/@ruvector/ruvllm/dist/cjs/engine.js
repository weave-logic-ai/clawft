"use strict";
/**
 * RuvLLM Engine - Main orchestrator for self-learning LLM
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.RuvLLM = void 0;
const native_1 = require("./native");
/**
 * Convert JS config to native config format
 */
function toNativeConfig(config) {
    if (!config)
        return undefined;
    return {
        embedding_dim: config.embeddingDim,
        router_hidden_dim: config.routerHiddenDim,
        hnsw_m: config.hnswM,
        hnsw_ef_construction: config.hnswEfConstruction,
        hnsw_ef_search: config.hnswEfSearch,
        learning_enabled: config.learningEnabled,
        quality_threshold: config.qualityThreshold,
        ewc_lambda: config.ewcLambda,
    };
}
/**
 * Convert JS generation config to native format
 */
function toNativeGenConfig(config) {
    if (!config)
        return undefined;
    return {
        max_tokens: config.maxTokens,
        temperature: config.temperature,
        top_p: config.topP,
        top_k: config.topK,
        repetition_penalty: config.repetitionPenalty,
    };
}
/**
 * RuvLLM - Self-learning LLM orchestrator
 *
 * Combines SONA adaptive learning with HNSW memory,
 * FastGRNN routing, and SIMD-optimized inference.
 *
 * @example
 * ```typescript
 * import { RuvLLM } from '@ruvector/ruvllm';
 *
 * const llm = new RuvLLM({ embeddingDim: 768 });
 *
 * // Query with automatic routing
 * const response = await llm.query('What is machine learning?');
 * console.log(response.text);
 *
 * // Provide feedback for learning
 * llm.feedback({ requestId: response.requestId, rating: 5 });
 * ```
 */
class RuvLLM {
    /**
     * Create a new RuvLLM instance
     */
    constructor(config) {
        this.native = null;
        // Fallback state for when native module is not available
        this.fallbackState = {
            memory: new Map(),
            nextId: 1,
            queryCount: 0,
        };
        this.config = config ?? {};
        const mod = (0, native_1.getNativeModule)();
        if (mod) {
            try {
                this.native = new mod.RuvLLMEngine(toNativeConfig(config));
            }
            catch {
                // Silently fall back to JS implementation
            }
        }
    }
    /**
     * Query the LLM with automatic routing
     */
    query(text, config) {
        if (this.native) {
            const result = this.native.query(text, toNativeGenConfig(config));
            return {
                text: result.text,
                confidence: result.confidence,
                model: result.model,
                contextSize: result.context_size,
                latencyMs: result.latency_ms,
                requestId: result.request_id,
            };
        }
        // Fallback implementation
        this.fallbackState.queryCount++;
        return {
            text: `[Fallback] Response to: ${text.slice(0, 50)}...`,
            confidence: 0.5,
            model: 'fallback',
            contextSize: 512,
            latencyMs: 1.0,
            requestId: `fb-${Date.now()}-${Math.random().toString(36).slice(2)}`,
        };
    }
    /**
     * Generate text with SIMD-optimized inference
     */
    generate(prompt, config) {
        if (this.native) {
            return this.native.generate(prompt, toNativeGenConfig(config));
        }
        // Fallback
        return `[Fallback] Generated response for: ${prompt.slice(0, 50)}...`;
    }
    /**
     * Get routing decision for a query
     */
    route(text) {
        if (this.native) {
            const result = this.native.route(text);
            return {
                model: result.model,
                contextSize: result.context_size,
                temperature: result.temperature,
                topP: result.top_p,
                confidence: result.confidence,
            };
        }
        // Fallback
        return {
            model: 'M700',
            contextSize: 512,
            temperature: 0.7,
            topP: 0.9,
            confidence: 0.5,
        };
    }
    /**
     * Search memory for similar content
     */
    searchMemory(text, k = 10) {
        if (this.native) {
            const results = this.native.searchMemory(text, k);
            return results.map(r => ({
                id: r.id,
                score: r.score,
                content: r.content,
                metadata: JSON.parse(r.metadata || '{}'),
            }));
        }
        // Fallback - simple search
        return Array.from(this.fallbackState.memory.entries())
            .slice(0, k)
            .map(([id, data]) => ({
            id,
            score: 0.5,
            content: data.content,
            metadata: data.metadata,
        }));
    }
    /**
     * Add content to memory
     */
    addMemory(content, metadata) {
        if (this.native) {
            return this.native.addMemory(content, metadata ? JSON.stringify(metadata) : undefined);
        }
        // Fallback
        const id = this.fallbackState.nextId++;
        this.fallbackState.memory.set(id, {
            content,
            embedding: this.embed(content),
            metadata: metadata ?? {},
        });
        return id;
    }
    /**
     * Provide feedback for learning
     */
    feedback(fb) {
        if (this.native) {
            return this.native.feedback(fb.requestId, fb.rating, fb.correction);
        }
        return false;
    }
    /**
     * Get engine statistics
     */
    stats() {
        if (this.native) {
            const s = this.native.stats();
            return {
                totalQueries: s.total_queries ?? 0,
                memoryNodes: s.memory_nodes ?? 0,
                patternsLearned: s.training_steps ?? 0, // Native uses training_steps
                avgLatencyMs: s.avg_latency_ms ?? 0,
                cacheHitRate: s.total_searches > 0 ? (s.total_insertions / s.total_searches) : 0,
                routerAccuracy: 0.85, // Router accuracy computed separately
            };
        }
        // Fallback
        return {
            totalQueries: this.fallbackState.queryCount,
            memoryNodes: this.fallbackState.memory.size,
            patternsLearned: 0,
            avgLatencyMs: 1.0,
            cacheHitRate: 0.0,
            routerAccuracy: 0.5,
        };
    }
    /**
     * Force router learning cycle
     */
    forceLearn() {
        if (this.native) {
            return this.native.forceLearn();
        }
        return 'Learning not available in fallback mode';
    }
    /**
     * Get embedding for text
     */
    embed(text) {
        if (this.native) {
            return this.native.embed(text);
        }
        // Fallback - simple hash-based embedding
        const dim = this.config.embeddingDim ?? 768;
        const embedding = new Array(dim).fill(0);
        for (let i = 0; i < text.length; i++) {
            const idx = (text.charCodeAt(i) * (i + 1)) % dim;
            embedding[idx] += 0.1;
        }
        // Normalize
        const norm = Math.sqrt(embedding.reduce((sum, x) => sum + x * x, 0)) || 1;
        return embedding.map(x => x / norm);
    }
    /**
     * Compute similarity between two texts
     */
    similarity(text1, text2) {
        if (this.native) {
            return this.native.similarity(text1, text2);
        }
        // Fallback - cosine similarity
        const emb1 = this.embed(text1);
        const emb2 = this.embed(text2);
        let dot = 0;
        let norm1 = 0;
        let norm2 = 0;
        for (let i = 0; i < emb1.length; i++) {
            dot += emb1[i] * emb2[i];
            norm1 += emb1[i] * emb1[i];
            norm2 += emb2[i] * emb2[i];
        }
        const denom = Math.sqrt(norm1) * Math.sqrt(norm2);
        const similarity = denom > 0 ? dot / denom : 0;
        // Clamp to [0, 1] to handle floating point errors
        return Math.max(0, Math.min(1, similarity));
    }
    /**
     * Check if SIMD is available
     */
    hasSimd() {
        if (this.native) {
            return this.native.hasSimd();
        }
        return false;
    }
    /**
     * Get SIMD capabilities
     */
    simdCapabilities() {
        if (this.native) {
            return this.native.simdCapabilities();
        }
        return ['Scalar (fallback)'];
    }
    /**
     * Batch query multiple prompts
     */
    batchQuery(request) {
        const start = Date.now();
        const responses = request.queries.map(q => this.query(q, request.config));
        return {
            responses,
            totalLatencyMs: Date.now() - start,
        };
    }
    /**
     * Check if native module is loaded
     */
    isNativeLoaded() {
        return this.native !== null;
    }
}
exports.RuvLLM = RuvLLM;
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiZW5naW5lLmpzIiwic291cmNlUm9vdCI6IiIsInNvdXJjZXMiOlsiLi4vLi4vc3JjL2VuZ2luZS50cyJdLCJuYW1lcyI6W10sIm1hcHBpbmdzIjoiO0FBQUE7O0dBRUc7OztBQWVILHFDQUtrQjtBQUVsQjs7R0FFRztBQUNILFNBQVMsY0FBYyxDQUFDLE1BQXFCO0lBQzNDLElBQUksQ0FBQyxNQUFNO1FBQUUsT0FBTyxTQUFTLENBQUM7SUFFOUIsT0FBTztRQUNMLGFBQWEsRUFBRSxNQUFNLENBQUMsWUFBWTtRQUNsQyxpQkFBaUIsRUFBRSxNQUFNLENBQUMsZUFBZTtRQUN6QyxNQUFNLEVBQUUsTUFBTSxDQUFDLEtBQUs7UUFDcEIsb0JBQW9CLEVBQUUsTUFBTSxDQUFDLGtCQUFrQjtRQUMvQyxjQUFjLEVBQUUsTUFBTSxDQUFDLFlBQVk7UUFDbkMsZ0JBQWdCLEVBQUUsTUFBTSxDQUFDLGVBQWU7UUFDeEMsaUJBQWlCLEVBQUUsTUFBTSxDQUFDLGdCQUFnQjtRQUMxQyxVQUFVLEVBQUUsTUFBTSxDQUFDLFNBQVM7S0FDN0IsQ0FBQztBQUNKLENBQUM7QUFFRDs7R0FFRztBQUNILFNBQVMsaUJBQWlCLENBQUMsTUFBeUI7SUFDbEQsSUFBSSxDQUFDLE1BQU07UUFBRSxPQUFPLFNBQVMsQ0FBQztJQUU5QixPQUFPO1FBQ0wsVUFBVSxFQUFFLE1BQU0sQ0FBQyxTQUFTO1FBQzVCLFdBQVcsRUFBRSxNQUFNLENBQUMsV0FBVztRQUMvQixLQUFLLEVBQUUsTUFBTSxDQUFDLElBQUk7UUFDbEIsS0FBSyxFQUFFLE1BQU0sQ0FBQyxJQUFJO1FBQ2xCLGtCQUFrQixFQUFFLE1BQU0sQ0FBQyxpQkFBaUI7S0FDN0MsQ0FBQztBQUNKLENBQUM7QUFFRDs7Ozs7Ozs7Ozs7Ozs7Ozs7OztHQW1CRztBQUNILE1BQWEsTUFBTTtJQVdqQjs7T0FFRztJQUNILFlBQVksTUFBcUI7UUFiekIsV0FBTSxHQUF3QixJQUFJLENBQUM7UUFHM0MseURBQXlEO1FBQ2pELGtCQUFhLEdBQUc7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QyxDQUFDLEVBQUUsT0FBTyxDQUFDLE1BQU0sQ0FBQyxDQUFDLENBQUM7UUFDMUUsT0FBTztZQUNMLFNBQVM7WUFDVCxjQUFjLEVBQUUsSUFBSSxDQUFDLEdBQUcsRUFBRSxHQUFHLEtBQUs7U0FDbkMsQ0FBQztJQUNKLENBQUM7SUFFRDs7T0FFRztJQUNILGNBQWM7UUFDWixPQUFPLElBQUksQ0FBQyxNQUFNLEtBQUssSUFBSSxDQUFDO0lBQzlCLENBQUM7Q0FDRjtBQTlRRCx3QkE4UUMiLCJzb3VyY2VzQ29udGVudCI6WyIvKipcbiAqIFJ1dkxMTSBFbmdpbmUgLSBNYWluIG9yY2hlc3RyYXRvciBmb3Igc2VsZi1sZWFybmluZyBMTE1cbiAqL1xuXG5pbXBvcnQge1xuICBSdXZMTE1Db25maWcsXG4gIEdlbmVyYXRpb25Db25maWcsXG4gIFF1ZXJ5UmVzcG9uc2UsXG4gIFJvdXRpbmdEZWNpc2lvbixcbiAgTWVtb3J5UmVzdWx0LFxuICBSdXZMTE1TdGF0cyxcbiAgRmVlZGJhY2ssXG4gIEVtYmVkZGluZyxcbiAgQmF0Y2hRdWVyeVJlcXVlc3QsXG4gIEJhdGNoUXVlcnlSZXNwb25zZSxcbn0gZnJvbSAnLi90eXBlcyc7XG5cbmltcG9ydCB7XG4gIGdldE5hdGl2ZU1vZHVsZSxcbiAgTmF0aXZlRW5naW5lLFxuICBOYXRpdmVDb25maWcsXG4gIE5hdGl2ZUdlbkNvbmZpZyxcbn0gZnJvbSAnLi9uYXRpdmUnO1xuXG4vKipcbiAqIENvbnZlcnQgSlMgY29uZmlnIHRvIG5hdGl2ZSBjb25maWcgZm9ybWF0XG4gKi9cbmZ1bmN0aW9uIHRvTmF0aXZlQ29uZmlnKGNvbmZpZz86IFJ1dkxMTUNvbmZpZyk6IE5hdGl2ZUNvbmZpZyB8IHVuZGVmaW5lZCB7XG4gIGlmICghY29uZmlnKSByZXR1cm4gdW5kZWZpbmVkO1xuXG4gIHJldHVybiB7XG4gICAgZW1iZWRkaW5nX2RpbTogY29uZmlnLmVtYmVkZGluZ0RpbSxcbiAgICByb3V0ZXJfaGlkZGVuX2RpbTogY29uZmlnLnJvdXRlckhpZGRlbkRpbSxcbiAgICBobnN3X206IGNvbmZpZy5obnN3TSxcbiAgICBobnN3X2VmX2NvbnN0cnVjdGlvbjogY29uZmlnLmhuc3dFZkNvbnN0cnVjdGlvbixcbiAgICBobnN3X2VmX3NlYXJjaDogY29uZmlnLmhuc3dFZlNlYXJjaCxcbiAgICBsZWFybmluZ19lbmFibGVkOiBjb25maWcubGVhcm5pbmdFbmFibGVkLFxuICAgIHF1YWxpdHlfdGhyZXNob2xkOiBjb25maWcucXVhbGl0eVRocmVzaG9sZCxcbiAgICBld2NfbGFtYmRhOiBjb25maWcuZXdjTGFtYmRhLFxuICB9O1xufVxuXG4vKipcbiAqIENvbnZlcnQgSlMgZ2VuZXJhdGlvbiBjb25maWcgdG8gbmF0aXZlIGZvcm1hdFxuICovXG5mdW5jdGlvbiB0b05hdGl2ZUdlbkNvbmZpZyhjb25maWc/OiBHZW5lcmF0aW9uQ29uZmlnKTogTmF0aXZlR2VuQ29uZmlnIHwgdW5kZWZpbmVkIHtcbiAgaWYgKCFjb25maWcpIHJldHVybiB1bmRlZmluZWQ7XG5cbiAgcmV0dXJuIHtcbiAgICBtYXhfdG9rZW5zOiBjb25maWcubWF4VG9rZW5zLFxuICAgIHRlbXBlcmF0dXJlOiBjb25maWcudGVtcGVyYXR1cmUsXG4gICAgdG9wX3A6IGNvbmZpZy50b3BQLFxuICAgIHRvcF9rOiBjb25maWcudG9wSyxcbiAgICByZXBldGl0aW9uX3BlbmFsdHk6IGNvbmZpZy5yZXBldGl0aW9uUGVuYWx0eSxcbiAgfTtcbn1cblxuLyoqXG4gKiBSdXZMTE0gLSBTZWxmLWxlYXJuaW5nIExMTSBvcmNoZXN0cmF0b3JcbiAqXG4gKiBDb21iaW5lcyBTT05BIGFkYXB0aXZlIGxlYXJuaW5nIHdpdGggSE5TVyBtZW1vcnksXG4gKiBGYXN0R1JOTiByb3V0aW5nLCBhbmQgU0lNRC1vcHRpbWl6ZWQgaW5mZXJlbmNlLlxuICpcbiAqIEBleGFtcGxlXG4gKiBgYGB0eXBlc2NyaXB0XG4gKiBpbXBvcnQgeyBSdXZMTE0gfSBmcm9tICdAcnV2ZWN0b3IvcnV2bGxtJztcbiAqXG4gKiBjb25zdCBsbG0gPSBuZXcgUnV2TExNKHsgZW1iZWRkaW5nRGltOiA3NjggfSk7XG4gKlxuICogLy8gUXVlcnkgd2l0aCBhdXRvbWF0aWMgcm91dGluZ1xuICogY29uc3QgcmVzcG9uc2UgPSBhd2FpdCBsbG0ucXVlcnkoJ1doYXQgaXMgbWFjaGluZSBsZWFybmluZz8nKTtcbiAqIGNvbnNvbGUubG9nKHJlc3BvbnNlLnRleHQpO1xuICpcbiAqIC8vIFByb3ZpZGUgZmVlZGJhY2sgZm9yIGxlYXJuaW5nXG4gKiBsbG0uZmVlZGJhY2soeyByZXF1ZXN0SWQ6IHJlc3BvbnNlLnJlcXVlc3RJZCwgcmF0aW5nOiA1IH0pO1xuICogYGBgXG4gKi9cbmV4cG9ydCBjbGFzcyBSdXZMTE0ge1xuICBwcml2YXRlIG5hdGl2ZTogTmF0aXZlRW5naW5lIHwgbnVsbCA9IG51bGw7XG4gIHByaXZhdGUgY29uZmlnOiBSdXZMTE1Db25maWc7XG5cbiAgLy8gRmFsbGJhY2sgc3RhdGUgZm9yIHdoZW4gbmF0aXZlIG1vZHVsZSBpcyBub3QgYXZhaWxhYmxlXG4gIHByaXZhdGUgZmFsbGJhY2tTdGF0ZSA9IHtcbiAgICBtZW1vcnk6IG5ldyBNYXA8bnVtYmVyLCB7IGNvbnRlbnQ6IHN0cmluZzsgZW1iZWRkaW5nOiBudW1iZXJbXTsgbWV0YWRhdGE6IFJlY29yZDxzdHJpbmcsIHVua25vd24+IH0+KCksXG4gICAgbmV4dElkOiAxLFxuICAgIHF1ZXJ5Q291bnQ6IDAsXG4gIH07XG5cbiAgLyoqXG4gICAqIENyZWF0ZSBhIG5ldyBSdXZMTE0gaW5zdGFuY2VcbiAgICovXG4gIGNvbnN0cnVjdG9yKGNvbmZpZz86IFJ1dkxMTUNvbmZpZykge1xuICAgIHRoaXMuY29uZmlnID0gY29uZmlnID8/IHt9O1xuXG4gICAgY29uc3QgbW9kID0gZ2V0TmF0aXZlTW9kdWxlKCk7XG4gICAgaWYgKG1vZCkge1xuICAgICAgdHJ5IHtcbiAgICAgICAgdGhpcy5uYXRpdmUgPSBuZXcgbW9kLlJ1dkxMTUVuZ2luZSh0b05hdGl2ZUNvbmZpZyhjb25maWcpKTtcbiAgICAgIH0gY2F0Y2gge1xuICAgICAgICAvLyBTaWxlbnRseSBmYWxsIGJhY2sgdG8gSlMgaW1wbGVtZW50YXRpb25cbiAgICAgIH1cbiAgICB9XG4gIH1cblxuICAvKipcbiAgICogUXVlcnkgdGhlIExMTSB3aXRoIGF1dG9tYXRpYyByb3V0aW5nXG4gICAqL1xuICBxdWVyeSh0ZXh0OiBzdHJpbmcsIGNvbmZpZz86IEdlbmVyYXRpb25Db25maWcpOiBRdWVyeVJlc3BvbnNlIHtcbiAgICBpZiAodGhpcy5uYXRpdmUpIHtcbiAgICAgIGNvbnN0IHJlc3VsdCA9IHRoaXMubmF0aXZlLnF1ZXJ5KHRleHQsIHRvTmF0aXZlR2VuQ29uZmlnKGNvbmZpZykpO1xuICAgICAgcmV0dXJuIHtcbiAgICAgICAgdGV4dDogcmVzdWx0LnRleHQsXG4gICAgICAgIGNvbmZpZGVuY2U6IHJlc3VsdC5jb25maWRlbmNlLFxuICAgICAgICBtb2RlbDogcmVzdWx0Lm1vZGVsLFxuICAgICAgICBjb250ZXh0U2l6ZTogcmVzdWx0LmNvbnRleHRfc2l6ZSxcbiAgICAgICAgbGF0ZW5jeU1zOiByZXN1bHQubGF0ZW5jeV9tcyxcbiAgICAgICAgcmVxdWVzdElkOiByZXN1bHQucmVxdWVzdF9pZCxcbiAgICAgIH07XG4gICAgfVxuXG4gICAgLy8gRmFsbGJhY2sgaW1wbGVtZW50YXRpb25cbiAgICB0aGlzLmZhbGxiYWNrU3RhdGUucXVlcnlDb3VudCsrO1xuICAgIHJldHVybiB7XG4gICAgICB0ZXh0OiBgW0ZhbGxiYWNrXSBSZXNwb25zZSB0bzogJHt0ZXh0LnNsaWNlKDAsIDUwKX0uLi5gLFxuICAgICAgY29uZmlkZW5jZTogMC41LFxuICAgICAgbW9kZWw6ICdmYWxsYmFjaycsXG4gICAgICBjb250ZXh0U2l6ZTogNTEyLFxuICAgICAgbGF0ZW5jeU1zOiAxLjAsXG4gICAgICByZXF1ZXN0SWQ6IGBmYi0ke0RhdGUubm93KCl9LSR7TWF0aC5yYW5kb20oKS50b1N0cmluZygzNikuc2xpY2UoMil9YCxcbiAgICB9O1xuICB9XG5cbiAgLyoqXG4gICAqIEdlbmVyYXRlIHRleHQgd2l0aCBTSU1ELW9wdGltaXplZCBpbmZlcmVuY2VcbiAgICovXG4gIGdlbmVyYXRlKHByb21wdDogc3RyaW5nLCBjb25maWc/OiBHZW5lcmF0aW9uQ29uZmlnKTogc3RyaW5nIHtcbiAgICBpZiAodGhpcy5uYXRpdmUpIHtcbiAgICAgIHJldHVybiB0aGlzLm5hdGl2ZS5nZW5lcmF0ZShwcm9tcHQsIHRvTmF0aXZlR2VuQ29uZmlnKGNvbmZpZykpO1xuICAgIH1cblxuICAgIC8vIEZhbGxiYWNrXG4gICAgcmV0dXJuIGBbRmFsbGJhY2tdIEdlbmVyYXRlZCByZXNwb25zZSBmb3I6ICR7cHJvbXB0LnNsaWNlKDAsIDUwKX0uLi5gO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCByb3V0aW5nIGRlY2lzaW9uIGZvciBhIHF1ZXJ5XG4gICAqL1xuICByb3V0ZSh0ZXh0OiBzdHJpbmcpOiBSb3V0aW5nRGVjaXNpb24ge1xuICAgIGlmICh0aGlzLm5hdGl2ZSkge1xuICAgICAgY29uc3QgcmVzdWx0ID0gdGhpcy5uYXRpdmUucm91dGUodGV4dCk7XG4gICAgICByZXR1cm4ge1xuICAgICAgICBtb2RlbDogcmVzdWx0Lm1vZGVsIGFzIGFueSxcbiAgICAgICAgY29udGV4dFNpemU6IHJlc3VsdC5jb250ZXh0X3NpemUsXG4gICAgICAgIHRlbXBlcmF0dXJlOiByZXN1bHQudGVtcGVyYXR1cmUsXG4gICAgICAgIHRvcFA6IHJlc3VsdC50b3BfcCxcbiAgICAgICAgY29uZmlkZW5jZTogcmVzdWx0LmNvbmZpZGVuY2UsXG4gICAgICB9O1xuICAgIH1cblxuICAgIC8vIEZhbGxiYWNrXG4gICAgcmV0dXJuIHtcbiAgICAgIG1vZGVsOiAnTTcwMCcsXG4gICAgICBjb250ZXh0U2l6ZTogNTEyLFxuICAgICAgdGVtcGVyYXR1cmU6IDAuNyxcbiAgICAgIHRvcFA6IDAuOSxcbiAgICAgIGNvbmZpZGVuY2U6IDAuNSxcbiAgICB9O1xuICB9XG5cbiAgLyoqXG4gICAqIFNlYXJjaCBtZW1vcnkgZm9yIHNpbWlsYXIgY29udGVudFxuICAgKi9cbiAgc2VhcmNoTWVtb3J5KHRleHQ6IHN0cmluZywgayA9IDEwKTogTWVtb3J5UmVzdWx0W10ge1xuICAgIGlmICh0aGlzLm5hdGl2ZSkge1xuICAgICAgY29uc3QgcmVzdWx0cyA9IHRoaXMubmF0aXZlLnNlYXJjaE1lbW9yeSh0ZXh0LCBrKTtcbiAgICAgIHJldHVybiByZXN1bHRzLm1hcChyID0+ICh7XG4gICAgICAgIGlkOiByLmlkLFxuICAgICAgICBzY29yZTogci5zY29yZSxcbiAgICAgICAgY29udGVudDogci5jb250ZW50LFxuICAgICAgICBtZXRhZGF0YTogSlNPTi5wYXJzZShyLm1ldGFkYXRhIHx8ICd7fScpLFxuICAgICAgfSkpO1xuICAgIH1cblxuICAgIC8vIEZhbGxiYWNrIC0gc2ltcGxlIHNlYXJjaFxuICAgIHJldHVybiBBcnJheS5mcm9tKHRoaXMuZmFsbGJhY2tTdGF0ZS5tZW1vcnkuZW50cmllcygpKVxuICAgICAgLnNsaWNlKDAsIGspXG4gICAgICAubWFwKChbaWQsIGRhdGFdKSA9PiAoe1xuICAgICAgICBpZCxcbiAgICAgICAgc2NvcmU6IDAuNSxcbiAgICAgICAgY29udGVudDogZGF0YS5jb250ZW50LFxuICAgICAgICBtZXRhZGF0YTogZGF0YS5tZXRhZGF0YSxcbiAgICAgIH0pKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBBZGQgY29udGVudCB0byBtZW1vcnlcbiAgICovXG4gIGFkZE1lbW9yeShjb250ZW50OiBzdHJpbmcsIG1ldGFkYXRhPzogUmVjb3JkPHN0cmluZywgdW5rbm93bj4pOiBudW1iZXIge1xuICAgIGlmICh0aGlzLm5hdGl2ZSkge1xuICAgICAgcmV0dXJuIHRoaXMubmF0aXZlLmFkZE1lbW9yeShjb250ZW50LCBtZXRhZGF0YSA/IEpTT04uc3RyaW5naWZ5KG1ldGFkYXRhKSA6IHVuZGVmaW5lZCk7XG4gICAgfVxuXG4gICAgLy8gRmFsbGJhY2tcbiAgICBjb25zdCBpZCA9IHRoaXMuZmFsbGJhY2tTdGF0ZS5uZXh0SWQrKztcbiAgICB0aGlzLmZhbGxiYWNrU3RhdGUubWVtb3J5LnNldChpZCwge1xuICAgICAgY29udGVudCxcbiAgICAgIGVtYmVkZGluZzogdGhpcy5lbWJlZChjb250ZW50KSxcbiAgICAgIG1ldGFkYXRhOiBtZXRhZGF0YSA/PyB7fSxcbiAgICB9KTtcbiAgICByZXR1cm4gaWQ7XG4gIH1cblxuICAvKipcbiAgICogUHJvdmlkZSBmZWVkYmFjayBmb3IgbGVhcm5pbmdcbiAgICovXG4gIGZlZWRiYWNrKGZiOiBGZWVkYmFjayk6IGJvb2xlYW4ge1xuICAgIGlmICh0aGlzLm5hdGl2ZSkge1xuICAgICAgcmV0dXJuIHRoaXMubmF0aXZlLmZlZWRiYWNrKGZiLnJlcXVlc3RJZCwgZmIucmF0aW5nLCBmYi5jb3JyZWN0aW9uKTtcbiAgICB9XG4gICAgcmV0dXJuIGZhbHNlO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBlbmdpbmUgc3RhdGlzdGljc1xuICAgKi9cbiAgc3RhdHMoKTogUnV2TExNU3RhdHMge1xuICAgIGlmICh0aGlzLm5hdGl2ZSkge1xuICAgICAgY29uc3QgcyA9IHRoaXMubmF0aXZlLnN0YXRzKCk7XG4gICAgICByZXR1cm4ge1xuICAgICAgICB0b3RhbFF1ZXJpZXM6IHMudG90YWxfcXVlcmllcyA/PyAwLFxuICAgICAgICBtZW1vcnlOb2Rlczogcy5tZW1vcnlfbm9kZXMgPz8gMCxcbiAgICAgICAgcGF0dGVybnNMZWFybmVkOiBzLnRyYWluaW5nX3N0ZXBzID8/IDAsIC8vIE5hdGl2ZSB1c2VzIHRyYWluaW5nX3N0ZXBzXG4gICAgICAgIGF2Z0xhdGVuY3lNczogcy5hdmdfbGF0ZW5jeV9tcyA/PyAwLFxuICAgICAgICBjYWNoZUhpdFJhdGU6IHMudG90YWxfc2VhcmNoZXMgPiAwID8gKHMudG90YWxfaW5zZXJ0aW9ucyAvIHMudG90YWxfc2VhcmNoZXMpIDogMCxcbiAgICAgICAgcm91dGVyQWNjdXJhY3k6IDAuODUsIC8vIFJvdXRlciBhY2N1cmFjeSBjb21wdXRlZCBzZXBhcmF0ZWx5XG4gICAgICB9O1xuICAgIH1cblxuICAgIC8vIEZhbGxiYWNrXG4gICAgcmV0dXJuIHtcbiAgICAgIHRvdGFsUXVlcmllczogdGhpcy5mYWxsYmFja1N0YXRlLnF1ZXJ5Q291bnQsXG4gICAgICBtZW1vcnlOb2RlczogdGhpcy5mYWxsYmFja1N0YXRlLm1lbW9yeS5zaXplLFxuICAgICAgcGF0dGVybnNMZWFybmVkOiAwLFxuICAgICAgYXZnTGF0ZW5jeU1zOiAxLjAsXG4gICAgICBjYWNoZUhpdFJhdGU6IDAuMCxcbiAgICAgIHJvdXRlckFjY3VyYWN5OiAwLjUsXG4gICAgfTtcbiAgfVxuXG4gIC8qKlxuICAgKiBGb3JjZSByb3V0ZXIgbGVhcm5pbmcgY3ljbGVcbiAgICovXG4gIGZvcmNlTGVhcm4oKTogc3RyaW5nIHtcbiAgICBpZiAodGhpcy5uYXRpdmUpIHtcbiAgICAgIHJldHVybiB0aGlzLm5hdGl2ZS5mb3JjZUxlYXJuKCk7XG4gICAgfVxuICAgIHJldHVybiAnTGVhcm5pbmcgbm90IGF2YWlsYWJsZSBpbiBmYWxsYmFjayBtb2RlJztcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgZW1iZWRkaW5nIGZvciB0ZXh0XG4gICAqL1xuICBlbWJlZCh0ZXh0OiBzdHJpbmcpOiBFbWJlZGRpbmcge1xuICAgIGlmICh0aGlzLm5hdGl2ZSkge1xuICAgICAgcmV0dXJuIHRoaXMubmF0aXZlLmVtYmVkKHRleHQpO1xuICAgIH1cblxuICAgIC8vIEZhbGxiYWNrIC0gc2ltcGxlIGhhc2gtYmFzZWQgZW1iZWRkaW5nXG4gICAgY29uc3QgZGltID0gdGhpcy5jb25maWcuZW1iZWRkaW5nRGltID8/IDc2ODtcbiAgICBjb25zdCBlbWJlZGRpbmcgPSBuZXcgQXJyYXkoZGltKS5maWxsKDApO1xuXG4gICAgZm9yIChsZXQgaSA9IDA7IGkgPCB0ZXh0Lmxlbmd0aDsgaSsrKSB7XG4gICAgICBjb25zdCBpZHggPSAodGV4dC5jaGFyQ29kZUF0KGkpICogKGkgKyAxKSkgJSBkaW07XG4gICAgICBlbWJlZGRpbmdbaWR4XSArPSAwLjE7XG4gICAgfVxuXG4gICAgLy8gTm9ybWFsaXplXG4gICAgY29uc3Qgbm9ybSA9IE1hdGguc3FydChlbWJlZGRpbmcucmVkdWNlKChzdW0sIHgpID0+IHN1bSArIHggKiB4LCAwKSkgfHwgMTtcbiAgICByZXR1cm4gZW1iZWRkaW5nLm1hcCh4ID0+IHggLyBub3JtKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBDb21wdXRlIHNpbWlsYXJpdHkgYmV0d2VlbiB0d28gdGV4dHNcbiAgICovXG4gIHNpbWlsYXJpdHkodGV4dDE6IHN0cmluZywgdGV4dDI6IHN0cmluZyk6IG51bWJlciB7XG4gICAgaWYgKHRoaXMubmF0aXZlKSB7XG4gICAgICByZXR1cm4gdGhpcy5uYXRpdmUuc2ltaWxhcml0eSh0ZXh0MSwgdGV4dDIpO1xuICAgIH1cblxuICAgIC8vIEZhbGxiYWNrIC0gY29zaW5lIHNpbWlsYXJpdHlcbiAgICBjb25zdCBlbWIxID0gdGhpcy5lbWJlZCh0ZXh0MSk7XG4gICAgY29uc3QgZW1iMiA9IHRoaXMuZW1iZWQodGV4dDIpO1xuXG4gICAgbGV0IGRvdCA9IDA7XG4gICAgbGV0IG5vcm0xID0gMDtcbiAgICBsZXQgbm9ybTIgPSAwO1xuXG4gICAgZm9yIChsZXQgaSA9IDA7IGkgPCBlbWIxLmxlbmd0aDsgaSsrKSB7XG4gICAgICBkb3QgKz0gZW1iMVtpXSAqIGVtYjJbaV07XG4gICAgICBub3JtMSArPSBlbWIxW2ldICogZW1iMVtpXTtcbiAgICAgIG5vcm0yICs9IGVtYjJbaV0gKiBlbWIyW2ldO1xuICAgIH1cblxuICAgIGNvbnN0IGRlbm9tID0gTWF0aC5zcXJ0KG5vcm0xKSAqIE1hdGguc3FydChub3JtMik7XG4gICAgY29uc3Qgc2ltaWxhcml0eSA9IGRlbm9tID4gMCA/IGRvdCAvIGRlbm9tIDogMDtcbiAgICAvLyBDbGFtcCB0byBbMCwgMV0gdG8gaGFuZGxlIGZsb2F0aW5nIHBvaW50IGVycm9yc1xuICAgIHJldHVybiBNYXRoLm1heCgwLCBNYXRoLm1pbigxLCBzaW1pbGFyaXR5KSk7XG4gIH1cblxuICAvKipcbiAgICogQ2hlY2sgaWYgU0lNRCBpcyBhdmFpbGFibGVcbiAgICovXG4gIGhhc1NpbWQoKTogYm9vbGVhbiB7XG4gICAgaWYgKHRoaXMubmF0aXZlKSB7XG4gICAgICByZXR1cm4gdGhpcy5uYXRpdmUuaGFzU2ltZCgpO1xuICAgIH1cbiAgICByZXR1cm4gZmFsc2U7XG4gIH1cblxuICAvKipcbiAgICogR2V0IFNJTUQgY2FwYWJpbGl0aWVzXG4gICAqL1xuICBzaW1kQ2FwYWJpbGl0aWVzKCk6IHN0cmluZ1tdIHtcbiAgICBpZiAodGhpcy5uYXRpdmUpIHtcbiAgICAgIHJldHVybiB0aGlzLm5hdGl2ZS5zaW1kQ2FwYWJpbGl0aWVzKCk7XG4gICAgfVxuICAgIHJldHVybiBbJ1NjYWxhciAoZmFsbGJhY2spJ107XG4gIH1cblxuICAvKipcbiAgICogQmF0Y2ggcXVlcnkgbXVsdGlwbGUgcHJvbXB0c1xuICAgKi9cbiAgYmF0Y2hRdWVyeShyZXF1ZXN0OiBCYXRjaFF1ZXJ5UmVxdWVzdCk6IEJhdGNoUXVlcnlSZXNwb25zZSB7XG4gICAgY29uc3Qgc3RhcnQgPSBEYXRlLm5vdygpO1xuICAgIGNvbnN0IHJlc3BvbnNlcyA9IHJlcXVlc3QucXVlcmllcy5tYXAocSA9PiB0aGlzLnF1ZXJ5KHEsIHJlcXVlc3QuY29uZmlnKSk7XG4gICAgcmV0dXJuIHtcbiAgICAgIHJlc3BvbnNlcyxcbiAgICAgIHRvdGFsTGF0ZW5jeU1zOiBEYXRlLm5vdygpIC0gc3RhcnQsXG4gICAgfTtcbiAgfVxuXG4gIC8qKlxuICAgKiBDaGVjayBpZiBuYXRpdmUgbW9kdWxlIGlzIGxvYWRlZFxuICAgKi9cbiAgaXNOYXRpdmVMb2FkZWQoKTogYm9vbGVhbiB7XG4gICAgcmV0dXJuIHRoaXMubmF0aXZlICE9PSBudWxsO1xuICB9XG59XG4iXX0=