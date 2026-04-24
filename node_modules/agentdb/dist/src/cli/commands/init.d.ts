/**
 * AgentDB Init Command - Initialize database with backend detection
 */
interface InitOptions {
    backend?: 'auto' | 'ruvector' | 'hnswlib';
    dimension?: number;
    model?: string;
    preset?: 'small' | 'medium' | 'large';
    inMemory?: boolean;
    dryRun?: boolean;
    dbPath?: string;
}
export declare function initCommand(options?: InitOptions): Promise<void>;
export {};
//# sourceMappingURL=init.d.ts.map