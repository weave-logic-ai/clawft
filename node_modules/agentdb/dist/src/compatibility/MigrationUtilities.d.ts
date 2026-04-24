/**
 * MigrationUtilities - Tools for migrating from v1.x to v2.0
 *
 * Provides code analysis, automatic migration, and validation
 * utilities to help users upgrade from v1.x to v2.0.
 */
import type { MigrationReport, ValidationResult, V1Config, V2Config } from './types';
export declare class MigrationUtilities {
    /**
     * Analyze v1 code for migration opportunities
     */
    static analyzeCode(code: string): MigrationReport;
    /**
     * Generate migration script (automatic code transformation)
     */
    static generateMigrationScript(code: string): string;
    /**
     * Validate migrated v2 config
     */
    static validateMigratedConfig(v2Config: V2Config): ValidationResult;
    /**
     * Convert v1 config to v2 config
     */
    static convertV1ConfigToV2(v1Config: V1Config): V2Config;
    /**
     * Generate migration guide
     */
    static generateMigrationGuide(report: MigrationReport): string;
}
//# sourceMappingURL=MigrationUtilities.d.ts.map