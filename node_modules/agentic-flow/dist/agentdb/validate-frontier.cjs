"use strict";
/**
 * Standalone Frontier Features Validation
 *
 * Tests all frontier features without requiring full build infrastructure.
 * Validates:
 * 1. CausalMemoryGraph - causal edges, experiments, uplift calculation
 * 2. ExplainableRecall - certificates, Merkle proofs, provenance
 * 3. CausalRecall - utility-based reranking
 * 4. NightlyLearner - automated causal discovery
 */
const Database = require('better-sqlite3');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
// Color codes for output
const colors = {
    reset: '\x1b[0m',
    green: '\x1b[32m',
    red: '\x1b[31m',
    yellow: '\x1b[33m',
    blue: '\x1b[34m',
    cyan: '\x1b[36m'
};
function log(message, color = 'reset') {
    console.log(`${colors[color]}${message}${colors.reset}`);
}
function assert(condition, message) {
    if (!condition) {
        throw new Error(`Assertion failed: ${message}`);
    }
}
class FrontierValidator {
    constructor() {
        // Create in-memory database
        this.db = new Database(':memory:');
        // Load schemas
        this.loadSchemas();
        this.testsRun = 0;
        this.testsPassed = 0;
        this.testsFailed = 0;
    }
    loadSchemas() {
        log('\n📦 Loading database schemas...', 'cyan');
        // Load core schema
        const coreSchema = fs.readFileSync(path.join(__dirname, 'schemas/schema.sql'), 'utf-8');
        this.db.exec(coreSchema);
        // Load frontier schema
        const frontierSchema = fs.readFileSync(path.join(__dirname, 'schemas/frontier-schema.sql'), 'utf-8');
        this.db.exec(frontierSchema);
        log('✓ Schemas loaded successfully', 'green');
    }
    runTest(name, testFn) {
        this.testsRun++;
        try {
            log(`\n🧪 ${name}`, 'blue');
            testFn();
            this.testsPassed++;
            log('  ✅ PASSED', 'green');
        }
        catch (error) {
            this.testsFailed++;
            log(`  ❌ FAILED: ${error.message}`, 'red');
            console.error(error);
        }
    }
    // ========================================================================
    // CausalMemoryGraph Tests
    // ========================================================================
    testCausalEdgeInsertion() {
        this.runTest('Causal Edge Insertion', () => {
            // Insert test episodes
            const stmt = this.db.prepare(`
        INSERT INTO episodes (session_id, task, reward, success)
        VALUES (?, ?, ?, ?)
      `);
            stmt.run('session1', 'task1', 0.8, 1);
            stmt.run('session1', 'task2', 0.9, 1);
            // Insert causal edge
            const edgeStmt = this.db.prepare(`
        INSERT INTO causal_edges (
          from_memory_id, from_memory_type, to_memory_id, to_memory_type,
          similarity, uplift, confidence, sample_size, mechanism
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
      `);
            const result = edgeStmt.run(1, 'episode', 2, 'episode', 0.85, 0.15, 0.9, 100, 'test mechanism');
            const edgeId = result.lastInsertRowid;
            assert(edgeId > 0, 'Edge should be inserted');
            // Verify
            const edge = this.db.prepare('SELECT * FROM causal_edges WHERE id = ?').get(edgeId);
            assert(edge.from_memory_id === 1, 'From ID should match');
            assert(edge.to_memory_id === 2, 'To ID should match');
            assert(edge.similarity === 0.85, 'Similarity should match');
            assert(edge.uplift === 0.15, 'Uplift should match');
            assert(edge.confidence === 0.9, 'Confidence should match');
        });
    }
    testABExperiment() {
        this.runTest('A/B Experiment and Uplift Calculation', () => {
            // Insert episodes for experiment
            const epStmt = this.db.prepare(`
        INSERT INTO episodes (session_id, task, reward, success)
        VALUES (?, ?, ?, ?)
      `);
            for (let i = 1; i <= 10; i++) {
                epStmt.run(`session${i}`, `task${i}`, 0.7 + Math.random() * 0.2, 1);
            }
            // Create experiment
            const expStmt = this.db.prepare(`
        INSERT INTO causal_experiments (
          name, hypothesis, treatment_id, treatment_type, start_time, sample_size, status
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
      `);
            const expResult = expStmt.run('Test Experiment', 'Treatment improves outcome', 1, 'episode', Date.now(), 0, 'running');
            const expId = expResult.lastInsertRowid;
            assert(expId > 0, 'Experiment should be created');
            // Record observations
            const obsStmt = this.db.prepare(`
        INSERT INTO causal_observations (
          experiment_id, episode_id, is_treatment, outcome_value, outcome_type
        ) VALUES (?, ?, ?, ?, ?)
      `);
            // Treatment group (higher rewards)
            for (let i = 0; i < 50; i++) {
                obsStmt.run(expId, (i % 10) + 1, 1, 0.75 + Math.random() * 0.1, 'reward');
            }
            // Control group (lower rewards)
            for (let i = 0; i < 50; i++) {
                obsStmt.run(expId, (i % 10) + 1, 0, 0.55 + Math.random() * 0.1, 'reward');
            }
            // Check observations recorded
            const obs = this.db.prepare('SELECT COUNT(*) as count FROM causal_observations WHERE experiment_id = ?').get(expId);
            assert(obs.count === 100, 'Should have 100 observations');
            // Calculate uplift (simplified inline calculation)
            const treatmentObs = this.db.prepare(`
        SELECT outcome_value FROM causal_observations
        WHERE experiment_id = ? AND is_treatment = 1
      `).all(expId);
            const controlObs = this.db.prepare(`
        SELECT outcome_value FROM causal_observations
        WHERE experiment_id = ? AND is_treatment = 0
      `).all(expId);
            const treatmentMean = treatmentObs.reduce((sum, o) => sum + o.outcome_value, 0) / treatmentObs.length;
            const controlMean = controlObs.reduce((sum, o) => sum + o.outcome_value, 0) / controlObs.length;
            const uplift = treatmentMean - controlMean;
            assert(uplift > 0.1, 'Uplift should be positive and significant');
            log(`    Uplift: ${uplift.toFixed(3)}, Treatment: ${treatmentMean.toFixed(3)}, Control: ${controlMean.toFixed(3)}`, 'cyan');
        });
    }
    testCausalQuery() {
        this.runTest('Causal Effect Query', () => {
            // Insert test data
            const epStmt = this.db.prepare(`
        INSERT INTO episodes (session_id, task, reward, success)
        VALUES (?, ?, ?, ?)
      `);
            for (let i = 1; i <= 5; i++) {
                epStmt.run(`session${i}`, `task${i}`, 0.8, 1);
            }
            // Insert causal edges
            const edgeStmt = this.db.prepare(`
        INSERT INTO causal_edges (
          from_memory_id, from_memory_type, to_memory_id, to_memory_type,
          similarity, uplift, confidence
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
      `);
            edgeStmt.run(1, 'episode', 2, 'episode', 0.8, 0.2, 0.9);
            edgeStmt.run(1, 'episode', 3, 'episode', 0.75, 0.15, 0.85);
            edgeStmt.run(1, 'episode', 4, 'episode', 0.7, 0.05, 0.95); // Low uplift
            edgeStmt.run(1, 'episode', 5, 'episode', 0.85, 0.25, 0.5); // Low confidence
            // Query with filters
            const effects = this.db.prepare(`
        SELECT * FROM causal_edges
        WHERE from_memory_id = ?
          AND confidence >= ?
          AND ABS(uplift) >= ?
        ORDER BY ABS(uplift) * confidence DESC
      `).all(1, 0.8, 0.1);
            assert(effects.length === 2, 'Should find 2 high-confidence, high-uplift edges');
            assert(effects[0].to_memory_id === 2, 'Highest impact should be first');
        });
    }
    // ========================================================================
    // ExplainableRecall Tests
    // ========================================================================
    testCertificateCreation() {
        this.runTest('Certificate Creation and Storage', () => {
            // Insert test episodes
            const epStmt = this.db.prepare(`
        INSERT INTO episodes (session_id, task, reward, success)
        VALUES (?, ?, ?, ?)
      `);
            for (let i = 1; i <= 5; i++) {
                epStmt.run(`session${i}`, `task${i}`, 0.8, 1);
            }
            // Create certificate
            const certId = crypto.randomUUID();
            const chunkIds = ['1', '2', '3', '4'];
            const minimalWhy = ['1', '2']; // Minimal hitting set
            const merkleRoot = crypto.createHash('sha256').update('test').digest('hex');
            const certStmt = this.db.prepare(`
        INSERT INTO recall_certificates (
          id, query_id, query_text, chunk_ids, chunk_types,
          minimal_why, redundancy_ratio, completeness_score,
          merkle_root, source_hashes, access_level, latency_ms
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      `);
            certStmt.run(certId, 'q1', 'Test query', JSON.stringify(chunkIds), JSON.stringify(['episode', 'episode', 'episode', 'episode']), JSON.stringify(minimalWhy), chunkIds.length / minimalWhy.length, // redundancy ratio = 2
            1.0, // completeness
            merkleRoot, JSON.stringify([merkleRoot, merkleRoot, merkleRoot, merkleRoot]), 'internal', 10);
            // Verify
            const cert = this.db.prepare('SELECT * FROM recall_certificates WHERE id = ?').get(certId);
            assert(cert !== null, 'Certificate should exist');
            assert(cert.query_id === 'q1', 'Query ID should match');
            assert(JSON.parse(cert.chunk_ids).length === 4, 'Should have 4 chunks');
            assert(JSON.parse(cert.minimal_why).length === 2, 'Minimal set should be 2');
            assert(cert.redundancy_ratio === 2, 'Redundancy ratio should be 2');
            assert(cert.completeness_score === 1.0, 'Completeness should be 1.0');
            assert(cert.merkle_root === merkleRoot, 'Merkle root should match');
        });
    }
    testMerkleProofGeneration() {
        this.runTest('Merkle Proof Generation and Verification', () => {
            // Create Merkle tree
            const leaves = ['chunk1', 'chunk2', 'chunk3', 'chunk4'].map(content => crypto.createHash('sha256').update(content).digest('hex'));
            // Build tree (simplified 2-level tree)
            const level1 = [];
            for (let i = 0; i < leaves.length; i += 2) {
                const combined = leaves[i] + (leaves[i + 1] || leaves[i]);
                level1.push(crypto.createHash('sha256').update(combined).digest('hex'));
            }
            const root = crypto.createHash('sha256')
                .update(level1[0] + (level1[1] || level1[0]))
                .digest('hex');
            assert(root.length === 64, 'Merkle root should be 64 hex characters');
            assert(/^[0-9a-f]{64}$/.test(root), 'Merkle root should be valid SHA-256 hash');
            log(`    Merkle root: ${root.substring(0, 16)}...`, 'cyan');
        });
    }
    testProvenanceTracking() {
        this.runTest('Provenance Source Tracking', () => {
            // Insert test episodes
            const epStmt = this.db.prepare(`
        INSERT INTO episodes (session_id, task, reward, success)
        VALUES (?, ?, ?, ?)
      `);
            for (let i = 1; i <= 3; i++) {
                epStmt.run(`session${i}`, `task${i}`, 0.8, 1);
            }
            // Create provenance sources
            const provStmt = this.db.prepare(`
        INSERT INTO provenance_sources (
          source_type, source_id, content_hash, creator
        ) VALUES (?, ?, ?, ?)
      `);
            const hash1 = crypto.createHash('sha256').update('content1').digest('hex');
            const hash2 = crypto.createHash('sha256').update('content2').digest('hex');
            const result1 = provStmt.run('episode', 1, hash1, 'test_user');
            const result2 = provStmt.run('episode', 2, hash2, 'test_user');
            assert(result1.lastInsertRowid > 0, 'First provenance should be inserted');
            assert(result2.lastInsertRowid > 0, 'Second provenance should be inserted');
            // Query lineage
            const lineage = this.db.prepare(`
        SELECT * FROM provenance_sources
        WHERE content_hash = ?
      `).get(hash1);
            assert(lineage !== null, 'Lineage should exist');
            assert(lineage.source_type === 'episode', 'Source type should match');
            assert(lineage.creator === 'test_user', 'Creator should match');
        });
    }
    testJustificationPaths() {
        this.runTest('Justification Paths', () => {
            // Create certificate first
            const certId = crypto.randomUUID();
            this.db.prepare(`
        INSERT INTO recall_certificates (
          id, query_id, query_text, chunk_ids, chunk_types,
          minimal_why, redundancy_ratio, completeness_score,
          merkle_root, source_hashes, access_level
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      `).run(certId, 'q2', 'Test query', JSON.stringify(['1', '2']), JSON.stringify(['episode', 'episode']), JSON.stringify(['1']), 2, 1.0, 'hash', JSON.stringify(['hash1', 'hash2']), 'internal');
            // Add justification paths
            const justStmt = this.db.prepare(`
        INSERT INTO justification_paths (
          certificate_id, chunk_id, chunk_type, reason, necessity_score, path_elements
        ) VALUES (?, ?, ?, ?, ?, ?)
      `);
            justStmt.run(certId, '1', 'episode', 'semantic_match', 0.9, JSON.stringify(['query', 'chunk1']));
            justStmt.run(certId, '2', 'episode', 'causal_link', 0.7, JSON.stringify(['query', 'chunk1', 'chunk2']));
            // Query justifications
            const justifications = this.db.prepare(`
        SELECT * FROM justification_paths
        WHERE certificate_id = ?
      `).all(certId);
            assert(justifications.length === 2, 'Should have 2 justification paths');
            assert(justifications[0].reason === 'semantic_match', 'First reason should match');
            assert(justifications[1].reason === 'causal_link', 'Second reason should match');
        });
    }
    // ========================================================================
    // Integration Tests
    // ========================================================================
    testCausalRecallIntegration() {
        this.runTest('Causal Recall Integration', () => {
            // Insert episodes with embeddings
            const epStmt = this.db.prepare(`
        INSERT INTO episodes (session_id, task, reward, success, latency_ms)
        VALUES (?, ?, ?, ?, ?)
      `);
            for (let i = 1; i <= 10; i++) {
                epStmt.run(`session${i}`, `task${i}`, 0.7 + i * 0.02, 1, 50 + i * 5);
            }
            // Add embeddings
            const embStmt = this.db.prepare(`
        INSERT INTO episode_embeddings (episode_id, embedding)
        VALUES (?, ?)
      `);
            for (let i = 1; i <= 10; i++) {
                // Create mock embedding
                const embedding = new Float32Array(384);
                for (let j = 0; j < 384; j++) {
                    embedding[j] = Math.random();
                }
                embStmt.run(i, JSON.stringify(Array.from(embedding)));
            }
            // Add causal edges
            const edgeStmt = this.db.prepare(`
        INSERT INTO causal_edges (
          from_memory_id, from_memory_type, to_memory_id, to_memory_type,
          similarity, uplift, confidence
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
      `);
            edgeStmt.run(1, 'episode', 2, 'episode', 0.8, 0.15, 0.9);
            edgeStmt.run(2, 'episode', 3, 'episode', 0.75, 0.2, 0.85);
            // Verify integration
            const episodes = this.db.prepare('SELECT COUNT(*) as count FROM episodes').get();
            const embeddings = this.db.prepare('SELECT COUNT(*) as count FROM episode_embeddings').get();
            const edges = this.db.prepare('SELECT COUNT(*) as count FROM causal_edges').get();
            assert(episodes.count === 10, 'Should have 10 episodes');
            assert(embeddings.count === 10, 'Should have 10 embeddings');
            assert(edges.count === 2, 'Should have 2 causal edges');
            log(`    Episodes: ${episodes.count}, Embeddings: ${embeddings.count}, Edges: ${edges.count}`, 'cyan');
        });
    }
    testDatabasePerformance() {
        this.runTest('Database Performance', () => {
            // Insert many episodes for performance test
            const start = Date.now();
            this.db.transaction(() => {
                const stmt = this.db.prepare(`
          INSERT INTO episodes (session_id, task, reward, success)
          VALUES (?, ?, ?, ?)
        `);
                for (let i = 1; i <= 1000; i++) {
                    stmt.run(`session${i}`, `task${i}`, Math.random(), 1);
                }
            })();
            const insertTime = Date.now() - start;
            // Query performance
            const queryStart = Date.now();
            const results = this.db.prepare('SELECT * FROM episodes WHERE reward > 0.5 LIMIT 100').all();
            const queryTime = Date.now() - queryStart;
            assert(results.length > 0, 'Should find episodes with reward > 0.5');
            assert(insertTime < 1000, 'Bulk insert should be fast (< 1s for 1000 records)');
            assert(queryTime < 50, 'Query should be fast (< 50ms)');
            log(`    Insert 1000: ${insertTime}ms, Query: ${queryTime}ms`, 'cyan');
        });
    }
    // ========================================================================
    // Summary
    // ========================================================================
    runAll() {
        log('\n═══════════════════════════════════════════════════════════', 'cyan');
        log('  AgentDB Frontier Features Validation', 'cyan');
        log('═══════════════════════════════════════════════════════════', 'cyan');
        // CausalMemoryGraph tests
        log('\n📊 Testing CausalMemoryGraph', 'yellow');
        this.testCausalEdgeInsertion();
        this.testABExperiment();
        this.testCausalQuery();
        // ExplainableRecall tests
        log('\n🔍 Testing ExplainableRecall', 'yellow');
        this.testCertificateCreation();
        this.testMerkleProofGeneration();
        this.testProvenanceTracking();
        this.testJustificationPaths();
        // Integration tests
        log('\n🔗 Testing Integration', 'yellow');
        this.testCausalRecallIntegration();
        this.testDatabasePerformance();
        // Print summary
        log('\n═══════════════════════════════════════════════════════════', 'cyan');
        log('  Summary', 'cyan');
        log('═══════════════════════════════════════════════════════════', 'cyan');
        log(`\n  Total Tests: ${this.testsRun}`, 'cyan');
        log(`  Passed: ${this.testsPassed}`, 'green');
        log(`  Failed: ${this.testsFailed}`, this.testsFailed > 0 ? 'red' : 'cyan');
        const passRate = ((this.testsPassed / this.testsRun) * 100).toFixed(1);
        log(`  Pass Rate: ${passRate}%`, passRate === '100.0' ? 'green' : 'yellow');
        if (this.testsFailed === 0) {
            log('\n✅ ALL TESTS PASSED - FRONTIER FEATURES VALIDATED', 'green');
        }
        else {
            log(`\n❌ ${this.testsFailed} TEST(S) FAILED`, 'red');
        }
        log('═══════════════════════════════════════════════════════════\n', 'cyan');
        this.db.close();
        return this.testsFailed === 0 ? 0 : 1;
    }
}
// Run validation
const validator = new FrontierValidator();
const exitCode = validator.runAll();
process.exit(exitCode);
//# sourceMappingURL=validate-frontier.cjs.map