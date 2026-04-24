/**
 * SONA MCP Tools
 *
 * Model Context Protocol tools for SONA (Self-Optimizing Neural Architecture)
 * Provides trajectory management, pattern discovery, and learning operations
 */
import { SONAProfile } from '../../services/sona-service';
export interface MCPTool {
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties: Record<string, any>;
        required?: string[];
    };
}
/**
 * SONA MCP Tools
 */
export declare const sonaMCPTools: MCPTool[];
/**
 * MCP Tool Handlers
 */
export declare const sonaMCPHandlers: {
    sona_trajectory_begin: (params: {
        embedding: number[];
        route?: string;
        profile?: SONAProfile;
    }) => Promise<{
        success: boolean;
        trajectoryId: any;
        route: string;
        profile: SONAProfile;
    }>;
    sona_trajectory_step: (params: {
        trajectoryId: string;
        activations: number[];
        attentionWeights: number[];
        reward: number;
    }) => Promise<{
        success: boolean;
        trajectoryId: string;
        reward: number;
    }>;
    sona_trajectory_context: (params: {
        trajectoryId: string;
        contextId: string;
    }) => Promise<{
        success: boolean;
        trajectoryId: string;
        contextId: string;
    }>;
    sona_trajectory_end: (params: {
        trajectoryId: string;
        qualityScore: number;
    }) => Promise<{
        success: boolean;
        trajectoryId: string;
        qualityScore: number;
        learningTriggered: boolean;
    }>;
    sona_trajectory_list: (params: {
        profile?: SONAProfile;
    }) => Promise<{
        count: any;
        trajectories: any;
    }>;
    sona_pattern_find: (params: {
        query: number[];
        k?: number;
        profile?: SONAProfile;
    }) => Promise<{
        count: any;
        patterns: any;
    }>;
    sona_apply_micro_lora: (params: {
        input: number[];
        profile?: SONAProfile;
    }) => Promise<{
        output: any;
        latencyMs: number;
        inputDim: number;
        outputDim: any;
    }>;
    sona_apply_base_lora: (params: {
        layerIndex: number;
        input: number[];
        profile?: SONAProfile;
    }) => Promise<{
        output: any;
        latencyMs: number;
        layerIndex: number;
        inputDim: number;
        outputDim: any;
    }>;
    sona_force_learn: (params: {
        profile?: SONAProfile;
    }) => Promise<{
        success: any;
        patternsLearned: any;
        profile: SONAProfile;
    }>;
    sona_get_stats: (params: {
        profile?: SONAProfile;
        engineStats?: boolean;
    }) => Promise<any>;
    sona_get_profile: (params: {
        profile: SONAProfile;
    }) => Promise<{
        profile: SONAProfile;
        configuration: any;
        characteristics: any;
    }>;
    sona_list_profiles: () => Promise<{
        profiles: {
            name: string;
            description: string;
            config: string;
            useCase: string;
        }[];
    }>;
    sona_set_enabled: (params: {
        enabled: boolean;
        profile?: SONAProfile;
    }) => Promise<{
        success: boolean;
        enabled: boolean;
        profile: SONAProfile;
    }>;
    sona_benchmark: (params: {
        iterations?: number;
        profile?: SONAProfile;
    }) => Promise<{
        iterations: number;
        profile: SONAProfile;
        microLora: {
            totalTimeMs: number;
            avgLatencyMs: number;
            opsPerSec: number;
        };
        baseLora: {
            totalTimeMs: number;
            avgLatencyMs: number;
            opsPerSec: number;
        };
        expected: {
            targetThroughput: number;
            targetLatency: number;
            perLayerCost: number;
        };
        meetsTarget: boolean;
    }>;
};
export default sonaMCPTools;
//# sourceMappingURL=sona-tools.d.ts.map