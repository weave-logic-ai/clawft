/**
 * Input Validation Utilities
 *
 * Provides secure input validation for RuVector integration:
 * - Task description validation
 * - Configuration validation
 * - Injection attack prevention
 * - Resource exhaustion prevention
 */
export class ValidationError extends Error {
    field;
    constructor(message, field) {
        super(message);
        this.field = field;
        this.name = 'ValidationError';
    }
}
/**
 * Input Validator
 *
 * Validates all external inputs to prevent:
 * - Injection attacks (XSS, SQL injection, prompt injection)
 * - Resource exhaustion (excessive length, recursion)
 * - Malicious content (scripts, control characters)
 */
export class InputValidator {
    // Suspicious patterns that could indicate attacks
    static SUSPICIOUS_PATTERNS = [
        /<script/i, // XSS attempt
        /javascript:/i, // JavaScript protocol
        /data:text\/html/i, // Data URI XSS
        /on\w+\s*=/i, // Event handlers
        /\beval\s*\(/i, // eval() calls
        /\bFunction\s*\(/i, // Function constructor
        /__proto__/, // Prototype pollution
        /\.\.\//, // Path traversal
        /[;\x00]/, // SQL injection chars
    ];
    // Control characters that should be removed
    static CONTROL_CHARS_REGEX = /[\x00-\x1F\x7F]/g;
    /**
     * Validate task description
     *
     * @param taskDescription - User-provided task description
     * @param options - Validation options
     * @returns Sanitized task description
     * @throws ValidationError if invalid
     */
    static validateTaskDescription(taskDescription, options) {
        const opts = {
            maxLength: options?.maxLength ?? 10000,
            minLength: options?.minLength ?? 1,
            allowEmpty: options?.allowEmpty ?? false,
            sanitize: options?.sanitize ?? true,
        };
        // Check for null/undefined
        if (taskDescription === null || taskDescription === undefined) {
            throw new ValidationError('Task description is required', 'taskDescription');
        }
        // Check type
        if (typeof taskDescription !== 'string') {
            throw new ValidationError(`Task description must be a string, got ${typeof taskDescription}`, 'taskDescription');
        }
        // Check empty
        if (!opts.allowEmpty && taskDescription.trim().length === 0) {
            throw new ValidationError('Task description cannot be empty', 'taskDescription');
        }
        // Check length
        if (taskDescription.length < opts.minLength) {
            throw new ValidationError(`Task description too short (min ${opts.minLength} chars)`, 'taskDescription');
        }
        if (taskDescription.length > opts.maxLength) {
            throw new ValidationError(`Task description too long (max ${opts.maxLength} chars)`, 'taskDescription');
        }
        // Sanitize if requested
        let sanitized = taskDescription;
        if (opts.sanitize) {
            // Remove control characters
            sanitized = sanitized.replace(InputValidator.CONTROL_CHARS_REGEX, '');
            // Check for suspicious patterns
            for (const pattern of InputValidator.SUSPICIOUS_PATTERNS) {
                if (pattern.test(sanitized)) {
                    throw new ValidationError('Task description contains suspicious content', 'taskDescription');
                }
            }
        }
        return sanitized.trim();
    }
    /**
     * Validate agent name
     *
     * @param agentName - Agent identifier
     * @returns Sanitized agent name
     * @throws ValidationError if invalid
     */
    static validateAgentName(agentName) {
        if (!agentName || typeof agentName !== 'string') {
            throw new ValidationError('Agent name is required', 'agentName');
        }
        if (agentName.length < 1 || agentName.length > 100) {
            throw new ValidationError('Agent name must be between 1-100 characters', 'agentName');
        }
        // Only allow alphanumeric, dash, underscore
        if (!/^[a-zA-Z0-9_-]+$/.test(agentName)) {
            throw new ValidationError('Agent name can only contain letters, numbers, dash, and underscore', 'agentName');
        }
        return agentName.toLowerCase();
    }
    /**
     * Validate confidence score
     *
     * @param confidence - Confidence score (0-1)
     * @returns Validated confidence
     * @throws ValidationError if invalid
     */
    static validateConfidence(confidence) {
        if (typeof confidence !== 'number') {
            throw new ValidationError(`Confidence must be a number, got ${typeof confidence}`, 'confidence');
        }
        if (Number.isNaN(confidence) || !Number.isFinite(confidence)) {
            throw new ValidationError('Confidence must be a valid number', 'confidence');
        }
        if (confidence < 0 || confidence > 1) {
            throw new ValidationError('Confidence must be between 0 and 1', 'confidence');
        }
        return confidence;
    }
    /**
     * Validate timeout value
     *
     * @param timeout - Timeout in milliseconds
     * @param min - Minimum allowed timeout (default: 100ms)
     * @param max - Maximum allowed timeout (default: 5 minutes)
     * @returns Validated timeout
     * @throws ValidationError if invalid
     */
    static validateTimeout(timeout, min = 100, max = 300000) {
        if (typeof timeout !== 'number') {
            throw new ValidationError(`Timeout must be a number, got ${typeof timeout}`, 'timeout');
        }
        if (Number.isNaN(timeout) || !Number.isFinite(timeout)) {
            throw new ValidationError('Timeout must be a valid number', 'timeout');
        }
        if (timeout < min) {
            throw new ValidationError(`Timeout too short (min ${min}ms)`, 'timeout');
        }
        if (timeout > max) {
            throw new ValidationError(`Timeout too long (max ${max}ms)`, 'timeout');
        }
        return Math.floor(timeout);
    }
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
    static validateStringArray(array, fieldName, maxItems = 100, maxLength = 1000) {
        if (!Array.isArray(array)) {
            throw new ValidationError(`${fieldName} must be an array`, fieldName);
        }
        if (array.length > maxItems) {
            throw new ValidationError(`${fieldName} has too many items (max ${maxItems})`, fieldName);
        }
        const validated = [];
        for (let i = 0; i < array.length; i++) {
            const item = array[i];
            if (typeof item !== 'string') {
                throw new ValidationError(`${fieldName}[${i}] must be a string`, fieldName);
            }
            if (item.length > maxLength) {
                throw new ValidationError(`${fieldName}[${i}] too long (max ${maxLength} chars)`, fieldName);
            }
            validated.push(item);
        }
        return validated;
    }
    /**
     * Validate configuration object
     *
     * @param config - Configuration to validate
     * @param schema - Validation schema
     * @returns Validated configuration
     * @throws ValidationError if invalid
     */
    static validateConfig(config, schema) {
        if (!config || typeof config !== 'object') {
            throw new ValidationError('Configuration must be an object', 'config');
        }
        const validated = {};
        const configObj = config;
        for (const [key, rules] of Object.entries(schema)) {
            const value = configObj[key];
            // Check required
            if (rules.required && (value === undefined || value === null)) {
                throw new ValidationError(`Configuration field '${key}' is required`, key);
            }
            // Skip if optional and not provided
            if (!rules.required && (value === undefined || value === null)) {
                continue;
            }
            // Check type
            if (typeof value !== rules.type) {
                throw new ValidationError(`Configuration field '${key}' must be ${rules.type}, got ${typeof value}`, key);
            }
            // Validate numbers
            if (rules.type === 'number') {
                if (Number.isNaN(value) || !Number.isFinite(value)) {
                    throw new ValidationError(`Configuration field '${key}' must be a valid number`, key);
                }
                if (rules.min !== undefined && value < rules.min) {
                    throw new ValidationError(`Configuration field '${key}' must be >= ${rules.min}`, key);
                }
                if (rules.max !== undefined && value > rules.max) {
                    throw new ValidationError(`Configuration field '${key}' must be <= ${rules.max}`, key);
                }
            }
            // Custom validator
            if (rules.validator && !rules.validator(value)) {
                throw new ValidationError(`Configuration field '${key}' failed validation`, key);
            }
            validated[key] = value;
        }
        return validated;
    }
    /**
     * Sanitize HTML to prevent XSS
     *
     * @param html - HTML string to sanitize
     * @returns Sanitized HTML (text only)
     */
    static sanitizeHtml(html) {
        if (typeof html !== 'string') {
            return '';
        }
        // Strip all HTML tags, keep text only
        return html
            .replace(/<[^>]*>/g, '')
            .replace(/&lt;/g, '<')
            .replace(/&gt;/g, '>')
            .replace(/&amp;/g, '&')
            .replace(/&quot;/g, '"')
            .replace(/&#x27;/g, "'");
    }
    /**
     * Validate email address
     *
     * @param email - Email to validate
     * @returns Normalized email
     * @throws ValidationError if invalid
     */
    static validateEmail(email) {
        if (!email || typeof email !== 'string') {
            throw new ValidationError('Email is required', 'email');
        }
        // Basic email regex
        const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
        if (!emailRegex.test(email)) {
            throw new ValidationError('Invalid email format', 'email');
        }
        if (email.length > 254) {
            throw new ValidationError('Email too long (max 254 chars)', 'email');
        }
        return email.toLowerCase().trim();
    }
}
/**
 * Validation middleware factory
 *
 * Creates validation middleware for Express/Fastify routes
 */
export function createValidationMiddleware(validator) {
    return async (req, res, next) => {
        try {
            await validator(req);
            next();
        }
        catch (error) {
            if (error instanceof ValidationError) {
                res.status(400).json({
                    error: 'Validation Error',
                    message: error.message,
                    field: error.field,
                });
            }
            else {
                res.status(500).json({
                    error: 'Internal Server Error',
                    message: 'Validation failed',
                });
            }
        }
    };
}
//# sourceMappingURL=input-validator.js.map