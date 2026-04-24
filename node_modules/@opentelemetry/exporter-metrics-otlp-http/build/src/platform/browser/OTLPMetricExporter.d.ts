import { ResourceMetrics } from '@opentelemetry/sdk-metrics';
import { OTLPMetricExporterOptions } from '../../OTLPMetricExporterOptions';
import { OTLPMetricExporterBase } from '../../OTLPMetricExporterBase';
import { OTLPExporterBrowserBase, OTLPExporterConfigBase } from '@opentelemetry/otlp-exporter-base';
import { IExportMetricsServiceResponse } from '@opentelemetry/otlp-transformer';
declare class OTLPExporterBrowserProxy extends OTLPExporterBrowserBase<ResourceMetrics, IExportMetricsServiceResponse> {
    constructor(config?: OTLPMetricExporterOptions & OTLPExporterConfigBase);
    getDefaultUrl(config: OTLPExporterConfigBase): string;
}
/**
 * Collector Metric Exporter for Web
 */
export declare class OTLPMetricExporter extends OTLPMetricExporterBase<OTLPExporterBrowserProxy> {
    constructor(config?: OTLPExporterConfigBase & OTLPMetricExporterOptions);
}
export {};
//# sourceMappingURL=OTLPMetricExporter.d.ts.map