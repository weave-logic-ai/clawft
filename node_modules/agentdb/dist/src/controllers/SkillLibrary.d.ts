/**
 * SkillLibrary - Lifelong Learning Skill Management
 *
 * Promotes high-reward trajectories into reusable skills.
 * Manages skill composition, relationships, and adaptive selection.
 *
 * Based on: "Voyager: An Open-Ended Embodied Agent with Large Language Models"
 * https://arxiv.org/abs/2305.16291
 */
import type { IDatabaseConnection } from '../types/database.types.js';
import { EmbeddingService } from './EmbeddingService.js';
import { VectorBackend } from '../backends/VectorBackend.js';
import { type QueryCacheConfig } from '../core/QueryCache.js';
export interface Skill {
    id?: number;
    name: string;
    description?: string;
    signature?: {
        inputs: Record<string, any>;
        outputs: Record<string, any>;
    };
    code?: string;
    successRate: number;
    uses?: number;
    avgReward?: number;
    avgLatencyMs?: number;
    createdFromEpisode?: number;
    metadata?: Record<string, any>;
}
export interface SkillLink {
    parentSkillId: number;
    childSkillId: number;
    relationship: 'prerequisite' | 'alternative' | 'refinement' | 'composition';
    weight: number;
    metadata?: Record<string, any>;
}
export interface SkillQuery {
    /** v2 API: task description */
    task?: string;
    /** v1 API: query string (alias for task) */
    query?: string;
    k?: number;
    minSuccessRate?: number;
    preferRecent?: boolean;
}
export declare class SkillLibrary {
    private db;
    private embedder;
    private vectorBackend;
    private graphBackend?;
    private queryCache;
    constructor(db: IDatabaseConnection, embedder: EmbeddingService, vectorBackend?: VectorBackend, graphBackend?: any, cacheConfig?: QueryCacheConfig);
    /**
     * Create a new skill manually or from an episode
     * Invalidates skill cache
     */
    createSkill(skill: Skill): Promise<number>;
    /**
     * Update skill statistics after use
     * Invalidates skill cache
     */
    updateSkillStats(skillId: number, success: boolean, reward: number, latencyMs: number): void;
    /**
     * Retrieve skills relevant to a task
     */
    searchSkills(query: SkillQuery): Promise<Skill[]>;
    retrieveSkills(query: SkillQuery): Promise<Skill[]>;
    /**
     * Legacy SQL-based skill retrieval (fallback when VectorBackend not available)
     */
    private retrieveSkillsLegacy;
    /**
     * Store skill embedding (legacy fallback)
     */
    private storeSkillEmbeddingLegacy;
    /**
     * Cosine similarity between two vectors
     */
    private cosineSimilarity;
    /**
     * Link two skills with a relationship
     */
    linkSkills(link: SkillLink): void;
    /**
     * Get skill composition plan (prerequisites and alternatives)
     */
    getSkillPlan(skillId: number): {
        skill: Skill;
        prerequisites: Skill[];
        alternatives: Skill[];
        refinements: Skill[];
    };
    /**
     * Consolidate high-reward episodes into skills with ML pattern extraction
     * This is the core learning mechanism enhanced with pattern analysis
     */
    consolidateEpisodesIntoSkills(config: {
        minAttempts?: number;
        minReward?: number;
        timeWindowDays?: number;
        extractPatterns?: boolean;
    }): Promise<{
        created: number;
        updated: number;
        patterns: Array<{
            task: string;
            commonPatterns: string[];
            successIndicators: string[];
            avgReward: number;
        }>;
    }>;
    /**
     * Extract common patterns from successful episodes using ML-inspired analysis
     */
    private extractPatternsFromEpisodes;
    /**
     * Extract keyword frequency from text array using NLP-inspired techniques
     */
    private extractKeywordFrequency;
    /**
     * Get top N keywords by frequency
     */
    private getTopKeywords;
    /**
     * Extract common patterns from episode metadata
     */
    private extractMetadataPatterns;
    /**
     * Analyze learning trend across episodes
     */
    private analyzeLearningTrend;
    /**
     * Calculate pattern confidence score based on sample size and success rate
     */
    private calculatePatternConfidence;
    /**
     * Prune underperforming skills
     * Invalidates cache on completion
     */
    pruneSkills(config: {
        minUses?: number;
        minSuccessRate?: number;
        maxAgeDays?: number;
    }): number;
    /**
     * Get query cache statistics
     */
    getCacheStats(): import("../core/QueryCache.js").CacheStatistics;
    /**
     * Clear query cache
     */
    clearCache(): void;
    /**
     * Prune expired cache entries
     */
    pruneCache(): number;
    /**
     * Warm cache with common skill queries
     */
    warmCache(commonTasks: string[]): Promise<void>;
    private getSkillById;
    private rowToSkill;
    private buildSkillText;
    /**
     * Compute composite skill score from similarity and metadata
     * VectorBackend provides normalized similarity (0-1)
     */
    private computeSkillScore;
}
//# sourceMappingURL=SkillLibrary.d.ts.map