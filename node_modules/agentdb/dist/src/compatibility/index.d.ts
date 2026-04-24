/**
 * Backwards Compatibility Layer - Main Exports
 *
 * Provides v1.x to v2.0 compatibility for AgenticFlow
 */
export { VersionDetector } from './VersionDetector';
export { V1toV2Adapter } from './V1toV2Adapter';
export { DeprecationWarnings } from './DeprecationWarnings';
export { MigrationUtilities } from './MigrationUtilities';
export type { APIVersion, CompatibilityConfig, V1Config, V2Config, DeprecationWarning, DeprecationConfig, MigrationReport, ValidationResult, VersionDetectionResult } from './types';
export declare const analyzeMigration: (code: string) => import("./types").MigrationReport;
export declare const migrateCode: (code: string) => string;
export declare const convertConfig: (v1Config: any) => import("./types").V2Config;
//# sourceMappingURL=index.d.ts.map