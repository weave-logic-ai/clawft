/**
 * Scientific Research: Research Paper Similarity and Discovery
 *
 * Use Case: Find similar research papers, identify research trends,
 * and discover cross-domain connections.
 *
 * Optimization Priority: QUALITY + CROSS-DOMAIN DISCOVERY
 */
export const RESEARCH_ATTENTION_CONFIG = {
    heads: 12, // High-quality attention (more than 8-head optimal)
    forwardPassTargetMs: 100, // 100ms acceptable (research workflow)
    batchSize: 128, // Large batches for corpus processing
    precision: 'float32', // Full precision for scientific data
    crossDomainDiscovery: true, // Enable cross-field connections
    // Hierarchical clustering for research topics
    clustering: {
        algorithm: 'hierarchical', // Better for taxonomy
        linkage: 'ward', // Ward's minimum variance
        hierarchicalLevels: 5, // Deep taxonomy (field > subfield > topic > subtopic > specific)
        semanticPurity: 0.90 // High purity for research domains
    },
    // High-recall for comprehensive literature review
    recallTarget: 0.98, // 98% recall (don't miss relevant papers)
    // Self-organizing for evolving research landscape
    selfHealing: {
        enabled: true,
        adaptationIntervalMs: 60000, // 1-minute adaptation (not real-time)
        degradationThreshold: 0.03, // 3% tolerance
        taxonomyUpdates: true // Update research taxonomy
    }
};
// Example: Research paper discovery with cross-domain connections
export async function discoverRelatedResearch(paperEmbedding, // Paper abstract + citations embeddings
researchCorpus, // HNSWGraph type
applyAttention, buildResearchTaxonomy, findCrossDomainConnections, findDomain, includeCrossDomain = true) {
    const config = RESEARCH_ATTENTION_CONFIG;
    // 12-head attention for nuanced understanding
    const enhanced = await applyAttention(paperEmbedding, config);
    // High-recall search (k=100 for comprehensive review)
    const candidates = await researchCorpus.search(enhanced, 100);
    // Hierarchical clustering for taxonomy
    const taxonomy = await buildResearchTaxonomy(candidates, config.clustering);
    // Cross-domain discovery (find connections across fields)
    const crossDomain = includeCrossDomain
        ? await findCrossDomainConnections(candidates, taxonomy)
        : [];
    return candidates.map((p) => ({
        paperId: p.id,
        title: p.metadata.title,
        authors: p.metadata.authors,
        similarity: p.score,
        domain: findDomain(p, taxonomy),
        crossDomainConnection: crossDomain.find((c) => c.paperId === p.id),
        citations: p.metadata.citations
    }));
}
// Performance targets for research
export const RESEARCH_PERFORMANCE_TARGETS = {
    recallAt100: 0.98, // 98% recall (comprehensive)
    p95LatencyMs: 200, // 200ms p95 (batch processing)
    crossDomainRate: 0.15, // 15% cross-domain discoveries
    expertAgreement: 0.85, // 85% agreement with experts
    uptimePercent: 99.0 // 99% uptime (2 nines)
};
// Research field-specific configurations
export const RESEARCH_CONFIG_VARIATIONS = {
    // Computer science (fast-moving field, many papers)
    computerScience: {
        ...RESEARCH_ATTENTION_CONFIG,
        heads: 10,
        forwardPassTargetMs: 50, // Faster for large corpus
        batchSize: 256,
        recallTarget: 0.97 // Slightly lower for speed
    },
    // Medicine (high precision required, slower pace)
    medicine: {
        ...RESEARCH_ATTENTION_CONFIG,
        heads: 16, // Maximum quality
        forwardPassTargetMs: 150,
        recallTarget: 0.99, // Highest recall
        precision: 'float32'
    },
    // Physics (mathematical precision, cross-domain links)
    physics: {
        ...RESEARCH_ATTENTION_CONFIG,
        heads: 14,
        crossDomainDiscovery: true,
        clustering: {
            ...RESEARCH_ATTENTION_CONFIG.clustering,
            hierarchicalLevels: 6 // Deeper for specialized subfields
        }
    },
    // Social sciences (qualitative, broader connections)
    socialSciences: {
        ...RESEARCH_ATTENTION_CONFIG,
        heads: 12,
        crossDomainDiscovery: true,
        clustering: {
            ...RESEARCH_ATTENTION_CONFIG.clustering,
            semanticPurity: 0.85 // More flexible clustering
        }
    }
};
// Search mode adaptations
export function adaptConfigToSearchMode(baseConfig, mode) {
    switch (mode) {
        case 'literature-review':
            return {
                ...baseConfig,
                heads: 12,
                recallTarget: 0.99, // Comprehensive recall
                crossDomainDiscovery: false // Stay within field
            };
        case 'novelty-discovery':
            return {
                ...baseConfig,
                heads: 16, // Maximum quality
                crossDomainDiscovery: true,
                clustering: {
                    ...baseConfig.clustering,
                    semanticPurity: 0.80 // More flexible for novel connections
                }
            };
        case 'citation-tracing':
            return {
                ...baseConfig,
                heads: 10,
                forwardPassTargetMs: 50,
                recallTarget: 0.95, // Fast, focused
                crossDomainDiscovery: false
            };
        case 'interdisciplinary':
            return {
                ...baseConfig,
                heads: 14,
                crossDomainDiscovery: true,
                clustering: {
                    ...baseConfig.clustering,
                    hierarchicalLevels: 6, // Deep taxonomy for connections
                    semanticPurity: 0.82 // Flexible for cross-field
                }
            };
    }
}
// Research stage adaptations
export function adaptConfigToResearchStage(baseConfig, stage) {
    switch (stage) {
        case 'initial-exploration':
            return {
                ...baseConfig,
                heads: 12,
                crossDomainDiscovery: true,
                recallTarget: 0.95 // Broad search
            };
        case 'hypothesis-formation':
            return {
                ...baseConfig,
                heads: 14,
                crossDomainDiscovery: true,
                recallTarget: 0.98 // Comprehensive
            };
        case 'validation':
            return {
                ...baseConfig,
                heads: 16, // Maximum precision
                recallTarget: 0.99,
                crossDomainDiscovery: false // Focused
            };
        case 'writing':
            return {
                ...baseConfig,
                heads: 10,
                forwardPassTargetMs: 50, // Fast lookups
                recallTarget: 0.95
            };
    }
}
export function analyzeCitationNetwork(papers) {
    // Placeholder implementation
    return {
        networkDensity: 0.15,
        clusteringCoefficient: 0.45,
        averagePathLength: 3.2,
        communityModularity: 0.78
    };
}
//# sourceMappingURL=scientific-research.js.map