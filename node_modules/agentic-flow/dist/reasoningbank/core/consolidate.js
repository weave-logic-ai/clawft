/**
 * Memory Consolidation
 * Algorithm 4 from ReasoningBank paper: Dedup, Contradict, Prune
 */
import { ulid } from 'ulid';
import { loadConfig } from '../utils/config.js';
import { cosineSimilarity } from '../utils/mmr.js';
import * as db from '../db/queries.js';
/**
 * Run consolidation: deduplicate, detect contradictions, prune old memories
 */
export async function consolidate() {
    const config = loadConfig();
    const startTime = Date.now();
    console.log('[INFO] Starting memory consolidation...');
    const runId = ulid();
    const memories = db.getAllActiveMemories();
    console.log(`[INFO] Processing ${memories.length} active memories`);
    let duplicatesFound = 0;
    let contradictionsFound = 0;
    let itemsPruned = 0;
    // Step 1: Deduplicate similar memories
    duplicatesFound = await deduplicateMemories(memories, config.consolidate.duplicate_threshold);
    // Step 2: Detect contradictions
    contradictionsFound = await detectContradictions(memories, config.consolidate.contradiction_threshold);
    // Step 3: Prune old, unused memories
    itemsPruned = db.pruneOldMemories({
        maxAgeDays: config.consolidate.prune_age_days,
        minConfidence: config.consolidate.min_confidence_keep
    });
    const durationMs = Date.now() - startTime;
    // Store consolidation run
    db.storeConsolidationRun({
        run_id: runId,
        items_processed: memories.length,
        duplicates_found: duplicatesFound,
        contradictions_found: contradictionsFound,
        items_pruned: itemsPruned,
        duration_ms: durationMs
    });
    console.log(`[INFO] Consolidation complete: ${duplicatesFound} dupes, ${contradictionsFound} contradictions, ${itemsPruned} pruned in ${durationMs}ms`);
    db.logMetric('rb.consolidate.duration_ms', durationMs);
    db.logMetric('rb.consolidate.duplicates', duplicatesFound);
    db.logMetric('rb.consolidate.contradictions', contradictionsFound);
    db.logMetric('rb.consolidate.pruned', itemsPruned);
    return {
        itemsProcessed: memories.length,
        duplicatesFound,
        contradictionsFound,
        itemsPruned,
        durationMs
    };
}
/**
 * Deduplicate highly similar memories
 */
async function deduplicateMemories(memories, threshold) {
    let duplicatesFound = 0;
    // Fetch embeddings for all memories
    const dbConn = db.getDb();
    const embeddingsMap = new Map();
    for (const mem of memories) {
        const row = dbConn.prepare('SELECT vector FROM pattern_embeddings WHERE id = ?').get(mem.id);
        if (row) {
            embeddingsMap.set(mem.id, new Float32Array(row.vector));
        }
    }
    // Compare all pairs
    for (let i = 0; i < memories.length; i++) {
        for (let j = i + 1; j < memories.length; j++) {
            const mem1 = memories[i];
            const mem2 = memories[j];
            const emb1 = embeddingsMap.get(mem1.id);
            const emb2 = embeddingsMap.get(mem2.id);
            if (!emb1 || !emb2)
                continue;
            const similarity = cosineSimilarity(emb1, emb2);
            if (similarity >= threshold) {
                // Mark as duplicate
                db.storeLink(mem1.id, mem2.id, 'duplicate_of', similarity);
                duplicatesFound++;
                // Merge: keep the one with higher usage
                if (mem1.usage_count < mem2.usage_count) {
                    // Delete mem1 (lower usage)
                    dbConn.prepare('DELETE FROM patterns WHERE id = ?').run(mem1.id);
                    console.log(`[INFO] Merged duplicate: ${mem1.pattern_data.title} → ${mem2.pattern_data.title}`);
                }
            }
        }
    }
    return duplicatesFound;
}
/**
 * Detect contradicting memories
 * Uses embedding similarity + semantic analysis
 */
async function detectContradictions(memories, threshold) {
    let contradictionsFound = 0;
    const dbConn = db.getDb();
    const embeddingsMap = new Map();
    for (const mem of memories) {
        const row = dbConn.prepare('SELECT vector FROM pattern_embeddings WHERE id = ?').get(mem.id);
        if (row) {
            embeddingsMap.set(mem.id, new Float32Array(row.vector));
        }
    }
    // Look for memories with high similarity but opposite outcomes
    for (let i = 0; i < memories.length; i++) {
        for (let j = i + 1; j < memories.length; j++) {
            const mem1 = memories[i];
            const mem2 = memories[j];
            const emb1 = embeddingsMap.get(mem1.id);
            const emb2 = embeddingsMap.get(mem2.id);
            if (!emb1 || !emb2)
                continue;
            const similarity = cosineSimilarity(emb1, emb2);
            // High similarity but different outcomes = potential contradiction
            if (similarity >= threshold) {
                const outcome1 = mem1.pattern_data.source?.outcome;
                const outcome2 = mem2.pattern_data.source?.outcome;
                if (outcome1 !== outcome2) {
                    db.storeLink(mem1.id, mem2.id, 'contradicts', similarity);
                    contradictionsFound++;
                    console.log(`[WARN] Contradiction detected: ${mem1.pattern_data.title} vs ${mem2.pattern_data.title}`);
                }
            }
        }
    }
    return contradictionsFound;
}
/**
 * Check if consolidation should run
 * Returns true if threshold of new memories is reached
 */
export function shouldConsolidate() {
    const config = loadConfig();
    const newCount = db.countNewMemoriesSinceConsolidation();
    return newCount >= config.consolidate.trigger_threshold;
}
//# sourceMappingURL=consolidate.js.map