/**
 * Example: BatchOperations Telemetry Integration
 *
 * Shows how to integrate OpenTelemetry observability into batch operations.
 */

import { traced, recordMetric, withBatchTelemetry } from '../src/observability';

class BatchOperationsWithTelemetry {
  @traced('batch.insert-episodes', {
    recordMetrics: true,
    attributes: { component: 'batch-operations' },
  })
  async insertEpisodes(episodes: any[]): Promise<number> {
    return withBatchTelemetry('insert', 'episodes', episodes.length, async () => {
      // Batch insert logic with transaction
      let completed = 0;

      // Process in batches
      const batchSize = 100;
      for (let i = 0; i < episodes.length; i += batchSize) {
        const batch = episodes.slice(i, i + batchSize);

        // Insert batch with telemetry
        // (already tracked by withBatchTelemetry)
        completed += batch.length;

        // Record progress
        recordMetric('operation', {
          operationType: 'batch_insert_progress',
          tableName: 'episodes',
          resultSize: completed,
        });
      }

      return completed;
    });
  }

  @traced('batch.insert-skills', { recordMetrics: true })
  async insertSkills(skills: any[]): Promise<number[]> {
    return withBatchTelemetry('insert', 'skills', skills.length, async () => {
      const skillIds: number[] = [];

      // Batch insert logic...

      recordMetric('operation', {
        operationType: 'batch_insert_skills',
        tableName: 'skills',
        resultSize: skillIds.length,
      });

      return skillIds;
    });
  }

  /**
   * Example: Parallel batch operations with telemetry
   */
  @traced('batch.parallel-insert', {
    recordMetrics: true,
    attributes: { parallel: true },
  })
  async batchInsertParallel(
    table: string,
    data: any[],
    columns: string[],
    config: any = {}
  ): Promise<any> {
    const startTime = Date.now();

    const result = await withBatchTelemetry('parallel_insert', table, data.length, async () => {
      // Parallel batch processing...
      const totalInserted = data.length;
      const chunksProcessed = Math.ceil(data.length / (config.chunkSize || 1000));

      // Record throughput metric
      const duration = Date.now() - startTime;
      const throughput = totalInserted / (duration / 1000); // rows per second

      recordMetric('operation', {
        operationType: 'parallel_batch_insert',
        tableName: table,
        resultSize: totalInserted,
      });

      return {
        totalInserted,
        chunksProcessed,
        duration,
        errors: [],
      };
    });

    // Record batch operation statistics
    recordMetric('query', {
      latencyMs: result.duration,
      operationType: 'parallel_batch_insert',
      tableName: table,
      success: result.errors.length === 0,
    });

    return result;
  }

  /**
   * Example: Data pruning with telemetry
   */
  @traced('batch.prune-data', { recordMetrics: true })
  async pruneData(config: any = {}): Promise<any> {
    const startTime = Date.now();

    const result = {
      episodesPruned: 0,
      skillsPruned: 0,
      patternsPruned: 0,
      spaceSaved: 0,
    };

    // Prune episodes with telemetry
    await withTelemetry('prune_episodes', 'episodes', async () => {
      result.episodesPruned = 100; // placeholder
    });

    // Prune skills with telemetry
    await withTelemetry('prune_skills', 'skills', async () => {
      result.skillsPruned = 50; // placeholder
    });

    // Record pruning metrics
    const duration = Date.now() - startTime;
    recordMetric('operation', {
      operationType: 'data_pruning',
      tableName: 'all',
      resultSize: result.episodesPruned + result.skillsPruned + result.patternsPruned,
    });

    recordMetric('query', {
      latencyMs: duration,
      operationType: 'data_pruning',
      success: true,
    });

    return result;
  }

  /**
   * Example: Database optimization with telemetry
   */
  @traced('batch.optimize-database', { recordMetrics: true })
  optimize(): void {
    const startTime = Date.now();

    // ANALYZE with telemetry
    recordMetric('operation', {
      operationType: 'analyze',
      tableName: 'all',
    });

    // VACUUM with telemetry
    const duration = Date.now() - startTime;
    recordMetric('query', {
      latencyMs: duration,
      operationType: 'optimize',
      success: true,
    });
  }
}

/**
 * Integration Summary for BatchOperations:
 *
 * 1. Add @traced decorator to:
 *    - insertEpisodes()
 *    - insertSkills()
 *    - insertPatterns()
 *    - batchInsertParallel()
 *    - pruneData()
 *    - optimize()
 *
 * 2. Use withBatchTelemetry() for:
 *    - All batch insert operations
 *    - Parallel processing
 *
 * 3. Record custom metrics for:
 *    - Throughput (rows/second)
 *    - Batch completion percentage
 *    - Space saved from pruning
 *    - Optimization duration
 */

export { BatchOperationsWithTelemetry };
