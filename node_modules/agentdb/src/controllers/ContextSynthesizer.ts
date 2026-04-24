/**
 * Context Synthesis - Generate coherent narratives from multiple memories
 *
 * Takes a collection of retrieved episodes/patterns and synthesizes
 * a coherent context summary with extracted patterns, success rates,
 * and actionable insights.
 */

export interface MemoryPattern {
  task: string;
  reward: number;
  success: boolean;
  critique?: string;
  input?: string;
  output?: string;
  similarity?: number;
  [key: string]: any;
}

export interface SynthesizedContext {
  summary: string;
  patterns: string[];
  successRate: number;
  averageReward: number;
  recommendations: string[];
  keyInsights: string[];
  totalMemories: number;
}

export class ContextSynthesizer {
  /**
   * Synthesize context from multiple memories
   *
   * @param memories - Retrieved episodes/patterns
   * @param options - Synthesis options
   * @returns Synthesized context with insights
   */
  static synthesize(
    memories: MemoryPattern[],
    options: {
      minPatternFrequency?: number;
      includeRecommendations?: boolean;
      maxSummaryLength?: number;
    } = {}
  ): SynthesizedContext {
    const minPatternFrequency = options.minPatternFrequency ?? 2;
    const includeRecommendations = options.includeRecommendations ?? true;

    if (memories.length === 0) {
      return {
        summary: 'No relevant memories found.',
        patterns: [],
        successRate: 0,
        averageReward: 0,
        recommendations: [],
        keyInsights: [],
        totalMemories: 0,
      };
    }

    // Calculate statistics
    const successCount = memories.filter(m => m.success).length;
    const successRate = successCount / memories.length;
    const averageReward = memories.reduce((sum, m) => sum + (m.reward || 0), 0) / memories.length;

    // Extract common patterns from critiques
    const patterns = this.extractPatterns(memories, minPatternFrequency);

    // Generate key insights
    const keyInsights = this.generateKeyInsights(memories, successRate, averageReward);

    // Generate recommendations
    const recommendations = includeRecommendations
      ? this.generateRecommendations(memories, patterns, successRate)
      : [];

    // Generate summary narrative
    const summary = this.generateSummary(memories, patterns, successRate, averageReward);

    return {
      summary,
      patterns,
      successRate,
      averageReward,
      recommendations,
      keyInsights,
      totalMemories: memories.length,
    };
  }

  /**
   * Extract common patterns from memory critiques
   */
  private static extractPatterns(memories: MemoryPattern[], minFrequency: number): string[] {
    const phraseCount = new Map<string, number>();

    // Extract meaningful phrases from critiques
    for (const memory of memories) {
      if (memory.critique) {
        const phrases = this.extractPhrases(memory.critique);
        for (const phrase of phrases) {
          phraseCount.set(phrase, (phraseCount.get(phrase) || 0) + 1);
        }
      }
    }

    // Filter by minimum frequency
    const patterns: string[] = [];
    for (const [phrase, count] of phraseCount.entries()) {
      if (count >= minFrequency) {
        patterns.push(`${phrase} (${count}/${memories.length} times)`);
      }
    }

    return patterns.slice(0, 10);  // Top 10 patterns
  }

  /**
   * Extract meaningful phrases from text
   */
  private static extractPhrases(text: string): string[] {
    // Simple phrase extraction (can be enhanced with NLP)
    const sentences = text.split(/[.!?]/).filter(s => s.trim().length > 0);
    const phrases: string[] = [];

    for (const sentence of sentences) {
      const trimmed = sentence.trim();
      // Extract phrases with key action words
      if (
        trimmed.match(/\b(use|implement|fix|improve|add|create|design|test)\b/i) &&
        trimmed.length > 10 &&
        trimmed.length < 100
      ) {
        phrases.push(trimmed.toLowerCase());
      }
    }

    return phrases;
  }

  /**
   * Generate key insights from memories
   */
  private static generateKeyInsights(
    memories: MemoryPattern[],
    successRate: number,
    averageReward: number
  ): string[] {
    const insights: string[] = [];

    // Success rate insight
    if (successRate >= 0.8) {
      insights.push(`High success rate (${(successRate * 100).toFixed(0)}%) indicates strong pattern match`);
    } else if (successRate < 0.5) {
      insights.push(`Low success rate (${(successRate * 100).toFixed(0)}%) suggests exploring alternative approaches`);
    }

    // Reward insight
    if (averageReward >= 0.8) {
      insights.push(`High average reward (${averageReward.toFixed(2)}) shows effective past solutions`);
    } else if (averageReward < 0.6) {
      insights.push(`Moderate reward (${averageReward.toFixed(2)}) indicates room for improvement`);
    }

    // Identify high-performing memories
    const topMemories = memories
      .filter(m => m.success && (m.reward || 0) >= 0.9)
      .slice(0, 3);

    if (topMemories.length > 0) {
      insights.push(`${topMemories.length} exemplary solution(s) found with reward ≥0.9`);
    }

    // Task diversity
    const uniqueTasks = new Set(memories.map(m => m.task)).size;
    if (uniqueTasks > 1) {
      insights.push(`${uniqueTasks} different task types provide diverse perspectives`);
    }

    return insights;
  }

  /**
   * Generate actionable recommendations
   */
  private static generateRecommendations(
    memories: MemoryPattern[],
    patterns: string[],
    successRate: number
  ): string[] {
    const recommendations: string[] = [];

    // Extract successful strategies
    const successfulMemories = memories.filter(m => m.success && (m.reward || 0) >= 0.8);

    if (successfulMemories.length > 0) {
      recommendations.push('Apply strategies from high-reward solutions');
    }

    // Pattern-based recommendations
    if (patterns.length > 0) {
      recommendations.push(`Follow common patterns: ${patterns[0].split(' (')[0]}`);
    }

    // Success rate recommendations
    if (successRate >= 0.7) {
      recommendations.push('Previous approaches were effective - follow similar methodology');
    } else {
      recommendations.push('Consider alternative approaches given mixed past results');
    }

    // Add general recommendations
    if (memories.length >= 5) {
      recommendations.push('Sufficient historical data available for confident decision-making');
    } else {
      recommendations.push('Limited data - proceed with caution and validate assumptions');
    }

    return recommendations.slice(0, 5);
  }

  /**
   * Generate narrative summary
   */
  private static generateSummary(
    memories: MemoryPattern[],
    patterns: string[],
    successRate: number,
    averageReward: number
  ): string {
    const parts: string[] = [];

    // Opening
    parts.push(`Based on ${memories.length} similar past ${memories.length === 1 ? 'experience' : 'experiences'}`);

    // Success rate
    const successPercent = (successRate * 100).toFixed(0);
    if (successRate >= 0.7) {
      parts.push(`with a high success rate of ${successPercent}%`);
    } else {
      parts.push(`with a ${successPercent}% success rate`);
    }

    // Average reward
    parts.push(`and average reward of ${averageReward.toFixed(2)}`);

    // Key patterns
    if (patterns.length > 0) {
      parts.push('. Common effective approaches include:');
      const topPatterns = patterns.slice(0, 3).map(p => p.split(' (')[0]);
      parts.push(topPatterns.map((p, i) => `${i + 1}) ${p}`).join(', '));
    }

    // High performers
    const topMemories = memories.filter(m => m.success && (m.reward || 0) >= 0.9);
    if (topMemories.length > 0) {
      parts.push(`. ${topMemories.length} exemplary solution(s) achieved reward ≥0.9`);
    }

    parts.push('.');

    return parts.join(' ');
  }

  /**
   * Extract actionable steps from successful memories
   */
  static extractActionableSteps(memories: MemoryPattern[]): string[] {
    const steps: string[] = [];
    const successfulMemories = memories.filter(m => m.success && (m.reward || 0) >= 0.8);

    for (const memory of successfulMemories) {
      if (memory.critique) {
        // Extract step-like phrases
        const matches = memory.critique.match(/\b\d+\.\s+([^.!?]+)/g);
        if (matches) {
          steps.push(...matches.map(m => m.trim()));
        }
      }
    }

    // Remove duplicates and return top 5
    return Array.from(new Set(steps)).slice(0, 5);
  }
}
