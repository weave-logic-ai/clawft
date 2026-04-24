import { OTLPExporterError } from '../../types';
/**
 * Send metrics/spans using browser navigator.sendBeacon
 * @param body
 * @param url
 * @param blobPropertyBag
 * @param onSuccess
 * @param onError
 */
export declare function sendWithBeacon(body: Uint8Array, url: string, blobPropertyBag: BlobPropertyBag, onSuccess: () => void, onError: (error: OTLPExporterError) => void): void;
/**
 * function to send metrics/spans using browser XMLHttpRequest
 *     used when navigator.sendBeacon is not available
 * @param body
 * @param url
 * @param headers
 * @param onSuccess
 * @param onError
 */
export declare function sendWithXhr(body: Uint8Array, url: string, headers: Record<string, string>, exporterTimeout: number, onSuccess: () => void, onError: (error: OTLPExporterError) => void): void;
//# sourceMappingURL=util.d.ts.map