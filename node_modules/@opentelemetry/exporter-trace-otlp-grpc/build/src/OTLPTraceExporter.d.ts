import { ReadableSpan, SpanExporter } from '@opentelemetry/sdk-trace-base';
import { OTLPGRPCExporterConfigNode, OTLPGRPCExporterNodeBase } from '@opentelemetry/otlp-grpc-exporter-base';
import { IExportTraceServiceResponse } from '@opentelemetry/otlp-transformer';
/**
 * OTLP Trace Exporter for Node
 */
export declare class OTLPTraceExporter extends OTLPGRPCExporterNodeBase<ReadableSpan, IExportTraceServiceResponse> implements SpanExporter {
    constructor(config?: OTLPGRPCExporterConfigNode);
    getDefaultUrl(config: OTLPGRPCExporterConfigNode): string;
    getUrlFromConfig(config: OTLPGRPCExporterConfigNode): string;
}
//# sourceMappingURL=OTLPTraceExporter.d.ts.map