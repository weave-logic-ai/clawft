/**
 * PII (Personally Identifiable Information) scrubber
 * Redacts sensitive information before storing memories
 */
import { loadConfig } from './config.js';
// Default PII patterns (regex-based)
const DEFAULT_PATTERNS = [
    // Email addresses
    { pattern: /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b/g, replacement: '[EMAIL]' },
    // US Social Security Numbers
    { pattern: /\b\d{3}-\d{2}-\d{4}\b/g, replacement: '[SSN]' },
    { pattern: /\b\d{9}\b/g, replacement: '[SSN]' },
    // API Keys (common patterns)
    { pattern: /\bsk-[a-zA-Z0-9]{48}\b/g, replacement: '[API_KEY]' }, // Anthropic
    { pattern: /\bghp_[a-zA-Z0-9]{36}\b/g, replacement: '[API_KEY]' }, // GitHub
    { pattern: /\bgho_[a-zA-Z0-9]{36}\b/g, replacement: '[API_KEY]' }, // GitHub OAuth
    { pattern: /\bxoxb-[a-zA-Z0-9\-]+\b/g, replacement: '[API_KEY]' }, // Slack
    { pattern: /\bAKIA[0-9A-Z]{16}\b/g, replacement: '[AWS_KEY]' }, // AWS
    // Credit card numbers (basic pattern)
    { pattern: /\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b/g, replacement: '[CREDIT_CARD]' },
    // Phone numbers (US format)
    { pattern: /\b\d{3}[-.\s]?\d{3}[-.\s]?\d{4}\b/g, replacement: '[PHONE]' },
    { pattern: /\b\(\d{3}\)\s?\d{3}[-.\s]?\d{4}\b/g, replacement: '[PHONE]' },
    // IP addresses
    { pattern: /\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b/g, replacement: '[IP]' },
    // URLs with tokens/keys in query params
    { pattern: /([?&])(token|key|apikey|api_key|secret)=[^&\s]+/gi, replacement: '$1$2=[REDACTED]' },
    // JWT tokens
    { pattern: /\beyJ[a-zA-Z0-9_-]+\.eyJ[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+\b/g, replacement: '[JWT]' }
];
/**
 * Scrub PII from text
 *
 * @param text - Text to scrub
 * @param customPatterns - Additional custom patterns to apply
 * @returns Scrubbed text with PII redacted
 */
export function scrubPII(text, customPatterns) {
    const config = loadConfig();
    // Check if PII scrubbing is enabled
    if (!config.governance?.pii_scrubber) {
        return text;
    }
    let scrubbed = text;
    const patterns = customPatterns || DEFAULT_PATTERNS;
    // Apply all redaction patterns
    for (const { pattern, replacement } of patterns) {
        scrubbed = scrubbed.replace(pattern, replacement);
    }
    return scrubbed;
}
/**
 * Check if text contains potential PII
 *
 * @param text - Text to check
 * @returns True if PII patterns are detected
 */
export function containsPII(text) {
    for (const { pattern } of DEFAULT_PATTERNS) {
        if (pattern.test(text)) {
            return true;
        }
    }
    return false;
}
/**
 * Get statistics about redacted content
 *
 * @param original - Original text
 * @param scrubbed - Scrubbed text
 * @returns Object with redaction statistics
 */
export function getRedactionStats(original, scrubbed) {
    const patterns = [];
    for (const { pattern, replacement } of DEFAULT_PATTERNS) {
        if (pattern.test(original) && scrubbed.includes(replacement)) {
            patterns.push(replacement);
        }
    }
    return {
        redacted: patterns.length > 0,
        originalLength: original.length,
        scrubbedLength: scrubbed.length,
        patterns
    };
}
/**
 * Scrub PII from memory object
 * Scrubs title, description, and content fields
 */
export function scrubMemory(memory) {
    return {
        ...memory,
        title: scrubPII(memory.title),
        description: scrubPII(memory.description),
        content: scrubPII(memory.content)
    };
}
//# sourceMappingURL=pii-scrubber.js.map