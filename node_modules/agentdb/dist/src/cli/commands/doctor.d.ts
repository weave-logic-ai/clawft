/**
 * Doctor command - Deep system diagnostics, health check, and optimization analysis
 * Verifies AgentDB installation, dependencies, functionality, and provides optimization recommendations
 */
interface DoctorOptions {
    dbPath?: string;
    verbose?: boolean;
}
export declare function doctorCommand(options?: DoctorOptions): Promise<void>;
export {};
//# sourceMappingURL=doctor.d.ts.map