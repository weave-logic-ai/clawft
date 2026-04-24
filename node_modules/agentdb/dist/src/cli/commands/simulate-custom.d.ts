/**
 * Custom simulation builder
 * Component registry and validation system
 */
export interface Component {
    id: string;
    name: string;
    category: string;
    description: string;
    optimal: boolean;
    metrics?: string;
    compatibility?: string[];
}
export declare class ComponentRegistry {
    private static components;
    static getByCategory(category: string): Component[];
    static getById(id: string): Component | undefined;
    static getAllCategories(): string[];
    static getOptimalComponents(): Component[];
    static validateCompatibility(componentIds: string[]): {
        valid: boolean;
        errors: string[];
    };
}
export declare function runCustomBuilder(): Promise<void>;
//# sourceMappingURL=simulate-custom.d.ts.map