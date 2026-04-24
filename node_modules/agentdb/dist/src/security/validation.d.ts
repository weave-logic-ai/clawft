/**
 * AgentDB v2 Security Validation
 *
 * Comprehensive input validation for RuVector integration:
 * - Vector dimension and value validation (NaN/Infinity prevention)
 * - ID sanitization (path traversal prevention)
 * - Search options validation (bounds checking)
 * - Cypher query parameter validation (injection prevention)
 * - Metadata sanitization (sensitive data protection)
 */
/**
 * Security limits for AgentDB v2
 */
export declare const SECURITY_LIMITS: {
    readonly MAX_VECTORS: 10000000;
    readonly MAX_DIMENSION: 4096;
    readonly MAX_BATCH_SIZE: 10000;
    readonly MAX_K: 10000;
    readonly QUERY_TIMEOUT_MS: 30000;
    readonly MAX_MEMORY_MB: 16384;
    readonly MAX_ID_LENGTH: 256;
    readonly MAX_METADATA_SIZE: 65536;
    readonly MAX_LABEL_LENGTH: 128;
    readonly MAX_PROPERTY_KEY_LENGTH: 128;
    readonly MAX_CYPHER_PARAMS: 100;
    readonly MIN_DIMENSION: 1;
    readonly MIN_K: 1;
    readonly MIN_THRESHOLD: 0;
    readonly MAX_THRESHOLD: 1;
    readonly MIN_EF_SEARCH: 1;
    readonly MAX_EF_SEARCH: 1000;
    readonly MIN_EF_CONSTRUCTION: 4;
    readonly MAX_EF_CONSTRUCTION: 500;
    readonly MAX_M: 64;
    readonly MIN_M: 2;
};
/**
 * Validate vector embedding data
 * Prevents NaN, Infinity, and dimension mismatches
 */
export declare function validateVector(embedding: Float32Array | number[], expectedDim: number, fieldName?: string): void;
/**
 * Validate vector ID
 * Prevents path traversal, excessive length, and malicious characters
 */
export declare function validateVectorId(id: string, fieldName?: string): string;
/**
 * Validate search options
 * Ensures k, threshold, and other parameters are within safe bounds
 */
export interface SearchOptions {
    k?: number;
    threshold?: number;
    efSearch?: number;
    filter?: Record<string, any>;
    includeMetadata?: boolean;
    includeVectors?: boolean;
}
export declare function validateSearchOptions(options: SearchOptions): SearchOptions;
/**
 * Validate HNSW index parameters
 */
export interface HNSWParams {
    M?: number;
    efConstruction?: number;
    efSearch?: number;
}
export declare function validateHNSWParams(params: HNSWParams): HNSWParams;
/**
 * Sanitize metadata to prevent sensitive data exposure
 * Removes fields that commonly contain secrets or PII
 */
export declare function sanitizeMetadata(metadata: Record<string, any>): Record<string, any>;
/**
 * Validate Cypher query parameters for graph operations
 * Prevents Cypher injection attacks
 */
export declare function validateCypherParams(params: Record<string, any>): Record<string, any>;
/**
 * Validate graph node label
 */
export declare function validateLabel(label: string): string;
/**
 * Validate batch size for bulk operations
 */
export declare function validateBatchSize(batchSize: number): number;
/**
 * Validate vector count doesn't exceed limits
 */
export declare function validateVectorCount(count: number): void;
/**
 * Safe logging that doesn't expose vectors or sensitive data
 */
export declare function safeLog(message: string, data?: any): void;
//# sourceMappingURL=validation.d.ts.map