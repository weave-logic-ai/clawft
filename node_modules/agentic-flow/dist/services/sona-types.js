/**
 * TypeScript interfaces for SONA engine
 *
 * Replaces 'any' types with proper interfaces for type safety
 */
/**
 * Validation utilities
 */
export class ValidationUtils {
    /**
     * Validate embedding dimensions
     */
    static validateEmbedding(embedding, expectedDim = 3072, name = 'embedding') {
        if (!embedding || !Array.isArray(embedding)) {
            throw new Error(`${name} must be an array, got ${typeof embedding}`);
        }
        if (embedding.length !== expectedDim) {
            throw new Error(`${name} must be ${expectedDim}D, got ${embedding.length}D`);
        }
        if (embedding.some(v => typeof v !== 'number' || !isFinite(v))) {
            throw new Error(`${name} contains invalid values (NaN or Infinity)`);
        }
    }
    /**
     * Validate quality score
     */
    static validateQuality(quality, name = 'quality') {
        if (typeof quality !== 'number' || !isFinite(quality)) {
            throw new Error(`${name} must be a finite number, got ${quality}`);
        }
        if (quality < 0 || quality > 1) {
            throw new Error(`${name} must be between 0 and 1, got ${quality}`);
        }
    }
    /**
     * Validate hidden states and attention weights
     */
    static validateStates(hiddenStates, attention, expectedDim = 3072) {
        this.validateEmbedding(hiddenStates, expectedDim, 'hiddenStates');
        this.validateEmbedding(attention, expectedDim, 'attention');
    }
    /**
     * Sanitize file path to prevent traversal attacks
     */
    static sanitizePath(inputPath, baseDir = process.cwd()) {
        const path = require('path');
        // Resolve to absolute path
        const normalized = path.isAbsolute(inputPath)
            ? inputPath
            : path.join(baseDir, inputPath);
        const resolved = path.resolve(normalized);
        // Ensure path is within baseDir
        if (!resolved.startsWith(baseDir)) {
            throw new Error(`Path traversal detected: ${inputPath} resolves outside base directory`);
        }
        return resolved;
    }
}
//# sourceMappingURL=sona-types.js.map