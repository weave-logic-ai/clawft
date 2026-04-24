/**
 * Compression Middleware for Proxy Responses
 * Provides 30-70% bandwidth reduction with Brotli/Gzip
 */
export type CompressionEncoding = 'br' | 'gzip' | 'identity';
export interface CompressionConfig {
    minSize: number;
    level?: number;
    preferredEncoding: CompressionEncoding;
    enableBrotli: boolean;
    enableGzip: boolean;
}
export interface CompressionResult {
    compressed: Buffer;
    encoding: CompressionEncoding;
    originalSize: number;
    compressedSize: number;
    ratio: number;
    duration: number;
}
export declare class CompressionMiddleware {
    private config;
    constructor(config?: Partial<CompressionConfig>);
    /**
     * Compress data using best available algorithm
     */
    compress(data: Buffer, acceptedEncodings?: string): Promise<CompressionResult>;
    /**
     * Compress using Brotli (best compression ratio)
     */
    private compressBrotli;
    /**
     * Compress using Gzip (faster, broader support)
     */
    private compressGzip;
    /**
     * Select best encoding based on client support
     */
    private selectEncoding;
    /**
     * Check if compression is recommended for content type
     */
    shouldCompress(contentType?: string): boolean;
    /**
     * Get compression statistics
     */
    getStats(): {
        config: Required<CompressionConfig>;
        capabilities: {
            brotli: boolean;
            gzip: boolean;
        };
    };
}
//# sourceMappingURL=compression-middleware.d.ts.map