/**
 * Input Validation Utilities
 *
 * Provides secure input validation for RuVector integration:
 * - Task description validation
 * - Configuration validation
 * - Injection attack prevention
 * - Resource exhaustion prevention
 */
export interface ValidationOptions {
    maxLength?: number;
    minLength?: number;
    allowEmpty?: boolean;
    sanitize?: boolean;
}
export declare class ValidationError extends Error {
    field?: string;
    constructor(message: string, field?: string);
}
/**
 * Input Validator
 *
 * Validates all external inputs to prevent:
 * - Injection attacks (XSS, SQL injection, prompt injection)
 * - Resource exhaustion (excessive length, recursion)
 * - Malicious content (scripts, control characters)
 */
export declare class InputValidator {
    private static readonly SUSPICIOUS_PATTERNS;
    private static readonly CONTROL_CHARS_REGEX;
    /**
     * Validate task description
     *
     * @param taskDescription - User-provided task description
     * @param options - Validation options
     * @returns Sanitized task description
     * @throws ValidationError if invalid
     */
    static validateTaskDescription(taskDescription: string, options?: ValidationOptions): string;
    /**
     * Validate agent name
     *
     * @param agentName - Agent identifier
     * @returns Sanitized agent name
     * @throws ValidationError if invalid
     */
    static validateAgentName(agentName: string): string;
    /**
     * Validate confidence score
     *
     * @param confidence - Confidence score (0-1)
     * @returns Validated confidence
     * @throws ValidationError if invalid
     */
    static validateConfidence(confidence: number): number;
    /**
     * Validate timeout value
     *
     * @param timeout - Timeout in milliseconds
     * @param min - Minimum allowed timeout (default: 100ms)
     * @param max - Maximum allowed timeout (default: 5 minutes)
     * @returns Validated timeout
     * @throws ValidationError if invalid
     */
    static validateTimeout(timeout: number, min?: number, max?: number): number;
    /**
     * Validate array of strings
     *
     * @param array - Array to validate
     * @param fieldName - Field name for error messages
     * @param maxItems - Maximum number of items
     * @param maxLength - Maximum length per item
     * @returns Validated array
     * @throws ValidationError if invalid
     */
    static validateStringArray(array: unknown, fieldName: string, maxItems?: number, maxLength?: number): string[];
    /**
     * Validate configuration object
     *
     * @param config - Configuration to validate
     * @param schema - Validation schema
     * @returns Validated configuration
     * @throws ValidationError if invalid
     */
    static validateConfig<T extends Record<string, any>>(config: unknown, schema: {
        [K in keyof T]: {
            type: 'string' | 'number' | 'boolean' | 'object';
            required?: boolean;
            min?: number;
            max?: number;
            validator?: (value: any) => boolean;
        };
    }): T;
    /**
     * Sanitize HTML to prevent XSS
     *
     * @param html - HTML string to sanitize
     * @returns Sanitized HTML (text only)
     */
    static sanitizeHtml(html: string): string;
    /**
     * Validate email address
     *
     * @param email - Email to validate
     * @returns Normalized email
     * @throws ValidationError if invalid
     */
    static validateEmail(email: string): string;
}
/**
 * Validation middleware factory
 *
 * Creates validation middleware for Express/Fastify routes
 */
export declare function createValidationMiddleware(validator: (req: any) => void | Promise<void>): (req: any, res: any, next: any) => Promise<void>;
//# sourceMappingURL=input-validator.d.ts.map