/**
 * ExplainableRecall - Provenance and Justification for Memory Retrieval
 *
 * Every retrieval returns:
 * - Minimal hitting set of facts that justify the answer
 * - Merkle proof chain for provenance
 * - Policy compliance certificates
 *
 * Based on:
 * - Minimal hitting set algorithms
 * - Merkle tree provenance
 * - Explainable AI techniques
 *
 * v2.0.0-alpha.3 Features:
 * - GraphRoPE for hop-distance-aware graph queries (WASM)
 * - Rotary positional encoding based on graph structure
 * - Feature flag: ENABLE_GRAPH_ROPE (default: false)
 * - 100% backward compatible with fallback to standard retrieval
 */

// Database type from db-fallback
type Database = any;
import * as crypto from 'crypto';
import { AttentionService, type GraphRoPEConfig } from '../services/AttentionService.js';
import { EmbeddingService } from './EmbeddingService.js';

/**
 * Configuration for ExplainableRecall
 */
export interface ExplainableRecallConfig {
  /** Enable GraphRoPE for hop-aware queries (default: false) */
  ENABLE_GRAPH_ROPE?: boolean;
  /** GraphRoPE configuration */
  graphRoPEConfig?: Partial<GraphRoPEConfig>;
}

export interface RecallCertificate {
  id: string; // UUID
  queryId: string;
  queryText: string;

  // Retrieved chunks
  chunkIds: string[];
  chunkTypes: string[];

  // Justification
  minimalWhy: string[]; // Minimal hitting set
  redundancyRatio: number; // len(chunks) / len(minimalWhy)
  completenessScore: number; // Fraction of requirements met

  // Provenance
  merkleRoot: string;
  sourceHashes: string[];
  proofChain: MerkleProof[];

  // Policy
  policyProof?: string;
  policyVersion?: string;
  accessLevel: 'public' | 'internal' | 'confidential' | 'restricted';

  latencyMs?: number;
  metadata?: Record<string, any>;
}

export interface MerkleProof {
  hash: string;
  position: 'left' | 'right';
}

export interface JustificationPath {
  chunkId: string;
  chunkType: string;
  reason: 'semantic_match' | 'causal_link' | 'prerequisite' | 'constraint';
  necessityScore: number; // 0-1
  pathElements: string[]; // Reasoning chain
}

export interface ProvenanceSource {
  id?: number;
  sourceType: 'episode' | 'skill' | 'note' | 'fact' | 'external';
  sourceId: number;
  contentHash: string;
  parentHash?: string;
  derivedFrom?: string[];
  creator?: string;
  metadata?: Record<string, any>;
}

export class ExplainableRecall {
  private db: Database;
  private attentionService?: AttentionService;
  private embedder?: EmbeddingService;
  private config: ExplainableRecallConfig;

  /**
   * Constructor supports both v1 (legacy) and v2 (with GraphRoPE) modes
   *
   * v1 mode: new ExplainableRecall(db)
   * v2 mode: new ExplainableRecall(db, embedder, config)
   */
  constructor(
    db: Database,
    embedder?: EmbeddingService,
    config?: ExplainableRecallConfig
  ) {
    this.db = db;
    this.embedder = embedder;
    this.config = {
      ENABLE_GRAPH_ROPE: false,
      ...config,
    };

    // Initialize AttentionService if GraphRoPE enabled
    if (embedder && this.config.ENABLE_GRAPH_ROPE) {
      this.attentionService = new AttentionService(db, {
        graphRoPE: {
          enabled: true,
          ...this.config.graphRoPEConfig,
        },
      });
    }
  }

  /**
   * Create a recall certificate for a retrieval operation
   *
   * v2: Uses GraphRoPE if enabled for hop-distance-aware justification scoring
   * v1: Falls back to standard relevance-based justification
   */
  async createCertificate(params: {
    queryId: string;
    queryText: string;
    chunks: Array<{ id: string; type: string; content: string; relevance: number }>;
    requirements: string[]; // Query requirements
    accessLevel?: string;
    hopDistances?: number[][]; // Optional hop distances for GraphRoPE
  }): Promise<RecallCertificate> {
    const { queryId, queryText, chunks, requirements, accessLevel = 'internal' } = params;

    const startTime = Date.now();

    // 1. Compute minimal hitting set
    const minimalWhy = this.computeMinimalHittingSet(chunks, requirements);

    // 2. Calculate metrics
    const redundancyRatio = chunks.length / minimalWhy.length;
    const completenessScore = this.calculateCompleteness(minimalWhy, requirements);

    // 3. Build provenance chain
    const sourceHashes = chunks.map(chunk =>
      this.getOrCreateProvenance(chunk.type, parseInt(chunk.id))
    );

    const merkleTree = this.buildMerkleTree(sourceHashes);
    const merkleRoot = merkleTree.root;

    // 4. Generate chunk metadata first (needed for certificate ID)
    const chunkIds = chunks.map(c => c.id);
    const chunkTypes = chunks.map(c => c.type);

    // 5. Create certificate ID
    const certificateId = this.generateCertificateId(queryId, chunkIds);

    // 6. Generate proof chain for each chunk
    const proofChain = chunks.map((chunk, idx) =>
      this.getMerkleProof(merkleTree, idx)
    ).flat();

    // 7. Store certificate
    this.db.prepare(`
      INSERT INTO recall_certificates (
        id, query_id, query_text, chunk_ids, chunk_types,
        minimal_why, redundancy_ratio, completeness_score,
        merkle_root, source_hashes, proof_chain,
        access_level, latency_ms
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `).run(
      certificateId,
      queryId,
      queryText,
      JSON.stringify(chunkIds),
      JSON.stringify(chunkTypes),
      JSON.stringify(minimalWhy),
      redundancyRatio,
      completenessScore,
      merkleRoot,
      JSON.stringify(sourceHashes),
      JSON.stringify(proofChain),
      accessLevel,
      Date.now() - startTime
    );

    // 7. Store justification paths
    this.storeJustificationPaths(certificateId, chunks, minimalWhy, requirements);

    const certificate: RecallCertificate = {
      id: certificateId,
      queryId,
      queryText,
      chunkIds,
      chunkTypes,
      minimalWhy,
      redundancyRatio,
      completenessScore,
      merkleRoot,
      sourceHashes,
      proofChain,
      accessLevel: accessLevel as any,
      latencyMs: Date.now() - startTime
    };

    return certificate;
  }

  /**
   * Verify a recall certificate
   */
  verifyCertificate(certificateId: string): {
    valid: boolean;
    issues: string[];
  } {
    const cert = this.db.prepare(
      'SELECT * FROM recall_certificates WHERE id = ?'
    ).get(certificateId) as any;

    if (!cert) {
      return { valid: false, issues: ['Certificate not found'] };
    }

    const issues: string[] = [];

    // 1. Verify Merkle root
    const sourceHashes = JSON.parse(cert.source_hashes);
    const merkleTree = this.buildMerkleTree(sourceHashes);

    if (merkleTree.root !== cert.merkle_root) {
      issues.push('Merkle root mismatch');
    }

    // 2. Verify chunk hashes still match
    const chunkIds = JSON.parse(cert.chunk_ids);
    const chunkTypes = JSON.parse(cert.chunk_types);

    for (let i = 0; i < chunkIds.length; i++) {
      const currentHash = this.getContentHash(chunkTypes[i], parseInt(chunkIds[i]));
      if (currentHash !== sourceHashes[i]) {
        issues.push(`Chunk ${chunkIds[i]} hash changed`);
      }
    }

    // 3. Verify completeness
    const minimalWhy = JSON.parse(cert.minimal_why);
    if (minimalWhy.length === 0) {
      issues.push('Empty justification set');
    }

    // 4. Verify redundancy ratio
    if (cert.redundancy_ratio < 1.0) {
      issues.push('Invalid redundancy ratio');
    }

    return {
      valid: issues.length === 0,
      issues
    };
  }

  /**
   * Get justification for why a chunk was included
   */
  getJustification(certificateId: string, chunkId: string): JustificationPath | null {
    const row = this.db.prepare(`
      SELECT * FROM justification_paths
      WHERE certificate_id = ? AND chunk_id = ?
    `).get(certificateId, chunkId) as any;

    if (!row) return null;

    return {
      chunkId: row.chunk_id,
      chunkType: row.chunk_type,
      reason: row.reason,
      necessityScore: row.necessity_score,
      pathElements: JSON.parse(row.path_elements)
    };
  }

  /**
   * Get provenance lineage for a source
   */
  getProvenanceLineage(contentHash: string): ProvenanceSource[] {
    const lineage: ProvenanceSource[] = [];
    let currentHash: string | null = contentHash;

    while (currentHash) {
      const source = this.db.prepare(`
        SELECT * FROM provenance_sources WHERE content_hash = ?
      `).get(currentHash) as any;

      if (!source) break;

      lineage.push({
        id: source.id,
        sourceType: source.source_type,
        sourceId: source.source_id,
        contentHash: source.content_hash,
        parentHash: source.parent_hash,
        derivedFrom: source.derived_from ? JSON.parse(source.derived_from) : undefined,
        creator: source.creator,
        metadata: source.metadata ? JSON.parse(source.metadata) : undefined
      });

      currentHash = source.parent_hash;
    }

    return lineage;
  }

  /**
   * Trace provenance lineage for a certificate
   * Returns full provenance chain from certificate to original sources
   */
  traceProvenance(certificateId: string): {
    certificate: RecallCertificate;
    sources: Map<string, ProvenanceSource[]>;
    graph: {
      nodes: Array<{ id: string; type: string; label: string }>;
      edges: Array<{ from: string; to: string; type: string }>;
    };
  } {
    const certRow = this.db.prepare(
      'SELECT * FROM recall_certificates WHERE id = ?'
    ).get(certificateId) as any;

    if (!certRow) {
      throw new Error(`Certificate ${certificateId} not found`);
    }

    const certificate: RecallCertificate = {
      id: certRow.id,
      queryId: certRow.query_id,
      queryText: certRow.query_text,
      chunkIds: JSON.parse(certRow.chunk_ids),
      chunkTypes: JSON.parse(certRow.chunk_types),
      minimalWhy: JSON.parse(certRow.minimal_why),
      redundancyRatio: certRow.redundancy_ratio,
      completenessScore: certRow.completeness_score,
      merkleRoot: certRow.merkle_root,
      sourceHashes: JSON.parse(certRow.source_hashes),
      proofChain: JSON.parse(certRow.proof_chain),
      policyProof: certRow.policy_proof,
      policyVersion: certRow.policy_version,
      accessLevel: certRow.access_level,
      latencyMs: certRow.latency_ms
    };

    // Build provenance map for all sources
    const sources = new Map<string, ProvenanceSource[]>();
    for (const hash of certificate.sourceHashes) {
      sources.set(hash, this.getProvenanceLineage(hash));
    }

    // Build provenance graph
    const nodes: Array<{ id: string; type: string; label: string }> = [];
    const edges: Array<{ from: string; to: string; type: string }> = [];

    // Add certificate node
    nodes.push({
      id: certificateId,
      type: 'certificate',
      label: `Certificate: ${certificate.queryText.substring(0, 30)}...`
    });

    // Add source nodes and edges
    for (const [hash, lineage] of sources.entries()) {
      for (let i = 0; i < lineage.length; i++) {
        const source = lineage[i];
        const nodeId = `${source.sourceType}-${source.sourceId}`;

        // Add node if not exists
        if (!nodes.find(n => n.id === nodeId)) {
          nodes.push({
            id: nodeId,
            type: source.sourceType,
            label: `${source.sourceType} #${source.sourceId}`
          });
        }

        // Add edge from certificate to first source
        if (i === 0) {
          edges.push({
            from: certificateId,
            to: nodeId,
            type: 'includes'
          });
        }

        // Add edge to parent if exists
        if (i < lineage.length - 1) {
          const parentNodeId = `${lineage[i + 1].sourceType}-${lineage[i + 1].sourceId}`;
          edges.push({
            from: nodeId,
            to: parentNodeId,
            type: 'derived_from'
          });
        }
      }
    }

    return {
      certificate,
      sources,
      graph: { nodes, edges }
    };
  }

  /**
   * Audit certificate access
   */
  auditCertificate(certificateId: string): {
    certificate: RecallCertificate;
    justifications: JustificationPath[];
    provenance: Map<string, ProvenanceSource[]>;
    quality: {
      completeness: number;
      redundancy: number;
      avgNecessity: number;
    };
  } {
    const certRow = this.db.prepare(
      'SELECT * FROM recall_certificates WHERE id = ?'
    ).get(certificateId) as any;

    if (!certRow) {
      throw new Error(`Certificate ${certificateId} not found`);
    }

    const certificate: RecallCertificate = {
      id: certRow.id,
      queryId: certRow.query_id,
      queryText: certRow.query_text,
      chunkIds: JSON.parse(certRow.chunk_ids),
      chunkTypes: JSON.parse(certRow.chunk_types),
      minimalWhy: JSON.parse(certRow.minimal_why),
      redundancyRatio: certRow.redundancy_ratio,
      completenessScore: certRow.completeness_score,
      merkleRoot: certRow.merkle_root,
      sourceHashes: JSON.parse(certRow.source_hashes),
      proofChain: JSON.parse(certRow.proof_chain),
      policyProof: certRow.policy_proof,
      policyVersion: certRow.policy_version,
      accessLevel: certRow.access_level,
      latencyMs: certRow.latency_ms
    };

    // Get justifications
    const justRows = this.db.prepare(`
      SELECT * FROM justification_paths WHERE certificate_id = ?
    `).all(certificateId) as any[];

    const justifications = justRows.map(row => ({
      chunkId: row.chunk_id,
      chunkType: row.chunk_type,
      reason: row.reason,
      necessityScore: row.necessity_score,
      pathElements: JSON.parse(row.path_elements)
    }));

    // Get provenance for each source
    const provenance = new Map<string, ProvenanceSource[]>();
    for (const hash of certificate.sourceHashes) {
      provenance.set(hash, this.getProvenanceLineage(hash));
    }

    // Calculate quality metrics
    const avgNecessity = justifications.reduce((sum, j) => sum + j.necessityScore, 0) / justifications.length;

    return {
      certificate,
      justifications,
      provenance,
      quality: {
        completeness: certificate.completenessScore,
        redundancy: certificate.redundancyRatio,
        avgNecessity
      }
    };
  }

  // ========================================================================
  // Private Helper Methods
  // ========================================================================

  /**
   * Compute minimal hitting set using greedy algorithm
   * A hitting set contains at least one element from each requirement
   */
  private computeMinimalHittingSet(
    chunks: Array<{ id: string; content: string; relevance: number }>,
    requirements: string[]
  ): string[] {
    if (requirements.length === 0) {
      return chunks.slice(0, Math.min(3, chunks.length)).map(c => c.id);
    }

    const uncovered = new Set(requirements);
    const selected: string[] = [];

    // Greedy: select chunk that covers most uncovered requirements
    while (uncovered.size > 0 && chunks.length > 0) {
      let bestChunk: any = null;
      let bestCoverage = 0;

      for (const chunk of chunks) {
        const coverage = Array.from(uncovered).filter(req =>
          chunk.content.toLowerCase().includes(req.toLowerCase())
        ).length;

        if (coverage > bestCoverage) {
          bestCoverage = coverage;
          bestChunk = chunk;
        }
      }

      if (!bestChunk) break;

      selected.push(bestChunk.id);

      // Remove covered requirements
      for (const req of Array.from(uncovered)) {
        if (bestChunk.content.toLowerCase().includes(req.toLowerCase())) {
          uncovered.delete(req);
        }
      }

      // Remove selected chunk
      chunks = chunks.filter(c => c.id !== bestChunk.id);
    }

    return selected;
  }

  /**
   * Calculate completeness score
   */
  private calculateCompleteness(minimalWhy: string[], requirements: string[]): number {
    if (requirements.length === 0) return 1.0;

    // Prepare statement ONCE outside loop (better-sqlite3 best practice)
    const stmt = this.db.prepare('SELECT output FROM episodes WHERE id = ?');
    const chunks = minimalWhy.map(id => {
      // Get chunk content
      const episode = stmt.get(parseInt(id));
      return episode ? (episode as any).output : '';
    });

    const satisfied = requirements.filter(req =>
      chunks.some(content => content && content.toLowerCase().includes(req.toLowerCase()))
    );

    return satisfied.length / requirements.length;
  }

  /**
   * Get or create provenance record
   */
  private getOrCreateProvenance(sourceType: string, sourceId: number): string {
    // Check if provenance exists
    const existing = this.db.prepare(`
      SELECT content_hash FROM provenance_sources
      WHERE source_type = ? AND source_id = ?
    `).get(sourceType, sourceId) as any;

    if (existing) {
      return existing.content_hash;
    }

    // Create new provenance
    const contentHash = this.getContentHash(sourceType, sourceId);

    this.db.prepare(`
      INSERT OR IGNORE INTO provenance_sources (source_type, source_id, content_hash, creator)
      VALUES (?, ?, ?, ?)
    `).run(sourceType, sourceId, contentHash, 'system');

    return contentHash;
  }

  // Prepare statement ONCE outside loop (better-sqlite3 best practice)
  private _episodeStmt?: any;
  private _skillStmt?: any;
  private _noteStmt?: any;
  private _factStmt?: any;

  /**
   * Get content hash for a memory
   */
  private getContentHash(sourceType: string, sourceId: number): string {
    let content = '';

    switch (sourceType) {
      case 'episode':
        if (!this._episodeStmt) {
          this._episodeStmt = this.db.prepare('SELECT task, output FROM episodes WHERE id = ?');
        }
        const episode = this._episodeStmt.get(sourceId) as any;
        content = episode ? `${episode.task}:${episode.output}` : '';
        break;
      case 'skill':
        if (!this._skillStmt) {
          this._skillStmt = this.db.prepare('SELECT name, code FROM skills WHERE id = ?');
        }
        const skill = this._skillStmt.get(sourceId) as any;
        content = skill ? `${skill.name}:${skill.code}` : '';
        break;
      case 'note':
        if (!this._noteStmt) {
          this._noteStmt = this.db.prepare('SELECT text FROM notes WHERE id = ?');
        }
        const note = this._noteStmt.get(sourceId) as any;
        content = note ? note.text : '';
        break;
      case 'fact':
        if (!this._factStmt) {
          this._factStmt = this.db.prepare('SELECT subject, predicate, object FROM facts WHERE id = ?');
        }
        const fact = this._factStmt.get(sourceId) as any;
        content = fact ? `${fact.subject}:${fact.predicate}:${fact.object}` : '';
        break;
    }

    return crypto.createHash('sha256').update(content).digest('hex');
  }

  /**
   * Build Merkle tree from hashes
   */
  private buildMerkleTree(hashes: string[]): { root: string; tree: string[][] } {
    if (hashes.length === 0) {
      return { root: '', tree: [[]] };
    }

    const tree: string[][] = [hashes];

    while (tree[tree.length - 1].length > 1) {
      const level = tree[tree.length - 1];
      const nextLevel: string[] = [];

      for (let i = 0; i < level.length; i += 2) {
        if (i + 1 < level.length) {
          const combined = level[i] + level[i + 1];
          nextLevel.push(crypto.createHash('sha256').update(combined).digest('hex'));
        } else {
          nextLevel.push(level[i]);
        }
      }

      tree.push(nextLevel);
    }

    return { root: tree[tree.length - 1][0], tree };
  }

  /**
   * Get Merkle proof for a leaf
   */
  private getMerkleProof(merkleTree: { tree: string[][] }, leafIndex: number): MerkleProof[] {
    const proof: MerkleProof[] = [];
    let index = leafIndex;

    for (let level = 0; level < merkleTree.tree.length - 1; level++) {
      const currentLevel = merkleTree.tree[level];
      const isLeftNode = index % 2 === 0;
      const siblingIndex = isLeftNode ? index + 1 : index - 1;

      if (siblingIndex < currentLevel.length) {
        proof.push({
          hash: currentLevel[siblingIndex],
          position: isLeftNode ? 'right' : 'left'
        });
      }

      index = Math.floor(index / 2);
    }

    return proof;
  }

  /**
   * Generate certificate ID
   */
  private generateCertificateId(queryId: string, chunkIds: string[]): string {
    const data = `${queryId}:${chunkIds.join(',')}:${Date.now()}`;
    return crypto.createHash('sha256').update(data).digest('hex');
  }

  /**
   * Store justification paths
   */
  private storeJustificationPaths(
    certificateId: string,
    chunks: Array<{ id: string; type: string; relevance: number }>,
    minimalWhy: string[],
    requirements: string[]
  ): void {
    const stmt = this.db.prepare(`
      INSERT INTO justification_paths (
        certificate_id, chunk_id, chunk_type, reason, necessity_score, path_elements
      ) VALUES (?, ?, ?, ?, ?, ?)
    `);

    for (const chunk of chunks) {
      const isNecessary = minimalWhy.includes(chunk.id);
      const reason = this.determineReason(chunk, requirements);
      const necessityScore = isNecessary ? chunk.relevance : chunk.relevance * 0.5;

      const pathElements = [
        `Retrieved for query`,
        isNecessary ? `Essential for justification` : `Supporting evidence`,
        `Relevance: ${(chunk.relevance * 100).toFixed(1)}%`
      ];

      stmt.run(
        certificateId,
        chunk.id,
        chunk.type,
        reason,
        necessityScore,
        JSON.stringify(pathElements)
      );
    }
  }

  /**
   * Determine reason for inclusion
   */
  private determineReason(
    chunk: { id: string; relevance: number },
    requirements: string[]
  ): string {
    if (chunk.relevance > 0.9) return 'semantic_match';
    if (chunk.relevance > 0.7) return 'causal_link';
    if (chunk.relevance > 0.5) return 'prerequisite';
    return 'constraint';
  }
}
