/**
 * Scientific Research: Research Paper Similarity and Discovery
 *
 * Use Case: Find similar research papers, identify research trends,
 * and discover cross-domain connections.
 *
 * Optimization Priority: QUALITY + CROSS-DOMAIN DISCOVERY
 */
import { UnifiedMetrics } from '../../types';
export declare const RESEARCH_ATTENTION_CONFIG: {
    heads: number;
    forwardPassTargetMs: number;
    batchSize: number;
    precision: "float32";
    crossDomainDiscovery: boolean;
    clustering: {
        algorithm: "hierarchical";
        linkage: "ward";
        hierarchicalLevels: number;
        semanticPurity: number;
    };
    recallTarget: number;
    selfHealing: {
        enabled: boolean;
        adaptationIntervalMs: number;
        degradationThreshold: number;
        taxonomyUpdates: boolean;
    };
};
export interface ResearchMetrics extends UnifiedMetrics {
    crossDomainConnections: number;
    citationAccuracy: number;
    taxonomyQuality: number;
    noveltyScore: number;
    expertAgreement: number;
}
export interface ResearchConnection {
    paperId: string;
    title: string;
    authors: string[];
    similarity: number;
    domain: string;
    crossDomainConnection?: {
        targetDomain: string;
        connectionType: string;
        noveltyScore: number;
    };
    citations: number;
}
export declare function discoverRelatedResearch(paperEmbedding: Float32Array, // Paper abstract + citations embeddings
researchCorpus: any, // HNSWGraph type
applyAttention: (data: Float32Array, config: any) => Promise<Float32Array>, buildResearchTaxonomy: (papers: any[], config: any) => Promise<any>, findCrossDomainConnections: (papers: any[], taxonomy: any) => Promise<any[]>, findDomain: (paper: any, taxonomy: any) => string, includeCrossDomain?: boolean): Promise<ResearchConnection[]>;
export declare const RESEARCH_PERFORMANCE_TARGETS: {
    recallAt100: number;
    p95LatencyMs: number;
    crossDomainRate: number;
    expertAgreement: number;
    uptimePercent: number;
};
export declare const RESEARCH_CONFIG_VARIATIONS: {
    computerScience: {
        heads: number;
        forwardPassTargetMs: number;
        batchSize: number;
        recallTarget: number;
        precision: "float32";
        crossDomainDiscovery: boolean;
        clustering: {
            algorithm: "hierarchical";
            linkage: "ward";
            hierarchicalLevels: number;
            semanticPurity: number;
        };
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
            taxonomyUpdates: boolean;
        };
    };
    medicine: {
        heads: number;
        forwardPassTargetMs: number;
        recallTarget: number;
        precision: "float32";
        batchSize: number;
        crossDomainDiscovery: boolean;
        clustering: {
            algorithm: "hierarchical";
            linkage: "ward";
            hierarchicalLevels: number;
            semanticPurity: number;
        };
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
            taxonomyUpdates: boolean;
        };
    };
    physics: {
        heads: number;
        crossDomainDiscovery: boolean;
        clustering: {
            hierarchicalLevels: number;
            algorithm: "hierarchical";
            linkage: "ward";
            semanticPurity: number;
        };
        forwardPassTargetMs: number;
        batchSize: number;
        precision: "float32";
        recallTarget: number;
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
            taxonomyUpdates: boolean;
        };
    };
    socialSciences: {
        heads: number;
        crossDomainDiscovery: boolean;
        clustering: {
            semanticPurity: number;
            algorithm: "hierarchical";
            linkage: "ward";
            hierarchicalLevels: number;
        };
        forwardPassTargetMs: number;
        batchSize: number;
        precision: "float32";
        recallTarget: number;
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
            taxonomyUpdates: boolean;
        };
    };
};
export declare function adaptConfigToSearchMode(baseConfig: typeof RESEARCH_ATTENTION_CONFIG, mode: 'literature-review' | 'novelty-discovery' | 'citation-tracing' | 'interdisciplinary'): typeof RESEARCH_ATTENTION_CONFIG;
export declare function adaptConfigToResearchStage(baseConfig: typeof RESEARCH_ATTENTION_CONFIG, stage: 'initial-exploration' | 'hypothesis-formation' | 'validation' | 'writing'): typeof RESEARCH_ATTENTION_CONFIG;
export interface CitationNetworkMetrics {
    networkDensity: number;
    clusteringCoefficient: number;
    averagePathLength: number;
    communityModularity: number;
}
export declare function analyzeCitationNetwork(papers: ResearchConnection[]): CitationNetworkMetrics;
//# sourceMappingURL=scientific-research.d.ts.map