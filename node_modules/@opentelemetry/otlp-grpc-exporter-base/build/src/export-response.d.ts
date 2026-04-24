export interface ExportResponseSuccess {
    status: 'success';
    data?: Uint8Array;
}
export interface ExportResponseFailure {
    status: 'failure';
    error: Error;
}
export declare type ExportResponse = ExportResponseSuccess | ExportResponseFailure;
//# sourceMappingURL=export-response.d.ts.map