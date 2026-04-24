import { parse } from 'yaml';
import { readFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
let configCache = null;
// Default configuration
const DEFAULT_CONFIG = {
    retrieve: {
        k: 3,
        alpha: 0.65,
        beta: 0.15,
        gamma: 0.20,
        delta: 0.10,
        recency_half_life_days: 45,
        min_score: 0.3
    },
    judge: {
        model: 'claude-sonnet-4-5-20250929',
        max_tokens: 512,
        temperature: 0,
        confidence_threshold: 0.5
    },
    distill: {
        model: undefined,
        max_tokens: undefined,
        temperature: undefined,
        max_items_success: 3,
        max_items_failure: 2,
        confidence_prior_success: 0.75,
        confidence_prior_failure: 0.60
    },
    consolidate: {
        duplicate_threshold: 0.95,
        contradiction_threshold: 0.85,
        trigger_threshold: 20,
        prune_age_days: 180,
        prune_min_confidence: 0.3,
        min_confidence_keep: 0.5
    },
    matts: {
        parallel_k: 3,
        sequential_k: 5,
        sequential_r: 5,
        sequential_stop_on_success: true,
        confidence_boost: 0.05
    },
    embeddings: {
        provider: 'claude',
        model: 'claude-sonnet-4-5-20250929',
        dims: 1024,
        dimensions: 1024,
        cache_ttl_seconds: 3600
    },
    governance: {
        scrub_pii: true,
        pii_scrubber: true,
        tenant_scoped: false
    },
    features: {
        enable_pre_task_hook: true,
        enable_post_task_hook: true,
        enable_matts_parallel: true
    }
};
// Try multiple paths to find config file
function findConfigPath() {
    const paths = [
        // Development: relative to source file
        join(__dirname, '../config/reasoningbank.yaml'),
        // npm package: relative to dist file
        join(__dirname, '../../config/reasoningbank.yaml'),
        // User override: current working directory
        join(process.cwd(), '.swarm/reasoningbank.yaml'),
        join(process.cwd(), 'reasoningbank.yaml')
    ];
    for (const path of paths) {
        if (existsSync(path)) {
            return path;
        }
    }
    return null;
}
export function loadConfig() {
    if (configCache)
        return configCache;
    const configPath = findConfigPath();
    // If no config file found, use defaults
    if (!configPath) {
        configCache = DEFAULT_CONFIG;
        return configCache;
    }
    try {
        const yamlContent = readFileSync(configPath, 'utf-8');
        const parsed = parse(yamlContent);
        // Handle nested reasoningbank: key
        const raw = parsed.reasoningbank || parsed;
        // Map the full config to our simplified interface
        configCache = {
            retrieve: {
                k: raw.retrieve?.k ?? 3,
                alpha: raw.retrieve?.alpha ?? 0.65,
                beta: raw.retrieve?.beta ?? 0.15,
                gamma: raw.retrieve?.gamma ?? 0.20,
                delta: raw.retrieve?.delta ?? 0.10,
                recency_half_life_days: raw.retrieve?.recency_half_life_days ?? 45,
                min_score: raw.retrieve?.min_score ?? 0.3
            },
            judge: {
                model: raw.judge?.model ?? 'claude-sonnet-4-5-20250929',
                max_tokens: raw.judge?.max_tokens ?? 512,
                temperature: raw.judge?.temperature ?? 0,
                confidence_threshold: raw.judge?.fallback_confidence ?? 0.5
            },
            distill: {
                model: raw.distill?.model,
                max_tokens: raw.distill?.max_tokens,
                temperature: raw.distill?.temperature,
                max_items_success: raw.distill?.max_items_per_trajectory ?? 3,
                max_items_failure: 2,
                confidence_prior_success: raw.distill?.success_confidence_prior ?? 0.75,
                confidence_prior_failure: raw.distill?.failure_confidence_prior ?? 0.60
            },
            consolidate: {
                duplicate_threshold: raw.consolidate?.dedup_similarity_threshold ?? 0.95,
                contradiction_threshold: raw.consolidate?.contradiction_threshold ?? 0.85,
                trigger_threshold: raw.consolidate?.run_every_new_items ?? 20,
                prune_age_days: raw.consolidate?.prune_age_days ?? 180,
                prune_min_confidence: raw.consolidate?.min_confidence_keep ?? 0.3,
                min_confidence_keep: raw.consolidate?.min_confidence_keep ?? 0.5
            },
            matts: {
                parallel_k: raw.matts?.parallel?.k ?? 3,
                sequential_k: raw.matts?.sequential?.r ?? 5,
                sequential_r: raw.matts?.sequential?.r ?? 5,
                sequential_stop_on_success: raw.matts?.sequential?.stop_on_success ?? true,
                confidence_boost: 0.05
            },
            embeddings: {
                provider: raw.embeddings?.provider ?? 'claude',
                model: raw.embeddings?.model ?? 'claude-sonnet-4-5-20250929',
                dims: raw.embeddings?.dimensions ?? 1024,
                dimensions: raw.embeddings?.dimensions ?? 1024,
                cache_ttl_seconds: raw.embeddings?.cache_ttl_seconds ?? 3600
            },
            governance: {
                scrub_pii: raw.governance?.pii_scrubber ?? true,
                pii_scrubber: raw.governance?.pii_scrubber ?? true,
                tenant_scoped: raw.governance?.tenant_scoped ?? false
            },
            features: {
                enable_pre_task_hook: raw.features?.enable_pre_task_hook ?? true,
                enable_post_task_hook: raw.features?.enable_post_task_hook ?? true,
                enable_matts_parallel: raw.features?.enable_matts_parallel ?? true
            }
        };
        return configCache;
    }
    catch (error) {
        // If config file exists but can't be read, use defaults
        console.warn(`[ReasoningBank] Could not load config from ${configPath}, using defaults:`, error instanceof Error ? error.message : String(error));
        configCache = DEFAULT_CONFIG;
        return configCache;
    }
}
export function clearConfigCache() {
    configCache = null;
}
//# sourceMappingURL=config.js.map