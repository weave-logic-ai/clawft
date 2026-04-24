import { ExportResult } from '@opentelemetry/core';
import { AggregationTemporality, AggregationTemporalitySelector, InstrumentType, PushMetricExporter, ResourceMetrics, Aggregation } from '@opentelemetry/sdk-metrics';
import { OTLPMetricExporterOptions } from './OTLPMetricExporterOptions';
import { OTLPExporterBase } from '@opentelemetry/otlp-exporter-base';
export declare const CumulativeTemporalitySelector: AggregationTemporalitySelector;
export declare const DeltaTemporalitySelector: AggregationTemporalitySelector;
export declare const LowMemoryTemporalitySelector: AggregationTemporalitySelector;
export declare class OTLPMetricExporterBase<T extends OTLPExporterBase<OTLPMetricExporterOptions, ResourceMetrics>> implements PushMetricExporter {
    _otlpExporter: T;
    private _aggregationTemporalitySelector;
    private _aggregationSelector;
    constructor(exporter: T, config?: OTLPMetricExporterOptions);
    export(metrics: ResourceMetrics, resultCallback: (result: ExportResult) => void): void;
    shutdown(): Promise<void>;
    forceFlush(): Promise<void>;
    selectAggregation(instrumentType: InstrumentType): Aggregation;
    selectAggregationTemporality(instrumentType: InstrumentType): AggregationTemporality;
}
//# sourceMappingURL=OTLPMetricExporterBase.d.ts.map