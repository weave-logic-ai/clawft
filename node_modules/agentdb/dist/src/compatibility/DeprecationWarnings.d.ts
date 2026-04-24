/**
 * DeprecationWarnings - Manages deprecation warnings for v1.x APIs
 *
 * Provides tiered warning system (silent → soft → prominent) with
 * configurable severity and migration guidance.
 */
import type { DeprecationConfig } from './types';
export declare class DeprecationWarnings {
    private warnings;
    private seenAPIs;
    private config;
    constructor(config?: DeprecationConfig);
    /**
     * Emit a deprecation warning
     */
    warn(api: string, details: {
        message: string;
        migration: string;
        documentation: string;
    }): void;
    /**
     * Get all warning messages
     */
    getWarnings(): string[];
    /**
     * Clear warning history
     */
    clearWarnings(): void;
    /**
     * Check if any warnings have been emitted
     */
    hasWarnings(): boolean;
    /**
     * Get warning count
     */
    getWarningCount(): number;
    /**
     * Build formatted warning message
     */
    private buildWarning;
    /**
     * Emit soft warning (minimal console output)
     */
    private emitSoftWarning;
    /**
     * Emit prominent warning (full console output with styling)
     */
    private emitProminentWarning;
    /**
     * Log warning to file
     */
    private logToFile;
}
//# sourceMappingURL=DeprecationWarnings.d.ts.map