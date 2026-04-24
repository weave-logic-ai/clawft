/**
 * Explain Hook - Explain routing decisions with full transparency
 */
import { z } from 'zod';
import * as path from 'path';
import { loadIntelligence, getAgentForFile, simpleEmbed, cosineSimilarity } from './shared.js';
export const hookExplainTool = {
    name: 'hook_explain',
    description: 'Explain why an agent would be selected for a task',
    parameters: z.object({
        task: z.string().describe('Task description'),
        file: z.string().optional().describe('Optional file context'),
        agent: z.string().optional().describe('Specific agent to explain selection for')
    }),
    execute: async ({ task, file, agent }, { onProgress }) => {
        const startTime = Date.now();
        const intel = loadIntelligence();
        // Collect all reasoning
        const reasons = [];
        // All agent scores
        const allScores = {};
        // 1. File pattern analysis
        if (file) {
            const ext = path.extname(file);
            const suggestedAgent = getAgentForFile(file);
            reasons.push({
                factor: 'File Extension',
                weight: 2.0,
                evidence: `${ext} files are typically handled by ${suggestedAgent}`,
                contributes: agent === suggestedAgent ? 'positive' : 'neutral'
            });
            allScores[suggestedAgent] = (allScores[suggestedAgent] || 0) + 2.0;
            // Learned patterns
            const state = `edit:${ext}`;
            if (intel.patterns[state]) {
                const patterns = intel.patterns[state];
                const sortedPatterns = Object.entries(patterns)
                    .sort((a, b) => b[1] - a[1]);
                for (const [patternAgent, value] of sortedPatterns.slice(0, 3)) {
                    if (typeof value === 'number') {
                        allScores[patternAgent] = (allScores[patternAgent] || 0) + value;
                        reasons.push({
                            factor: 'Learned Pattern',
                            weight: value,
                            evidence: `Historical Q-value of ${value.toFixed(2)} for ${patternAgent} on ${ext} files`,
                            contributes: agent === patternAgent
                                ? (value > 0 ? 'positive' : 'negative')
                                : 'neutral'
                        });
                    }
                }
            }
        }
        // 2. Task keyword analysis
        const taskLower = task.toLowerCase();
        const keywordMatches = [];
        const keywordMap = {
            'test-engineer': ['test', 'spec', 'coverage'],
            'documentation-specialist': ['document', 'readme', 'docs'],
            'security-specialist': ['security', 'auth', 'vulnerability'],
            'database-specialist': ['database', 'sql', 'query'],
            'optimizer': ['optimize', 'performance', 'speed'],
            'researcher': ['research', 'find', 'explore']
        };
        for (const [agentType, keywords] of Object.entries(keywordMap)) {
            for (const keyword of keywords) {
                if (taskLower.includes(keyword)) {
                    keywordMatches.push(keyword);
                    allScores[agentType] = (allScores[agentType] || 0) + 1.5;
                    reasons.push({
                        factor: 'Keyword Match',
                        weight: 1.5,
                        evidence: `Task mentions "${keyword}" → suggests ${agentType}`,
                        contributes: agent === agentType ? 'positive' : 'neutral'
                    });
                }
            }
        }
        // 3. Memory similarity analysis
        if (intel.memories.length > 0) {
            const taskEmbed = simpleEmbed(task);
            const similarMemories = [];
            for (const mem of intel.memories) {
                if (mem.embedding) {
                    const similarity = cosineSimilarity(taskEmbed, mem.embedding);
                    if (similarity > 0.3) {
                        similarMemories.push({
                            content: mem.content.slice(0, 100),
                            similarity
                        });
                    }
                }
            }
            if (similarMemories.length > 0) {
                similarMemories.sort((a, b) => b.similarity - a.similarity);
                const topMemory = similarMemories[0];
                reasons.push({
                    factor: 'Memory Similarity',
                    weight: topMemory.similarity,
                    evidence: `Similar past task: "${topMemory.content}..."`,
                    contributes: 'positive'
                });
            }
        }
        // 4. Error pattern analysis
        const relevantErrors = intel.errorPatterns.filter(ep => task.toLowerCase().includes(ep.errorType.toLowerCase()) ||
            ep.context.toLowerCase().includes(task.toLowerCase().split(' ')[0]));
        if (relevantErrors.length > 0) {
            for (const ep of relevantErrors.slice(0, 2)) {
                const bestAgent = Object.entries(ep.agentSuccess)
                    .sort((a, b) => b[1] - a[1])[0];
                if (bestAgent) {
                    reasons.push({
                        factor: 'Error History',
                        weight: 1.0,
                        evidence: `${bestAgent[0]} has fixed ${ep.errorType} ${bestAgent[1]} times`,
                        contributes: agent === bestAgent[0] ? 'positive' : 'neutral'
                    });
                }
            }
        }
        // 5. Calculate final scores and ranking
        const ranking = Object.entries(allScores)
            .sort((a, b) => b[1] - a[1])
            .map(([agentName, score], index) => ({
            rank: index + 1,
            agent: agentName,
            score,
            isRequested: agent === agentName
        }));
        // 6. Generate explanation summary
        const topAgent = ranking[0];
        const requestedAgentRank = agent
            ? ranking.find(r => r.agent === agent)
            : null;
        let summary;
        if (!agent) {
            summary = `${topAgent?.agent || 'coder'} is recommended with score ${topAgent?.score.toFixed(2) || 0}`;
        }
        else if (requestedAgentRank?.rank === 1) {
            summary = `${agent} is the top choice with score ${requestedAgentRank.score.toFixed(2)}`;
        }
        else if (requestedAgentRank) {
            summary = `${agent} ranks #${requestedAgentRank.rank} with score ${requestedAgentRank.score.toFixed(2)}. ${topAgent?.agent} is preferred.`;
        }
        else {
            summary = `${agent} has no specific advantages for this task. Consider ${topAgent?.agent || 'coder'}.`;
        }
        const latency = Date.now() - startTime;
        return {
            success: true,
            summary,
            reasons,
            ranking: ranking.slice(0, 5),
            requestedAgent: agent || null,
            recommendedAgent: topAgent?.agent || 'coder',
            memoryCount: intel.memories.length,
            patternCount: Object.keys(intel.patterns).length,
            latencyMs: latency,
            timestamp: new Date().toISOString()
        };
    }
};
//# sourceMappingURL=explain.js.map