/**
 * Multi-level help system for AgentDB simulation CLI
 * Provides beautiful formatting with colors, tables, and examples
 */
export interface HelpSection {
    title: string;
    content: string[];
}
export interface CommandHelp {
    name: string;
    description: string;
    usage: string[];
    sections: HelpSection[];
    examples?: string[];
}
export declare class HelpFormatter {
    /**
     * Format top-level help (Level 1)
     */
    static formatTopLevel(): string;
    /**
     * Format scenario-specific help (Level 2)
     */
    static formatScenarioHelp(scenario: string): string;
    /**
     * Format custom builder help (Level 3)
     */
    static formatCustomHelp(): string;
    /**
     * Format scenario list
     */
    private static formatScenarioList;
    /**
     * Format scenario list with descriptions
     */
    static formatList(): string;
}
//# sourceMappingURL=help-formatter.d.ts.map