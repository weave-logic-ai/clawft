/**
 * Report generation in multiple formats
 * Supports markdown, JSON, and HTML output
 */
import type { SimulationReport } from './simulation-runner.js';
export declare class ReportGenerator {
    /**
     * Generate markdown report
     */
    generateMarkdown(report: SimulationReport): Promise<string>;
    /**
     * Generate JSON report
     */
    generateJSON(report: SimulationReport): Promise<string>;
    /**
     * Generate HTML report
     */
    generateHTML(report: SimulationReport): Promise<string>;
    /**
     * Save report to file
     */
    saveReport(report: SimulationReport, outputPath: string, format: 'md' | 'json' | 'html'): Promise<string>;
}
//# sourceMappingURL=report-generator.d.ts.map