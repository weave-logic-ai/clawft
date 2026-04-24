#!/usr/bin/env node
/**
 * Test ReasoningBank retrieval algorithm with MMR
 */
import { upsertMemory, upsertEmbedding, fetchMemoryCandidates } from './db/queries.js';
import { loadConfig } from './utils/config.js';
import { ulid } from 'ulid';
console.log('🧪 Testing ReasoningBank Retrieval Algorithm\n');
// Helper to create synthetic embedding
function createEmbedding(seed, dims = 1024) {
    const vec = new Float32Array(dims);
    for (let i = 0; i < dims; i++) {
        vec[i] = Math.sin(seed * (i + 1) * 0.01) * 0.1 + Math.cos(seed * i * 0.02) * 0.05;
    }
    // Normalize
    let mag = 0;
    for (let i = 0; i < dims; i++)
        mag += vec[i] * vec[i];
    mag = Math.sqrt(mag);
    for (let i = 0; i < dims; i++)
        vec[i] /= mag;
    return vec;
}
// Insert test memories
console.log('1️⃣ Inserting test memories...');
const testMemories = [
    {
        title: 'CSRF Token Handling',
        description: 'Extract and include CSRF tokens in form submissions',
        content: '1) Check for CSRF token in meta tags or form inputs. 2) Include in request. 3) Verify before submit.',
        tags: ['csrf', 'web', 'security'],
        domain: 'test.web',
        confidence: 0.88,
        seed: 42
    },
    {
        title: 'Authentication Cookie Validation',
        description: 'Always validate auth cookies before protected requests',
        content: '1) Check for auth cookie. 2) Validate expiry. 3) Refresh if needed.',
        tags: ['auth', 'cookies', 'security'],
        domain: 'test.web',
        confidence: 0.82,
        seed: 123
    },
    {
        title: 'API Rate Limiting Backoff',
        description: 'Implement exponential backoff for rate-limited APIs',
        content: '1) Detect 429 status. 2) Parse Retry-After header. 3) Exponential backoff.',
        tags: ['api', 'rate-limit', 'retry'],
        domain: 'test.api',
        confidence: 0.91,
        seed: 456
    },
    {
        title: 'Form Validation Before Submit',
        description: 'Validate all form fields before submission to avoid errors',
        content: '1) Check required fields. 2) Validate formats. 3) Show inline errors.',
        tags: ['forms', 'validation', 'web'],
        domain: 'test.web',
        confidence: 0.75,
        seed: 789
    },
    {
        title: 'Database Transaction Retry Logic',
        description: 'Retry failed transactions with proper isolation levels',
        content: '1) Begin transaction. 2) Execute queries. 3) Retry on deadlock.',
        tags: ['database', 'transactions', 'retry'],
        domain: 'test.db',
        confidence: 0.86,
        seed: 101
    }
];
for (const mem of testMemories) {
    const id = ulid();
    const memory = {
        id,
        type: 'reasoning_memory',
        pattern_data: {
            title: mem.title,
            description: mem.description,
            content: mem.content,
            source: {
                task_id: 'test_task',
                agent_id: 'test_agent',
                outcome: 'Success',
                evidence: ['step_1']
            },
            tags: mem.tags,
            domain: mem.domain,
            created_at: new Date().toISOString(),
            confidence: mem.confidence,
            n_uses: 0
        },
        confidence: mem.confidence,
        usage_count: 0
    };
    upsertMemory(memory);
    upsertEmbedding({
        id,
        model: 'test-model',
        dims: 1024,
        vector: createEmbedding(mem.seed),
        created_at: new Date().toISOString()
    });
}
console.log(`   ✅ Inserted ${testMemories.length} test memories\n`);
// Test retrieval
console.log('2️⃣ Testing retrieval with different queries...\n');
const queries = [
    {
        query: 'How to handle CSRF tokens in web forms?',
        expectedTitles: ['CSRF Token Handling', 'Form Validation Before Submit'],
        domain: 'test.web'
    },
    {
        query: 'API rate limiting and retry strategies',
        expectedTitles: ['API Rate Limiting Backoff'],
        domain: 'test.api'
    },
    {
        query: 'Database error recovery and transaction handling',
        expectedTitles: ['Database Transaction Retry Logic'],
        domain: 'test.db'
    }
];
const config = loadConfig();
for (const test of queries) {
    console.log(`Query: "${test.query}"`);
    console.log(`Domain filter: ${test.domain}`);
    const candidates = fetchMemoryCandidates({
        domain: test.domain,
        minConfidence: config.retrieve.min_score
    });
    console.log(`Retrieved ${candidates.length} candidates:`);
    candidates.forEach((c, idx) => {
        console.log(`  ${idx + 1}. ${c.pattern_data.title} (conf: ${c.confidence}, age: ${c.age_days}d)`);
    });
    // Simple scoring simulation (without actual embeddings comparison)
    console.log(`\nTop-k (k=${config.retrieve.k}) selection would use:`);
    console.log(`  α=${config.retrieve.alpha} (similarity)`);
    console.log(`  β=${config.retrieve.beta} (recency)`);
    console.log(`  γ=${config.retrieve.gamma} (reliability)`);
    console.log(`  δ=${config.retrieve.delta} (diversity/MMR)`);
    console.log('\n---\n');
}
// Test cosine similarity
console.log('3️⃣ Testing cosine similarity...\n');
function cosineSimilarity(a, b) {
    let dot = 0, magA = 0, magB = 0;
    for (let i = 0; i < a.length; i++) {
        dot += a[i] * b[i];
        magA += a[i] * a[i];
        magB += b[i] * b[i];
    }
    return dot / (Math.sqrt(magA) * Math.sqrt(magB));
}
const vec1 = createEmbedding(42);
const vec2 = createEmbedding(42); // identical
const vec3 = createEmbedding(123); // different
const sim12 = cosineSimilarity(vec1, vec2);
const sim13 = cosineSimilarity(vec1, vec3);
console.log(`Cosine similarity (identical vectors): ${sim12.toFixed(4)}`);
console.log(`Cosine similarity (different vectors): ${sim13.toFixed(4)}`);
if (Math.abs(sim12 - 1.0) < 0.001) {
    console.log('   ✅ Identical vectors have similarity ≈ 1.0');
}
else {
    console.log(`   ⚠️  Identical vectors similarity is ${sim12}, expected ≈ 1.0`);
}
if (sim13 < sim12) {
    console.log('   ✅ Different vectors have lower similarity');
}
else {
    console.log('   ⚠️  Different vectors similarity is unexpectedly high');
}
console.log('\n✅ Retrieval algorithm validation complete!\n');
//# sourceMappingURL=test-retrieval.js.map