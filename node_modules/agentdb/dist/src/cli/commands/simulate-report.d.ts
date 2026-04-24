/**
 * Report viewer and management
 * View, list, and export simulation reports
 */
export interface ReportMetadata {
    id: string;
    scenarioId: string;
    timestamp: string;
    path: string;
    format: string;
    coherenceScore: number;
    successRate: number;
}
export declare function viewReport(reportId?: string): Promise<void>;
//# sourceMappingURL=simulate-report.d.ts.map