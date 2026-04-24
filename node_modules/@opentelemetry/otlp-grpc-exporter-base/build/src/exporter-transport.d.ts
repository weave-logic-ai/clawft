import { ExportResponse } from './export-response';
export interface IExporterTransport {
    send(data: Uint8Array): Promise<ExportResponse>;
    shutdown(): void;
}
//# sourceMappingURL=exporter-transport.d.ts.map