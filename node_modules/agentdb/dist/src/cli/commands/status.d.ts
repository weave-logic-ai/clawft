/**
 * AgentDB Status Command - Show database and backend status
 */
interface StatusOptions {
    dbPath?: string;
    verbose?: boolean;
}
export declare function statusCommand(options?: StatusOptions): Promise<void>;
export {};
//# sourceMappingURL=status.d.ts.map