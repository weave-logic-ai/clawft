/**
 * AgentDB Observability
 *
 * OpenTelemetry-based observability for production monitoring.
 */
export { TelemetryManager, recordMetric, traced, withSpan } from './telemetry';
export { withTelemetry, withBatchTelemetry, recordCacheAccess, createSpanAttributes, recordErrorWithContext, } from './integration';
//# sourceMappingURL=index.js.map