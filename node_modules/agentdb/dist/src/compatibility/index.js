/**
 * Backwards Compatibility Layer - Main Exports
 *
 * Provides v1.x to v2.0 compatibility for AgenticFlow
 */
export { VersionDetector } from './VersionDetector';
export { V1toV2Adapter } from './V1toV2Adapter';
export { DeprecationWarnings } from './DeprecationWarnings';
export { MigrationUtilities } from './MigrationUtilities';
// Re-export convenience functions for quick migration analysis
import { MigrationUtilities } from './MigrationUtilities';
export const analyzeMigration = (code) => {
    return MigrationUtilities.analyzeCode(code);
};
export const migrateCode = (code) => {
    return MigrationUtilities.generateMigrationScript(code);
};
export const convertConfig = (v1Config) => {
    return MigrationUtilities.convertV1ConfigToV2(v1Config);
};
//# sourceMappingURL=index.js.map