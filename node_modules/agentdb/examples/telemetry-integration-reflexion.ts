/**
 * Example: ReflexionMemory Telemetry Integration
 *
 * Shows how to integrate OpenTelemetry observability into ReflexionMemory.
 * This is a reference implementation - actual integration should be done
 * by modifying the original files.
 */

import { traced, recordMetric, withTelemetry } from '../src/observability';

/**
 * Example: Integrating telemetry into storeEpisode
 */
class ReflexionMemoryWithTelemetry {
  @traced('reflexion.store-episode', {
    recordMetrics: true,
    attributes: { component: 'reflexion-memory' },
  })
  async storeEpisode(episode: any): Promise<number> {
    // The @traced decorator automatically:
    // - Creates a span for this operation
    // - Records execution time
    // - Captures errors
    // - Records metrics (latency, success/failure)

    // Original implementation...
    const episodeId = 123; // placeholder

    // Optionally record custom metrics
    recordMetric('operation', {
      operationType: 'store_episode',
      tableName: 'episodes',
      resultSize: 1,
    });

    return episodeId;
  }

  /**
   * Example: Using withTelemetry helper
   */
  async retrieveRelevant(query: any): Promise<any[]> {
    return withTelemetry('retrieve_relevant', 'episodes', async () => {
      // Your retrieval logic here
      const results: any[] = [];

      // The withTelemetry wrapper automatically:
      // - Times the operation
      // - Records metrics
      // - Handles errors
      // - Creates distributed traces

      return results;
    });
  }

  /**
   * Example: Manual telemetry for cache operations
   */
  async getCachedData(key: string): Promise<any> {
    const cached = this.cache.get(key);

    // Record cache hit/miss
    recordMetric(cached ? 'cache_hit' : 'cache_miss', { key });

    if (cached) {
      return cached;
    }

    // Fetch from database with telemetry
    return withTelemetry('fetch_episode', 'episodes', async () => {
      const data = await this.fetchFromDatabase(key);
      this.cache.set(key, data);
      return data;
    });
  }

  /**
   * Example: Error handling with telemetry
   */
  async processEpisode(episode: any): Promise<void> {
    try {
      await withTelemetry('process_episode', 'episodes', async () => {
        // Processing logic...
      });
    } catch (error) {
      // Error is automatically recorded by withTelemetry
      // But you can add custom context if needed
      recordMetric('error', {
        errorType: 'ProcessingError',
        operation: 'process_episode',
        message: error instanceof Error ? error.message : 'Unknown error',
      });
      throw error;
    }
  }

  // Placeholder methods
  private cache = new Map();
  private async fetchFromDatabase(key: string): Promise<any> {
    return null;
  }
}

/**
 * Integration Summary for ReflexionMemory:
 *
 * 1. Add @traced decorator to key methods:
 *    - storeEpisode()
 *    - retrieveRelevant()
 *    - getCritiqueSummary()
 *    - getSuccessStrategies()
 *    - getTaskStats()
 *
 * 2. Use withTelemetry() for database operations:
 *    - All SQL queries
 *    - Vector searches
 *    - Graph queries
 *
 * 3. Record cache metrics:
 *    - Cache hits/misses in QueryCache
 *    - Use recordCacheAccess() helper
 *
 * 4. Custom metrics for:
 *    - Episode count by session
 *    - Embedding generation time
 *    - GNN enhancement duration
 */

export { ReflexionMemoryWithTelemetry };
