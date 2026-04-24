/**
 * Compression Middleware for Proxy Responses
 * Provides 30-70% bandwidth reduction with Brotli/Gzip
 */
import { brotliCompress, gzip, constants } from 'zlib';
import { promisify } from 'util';
import { logger } from './logger.js';
const brotliCompressAsync = promisify(brotliCompress);
const gzipAsync = promisify(gzip);
export class CompressionMiddleware {
    config;
    constructor(config = {}) {
        this.config = {
            minSize: config.minSize || 1024, // 1KB minimum
            level: config.level ?? constants.BROTLI_DEFAULT_QUALITY,
            preferredEncoding: config.preferredEncoding || 'br',
            enableBrotli: config.enableBrotli ?? true,
            enableGzip: config.enableGzip ?? true
        };
    }
    /**
     * Compress data using best available algorithm
     */
    async compress(data, acceptedEncodings) {
        const startTime = Date.now();
        const originalSize = data.length;
        // Skip compression for small payloads
        if (originalSize < this.config.minSize) {
            return {
                compressed: data,
                encoding: 'identity',
                originalSize,
                compressedSize: originalSize,
                ratio: 1.0,
                duration: 0
            };
        }
        // Determine encoding based on accept-encoding header
        const encoding = this.selectEncoding(acceptedEncodings);
        let compressed;
        try {
            switch (encoding) {
                case 'br':
                    compressed = await this.compressBrotli(data);
                    break;
                case 'gzip':
                    compressed = await this.compressGzip(data);
                    break;
                default:
                    compressed = data;
            }
        }
        catch (error) {
            logger.error('Compression failed, using uncompressed', {
                error: error.message
            });
            compressed = data;
        }
        const duration = Date.now() - startTime;
        const compressedSize = compressed.length;
        const ratio = compressedSize / originalSize;
        logger.debug('Compression complete', {
            encoding,
            originalSize,
            compressedSize,
            ratio: `${(ratio * 100).toFixed(2)}%`,
            savings: `${((1 - ratio) * 100).toFixed(2)}%`,
            duration
        });
        return {
            compressed,
            encoding,
            originalSize,
            compressedSize,
            ratio,
            duration
        };
    }
    /**
     * Compress using Brotli (best compression ratio)
     */
    async compressBrotli(data) {
        return brotliCompressAsync(data, {
            params: {
                [constants.BROTLI_PARAM_QUALITY]: this.config.level,
                [constants.BROTLI_PARAM_MODE]: constants.BROTLI_MODE_TEXT
            }
        });
    }
    /**
     * Compress using Gzip (faster, broader support)
     */
    async compressGzip(data) {
        return gzipAsync(data, {
            level: Math.min(this.config.level, 9) // Gzip max level is 9
        });
    }
    /**
     * Select best encoding based on client support
     */
    selectEncoding(acceptedEncodings) {
        if (!acceptedEncodings) {
            return this.config.preferredEncoding === 'br' && this.config.enableBrotli
                ? 'br'
                : this.config.enableGzip
                    ? 'gzip'
                    : 'identity';
        }
        const encodings = acceptedEncodings.toLowerCase().split(',').map(e => e.trim());
        // Prefer Brotli if supported and enabled
        if (encodings.includes('br') && this.config.enableBrotli) {
            return 'br';
        }
        // Fall back to Gzip if supported and enabled
        if (encodings.includes('gzip') && this.config.enableGzip) {
            return 'gzip';
        }
        return 'identity';
    }
    /**
     * Check if compression is recommended for content type
     */
    shouldCompress(contentType) {
        if (!contentType)
            return true;
        const type = contentType.toLowerCase();
        // Compressible types
        const compressible = [
            'text/',
            'application/json',
            'application/javascript',
            'application/xml',
            'application/x-www-form-urlencoded'
        ];
        return compressible.some(prefix => type.includes(prefix));
    }
    /**
     * Get compression statistics
     */
    getStats() {
        return {
            config: this.config,
            capabilities: {
                brotli: this.config.enableBrotli,
                gzip: this.config.enableGzip
            }
        };
    }
}
//# sourceMappingURL=compression-middleware.js.map