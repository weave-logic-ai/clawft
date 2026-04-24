#!/usr/bin/env node
/**
 * Interactive CLI wizard for agentic-flow configuration
 * Supports both interactive mode and direct CLI arguments
 */
export declare class ConfigWizard {
    private envPath;
    private currentConfig;
    constructor(envPath?: string);
    private loadExistingConfig;
    private saveConfig;
    runInteractive(): Promise<void>;
    set(key: string, value: string): void;
    get(key: string): void;
    delete(key: string): void;
    list(): void;
    showSummary(): void;
    reset(): void;
}
export declare function handleConfigCommand(args: string[]): Promise<void>;
//# sourceMappingURL=config-wizard.d.ts.map