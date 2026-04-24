// Retry utility with exponential backoff
import { logger } from './logger.js';
const defaultOptions = {
    maxAttempts: 3,
    baseDelay: 1000,
    maxDelay: 10000,
    shouldRetry: (error) => {
        // Retry on network errors, rate limits, and server errors
        if (error?.status >= 500)
            return true;
        if (error?.status === 429)
            return true;
        if (error?.code === 'ECONNRESET')
            return true;
        if (error?.code === 'ETIMEDOUT')
            return true;
        return false;
    }
};
export async function withRetry(fn, options = {}) {
    const opts = { ...defaultOptions, ...options };
    let lastError;
    for (let attempt = 1; attempt <= opts.maxAttempts; attempt++) {
        try {
            logger.debug('Attempting operation', { attempt, maxAttempts: opts.maxAttempts });
            return await fn();
        }
        catch (error) {
            lastError = error;
            if (attempt >= opts.maxAttempts) {
                logger.error('Max retry attempts reached', {
                    attempt,
                    maxAttempts: opts.maxAttempts,
                    error
                });
                throw error;
            }
            if (!opts.shouldRetry(error)) {
                logger.warn('Error not retryable', { error });
                throw error;
            }
            // Calculate backoff delay with jitter
            const delay = Math.min(opts.baseDelay * Math.pow(2, attempt - 1) + Math.random() * 1000, opts.maxDelay);
            logger.warn('Operation failed, retrying', {
                attempt,
                nextAttempt: attempt + 1,
                delayMs: Math.round(delay),
                error
            });
            await new Promise(resolve => setTimeout(resolve, delay));
        }
    }
    throw lastError;
}
//# sourceMappingURL=retry.js.map