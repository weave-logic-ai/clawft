/// <reference types="node" />
import type * as http from 'http';
import type * as https from 'https';
import { OTLPExporterBase } from '../../OTLPExporterBase';
import { OTLPExporterNodeConfigBase, CompressionAlgorithm } from './types';
import * as otlpTypes from '../../types';
import { ISerializer } from '@opentelemetry/otlp-transformer';
/**
 * Collector Metric Exporter abstract base class
 */
export declare abstract class OTLPExporterNodeBase<ExportItem, ServiceResponse> extends OTLPExporterBase<OTLPExporterNodeConfigBase, ExportItem> {
    DEFAULT_HEADERS: Record<string, string>;
    headers: Record<string, string>;
    agent: http.Agent | https.Agent | undefined;
    compression: CompressionAlgorithm;
    private _serializer;
    private _contentType;
    constructor(config: OTLPExporterNodeConfigBase | undefined, serializer: ISerializer<ExportItem[], ServiceResponse>, contentType: string);
    onInit(_config: OTLPExporterNodeConfigBase): void;
    send(objects: ExportItem[], onSuccess: () => void, onError: (error: otlpTypes.OTLPExporterError) => void): void;
    onShutdown(): void;
}
//# sourceMappingURL=OTLPExporterNodeBase.d.ts.map