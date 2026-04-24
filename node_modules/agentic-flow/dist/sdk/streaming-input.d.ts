/**
 * Streaming Input Mode - Stream prompts as async iterables
 *
 * Enables streaming prompts to the SDK, allowing:
 * - Multi-turn conversations without separate query calls
 * - Interactive user input during execution
 * - Pipeline-style prompt processing
 */
/**
 * SDK User Message format
 */
export interface SDKUserMessage {
    type: 'user';
    message: {
        content: Array<{
            type: 'text';
            text: string;
        } | {
            type: 'image';
            source: {
                type: 'base64';
                media_type: string;
                data: string;
            };
        }>;
    };
}
/**
 * Streaming prompt source interface
 */
export interface PromptSource {
    next(): Promise<{
        done: boolean;
        value?: SDKUserMessage;
    }>;
    [Symbol.asyncIterator](): AsyncIterator<SDKUserMessage>;
}
/**
 * Create a user text message
 */
export declare function createTextMessage(text: string): SDKUserMessage;
/**
 * Create an image message from base64 data
 */
export declare function createImageMessage(base64Data: string, mediaType?: string): SDKUserMessage;
/**
 * Create a mixed content message (text + images)
 */
export declare function createMixedMessage(parts: Array<{
    type: 'text';
    text: string;
} | {
    type: 'image';
    data: string;
    mediaType?: string;
}>): SDKUserMessage;
/**
 * StreamingPromptBuilder - Build streaming prompts with fluent API
 */
export declare class StreamingPromptBuilder {
    private messages;
    private delayMs;
    /**
     * Add a text message
     */
    text(content: string): this;
    /**
     * Add an image message
     */
    image(base64Data: string, mediaType?: string): this;
    /**
     * Set delay between messages (ms)
     */
    delay(ms: number): this;
    /**
     * Build async iterable for SDK
     */
    build(): AsyncIterable<SDKUserMessage>;
}
/**
 * Create a streaming prompt builder
 */
export declare function streamingPrompt(): StreamingPromptBuilder;
/**
 * InteractivePromptStream - Stream prompts with user input callbacks
 */
export declare class InteractivePromptStream implements AsyncIterable<SDKUserMessage> {
    private queue;
    private waiting;
    private closed;
    /**
     * Push a message to the stream
     */
    push(message: SDKUserMessage): void;
    /**
     * Push a text message
     */
    pushText(text: string): void;
    /**
     * Close the stream
     */
    close(): void;
    /**
     * Check if stream is closed
     */
    isClosed(): boolean;
    /**
     * Get next message
     */
    private next;
    [Symbol.asyncIterator](): AsyncIterator<SDKUserMessage>;
}
/**
 * Create an interactive prompt stream
 */
export declare function createInteractiveStream(): InteractivePromptStream;
/**
 * Pipeline prompts from multiple sources
 */
export declare function pipelinePrompts(...sources: Array<AsyncIterable<SDKUserMessage>>): AsyncIterable<SDKUserMessage>;
/**
 * Create a prompt stream from an array
 */
export declare function fromArray(messages: string[]): AsyncIterable<SDKUserMessage>;
/**
 * Create a prompt stream from readline (stdin)
 */
export declare function fromReadline(readline: any, prompt?: string): InteractivePromptStream;
/**
 * Transform a prompt stream
 */
export declare function transformPrompts(source: AsyncIterable<SDKUserMessage>, transform: (message: SDKUserMessage) => SDKUserMessage | Promise<SDKUserMessage>): AsyncIterable<SDKUserMessage>;
/**
 * Filter a prompt stream
 */
export declare function filterPrompts(source: AsyncIterable<SDKUserMessage>, predicate: (message: SDKUserMessage) => boolean | Promise<boolean>): AsyncIterable<SDKUserMessage>;
/**
 * Rate-limit a prompt stream
 */
export declare function rateLimitPrompts(source: AsyncIterable<SDKUserMessage>, minIntervalMs: number): AsyncIterable<SDKUserMessage>;
/**
 * Batch prompts together
 */
export declare function batchPrompts(source: AsyncIterable<SDKUserMessage>, batchSize: number, separator?: string): AsyncIterable<SDKUserMessage>;
/**
 * Wrap a string or array as streaming input for SDK
 */
export declare function toStreamingInput(input: string | string[] | AsyncIterable<SDKUserMessage>): AsyncIterable<SDKUserMessage>;
/**
 * Log streaming input for debugging
 */
export declare function logStreamingInput(source: AsyncIterable<SDKUserMessage>, label?: string): AsyncIterable<SDKUserMessage>;
//# sourceMappingURL=streaming-input.d.ts.map