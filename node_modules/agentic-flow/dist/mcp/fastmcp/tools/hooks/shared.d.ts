/**
 * Shared utilities for hook tools
 */
export interface IntelligenceData {
    patterns: Record<string, Record<string, number>>;
    sequences: Record<string, Array<{
        file: string;
        score: number;
    }>>;
    memories: Array<{
        content: string;
        type: string;
        created: string;
        embedding?: number[];
    }>;
    dirPatterns: Record<string, string>;
    errorPatterns: Array<{
        errorType: string;
        context: string;
        resolution: string;
        agentSuccess: Record<string, number>;
    }>;
    metrics: {
        totalRoutes: number;
        successfulRoutes: number;
        routingHistory: Array<{
            timestamp: string;
            task: string;
            agent: string;
            success: boolean;
        }>;
    };
    pretrained?: {
        date: string;
        stats: Record<string, number>;
    };
}
export declare function loadIntelligence(): IntelligenceData;
export declare function saveIntelligence(data: IntelligenceData): void;
export declare const agentMapping: Record<string, string>;
export declare function getAgentForFile(filePath: string): string;
export declare function simpleEmbed(text: string): number[];
export declare function cosineSimilarity(a: number[], b: number[]): number;
export declare const dangerousPatterns: RegExp[];
export declare function assessCommandRisk(command: string): number;
//# sourceMappingURL=shared.d.ts.map