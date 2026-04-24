import { ResourceMetrics } from '@opentelemetry/sdk-metrics';
import { OTLPMetricExporterOptions } from '../../OTLPMetricExporterOptions';
import { OTLPMetricExporterBase } from '../../OTLPMetricExporterBase';
import { OTLPExporterNodeBase, OTLPExporterNodeConfigBase } from '@opentelemetry/otlp-exporter-base';
import { IExportMetricsServiceResponse } from '@opentelemetry/otlp-transformer';
declare class OTLPExporterNodeProxy extends OTLPExporterNodeBase<ResourceMetrics, IExportMetricsServiceResponse> {
    constructor(config?: OTLPExporterNodeConfigBase & OTLPMetricExporterOptions);
    getDefaultUrl(config: OTLPExporterNodeConfigBase): string;
}
/**
 * Collector Metric Exporter for Node
 */
export declare class OTLPMetricExporter extends OTLPMetricExporterBase<OTLPExporterNodeProxy> {
    constructor(config?: OTLPExporterNodeConfigBase & OTLPMetricExporterOptions);
}
export {};
//# sourceMappingURL=OTLPMetricExporter.d.ts.map