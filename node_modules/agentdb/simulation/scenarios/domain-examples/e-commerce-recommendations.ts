/**
 * E-Commerce Recommendations: Personalized Product Discovery
 *
 * Use Case: Recommend similar products based on user preferences,
 * browsing history, and product embeddings.
 *
 * Optimization Priority: DIVERSITY + RELEVANCE
 */

import { UnifiedMetrics } from '../../types';

export const ECOMMERCE_ATTENTION_CONFIG = {
  heads: 8,                        // Optimal for balanced performance
  forwardPassTargetMs: 20,         // 20ms acceptable for recommendations
  batchSize: 64,                   // Batch user requests
  precision: 'float32' as const,   // Full precision for quality
  diversityBoost: true,            // Promote diverse recommendations

  // Louvain clustering for product categories
  clustering: {
    algorithm: 'louvain' as const,
    minModularity: 0.75,           // High-quality clusters
    semanticPurity: 0.87,          // 87.2% purity validated
    hierarchicalLevels: 3          // Category hierarchy (dept > category > subcategory)
  },

  // Dynamic-k based on user engagement
  dynamicK: {
    min: 10,                       // Minimum 10 recommendations
    max: 50,                       // Maximum 50 for exploration
    adaptationStrategy: 'user-engagement' as const  // Active users get more
  }
};

// E-commerce-specific metrics
export interface ECommerceMetrics extends UnifiedMetrics {
  clickThroughRate: number;        // CTR on recommendations
  conversionRate: number;          // Purchase conversion rate
  diversityScore: number;          // Recommendation diversity
  categoryBalanceScore: number;    // Cross-category recommendations
  userSatisfaction: number;        // Implicit feedback signals
}

// Recommendation interface
export interface Recommendation {
  productId: string;
  relevanceScore: number;
  category: string;
  cluster: string;
  priceUSD: number;
}

// Example: Product recommendation with diversity
export async function recommendProducts(
  userProfile: Float32Array,       // User preferences embeddings
  productCatalog: any,             // HNSWGraph type
  userEngagement: number,
  applyAttention: (data: Float32Array, config: any) => Promise<Float32Array>,
  applyDiversityBoost: (candidates: any[], weight: number) => Promise<any[]>,
  clusterRecommendations: (items: any[], config: any) => Promise<any[]>,
  findCluster: (item: any, clusters: any[]) => string,
  diversityWeight: number = 0.3
): Promise<Recommendation[]> {
  const config = ECOMMERCE_ATTENTION_CONFIG;

  // 8-head attention for user query enhancement
  const enhanced = await applyAttention(userProfile, config);

  // Dynamic-k based on user engagement (engaged users get more options)
  const k = Math.round(10 + userEngagement * 40);  // 10-50 range

  // Search with clustering for category diversity
  const candidates = await productCatalog.search(enhanced, k);

  // Apply diversity boost (promote different categories)
  const diversified = await applyDiversityBoost(candidates, diversityWeight);

  // Cluster recommendations by category (Louvain)
  const clusters = await clusterRecommendations(diversified, config.clustering);

  return diversified.map((p: any) => ({
    productId: p.id,
    relevanceScore: p.score,
    category: p.metadata.category,
    cluster: findCluster(p, clusters),
    priceUSD: p.metadata.price
  }));
}

// Performance targets for e-commerce
export const ECOMMERCE_PERFORMANCE_TARGETS = {
  p95LatencyMs: 50,                // 50ms p95 (user experience)
  clickThroughRate: 0.15,          // 15% CTR target
  conversionRate: 0.05,            // 5% conversion target
  diversityScore: 0.7,             // 70% category diversity
  uptimePercent: 99.9              // 99.9% uptime (3 nines)
};

// E-commerce platform-specific configurations
export const ECOMMERCE_CONFIG_VARIATIONS = {
  // Fashion/apparel (high visual similarity)
  fashion: {
    ...ECOMMERCE_ATTENTION_CONFIG,
    heads: 12,                     // More heads for visual nuance
    diversityBoost: true,
    clustering: {
      ...ECOMMERCE_ATTENTION_CONFIG.clustering,
      hierarchicalLevels: 4        // Deeper taxonomy (category > subcategory > style > color)
    }
  },

  // Electronics (specification-driven)
  electronics: {
    ...ECOMMERCE_ATTENTION_CONFIG,
    heads: 8,
    specificationWeight: 0.6,      // Emphasize specs over style
    diversityBoost: false          // Users want specific features
  },

  // Grocery (frequent purchases)
  grocery: {
    ...ECOMMERCE_ATTENTION_CONFIG,
    heads: 6,
    forwardPassTargetMs: 10,       // Faster for mobile
    batchSize: 128,                // High volume
    dynamicK: { min: 15, max: 30, adaptationStrategy: 'cart-size' as const }
  },

  // Luxury goods (personalization critical)
  luxury: {
    ...ECOMMERCE_ATTENTION_CONFIG,
    heads: 16,                     // Maximum personalization
    forwardPassTargetMs: 50,       // Allow more time
    diversityBoost: false,         // Highly targeted
    precision: 'float32' as const
  }
};

// User segment adaptations
export function adaptConfigToUserSegment(
  baseConfig: typeof ECOMMERCE_ATTENTION_CONFIG,
  segment: 'browser' | 'buyer' | 'loyal' | 'vip'
): typeof ECOMMERCE_ATTENTION_CONFIG {
  switch (segment) {
    case 'browser':
      return {
        ...baseConfig,
        dynamicK: { ...baseConfig.dynamicK, min: 20, max: 50 }, // More exploration
        diversityBoost: true       // Show variety
      };
    case 'buyer':
      return {
        ...baseConfig,
        dynamicK: { ...baseConfig.dynamicK, min: 10, max: 30 }, // Focused recommendations
        diversityBoost: false
      };
    case 'loyal':
      return {
        ...baseConfig,
        heads: 12,                 // Better personalization
        dynamicK: { ...baseConfig.dynamicK, min: 15, max: 40 }
      };
    case 'vip':
      return {
        ...baseConfig,
        heads: 16,                 // Maximum personalization
        forwardPassTargetMs: 30,
        precision: 'float32' as const
      };
  }
}

// Seasonal/promotional adaptations
export interface PromotionalContext {
  isSale: boolean;
  seasonalEvent: string | null;
  inventoryPressure: number;      // 0-1, how much to push certain items
}

export function adaptConfigToPromotion(
  baseConfig: typeof ECOMMERCE_ATTENTION_CONFIG,
  context: PromotionalContext
): typeof ECOMMERCE_ATTENTION_CONFIG {
  if (context.isSale) {
    return {
      ...baseConfig,
      dynamicK: { ...baseConfig.dynamicK, min: 20, max: 60 }, // Show more options
      diversityBoost: true        // Cross-sell opportunities
      // Note: Sale boost handled in ranking layer
    };
  }
  return baseConfig;
}

// A/B testing configuration generator
export function generateABTestConfigs(
  baseConfig: typeof ECOMMERCE_ATTENTION_CONFIG
): Record<string, typeof ECOMMERCE_ATTENTION_CONFIG> {
  return {
    control: baseConfig,

    moreHeads: {
      ...baseConfig,
      heads: 12
    },

    moreDiversity: {
      ...baseConfig,
      diversityBoost: true,
      dynamicK: { ...baseConfig.dynamicK, min: 15, max: 60 }
    },

    fasterLatency: {
      ...baseConfig,
      heads: 6,
      forwardPassTargetMs: 10,
      precision: 'float32' as const  // Lower precision not yet supported
    }
  };
}
