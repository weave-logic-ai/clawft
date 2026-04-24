/**
 * TypeScript type definitions for AgentDB wrapper
 *
 * Provides clean, typed interfaces for all AgentDB operations
 * following v2.0.0-alpha.2.11 specification
 */
/**
 * Error types for AgentDB operations
 */
export class AgentDBError extends Error {
    code;
    operation;
    details;
    constructor(message, code, operation, details) {
        super(message);
        this.code = code;
        this.operation = operation;
        this.details = details;
        this.name = 'AgentDBError';
    }
}
/**
 * Validation error for invalid inputs
 */
export class ValidationError extends AgentDBError {
    constructor(message, details) {
        super(message, 'VALIDATION_ERROR', 'insert', details);
        this.name = 'ValidationError';
    }
}
/**
 * Database error for storage issues
 */
export class DatabaseError extends AgentDBError {
    constructor(message, operation, details) {
        super(message, 'DATABASE_ERROR', operation, details);
        this.name = 'DatabaseError';
    }
}
/**
 * Index error for HNSW operations
 */
export class IndexError extends AgentDBError {
    constructor(message, operation, details) {
        super(message, 'INDEX_ERROR', operation, details);
        this.name = 'IndexError';
    }
}
//# sourceMappingURL=agentdb.js.map