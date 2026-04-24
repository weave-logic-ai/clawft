/**
 * Telemetry integration helpers
 *
 * Provides integration utilities for adding telemetry to AgentDB components.
 */

import { Span } from '@opentelemetry/api';
import { TelemetryManager, recordMetric, withSpan } from './telemetry';

/**
 * Wrap a database operation with telemetry
 */
export async function withTelemetry<T>(
  operationType: string,
  tableName: string,
  operation: () => Promise<T>
): Promise<T> {
  const startTime = Date.now();
  const telemetry = TelemetryManager.getInstance();

  return withSpan(
    `agentdb.${operationType}`,
    async (span) => {
      if (span) {
        span.setAttribute('operation_type', operationType);
        span.setAttribute('table_name', tableName);
      }

      try {
        const result = await operation();
        const latency = Date.now() - startTime;

        // Record metrics
        recordMetric('query', {
          latencyMs: latency,
          operationType,
          tableName,
          success: true,
        });

        recordMetric('operation', {
          operationType,
          tableName,
          resultSize: JSON.stringify(result).length,
        });

        if (span) {
          span.setAttribute('result_size', JSON.stringify(result).length);
        }

        return result;
      } catch (error) {
        const latency = Date.now() - startTime;

        // Record error metrics
        recordMetric('query', {
          latencyMs: latency,
          operationType,
          tableName,
          success: false,
        });

        recordMetric('error', {
          errorType: error instanceof Error ? error.name : 'UnknownError',
          operation: operationType,
          message: error instanceof Error ? error.message : 'Unknown error',
        });

        throw error;
      }
    },
    {
      component: 'agentdb',
      version: '2.0.0-alpha',
    }
  );
}

/**
 * Wrap a batch operation with telemetry
 */
export async function withBatchTelemetry<T>(
  operationType: string,
  tableName: string,
  batchSize: number,
  operation: () => Promise<T>
): Promise<T> {
  return withSpan(`agentdb.batch.${operationType}`, async (span) => {
    const startTime = Date.now();

    if (span) {
      span.setAttribute('operation_type', operationType);
      span.setAttribute('table_name', tableName);
      span.setAttribute('batch_size', batchSize);
    }

    try {
      const result = await operation();
      const latency = Date.now() - startTime;

      // Record batch metrics
      recordMetric('query', {
        latencyMs: latency,
        operationType: `batch_${operationType}`,
        tableName,
        success: true,
      });

      recordMetric('operation', {
        operationType: `batch_${operationType}`,
        tableName,
        resultSize: batchSize,
      });

      if (span) {
        span.setAttribute('latency_ms', latency);
        span.setAttribute('throughput', batchSize / (latency / 1000));
      }

      return result;
    } catch (error) {
      const latency = Date.now() - startTime;

      recordMetric('error', {
        errorType: error instanceof Error ? error.name : 'UnknownError',
        operation: `batch_${operationType}`,
        message: error instanceof Error ? error.message : 'Unknown error',
      });

      throw error;
    }
  });
}

/**
 * Record cache access with telemetry
 */
export function recordCacheAccess(key: string, hit: boolean): void {
  if (hit) {
    recordMetric('cache_hit', { key });
  } else {
    recordMetric('cache_miss', { key });
  }
}

/**
 * Create span attributes from operation context
 */
export function createSpanAttributes(context: {
  operation?: string;
  table?: string;
  filters?: Record<string, any>;
  limit?: number;
  offset?: number;
}): Record<string, any> {
  const attributes: Record<string, any> = {};

  if (context.operation) {
    attributes['db.operation'] = context.operation;
  }
  if (context.table) {
    attributes['db.table'] = context.table;
  }
  if (context.filters) {
    attributes['db.filters'] = JSON.stringify(context.filters);
  }
  if (context.limit !== undefined) {
    attributes['db.limit'] = context.limit;
  }
  if (context.offset !== undefined) {
    attributes['db.offset'] = context.offset;
  }

  return attributes;
}

/**
 * Enhanced error recording with context
 */
export function recordErrorWithContext(
  error: Error,
  operation: string,
  context?: Record<string, any>
): void {
  recordMetric('error', {
    errorType: error.name,
    operation,
    message: error.message,
  });

  const telemetry = TelemetryManager.getInstance();
  const span = telemetry.startSpan(`error.${operation}`);

  if (span) {
    span.setAttribute('error.type', error.name);
    span.setAttribute('error.message', error.message);
    span.setAttribute('error.stack', error.stack || '');

    if (context) {
      Object.entries(context).forEach(([key, value]) => {
        span.setAttribute(`context.${key}`, JSON.stringify(value));
      });
    }

    span.recordException(error);
    span.end();
  }
}
