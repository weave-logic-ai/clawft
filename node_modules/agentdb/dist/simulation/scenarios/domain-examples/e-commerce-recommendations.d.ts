/**
 * E-Commerce Recommendations: Personalized Product Discovery
 *
 * Use Case: Recommend similar products based on user preferences,
 * browsing history, and product embeddings.
 *
 * Optimization Priority: DIVERSITY + RELEVANCE
 */
import { UnifiedMetrics } from '../../types';
export declare const ECOMMERCE_ATTENTION_CONFIG: {
    heads: number;
    forwardPassTargetMs: number;
    batchSize: number;
    precision: "float32";
    diversityBoost: boolean;
    clustering: {
        algorithm: "louvain";
        minModularity: number;
        semanticPurity: number;
        hierarchicalLevels: number;
    };
    dynamicK: {
        min: number;
        max: number;
        adaptationStrategy: "user-engagement";
    };
};
export interface ECommerceMetrics extends UnifiedMetrics {
    clickThroughRate: number;
    conversionRate: number;
    diversityScore: number;
    categoryBalanceScore: number;
    userSatisfaction: number;
}
export interface Recommendation {
    productId: string;
    relevanceScore: number;
    category: string;
    cluster: string;
    priceUSD: number;
}
export declare function recommendProducts(userProfile: Float32Array, // User preferences embeddings
productCatalog: any, // HNSWGraph type
userEngagement: number, applyAttention: (data: Float32Array, config: any) => Promise<Float32Array>, applyDiversityBoost: (candidates: any[], weight: number) => Promise<any[]>, clusterRecommendations: (items: any[], config: any) => Promise<any[]>, findCluster: (item: any, clusters: any[]) => string, diversityWeight?: number): Promise<Recommendation[]>;
export declare const ECOMMERCE_PERFORMANCE_TARGETS: {
    p95LatencyMs: number;
    clickThroughRate: number;
    conversionRate: number;
    diversityScore: number;
    uptimePercent: number;
};
export declare const ECOMMERCE_CONFIG_VARIATIONS: {
    fashion: {
        heads: number;
        diversityBoost: boolean;
        clustering: {
            hierarchicalLevels: number;
            algorithm: "louvain";
            minModularity: number;
            semanticPurity: number;
        };
        forwardPassTargetMs: number;
        batchSize: number;
        precision: "float32";
        dynamicK: {
            min: number;
            max: number;
            adaptationStrategy: "user-engagement";
        };
    };
    electronics: {
        heads: number;
        specificationWeight: number;
        diversityBoost: boolean;
        forwardPassTargetMs: number;
        batchSize: number;
        precision: "float32";
        clustering: {
            algorithm: "louvain";
            minModularity: number;
            semanticPurity: number;
            hierarchicalLevels: number;
        };
        dynamicK: {
            min: number;
            max: number;
            adaptationStrategy: "user-engagement";
        };
    };
    grocery: {
        heads: number;
        forwardPassTargetMs: number;
        batchSize: number;
        dynamicK: {
            min: number;
            max: number;
            adaptationStrategy: "cart-size";
        };
        precision: "float32";
        diversityBoost: boolean;
        clustering: {
            algorithm: "louvain";
            minModularity: number;
            semanticPurity: number;
            hierarchicalLevels: number;
        };
    };
    luxury: {
        heads: number;
        forwardPassTargetMs: number;
        diversityBoost: boolean;
        precision: "float32";
        batchSize: number;
        clustering: {
            algorithm: "louvain";
            minModularity: number;
            semanticPurity: number;
            hierarchicalLevels: number;
        };
        dynamicK: {
            min: number;
            max: number;
            adaptationStrategy: "user-engagement";
        };
    };
};
export declare function adaptConfigToUserSegment(baseConfig: typeof ECOMMERCE_ATTENTION_CONFIG, segment: 'browser' | 'buyer' | 'loyal' | 'vip'): typeof ECOMMERCE_ATTENTION_CONFIG;
export interface PromotionalContext {
    isSale: boolean;
    seasonalEvent: string | null;
    inventoryPressure: number;
}
export declare function adaptConfigToPromotion(baseConfig: typeof ECOMMERCE_ATTENTION_CONFIG, context: PromotionalContext): typeof ECOMMERCE_ATTENTION_CONFIG;
export declare function generateABTestConfigs(baseConfig: typeof ECOMMERCE_ATTENTION_CONFIG): Record<string, typeof ECOMMERCE_ATTENTION_CONFIG>;
//# sourceMappingURL=e-commerce-recommendations.d.ts.map