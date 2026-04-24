import type { ChannelCredentials, Metadata } from '@grpc/grpc-js';
import { CompressionAlgorithm, OTLPExporterConfigBase, OTLPExporterError } from '@opentelemetry/otlp-exporter-base';
/**
 * Queue item to be used to save temporary spans/metrics/logs in case the GRPC service
 * hasn't been fully initialized yet
 */
export interface GRPCQueueItem<ExportedItem> {
    objects: ExportedItem[];
    onSuccess: () => void;
    onError: (error: OTLPExporterError) => void;
}
/**
 * OTLP Exporter Config for Node
 */
export interface OTLPGRPCExporterConfigNode extends OTLPExporterConfigBase {
    credentials?: ChannelCredentials;
    metadata?: Metadata;
    compression?: CompressionAlgorithm;
}
//# sourceMappingURL=types.d.ts.map