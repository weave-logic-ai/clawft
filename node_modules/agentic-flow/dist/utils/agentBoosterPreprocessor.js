/**
 * Agent Booster Pre-Processor
 *
 * Detects code editing intents in agent tasks and attempts Agent Booster
 * pattern matching before falling back to LLM.
 */
import { execSync } from 'child_process';
import { existsSync, readFileSync, writeFileSync } from 'fs';
import { extname } from 'path';
export class AgentBoosterPreprocessor {
    confidenceThreshold;
    enabledIntents;
    constructor(options = {}) {
        this.confidenceThreshold = options.confidenceThreshold || 0.7;
        this.enabledIntents = new Set(options.enabledIntents || [
            'var_to_const',
            'add_types',
            'add_error_handling',
            'async_await',
            'add_logging',
            'remove_console',
            'format_code'
        ]);
    }
    /**
     * Detect if a task is a code editing intent that Agent Booster can handle
     */
    detectIntent(task) {
        const patterns = [
            {
                regex: /convert\s+(all\s+)?var\s+to\s+const/i,
                type: 'var_to_const',
                extractor: this.extractVarToConst.bind(this)
            },
            {
                regex: /add\s+type\s+(annotations?|hints?)/i,
                type: 'add_types',
                extractor: this.extractAddTypes.bind(this)
            },
            {
                regex: /add\s+error\s+handling/i,
                type: 'add_error_handling',
                extractor: this.extractAddErrorHandling.bind(this)
            },
            {
                regex: /convert\s+to\s+async\s*\/?\s*await/i,
                type: 'async_await',
                extractor: this.extractAsyncAwait.bind(this)
            },
            {
                regex: /add\s+(logging|console\.log)/i,
                type: 'add_logging',
                extractor: this.extractAddLogging.bind(this)
            },
            {
                regex: /remove\s+(all\s+)?console\.(log|debug|warn)/i,
                type: 'remove_console',
                extractor: this.extractRemoveConsole.bind(this)
            }
        ];
        for (const pattern of patterns) {
            if (pattern.regex.test(task) && this.enabledIntents.has(pattern.type)) {
                const filePath = this.extractFilePath(task);
                if (filePath && existsSync(filePath)) {
                    const originalCode = readFileSync(filePath, 'utf-8');
                    const targetCode = pattern.extractor(originalCode);
                    if (targetCode) {
                        return {
                            type: pattern.type,
                            task,
                            filePath,
                            originalCode,
                            targetCode,
                            confidence: 0.8 // Initial confidence for pattern match
                        };
                    }
                }
            }
        }
        return null;
    }
    /**
     * Try to apply edit using Agent Booster
     */
    async tryApply(intent) {
        if (!intent.filePath || !intent.originalCode || !intent.targetCode) {
            return {
                success: false,
                method: 'llm_required',
                reason: 'Missing file path or code'
            };
        }
        try {
            const language = this.detectLanguage(intent.filePath);
            const cmd = `npx --yes agent-booster@0.2.2 apply --language ${language}`;
            let result;
            try {
                result = execSync(cmd, {
                    encoding: 'utf-8',
                    input: JSON.stringify({
                        code: intent.originalCode,
                        edit: intent.targetCode
                    }),
                    maxBuffer: 10 * 1024 * 1024,
                    timeout: 10000
                });
            }
            catch (execError) {
                // execSync throws on non-zero exit, but agent-booster returns JSON even on stderr
                // Try to parse stdout anyway
                if (execError.stdout) {
                    result = execError.stdout.toString();
                }
                else {
                    throw new Error(`execSync failed: ${execError.message}`);
                }
            }
            const parsed = JSON.parse(result);
            if (parsed.success && parsed.confidence >= this.confidenceThreshold) {
                // High confidence - use Agent Booster result
                writeFileSync(intent.filePath, parsed.output);
                return {
                    success: true,
                    method: 'agent_booster',
                    output: parsed.output,
                    latency: parsed.latency,
                    confidence: parsed.confidence,
                    strategy: parsed.strategy
                };
            }
            else {
                // Low confidence - require LLM
                return {
                    success: false,
                    method: 'llm_required',
                    confidence: parsed.confidence,
                    reason: `Agent Booster confidence too low (${(parsed.confidence * 100).toFixed(1)}%). Using LLM for complex edit.`
                };
            }
        }
        catch (error) {
            return {
                success: false,
                method: 'llm_required',
                reason: `Agent Booster error: ${error.message}`
            };
        }
    }
    /**
     * Extract file path from task description
     */
    extractFilePath(task) {
        // Pattern: "in <file>" or "to <file>" or just "<file.ext>"
        const patterns = [
            /(?:in|to|from|for|file:?)\s+([^\s]+\.(?:js|ts|tsx|jsx|py|rs|go|java|c|cpp|h|hpp))/i,
            /([^\s]+\.(?:js|ts|tsx|jsx|py|rs|go|java|c|cpp|h|hpp))/i
        ];
        for (const pattern of patterns) {
            const match = task.match(pattern);
            if (match) {
                return match[1];
            }
        }
        return null;
    }
    /**
     * Detect programming language from file extension
     */
    detectLanguage(filePath) {
        const ext = extname(filePath).slice(1);
        const langMap = {
            'ts': 'typescript',
            'tsx': 'typescript',
            'js': 'javascript',
            'jsx': 'javascript',
            'py': 'python',
            'rs': 'rust',
            'go': 'go',
            'java': 'java',
            'c': 'c',
            'cpp': 'cpp',
            'h': 'c',
            'hpp': 'cpp'
        };
        return langMap[ext] || 'javascript';
    }
    /**
     * Extract var to const transformation
     */
    extractVarToConst(code) {
        // Simple transformation: replace var with const
        if (code.includes('var ')) {
            return code.replace(/\bvar\b/g, 'const');
        }
        return null;
    }
    /**
     * Extract add types transformation (TypeScript)
     */
    extractAddTypes(code) {
        // Simple type annotation: function params and return types
        const functionPattern = /function\s+(\w+)\s*\(([^)]*)\)\s*\{/g;
        let hasUntypedFunctions = false;
        let transformed = code;
        code.replace(functionPattern, (match, name, params) => {
            if (!params.includes(':')) {
                hasUntypedFunctions = true;
                // Add basic type hints
                const typedParams = params.split(',').map((p) => {
                    const trimmed = p.trim();
                    if (trimmed && !trimmed.includes(':')) {
                        return `${trimmed}: any`;
                    }
                    return p;
                }).join(', ');
                transformed = transformed.replace(match, `function ${name}(${typedParams}): any {`);
            }
            return match;
        });
        return hasUntypedFunctions ? transformed : null;
    }
    /**
     * Extract add error handling transformation
     */
    extractAddErrorHandling(code) {
        // Wrap risky operations in try-catch
        const riskyPatterns = [
            /JSON\.parse\(/,
            /fetch\(/,
            /\bawait\s+/
        ];
        for (const pattern of riskyPatterns) {
            if (pattern.test(code)) {
                // This is complex - better handled by LLM
                return null;
            }
        }
        return null;
    }
    /**
     * Extract async/await transformation
     */
    extractAsyncAwait(code) {
        // Convert .then() chains to async/await
        if (code.includes('.then(')) {
            // This is complex - better handled by LLM
            return null;
        }
        return null;
    }
    /**
     * Extract add logging transformation
     */
    extractAddLogging(code) {
        // Add console.log before function returns
        // This is complex - better handled by LLM
        return null;
    }
    /**
     * Extract remove console transformation
     */
    extractRemoveConsole(code) {
        if (code.includes('console.')) {
            // Remove console statements but preserve line structure
            // Match optional leading whitespace, console call, and keep one newline if present
            return code.replace(/^[ \t]*console\.(log|debug|warn|info|error)\([^)]*\);?\s*$/gm, '');
        }
        return null;
    }
}
//# sourceMappingURL=agentBoosterPreprocessor.js.map