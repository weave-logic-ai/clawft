/**
 * Streaming response support for RuvLLM
 */
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
export class StreamingGenerator {
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
/**
 * Create a readable stream from response
 * (For Node.js stream compatibility)
 */
export function createReadableStream(generator) {
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
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoic3RyZWFtaW5nLmpzIiwic291cmNlUm9vdCI6IiIsInNvdXJjZXMiOlsiLi4vLi4vc3JjL3N0cmVhbWluZy50cyJdLCJuYW1lcyI6W10sIm1hcHBpbmdzIjoiQUFBQTs7R0FFRztBQVNIOzs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7R0FxQkc7QUFDSCxNQUFNLE9BQU8sa0JBQWtCO0lBTTdCLFlBQVksR0FHWDtRQUNDLElBQUksQ0FBQyxHQUFHLEdBQUcsR0FBRyxDQUFDO0lBQ2pCLENBQUM7SUFFRDs7Ozs7T0FLRztJQUNILEtBQUssQ0FBQyxDQUFDLE1BQU0sQ0FDWCxNQUFjLEVBQ2QsTUFBeUI7UUFFekIsTUFBTSxLQUFLLEdBQUcsSUFBSSxDQUFDLEdBQUcsRUFBRSxDQUFDO1FBRXpCLG9FQUFvRTtRQUNwRSxNQUFNLFFBQVEsR0FBRyxJQUFJLENBQUMsR0FBRyxDQUFDLFFBQVEsQ0FBQyxNQUFNLEVBQUUsTUFBTSxDQUFDLENBQUM7UUFFbkQsdUNBQXVDO1FBQ3ZDLE1BQU0sS0FBSyxHQUFHLFFBQVEsQ0FBQyxLQUFLLENBQUMsT0FBTyxDQUFDLENBQUM7UUFDdEMsSUFBSSxXQUFXLEdBQUcsRUFBRSxDQUFDO1FBQ3JCLElBQUksVUFBVSxHQUFHLENBQUMsQ0FBQztRQUVuQixLQUFLLElBQUksQ0FBQyxHQUFHLENBQUMsRUFBRSxDQUFDLEdBQUcsS0FBSyxDQUFDLE1BQU0sRUFBRSxDQUFDLEVBQUUsRUFBRSxDQUFDO1lBQ3RDLFdBQVcsSUFBSSxLQUFLLENBQUMsQ0FBQyxDQUFDLENBQUM7WUFDeEIsVUFBVSxFQUFFLENBQUM7WUFFYixtQ0FBbUM7WUFDbkMsSUFBSSxVQUFVLEdBQUcsQ0FBQyxLQUFLLENBQUMsSUFBSSxDQUFDLEtBQUssS0FBSyxDQUFDLE1BQU0sR0FBRyxDQUFDLEVBQUUsQ0FBQztnQkFDbkQsTUFBTTtvQkFDSixJQUFJLEVBQUUsS0FBSyxDQUFDLEtBQUssQ0FBQyxJQUFJLENBQUMsR0FBRyxDQUFDLENBQUMsRUFBRSxDQUFDLEdBQUcsQ0FBQyxDQUFDLEVBQUUsQ0FBQyxHQUFHLENBQUMsQ0FBQyxDQUFDLElBQUksQ0FBQyxFQUFFLENBQUM7b0JBQ3JELElBQUksRUFBRSxDQUFDLEtBQUssS0FBSyxDQUFDLE1BQU0sR0FBRyxDQUFDO29CQUM1QixVQUFVO29CQUNWLFNBQVMsRUFBRSxJQUFJLENBQUMsR0FBRyxFQUFFLEdBQUcsS0FBSztpQkFDOUIsQ0FBQztnQkFFRixvQ0FBb0M7Z0JBQ3BDLE1BQU0sSUFBSSxDQUFDLEtBQUssQ0FBQyxFQUFFLENBQUMsQ0FBQztZQUN2QixDQUFDO1FBQ0gsQ0FBQztJQUNILENBQUM7SUFFRDs7T0FFRztJQUNILEtBQUssQ0FBQyxtQkFBbUIsQ0FDdkIsTUFBYyxFQUNkLE9BQXNCO1FBRXRCLE1BQU0sS0FBSyxHQUFHLElBQUksQ0FBQyxHQUFHLEVBQUUsQ0FBQztRQUN6QixJQUFJLFFBQVEsR0FBRyxFQUFFLENBQUM7UUFDbEIsSUFBSSxVQUFVLEdBQUcsQ0FBQyxDQUFDO1FBRW5CLElBQUksQ0FBQztZQUNILElBQUksS0FBSyxFQUFFLE1BQU0sS0FBSyxJQUFJLElBQUksQ0FBQyxNQUFNLENBQUMsTUFBTSxFQUFFLE9BQU8sQ0FBQyxFQUFFLENBQUM7Z0JBQ3ZELFFBQVEsSUFBSSxLQUFLLENBQUMsSUFBSSxDQUFDO2dCQUN2QixVQUFVLEdBQUcsS0FBSyxDQUFDLFVBQVUsQ0FBQztnQkFFOUIsSUFBSSxPQUFPLENBQUMsT0FBTyxFQUFFLENBQUM7b0JBQ3BCLE9BQU8sQ0FBQyxPQUFPLENBQUMsS0FBSyxDQUFDLENBQUM7Z0JBQ3pCLENBQUM7WUFDSCxDQUFDO1lBRUQsTUFBTSxRQUFRLEdBQWtCO2dCQUM5QixJQUFJLEVBQUUsUUFBUSxDQUFDLElBQUksRUFBRTtnQkFDckIsVUFBVSxFQUFFLEdBQUc7Z0JBQ2YsS0FBSyxFQUFFLFdBQVc7Z0JBQ2xCLFdBQVcsRUFBRSxVQUFVO2dCQUN2QixTQUFTLEVBQUUsSUFBSSxDQUFDLEdBQUcsRUFBRSxHQUFHLEtBQUs7Z0JBQzdCLFNBQVMsRUFBRSxVQUFVLElBQUksQ0FBQyxHQUFHLEVBQUUsSUFBSSxJQUFJLENBQUMsTUFBTSxFQUFFLENBQUMsUUFBUSxDQUFDLEVBQUUsQ0FBQyxDQUFDLEtBQUssQ0FBQyxDQUFDLENBQUMsRUFBRTthQUN6RSxDQUFDO1lBRUYsSUFBSSxPQUFPLENBQUMsVUFBVSxFQUFFLENBQUM7Z0JBQ3ZCLE9BQU8sQ0FBQyxVQUFVLENBQUMsUUFBUSxDQUFDLENBQUM7WUFDL0IsQ0FBQztZQUVELE9BQU8sUUFBUSxDQUFDO1FBQ2xCLENBQUM7UUFBQyxPQUFPLEtBQUssRUFBRSxDQUFDO1lBQ2YsSUFBSSxPQUFPLENBQUMsT0FBTyxFQUFFLENBQUM7Z0JBQ3BCLE9BQU8sQ0FBQyxPQUFPLENBQUMsS0FBYyxDQUFDLENBQUM7WUFDbEMsQ0FBQztZQUNELE1BQU0sS0FBSyxDQUFDO1FBQ2QsQ0FBQztJQUNILENBQUM7SUFFRDs7T0FFRztJQUNILEtBQUssQ0FBQyxPQUFPLENBQUMsTUFBYyxFQUFFLE1BQXlCO1FBQ3JELElBQUksTUFBTSxHQUFHLEVBQUUsQ0FBQztRQUNoQixJQUFJLEtBQUssRUFBRSxNQUFNLEtBQUssSUFBSSxJQUFJLENBQUMsTUFBTSxDQUFDLE1BQU0sRUFBRSxNQUFNLENBQUMsRUFBRSxDQUFDO1lBQ3RELE1BQU0sR0FBRyxLQUFLLENBQUMsSUFBSSxDQUFDLENBQUMsMkJBQTJCO1FBQ2xELENBQUM7UUFDRCxPQUFPLE1BQU0sQ0FBQyxJQUFJLEVBQUUsQ0FBQztJQUN2QixDQUFDO0lBRU8sS0FBSyxDQUFDLEVBQVU7UUFDdEIsT0FBTyxJQUFJLE9BQU8sQ0FBQyxPQUFPLENBQUMsRUFBRSxDQUFDLFVBQVUsQ0FBQyxPQUFPLEVBQUUsRUFBRSxDQUFDLENBQUMsQ0FBQztJQUN6RCxDQUFDO0NBQ0Y7QUFFRDs7O0dBR0c7QUFDSCxNQUFNLFVBQVUsb0JBQW9CLENBQ2xDLFNBQXNDO0lBRXRDLE9BQU8sSUFBSSxjQUFjLENBQUM7UUFDeEIsS0FBSyxDQUFDLElBQUksQ0FBQyxVQUFVO1lBQ25CLE1BQU0sRUFBRSxLQUFLLEVBQUUsSUFBSSxFQUFFLEdBQUcsTUFBTSxTQUFTLENBQUMsSUFBSSxFQUFFLENBQUM7WUFDL0MsSUFBSSxJQUFJLEVBQUUsQ0FBQztnQkFDVCxVQUFVLENBQUMsS0FBSyxFQUFFLENBQUM7WUFDckIsQ0FBQztpQkFBTSxDQUFDO2dCQUNOLFVBQVUsQ0FBQyxPQUFPLENBQUMsS0FBSyxDQUFDLElBQUksQ0FBQyxDQUFDO1lBQ2pDLENBQUM7UUFDSCxDQUFDO0tBQ0YsQ0FBQyxDQUFDO0FBQ0wsQ0FBQyIsInNvdXJjZXNDb250ZW50IjpbIi8qKlxuICogU3RyZWFtaW5nIHJlc3BvbnNlIHN1cHBvcnQgZm9yIFJ1dkxMTVxuICovXG5cbmltcG9ydCB7XG4gIFN0cmVhbUNodW5rLFxuICBTdHJlYW1PcHRpb25zLFxuICBRdWVyeVJlc3BvbnNlLFxuICBHZW5lcmF0aW9uQ29uZmlnLFxufSBmcm9tICcuL3R5cGVzJztcblxuLyoqXG4gKiBBc3luYyBnZW5lcmF0b3IgZm9yIHN0cmVhbWluZyByZXNwb25zZXNcbiAqXG4gKiBAZXhhbXBsZVxuICogYGBgdHlwZXNjcmlwdFxuICogaW1wb3J0IHsgUnV2TExNLCBTdHJlYW1pbmdHZW5lcmF0b3IgfSBmcm9tICdAcnV2ZWN0b3IvcnV2bGxtJztcbiAqXG4gKiBjb25zdCBsbG0gPSBuZXcgUnV2TExNKCk7XG4gKiBjb25zdCBzdHJlYW1lciA9IG5ldyBTdHJlYW1pbmdHZW5lcmF0b3IobGxtKTtcbiAqXG4gKiAvLyBTdHJlYW0gd2l0aCBhc3luYyBpdGVyYXRvclxuICogZm9yIGF3YWl0IChjb25zdCBjaHVuayBvZiBzdHJlYW1lci5zdHJlYW0oJ1dyaXRlIGEgc3RvcnknKSkge1xuICogICBwcm9jZXNzLnN0ZG91dC53cml0ZShjaHVuay50ZXh0KTtcbiAqIH1cbiAqXG4gKiAvLyBTdHJlYW0gd2l0aCBjYWxsYmFja3NcbiAqIGF3YWl0IHN0cmVhbWVyLnN0cmVhbVdpdGhDYWxsYmFja3MoJ1dyaXRlIGEgcG9lbScsIHtcbiAqICAgb25DaHVuazogKGNodW5rKSA9PiBjb25zb2xlLmxvZyhjaHVuay50ZXh0KSxcbiAqICAgb25Db21wbGV0ZTogKHJlc3BvbnNlKSA9PiBjb25zb2xlLmxvZygnRG9uZSEnLCByZXNwb25zZS5sYXRlbmN5TXMpLFxuICogfSk7XG4gKiBgYGBcbiAqL1xuZXhwb3J0IGNsYXNzIFN0cmVhbWluZ0dlbmVyYXRvciB7XG4gIHByaXZhdGUgbGxtOiB7XG4gICAgZ2VuZXJhdGU6IChwcm9tcHQ6IHN0cmluZywgY29uZmlnPzogR2VuZXJhdGlvbkNvbmZpZykgPT4gc3RyaW5nO1xuICAgIHF1ZXJ5OiAodGV4dDogc3RyaW5nLCBjb25maWc/OiBHZW5lcmF0aW9uQ29uZmlnKSA9PiBRdWVyeVJlc3BvbnNlO1xuICB9O1xuXG4gIGNvbnN0cnVjdG9yKGxsbToge1xuICAgIGdlbmVyYXRlOiAocHJvbXB0OiBzdHJpbmcsIGNvbmZpZz86IEdlbmVyYXRpb25Db25maWcpID0+IHN0cmluZztcbiAgICBxdWVyeTogKHRleHQ6IHN0cmluZywgY29uZmlnPzogR2VuZXJhdGlvbkNvbmZpZykgPT4gUXVlcnlSZXNwb25zZTtcbiAgfSkge1xuICAgIHRoaXMubGxtID0gbGxtO1xuICB9XG5cbiAgLyoqXG4gICAqIFN0cmVhbSByZXNwb25zZSBhcyBhc3luYyBnZW5lcmF0b3JcbiAgICpcbiAgICogTm90ZTogVGhpcyBzaW11bGF0ZXMgc3RyZWFtaW5nIGJ5IGNodW5raW5nIHRoZSBmdWxsIHJlc3BvbnNlLlxuICAgKiBOYXRpdmUgc3RyZWFtaW5nIHJlcXVpcmVzIG5hdGl2ZSBtb2R1bGUgc3VwcG9ydC5cbiAgICovXG4gIGFzeW5jICpzdHJlYW0oXG4gICAgcHJvbXB0OiBzdHJpbmcsXG4gICAgY29uZmlnPzogR2VuZXJhdGlvbkNvbmZpZ1xuICApOiBBc3luY0dlbmVyYXRvcjxTdHJlYW1DaHVuaz4ge1xuICAgIGNvbnN0IHN0YXJ0ID0gRGF0ZS5ub3coKTtcblxuICAgIC8vIEdlbmVyYXRlIGZ1bGwgcmVzcG9uc2UgKG5hdGl2ZSBzdHJlYW1pbmcgd291bGQgeWllbGQgcmVhbCBjaHVua3MpXG4gICAgY29uc3QgZnVsbFRleHQgPSB0aGlzLmxsbS5nZW5lcmF0ZShwcm9tcHQsIGNvbmZpZyk7XG5cbiAgICAvLyBTaW11bGF0ZSBzdHJlYW1pbmcgYnkgeWllbGRpbmcgd29yZHNcbiAgICBjb25zdCB3b3JkcyA9IGZ1bGxUZXh0LnNwbGl0KC8oXFxzKykvKTtcbiAgICBsZXQgYWNjdW11bGF0ZWQgPSAnJztcbiAgICBsZXQgdG9rZW5Db3VudCA9IDA7XG5cbiAgICBmb3IgKGxldCBpID0gMDsgaSA8IHdvcmRzLmxlbmd0aDsgaSsrKSB7XG4gICAgICBhY2N1bXVsYXRlZCArPSB3b3Jkc1tpXTtcbiAgICAgIHRva2VuQ291bnQrKztcblxuICAgICAgLy8gWWllbGQgZXZlcnkgZmV3IHRva2VucyBvciBhdCBlbmRcbiAgICAgIGlmICh0b2tlbkNvdW50ICUgMyA9PT0gMCB8fCBpID09PSB3b3Jkcy5sZW5ndGggLSAxKSB7XG4gICAgICAgIHlpZWxkIHtcbiAgICAgICAgICB0ZXh0OiB3b3Jkcy5zbGljZShNYXRoLm1heCgwLCBpIC0gMiksIGkgKyAxKS5qb2luKCcnKSxcbiAgICAgICAgICBkb25lOiBpID09PSB3b3Jkcy5sZW5ndGggLSAxLFxuICAgICAgICAgIHRva2VuQ291bnQsXG4gICAgICAgICAgbGF0ZW5jeU1zOiBEYXRlLm5vdygpIC0gc3RhcnQsXG4gICAgICAgIH07XG5cbiAgICAgICAgLy8gU21hbGwgZGVsYXkgdG8gc2ltdWxhdGUgc3RyZWFtaW5nXG4gICAgICAgIGF3YWl0IHRoaXMuZGVsYXkoMTApO1xuICAgICAgfVxuICAgIH1cbiAgfVxuXG4gIC8qKlxuICAgKiBTdHJlYW0gd2l0aCBjYWxsYmFjayBoYW5kbGVyc1xuICAgKi9cbiAgYXN5bmMgc3RyZWFtV2l0aENhbGxiYWNrcyhcbiAgICBwcm9tcHQ6IHN0cmluZyxcbiAgICBvcHRpb25zOiBTdHJlYW1PcHRpb25zXG4gICk6IFByb21pc2U8UXVlcnlSZXNwb25zZT4ge1xuICAgIGNvbnN0IHN0YXJ0ID0gRGF0ZS5ub3coKTtcbiAgICBsZXQgZnVsbFRleHQgPSAnJztcbiAgICBsZXQgdG9rZW5Db3VudCA9IDA7XG5cbiAgICB0cnkge1xuICAgICAgZm9yIGF3YWl0IChjb25zdCBjaHVuayBvZiB0aGlzLnN0cmVhbShwcm9tcHQsIG9wdGlvbnMpKSB7XG4gICAgICAgIGZ1bGxUZXh0ICs9IGNodW5rLnRleHQ7XG4gICAgICAgIHRva2VuQ291bnQgPSBjaHVuay50b2tlbkNvdW50O1xuXG4gICAgICAgIGlmIChvcHRpb25zLm9uQ2h1bmspIHtcbiAgICAgICAgICBvcHRpb25zLm9uQ2h1bmsoY2h1bmspO1xuICAgICAgICB9XG4gICAgICB9XG5cbiAgICAgIGNvbnN0IHJlc3BvbnNlOiBRdWVyeVJlc3BvbnNlID0ge1xuICAgICAgICB0ZXh0OiBmdWxsVGV4dC50cmltKCksXG4gICAgICAgIGNvbmZpZGVuY2U6IDAuOCxcbiAgICAgICAgbW9kZWw6ICdzdHJlYW1pbmcnLFxuICAgICAgICBjb250ZXh0U2l6ZTogdG9rZW5Db3VudCxcbiAgICAgICAgbGF0ZW5jeU1zOiBEYXRlLm5vdygpIC0gc3RhcnQsXG4gICAgICAgIHJlcXVlc3RJZDogYHN0cmVhbS0ke0RhdGUubm93KCl9LSR7TWF0aC5yYW5kb20oKS50b1N0cmluZygzNikuc2xpY2UoMil9YCxcbiAgICAgIH07XG5cbiAgICAgIGlmIChvcHRpb25zLm9uQ29tcGxldGUpIHtcbiAgICAgICAgb3B0aW9ucy5vbkNvbXBsZXRlKHJlc3BvbnNlKTtcbiAgICAgIH1cblxuICAgICAgcmV0dXJuIHJlc3BvbnNlO1xuICAgIH0gY2F0Y2ggKGVycm9yKSB7XG4gICAgICBpZiAob3B0aW9ucy5vbkVycm9yKSB7XG4gICAgICAgIG9wdGlvbnMub25FcnJvcihlcnJvciBhcyBFcnJvcik7XG4gICAgICB9XG4gICAgICB0aHJvdyBlcnJvcjtcbiAgICB9XG4gIH1cblxuICAvKipcbiAgICogQ29sbGVjdCBzdHJlYW0gaW50byBzaW5nbGUgcmVzcG9uc2VcbiAgICovXG4gIGFzeW5jIGNvbGxlY3QocHJvbXB0OiBzdHJpbmcsIGNvbmZpZz86IEdlbmVyYXRpb25Db25maWcpOiBQcm9taXNlPHN0cmluZz4ge1xuICAgIGxldCByZXN1bHQgPSAnJztcbiAgICBmb3IgYXdhaXQgKGNvbnN0IGNodW5rIG9mIHRoaXMuc3RyZWFtKHByb21wdCwgY29uZmlnKSkge1xuICAgICAgcmVzdWx0ID0gY2h1bmsudGV4dDsgLy8gRWFjaCBjaHVuayBpcyBjdW11bGF0aXZlXG4gICAgfVxuICAgIHJldHVybiByZXN1bHQudHJpbSgpO1xuICB9XG5cbiAgcHJpdmF0ZSBkZWxheShtczogbnVtYmVyKTogUHJvbWlzZTx2b2lkPiB7XG4gICAgcmV0dXJuIG5ldyBQcm9taXNlKHJlc29sdmUgPT4gc2V0VGltZW91dChyZXNvbHZlLCBtcykpO1xuICB9XG59XG5cbi8qKlxuICogQ3JlYXRlIGEgcmVhZGFibGUgc3RyZWFtIGZyb20gcmVzcG9uc2VcbiAqIChGb3IgTm9kZS5qcyBzdHJlYW0gY29tcGF0aWJpbGl0eSlcbiAqL1xuZXhwb3J0IGZ1bmN0aW9uIGNyZWF0ZVJlYWRhYmxlU3RyZWFtKFxuICBnZW5lcmF0b3I6IEFzeW5jR2VuZXJhdG9yPFN0cmVhbUNodW5rPlxuKTogUmVhZGFibGVTdHJlYW08c3RyaW5nPiB7XG4gIHJldHVybiBuZXcgUmVhZGFibGVTdHJlYW0oe1xuICAgIGFzeW5jIHB1bGwoY29udHJvbGxlcikge1xuICAgICAgY29uc3QgeyB2YWx1ZSwgZG9uZSB9ID0gYXdhaXQgZ2VuZXJhdG9yLm5leHQoKTtcbiAgICAgIGlmIChkb25lKSB7XG4gICAgICAgIGNvbnRyb2xsZXIuY2xvc2UoKTtcbiAgICAgIH0gZWxzZSB7XG4gICAgICAgIGNvbnRyb2xsZXIuZW5xdWV1ZSh2YWx1ZS50ZXh0KTtcbiAgICAgIH1cbiAgICB9LFxuICB9KTtcbn1cbiJdfQ==