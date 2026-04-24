import { ReadableSpan, SpanExporter } from '@opentelemetry/sdk-trace-base';
import { OTLPExporterNodeConfigBase, OTLPExporterNodeBase } from '@opentelemetry/otlp-exporter-base';
import { IExportTraceServiceResponse } from '@opentelemetry/otlp-transformer';
/**
 * Collector Trace Exporter for Node with protobuf
 */
export declare class OTLPTraceExporter extends OTLPExporterNodeBase<ReadableSpan, IExportTraceServiceResponse> implements SpanExporter {
    constructor(config?: OTLPExporterNodeConfigBase);
    getDefaultUrl(config: OTLPExporterNodeConfigBase): string;
}
//# sourceMappingURL=OTLPTraceExporter.d.ts.map