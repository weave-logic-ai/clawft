import { CompressionAlgorithm } from '@opentelemetry/otlp-exporter-base';
import type { ChannelCredentials } from '@grpc/grpc-js';
export declare const DEFAULT_COLLECTOR_URL = "http://localhost:4317";
export declare function validateAndNormalizeUrl(url: string): string;
export declare function configureCredentials(credentials: ChannelCredentials | undefined, endpoint: string): ChannelCredentials;
/**
 * Exported for testing
 */
export declare function getCredentialsFromEnvironment(): ChannelCredentials;
export declare function configureCompression(compression: CompressionAlgorithm | undefined): CompressionAlgorithm;
//# sourceMappingURL=util.d.ts.map