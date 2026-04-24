"use strict";
/**
 * Streaming response support for RuvLLM
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.StreamingGenerator = void 0;
exports.createReadableStream = createReadableStream;
/**
 * Async generator for streaming responses
 *
 * @example
 * ```typescript
 * import { RuvLLM, StreamingGenerator } from '@ruvector/ruvllm';
 *
 * const llm = new RuvLLM();
 * const streamer = new StreamingGenerator(llm);
 *
 * // Stream with async iterator
 * for await (const chunk of streamer.stream('Write a story')) {
 *   process.stdout.write(chunk.text);
 * }
 *
 * // Stream with callbacks
 * await streamer.streamWithCallbacks('Write a poem', {
 *   onChunk: (chunk) => console.log(chunk.text),
 *   onComplete: (response) => console.log('Done!', response.latencyMs),
 * });
 * ```
 */
class StreamingGenerator {
    constructor(llm) {
        this.llm = llm;
    }
    /**
     * Stream response as async generator
     *
     * Note: This simulates streaming by chunking the full response.
     * Native streaming requires native module support.
     */
    async *stream(prompt, config) {
        const start = Date.now();
        // Generate full response (native streaming would yield real chunks)
        const fullText = this.llm.generate(prompt, config);
        // Simulate streaming by yielding words
        const words = fullText.split(/(\s+)/);
        let accumulated = '';
        let tokenCount = 0;
        for (let i = 0; i < words.length; i++) {
            accumulated += words[i];
            tokenCount++;
            // Yield every few tokens or at end
            if (tokenCount % 3 === 0 || i === words.length - 1) {
                yield {
                    text: words.slice(Math.max(0, i - 2), i + 1).join(''),
                    done: i === words.length - 1,
                    tokenCount,
                    latencyMs: Date.now() - start,
                };
                // Small delay to simulate streaming
                await this.delay(10);
            }
        }
    }
    /**
     * Stream with callback handlers
     */
    async streamWithCallbacks(prompt, options) {
        const start = Date.now();
        let fullText = '';
        let tokenCount = 0;
        try {
            for await (const chunk of this.stream(prompt, options)) {
                fullText += chunk.text;
                tokenCount = chunk.tokenCount;
                if (options.onChunk) {
                    options.onChunk(chunk);
                }
            }
            const response = {
                text: fullText.trim(),
                confidence: 0.8,
                model: 'streaming',
                contextSize: tokenCount,
                latencyMs: Date.now() - start,
                requestId: `stream-${Date.now()}-${Math.random().toString(36).slice(2)}`,
            };
            if (options.onComplete) {
                options.onComplete(response);
            }
            return response;
        }
        catch (error) {
            if (options.onError) {
                options.onError(error);
            }
            throw error;
        }
    }
    /**
     * Collect stream into single response
     */
    async collect(prompt, config) {
        let result = '';
        for await (const chunk of this.stream(prompt, config)) {
            result = chunk.text; // Each chunk is cumulative
        }
        return result.trim();
    }
    delay(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }
}
exports.StreamingGenerator = StreamingGenerator;
/**
 * Create a readable stream from response
 * (For Node.js stream compatibility)
 */
function createReadableStream(generator) {
    return new ReadableStream({
        async pull(controller) {
            const { value, done } = await generator.next();
            if (done) {
                controller.close();
            }
            else {
                controller.enqueue(value.text);
            }
        },
    });
}
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoic3RyZWFtaW5nLmpzIiwic291cmNlUm9vdCI6IiIsInNvdXJjZXMiOlsiLi4vLi4vc3JjL3N0cmVhbWluZy50cyJdLCJuYW1lcyI6W10sIm1hcHBpbmdzIjoiO0FBQUE7O0dBRUc7OztBQWtKSCxvREFhQztBQXRKRDs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7O0dBcUJHO0FBQ0gsTUFBYSxrQkFBa0I7SUFNN0IsWUFBWSxHQUdYO1FBQ0MsSUFBSSxDQUFDLEdBQUcsR0FBRyxHQUFHLENBQUM7SUFDakIsQ0FBQztJQUVEOzs7OztPQUtHO0lBQ0gsS0FBSyxDQUFDLENBQUMsTUFBTSxDQUNYLE1BQWMsRUFDZCxNQUF5QjtRQUV6QixNQUFNLEtBQUssR0FBRyxJQUFJLENBQUMsR0FBRyxFQUFFLENBQUM7UUFFekIsb0VBQW9FO1FBQ3BFLE1BQU0sUUFBUSxHQUFHLElBQUksQ0FBQyxHQUFHLENBQUMsUUFBUSxDQUFDLE1BQU0sRUFBRSxNQUFNLENBQUMsQ0FBQztRQUVuRCx1Q0FBdUM7UUFDdkMsTUFBTSxLQUFLLEdBQUcsUUFBUSxDQUFDLEtBQUssQ0FBQyxPQUFPLENBQUMsQ0FBQztRQUN0QyxJQUFJLFdBQVcsR0FBRyxFQUFFLENBQUM7UUFDckIsSUFBSSxVQUFVLEdBQUcsQ0FBQyxDQUFDO1FBRW5CLEtBQUssSUFBSSxDQUFDLEdBQUcsQ0FBQyxFQUFFLENBQUMsR0FBRyxLQUFLLENBQUMsTUFBTSxFQUFFLENBQUMsRUFBRSxFQUFFLENBQUM7WUFDdEMsV0FBVyxJQUFJLEtBQUssQ0FBQyxDQUFDLENBQUMsQ0FBQztZQUN4QixVQUFVLEVBQUUsQ0FBQztZQUViLG1DQUFtQztZQUNuQyxJQUFJLFVBQVUsR0FBRyxDQUFDLEtBQUssQ0FBQyxJQUFJLENBQUMsS0FBSyxLQUFLLENBQUMsTUFBTSxHQUFHLENBQUMsRUFBRSxDQUFDO2dCQUNuRCxNQUFNO29CQUNKLElBQUksRUFBRSxLQUFLLENBQUMsS0FBSyxDQUFDLElBQUksQ0FBQyxHQUFHLENBQUMsQ0FBQyxFQUFFLENB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QVU7WUFDbkIsTUFBTSxFQUFFLEtBQUssRUFBRSxJQUFJLEVBQUUsR0FBRyxNQUFNLFNBQVMsQ0FBQyxJQUFJLEVBQUUsQ0FBQztZQUMvQyxJQUFJLElBQUksRUFBRSxDQUFDO2dCQUNULFVBQVUsQ0FBQyxLQUFLLEVBQUUsQ0FBQztZQUNyQixDQUFDO2lCQUFNLENBQUM7Z0JBQ04sVUFBVSxDQUFDLE9BQU8sQ0FBQyxLQUFLLENBQUMsSUFBSSxDQUFDLENBQUM7WUFDakMsQ0FBQztRQUNILENBQUM7S0FDRixDQUFDLENBQUM7QUFDTCxDQUFDIiwic291cmNlc0NvbnRlbnQiOlsiLyoqXG4gKiBTdHJlYW1pbmcgcmVzcG9uc2Ugc3VwcG9ydCBmb3IgUnV2TExNXG4gKi9cblxuaW1wb3J0IHtcbiAgU3RyZWFtQ2h1bmssXG4gIFN0cmVhbU9wdGlvbnMsXG4gIFF1ZXJ5UmVzcG9uc2UsXG4gIEdlbmVyYXRpb25Db25maWcsXG59IGZyb20gJy4vdHlwZXMnO1xuXG4vKipcbiAqIEFzeW5jIGdlbmVyYXRvciBmb3Igc3RyZWFtaW5nIHJlc3BvbnNlc1xuICpcbiAqIEBleGFtcGxlXG4gKiBgYGB0eXBlc2NyaXB0XG4gKiBpbXBvcnQgeyBSdXZMTE0sIFN0cmVhbWluZ0dlbmVyYXRvciB9IGZyb20gJ0BydXZlY3Rvci9ydXZsbG0nO1xuICpcbiAqIGNvbnN0IGxsbSA9IG5ldyBSdXZMTE0oKTtcbiAqIGNvbnN0IHN0cmVhbWVyID0gbmV3IFN0cmVhbWluZ0dlbmVyYXRvcihsbG0pO1xuICpcbiAqIC8vIFN0cmVhbSB3aXRoIGFzeW5jIGl0ZXJhdG9yXG4gKiBmb3IgYXdhaXQgKGNvbnN0IGNodW5rIG9mIHN0cmVhbWVyLnN0cmVhbSgnV3JpdGUgYSBzdG9yeScpKSB7XG4gKiAgIHByb2Nlc3Muc3Rkb3V0LndyaXRlKGNodW5rLnRleHQpO1xuICogfVxuICpcbiAqIC8vIFN0cmVhbSB3aXRoIGNhbGxiYWNrc1xuICogYXdhaXQgc3RyZWFtZXIuc3RyZWFtV2l0aENhbGxiYWNrcygnV3JpdGUgYSBwb2VtJywge1xuICogICBvbkNodW5rOiAoY2h1bmspID0+IGNvbnNvbGUubG9nKGNodW5rLnRleHQpLFxuICogICBvbkNvbXBsZXRlOiAocmVzcG9uc2UpID0+IGNvbnNvbGUubG9nKCdEb25lIScsIHJlc3BvbnNlLmxhdGVuY3lNcyksXG4gKiB9KTtcbiAqIGBgYFxuICovXG5leHBvcnQgY2xhc3MgU3RyZWFtaW5nR2VuZXJhdG9yIHtcbiAgcHJpdmF0ZSBsbG06IHtcbiAgICBnZW5lcmF0ZTogKHByb21wdDogc3RyaW5nLCBjb25maWc/OiBHZW5lcmF0aW9uQ29uZmlnKSA9PiBzdHJpbmc7XG4gICAgcXVlcnk6ICh0ZXh0OiBzdHJpbmcsIGNvbmZpZz86IEdlbmVyYXRpb25Db25maWcpID0+IFF1ZXJ5UmVzcG9uc2U7XG4gIH07XG5cbiAgY29uc3RydWN0b3IobGxtOiB7XG4gICAgZ2VuZXJhdGU6IChwcm9tcHQ6IHN0cmluZywgY29uZmlnPzogR2VuZXJhdGlvbkNvbmZpZykgPT4gc3RyaW5nO1xuICAgIHF1ZXJ5OiAodGV4dDogc3RyaW5nLCBjb25maWc/OiBHZW5lcmF0aW9uQ29uZmlnKSA9PiBRdWVyeVJlc3BvbnNlO1xuICB9KSB7XG4gICAgdGhpcy5sbG0gPSBsbG07XG4gIH1cblxuICAvKipcbiAgICogU3RyZWFtIHJlc3BvbnNlIGFzIGFzeW5jIGdlbmVyYXRvclxuICAgKlxuICAgKiBOb3RlOiBUaGlzIHNpbXVsYXRlcyBzdHJlYW1pbmcgYnkgY2h1bmtpbmcgdGhlIGZ1bGwgcmVzcG9uc2UuXG4gICAqIE5hdGl2ZSBzdHJlYW1pbmcgcmVxdWlyZXMgbmF0aXZlIG1vZHVsZSBzdXBwb3J0LlxuICAgKi9cbiAgYXN5bmMgKnN0cmVhbShcbiAgICBwcm9tcHQ6IHN0cmluZyxcbiAgICBjb25maWc/OiBHZW5lcmF0aW9uQ29uZmlnXG4gICk6IEFzeW5jR2VuZXJhdG9yPFN0cmVhbUNodW5rPiB7XG4gICAgY29uc3Qgc3RhcnQgPSBEYXRlLm5vdygpO1xuXG4gICAgLy8gR2VuZXJhdGUgZnVsbCByZXNwb25zZSAobmF0aXZlIHN0cmVhbWluZyB3b3VsZCB5aWVsZCByZWFsIGNodW5rcylcbiAgICBjb25zdCBmdWxsVGV4dCA9IHRoaXMubGxtLmdlbmVyYXRlKHByb21wdCwgY29uZmlnKTtcblxuICAgIC8vIFNpbXVsYXRlIHN0cmVhbWluZyBieSB5aWVsZGluZyB3b3Jkc1xuICAgIGNvbnN0IHdvcmRzID0gZnVsbFRleHQuc3BsaXQoLyhcXHMrKS8pO1xuICAgIGxldCBhY2N1bXVsYXRlZCA9ICcnO1xuICAgIGxldCB0b2tlbkNvdW50ID0gMDtcblxuICAgIGZvciAobGV0IGkgPSAwOyBpIDwgd29yZHMubGVuZ3RoOyBpKyspIHtcbiAgICAgIGFjY3VtdWxhdGVkICs9IHdvcmRzW2ldO1xuICAgICAgdG9rZW5Db3VudCsrO1xuXG4gICAgICAvLyBZaWVsZCBldmVyeSBmZXcgdG9rZW5zIG9yIGF0IGVuZFxuICAgICAgaWYgKHRva2VuQ291bnQgJSAzID09PSAwIHx8IGkgPT09IHdvcmRzLmxlbmd0aCAtIDEpIHtcbiAgICAgICAgeWllbGQge1xuICAgICAgICAgIHRleHQ6IHdvcmRzLnNsaWNlKE1hdGgubWF4KDAsIGkgLSAyKSwgaSArIDEpLmpvaW4oJycpLFxuICAgICAgICAgIGRvbmU6IGkgPT09IHdvcmRzLmxlbmd0aCAtIDEsXG4gICAgICAgICAgdG9rZW5Db3VudCxcbiAgICAgICAgICBsYXRlbmN5TXM6IERhdGUubm93KCkgLSBzdGFydCxcbiAgICAgICAgfTtcblxuICAgICAgICAvLyBTbWFsbCBkZWxheSB0byBzaW11bGF0ZSBzdHJlYW1pbmdcbiAgICAgICAgYXdhaXQgdGhpcy5kZWxheSgxMCk7XG4gICAgICB9XG4gICAgfVxuICB9XG5cbiAgLyoqXG4gICAqIFN0cmVhbSB3aXRoIGNhbGxiYWNrIGhhbmRsZXJzXG4gICAqL1xuICBhc3luYyBzdHJlYW1XaXRoQ2FsbGJhY2tzKFxuICAgIHByb21wdDogc3RyaW5nLFxuICAgIG9wdGlvbnM6IFN0cmVhbU9wdGlvbnNcbiAgKTogUHJvbWlzZTxRdWVyeVJlc3BvbnNlPiB7XG4gICAgY29uc3Qgc3RhcnQgPSBEYXRlLm5vdygpO1xuICAgIGxldCBmdWxsVGV4dCA9ICcnO1xuICAgIGxldCB0b2tlbkNvdW50ID0gMDtcblxuICAgIHRyeSB7XG4gICAgICBmb3IgYXdhaXQgKGNvbnN0IGNodW5rIG9mIHRoaXMuc3RyZWFtKHByb21wdCwgb3B0aW9ucykpIHtcbiAgICAgICAgZnVsbFRleHQgKz0gY2h1bmsudGV4dDtcbiAgICAgICAgdG9rZW5Db3VudCA9IGNodW5rLnRva2VuQ291bnQ7XG5cbiAgICAgICAgaWYgKG9wdGlvbnMub25DaHVuaykge1xuICAgICAgICAgIG9wdGlvbnMub25DaHVuayhjaHVuayk7XG4gICAgICAgIH1cbiAgICAgIH1cblxuICAgICAgY29uc3QgcmVzcG9uc2U6IFF1ZXJ5UmVzcG9uc2UgPSB7XG4gICAgICAgIHRleHQ6IGZ1bGxUZXh0LnRyaW0oKSxcbiAgICAgICAgY29uZmlkZW5jZTogMC44LFxuICAgICAgICBtb2RlbDogJ3N0cmVhbWluZycsXG4gICAgICAgIGNvbnRleHRTaXplOiB0b2tlbkNvdW50LFxuICAgICAgICBsYXRlbmN5TXM6IERhdGUubm93KCkgLSBzdGFydCxcbiAgICAgICAgcmVxdWVzdElkOiBgc3RyZWFtLSR7RGF0ZS5ub3coKX0tJHtNYXRoLnJhbmRvbSgpLnRvU3RyaW5nKDM2KS5zbGljZSgyKX1gLFxuICAgICAgfTtcblxuICAgICAgaWYgKG9wdGlvbnMub25Db21wbGV0ZSkge1xuICAgICAgICBvcHRpb25zLm9uQ29tcGxldGUocmVzcG9uc2UpO1xuICAgICAgfVxuXG4gICAgICByZXR1cm4gcmVzcG9uc2U7XG4gICAgfSBjYXRjaCAoZXJyb3IpIHtcbiAgICAgIGlmIChvcHRpb25zLm9uRXJyb3IpIHtcbiAgICAgICAgb3B0aW9ucy5vbkVycm9yKGVycm9yIGFzIEVycm9yKTtcbiAgICAgIH1cbiAgICAgIHRocm93IGVycm9yO1xuICAgIH1cbiAgfVxuXG4gIC8qKlxuICAgKiBDb2xsZWN0IHN0cmVhbSBpbnRvIHNpbmdsZSByZXNwb25zZVxuICAgKi9cbiAgYXN5bmMgY29sbGVjdChwcm9tcHQ6IHN0cmluZywgY29uZmlnPzogR2VuZXJhdGlvbkNvbmZpZyk6IFByb21pc2U8c3RyaW5nPiB7XG4gICAgbGV0IHJlc3VsdCA9ICcnO1xuICAgIGZvciBhd2FpdCAoY29uc3QgY2h1bmsgb2YgdGhpcy5zdHJlYW0ocHJvbXB0LCBjb25maWcpKSB7XG4gICAgICByZXN1bHQgPSBjaHVuay50ZXh0OyAvLyBFYWNoIGNodW5rIGlzIGN1bXVsYXRpdmVcbiAgICB9XG4gICAgcmV0dXJuIHJlc3VsdC50cmltKCk7XG4gIH1cblxuICBwcml2YXRlIGRlbGF5KG1zOiBudW1iZXIpOiBQcm9taXNlPHZvaWQ+IHtcbiAgICByZXR1cm4gbmV3IFByb21pc2UocmVzb2x2ZSA9PiBzZXRUaW1lb3V0KHJlc29sdmUsIG1zKSk7XG4gIH1cbn1cblxuLyoqXG4gKiBDcmVhdGUgYSByZWFkYWJsZSBzdHJlYW0gZnJvbSByZXNwb25zZVxuICogKEZvciBOb2RlLmpzIHN0cmVhbSBjb21wYXRpYmlsaXR5KVxuICovXG5leHBvcnQgZnVuY3Rpb24gY3JlYXRlUmVhZGFibGVTdHJlYW0oXG4gIGdlbmVyYXRvcjogQXN5bmNHZW5lcmF0b3I8U3RyZWFtQ2h1bms+XG4pOiBSZWFkYWJsZVN0cmVhbTxzdHJpbmc+IHtcbiAgcmV0dXJuIG5ldyBSZWFkYWJsZVN0cmVhbSh7XG4gICAgYXN5bmMgcHVsbChjb250cm9sbGVyKSB7XG4gICAgICBjb25zdCB7IHZhbHVlLCBkb25lIH0gPSBhd2FpdCBnZW5lcmF0b3IubmV4dCgpO1xuICAgICAgaWYgKGRvbmUpIHtcbiAgICAgICAgY29udHJvbGxlci5jbG9zZSgpO1xuICAgICAgfSBlbHNlIHtcbiAgICAgICAgY29udHJvbGxlci5lbnF1ZXVlKHZhbHVlLnRleHQpO1xuICAgICAgfVxuICAgIH0sXG4gIH0pO1xufVxuIl19