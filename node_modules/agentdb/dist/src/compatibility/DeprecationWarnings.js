/**
 * DeprecationWarnings - Manages deprecation warnings for v1.x APIs
 *
 * Provides tiered warning system (silent → soft → prominent) with
 * configurable severity and migration guidance.
 */
import * as fs from 'fs';
export class DeprecationWarnings {
    warnings = [];
    seenAPIs = new Set();
    config;
    constructor(config = {}) {
        this.config = {
            emitWarnings: config.emitWarnings ?? true,
            throwOnDeprecated: config.throwOnDeprecated ?? false,
            logToFile: config.logToFile,
            severity: config.severity ?? 'soft'
        };
    }
    /**
     * Emit a deprecation warning
     */
    warn(api, details) {
        // Build warning message
        const warning = this.buildWarning(api, details);
        // Store in history
        this.warnings.push(warning);
        // Check if already warned for this API (avoid spam)
        const alreadyWarned = this.seenAPIs.has(api);
        this.seenAPIs.add(api);
        // Emit based on severity
        if (this.config.severity === 'silent') {
            // Silent mode: no console output
            return;
        }
        if (!alreadyWarned && this.config.emitWarnings) {
            if (this.config.severity === 'soft') {
                this.emitSoftWarning(api, details);
            }
            else if (this.config.severity === 'prominent') {
                this.emitProminentWarning(api, details);
            }
        }
        // Log to file if configured
        if (this.config.logToFile) {
            this.logToFile(warning);
        }
        // Throw error in strict mode
        if (this.config.throwOnDeprecated) {
            throw new Error(`Deprecated API usage: ${api}\n${details.message}\nMigration: ${details.migration}`);
        }
    }
    /**
     * Get all warning messages
     */
    getWarnings() {
        return [...this.warnings];
    }
    /**
     * Clear warning history
     */
    clearWarnings() {
        this.warnings = [];
        this.seenAPIs.clear();
    }
    /**
     * Check if any warnings have been emitted
     */
    hasWarnings() {
        return this.warnings.length > 0;
    }
    /**
     * Get warning count
     */
    getWarningCount() {
        return this.warnings.length;
    }
    /**
     * Build formatted warning message
     */
    buildWarning(api, details) {
        return `[DEPRECATED] ${api}: ${details.message}\nMigration: ${details.migration}\nDocumentation: ${details.documentation}`;
    }
    /**
     * Emit soft warning (minimal console output)
     */
    emitSoftWarning(api, details) {
        console.warn(`⚠️  ${api} is deprecated. ${details.migration}`);
    }
    /**
     * Emit prominent warning (full console output with styling)
     */
    emitProminentWarning(api, details) {
        const border = '═'.repeat(70);
        const message = [
            '',
            border,
            `⛔ DEPRECATED API: ${api}`,
            border,
            '',
            `Message: ${details.message}`,
            '',
            `Migration Guide:`,
            `  ${details.migration}`,
            '',
            `Documentation:`,
            `  ${details.documentation}`,
            '',
            border,
            ''
        ].join('\n');
        console.error(message);
    }
    /**
     * Log warning to file
     */
    logToFile(warning) {
        if (!this.config.logToFile)
            return;
        try {
            const timestamp = new Date().toISOString();
            const logEntry = `[${timestamp}] ${warning}\n\n`;
            fs.appendFileSync(this.config.logToFile, logEntry, 'utf-8');
        }
        catch (error) {
            // Silently fail file logging to not disrupt application
            console.error('Failed to log deprecation warning to file:', error);
        }
    }
}
//# sourceMappingURL=DeprecationWarnings.js.map