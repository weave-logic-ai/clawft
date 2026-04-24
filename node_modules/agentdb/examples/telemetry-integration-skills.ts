/**
 * Example: SkillLibrary Telemetry Integration
 *
 * Shows how to integrate OpenTelemetry observability into SkillLibrary.
 */

import { traced, recordMetric, withTelemetry } from '../src/observability';

class SkillLibraryWithTelemetry {
  @traced('skills.create-skill', {
    recordMetrics: true,
    attributes: { component: 'skill-library' },
  })
  async createSkill(skill: any): Promise<number> {
    // Automatic tracing and metrics
    const skillId = 456; // placeholder

    recordMetric('operation', {
      operationType: 'create_skill',
      tableName: 'skills',
      resultSize: 1,
    });

    return skillId;
  }

  @traced('skills.retrieve-skills', { recordMetrics: true })
  async retrieveSkills(query: any): Promise<any[]> {
    // Check cache first
    const cacheKey = `skill:${query.task}`;
    const cached = this.cache.get(cacheKey);

    recordMetric(cached ? 'cache_hit' : 'cache_miss', { key: cacheKey });

    if (cached) {
      return cached;
    }

    // Fetch from database with telemetry
    return withTelemetry('search_skills', 'skills', async () => {
      const results: any[] = [];

      // Vector search with telemetry
      recordMetric('operation', {
        operationType: 'vector_search',
        tableName: 'skills',
        resultSize: results.length,
      });

      return results;
    });
  }

  @traced('skills.update-stats', { recordMetrics: true })
  updateSkillStats(skillId: number, success: boolean, reward: number, latencyMs: number): void {
    // Update stats with telemetry
    recordMetric('operation', {
      operationType: 'update_stats',
      tableName: 'skills',
      resultSize: 1,
    });
  }

  /**
   * Example: Batch operations with telemetry
   */
  @traced('skills.consolidate-episodes', {
    recordMetrics: true,
    attributes: { operation: 'consolidation' },
  })
  async consolidateEpisodesIntoSkills(config: any): Promise<any> {
    const startTime = Date.now();

    const result = await withTelemetry('consolidate_episodes', 'episodes', async () => {
      // Consolidation logic...
      return {
        created: 10,
        updated: 5,
        patterns: [],
      };
    });

    // Record consolidation metrics
    recordMetric('operation', {
      operationType: 'consolidation',
      tableName: 'skills',
      resultSize: result.created + result.updated,
    });

    const duration = Date.now() - startTime;
    recordMetric('query', {
      latencyMs: duration,
      operationType: 'consolidation',
      tableName: 'skills',
      success: true,
    });

    return result;
  }

  /**
   * Example: Pattern extraction with telemetry
   */
  private async extractPatternsFromEpisodes(episodeIds: number[]): Promise<any> {
    return withTelemetry('extract_patterns', 'episodes', async () => {
      // Pattern extraction logic...

      // Record custom metric for pattern quality
      const patterns = { commonPatterns: [], successIndicators: [] };

      recordMetric('operation', {
        operationType: 'pattern_extraction',
        tableName: 'episodes',
        resultSize: patterns.commonPatterns.length,
      });

      return patterns;
    });
  }

  // Placeholder
  private cache = new Map();
}

/**
 * Integration Summary for SkillLibrary:
 *
 * 1. Add @traced decorator to:
 *    - createSkill()
 *    - retrieveSkills()
 *    - updateSkillStats()
 *    - consolidateEpisodesIntoSkills()
 *    - pruneSkills()
 *
 * 2. Use withTelemetry() for:
 *    - Vector searches
 *    - SQL queries
 *    - Pattern extraction
 *
 * 3. Record custom metrics for:
 *    - Skill consolidation results
 *    - Pattern extraction quality
 *    - Skill usage statistics
 */

export { SkillLibraryWithTelemetry };
