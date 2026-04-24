/**
 * Transfer Hook - Cross-project pattern transfer
 * Enables knowledge sharing between projects
 */
import { z } from 'zod';
import * as path from 'path';
import * as fs from 'fs';
import { loadIntelligence, saveIntelligence, simpleEmbed } from './shared.js';
const INTELLIGENCE_PATH = '.agentic-flow/intelligence.json';
export const hookTransferTool = {
    name: 'hook_transfer',
    description: 'Transfer learned patterns from another project',
    parameters: z.object({
        sourceProject: z.string().describe('Path to source project'),
        minConfidence: z.number().optional().default(0.5).describe('Minimum pattern score to transfer'),
        maxPatterns: z.number().optional().default(50).describe('Maximum patterns to transfer'),
        mode: z.enum(['merge', 'replace', 'additive']).optional().default('merge')
            .describe('Transfer mode: merge combines, replace overwrites, additive only adds new')
    }),
    execute: async ({ sourceProject, minConfidence, maxPatterns, mode }, { onProgress }) => {
        const startTime = Date.now();
        onProgress?.({ progress: 0.1, message: 'Loading source intelligence...' });
        // Load source intelligence
        const sourcePath = path.join(sourceProject, INTELLIGENCE_PATH);
        if (!fs.existsSync(sourcePath)) {
            throw new Error(`Source intelligence not found at ${sourcePath}. Run pretrain on source project first.`);
        }
        let sourceIntel;
        try {
            sourceIntel = JSON.parse(fs.readFileSync(sourcePath, 'utf-8'));
        }
        catch (e) {
            throw new Error(`Failed to parse source intelligence: ${e}`);
        }
        // Load target (current) intelligence
        const targetIntel = loadIntelligence();
        onProgress?.({ progress: 0.3, message: 'Analyzing patterns...' });
        // Track transfer stats
        const stats = {
            patternsTransferred: 0,
            memoriesTransferred: 0,
            sequencesTransferred: 0,
            errorsTransferred: 0,
            skipped: 0
        };
        // Transfer patterns
        const patternsToTransfer = [];
        for (const [state, agents] of Object.entries(sourceIntel.patterns || {})) {
            for (const [agent, score] of Object.entries(agents)) {
                if (typeof score === 'number' && score >= minConfidence) {
                    patternsToTransfer.push({ state, agent, score });
                }
            }
        }
        // Sort by score and limit
        patternsToTransfer.sort((a, b) => b.score - a.score);
        const selectedPatterns = patternsToTransfer.slice(0, maxPatterns);
        onProgress?.({ progress: 0.5, message: 'Transferring patterns...' });
        // Apply patterns based on mode
        for (const pattern of selectedPatterns) {
            if (!targetIntel.patterns[pattern.state]) {
                targetIntel.patterns[pattern.state] = {};
            }
            const currentValue = targetIntel.patterns[pattern.state][pattern.agent] || 0;
            switch (mode) {
                case 'replace':
                    // Overwrite with source value (reduced confidence)
                    targetIntel.patterns[pattern.state][pattern.agent] = pattern.score * 0.7;
                    stats.patternsTransferred++;
                    break;
                case 'additive':
                    // Only add if not present
                    if (currentValue === 0) {
                        targetIntel.patterns[pattern.state][pattern.agent] = pattern.score * 0.5;
                        stats.patternsTransferred++;
                    }
                    else {
                        stats.skipped++;
                    }
                    break;
                case 'merge':
                default:
                    // Weighted average
                    targetIntel.patterns[pattern.state][pattern.agent] =
                        currentValue * 0.6 + pattern.score * 0.4;
                    stats.patternsTransferred++;
                    break;
            }
        }
        onProgress?.({ progress: 0.7, message: 'Transferring memories...' });
        // Transfer high-quality memories
        if (sourceIntel.memories && sourceIntel.memories.length > 0) {
            const existingContents = new Set(targetIntel.memories.map(m => m.content.slice(0, 50)));
            for (const mem of sourceIntel.memories) {
                // Skip duplicates
                if (existingContents.has(mem.content.slice(0, 50))) {
                    continue;
                }
                // Only transfer project-type memories (not ephemeral)
                if (mem.type === 'project' || mem.type === 'success') {
                    targetIntel.memories.push({
                        ...mem,
                        type: 'transferred',
                        created: new Date().toISOString(),
                        embedding: mem.embedding || simpleEmbed(mem.content)
                    });
                    stats.memoriesTransferred++;
                    if (stats.memoriesTransferred >= 20)
                        break; // Limit
                }
            }
        }
        // Transfer error patterns (valuable for learning)
        if (sourceIntel.errorPatterns && sourceIntel.errorPatterns.length > 0) {
            const existingErrors = new Set(targetIntel.errorPatterns.map(e => e.errorType));
            for (const ep of sourceIntel.errorPatterns) {
                if (!existingErrors.has(ep.errorType) && ep.resolution) {
                    targetIntel.errorPatterns.push(ep);
                    stats.errorsTransferred++;
                    if (stats.errorsTransferred >= 10)
                        break; // Limit
                }
            }
        }
        onProgress?.({ progress: 0.9, message: 'Saving intelligence...' });
        // Save updated intelligence
        saveIntelligence(targetIntel);
        onProgress?.({ progress: 1.0, message: 'Transfer complete!' });
        const latency = Date.now() - startTime;
        return {
            success: true,
            sourceProject,
            mode,
            transferred: {
                patterns: stats.patternsTransferred,
                memories: stats.memoriesTransferred,
                sequences: stats.sequencesTransferred,
                errorPatterns: stats.errorsTransferred
            },
            skipped: stats.skipped,
            targetStats: {
                totalPatterns: Object.keys(targetIntel.patterns).length,
                totalMemories: targetIntel.memories.length,
                totalErrors: targetIntel.errorPatterns.length
            },
            latencyMs: latency,
            timestamp: new Date().toISOString()
        };
    }
};
//# sourceMappingURL=transfer.js.map