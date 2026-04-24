/**
 * AgentDB Observability
 *
 * OpenTelemetry-based observability for production monitoring.
 */

export { TelemetryManager, TelemetryConfig, recordMetric, traced, withSpan } from './telemetry';

export {
  withTelemetry,
  withBatchTelemetry,
  recordCacheAccess,
  createSpanAttributes,
  recordErrorWithContext,
} from './integration';
