/**
 * AgentDB Migration Command
 * Migrate legacy AgentDB v1 and claude-flow memory databases to v2 format
 * with RuVector GNN optimization
 */
interface MigrationOptions {
    sourceDb: string;
    targetDb?: string;
    optimize?: boolean;
    dryRun?: boolean;
    verbose?: boolean;
}
export declare function migrateCommand(options: MigrationOptions): Promise<void>;
export {};
//# sourceMappingURL=migrate.d.ts.map