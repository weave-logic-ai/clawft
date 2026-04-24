import { OTLPExporterBase } from '../../OTLPExporterBase';
import { OTLPExporterConfigBase } from '../../types';
import * as otlpTypes from '../../types';
import { ISerializer } from '@opentelemetry/otlp-transformer';
/**
 * Collector Metric Exporter abstract base class
 */
export declare abstract class OTLPExporterBrowserBase<ExportItem, ServiceResponse> extends OTLPExporterBase<OTLPExporterConfigBase, ExportItem> {
    protected _headers: Record<string, string>;
    private _useXHR;
    private _contentType;
    private _serializer;
    /**
     * @param config
     * @param serializer
     * @param contentType
     */
    constructor(config: OTLPExporterConfigBase | undefined, serializer: ISerializer<ExportItem[], ServiceResponse>, contentType: string);
    onInit(): void;
    onShutdown(): void;
    send(items: ExportItem[], onSuccess: () => void, onError: (error: otlpTypes.OTLPExporterError) => void): void;
}
//# sourceMappingURL=OTLPExporterBrowserBase.d.ts.map