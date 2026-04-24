/// <reference types="node" />
import * as http from 'http';
import * as https from 'https';
import { OTLPExporterNodeBase } from './OTLPExporterNodeBase';
import { OTLPExporterNodeConfigBase } from '.';
import { CompressionAlgorithm } from './types';
import { OTLPExporterError } from '../../types';
/**
 * Sends data using http
 * @param collector
 * @param data
 * @param contentType
 * @param onSuccess
 * @param onError
 */
export declare function sendWithHttp<ExportItem, ServiceResponse>(collector: OTLPExporterNodeBase<ExportItem, ServiceResponse>, data: string | Uint8Array, contentType: string, onSuccess: () => void, onError: (error: OTLPExporterError) => void): void;
export declare function createHttpAgent(config: OTLPExporterNodeConfigBase): http.Agent | https.Agent | undefined;
export declare function configureCompression(compression: CompressionAlgorithm | undefined): CompressionAlgorithm;
//# sourceMappingURL=util.d.ts.map