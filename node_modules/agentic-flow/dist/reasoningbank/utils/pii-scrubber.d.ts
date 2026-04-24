/**
 * PII (Personally Identifiable Information) scrubber
 * Redacts sensitive information before storing memories
 */
/**
 * Scrub PII from text
 *
 * @param text - Text to scrub
 * @param customPatterns - Additional custom patterns to apply
 * @returns Scrubbed text with PII redacted
 */
export declare function scrubPII(text: string, customPatterns?: Array<{
    pattern: RegExp;
    replacement: string;
}>): string;
/**
 * Check if text contains potential PII
 *
 * @param text - Text to check
 * @returns True if PII patterns are detected
 */
export declare function containsPII(text: string): boolean;
/**
 * Get statistics about redacted content
 *
 * @param original - Original text
 * @param scrubbed - Scrubbed text
 * @returns Object with redaction statistics
 */
export declare function getRedactionStats(original: string, scrubbed: string): {
    redacted: boolean;
    originalLength: number;
    scrubbedLength: number;
    patterns: string[];
};
/**
 * Scrub PII from memory object
 * Scrubs title, description, and content fields
 */
export declare function scrubMemory(memory: {
    title: string;
    description: string;
    content: string;
    [key: string]: any;
}): typeof memory;
//# sourceMappingURL=pii-scrubber.d.ts.map