/**
 * MMR (Maximal Marginal Relevance) Diversity Ranking
 *
 * Implements MMR algorithm to select diverse results that balance
 * relevance to query with diversity from already-selected results.
 *
 * Formula: MMR = argmax [λ × Sim(Di, Q) - (1-λ) × max Sim(Di, Dj)]
 *                Di∈R\S              Dj∈S
 *
 * Where:
 * - Di = candidate document
 * - Q = query
 * - S = already selected documents
 * - λ = balance parameter (0 = max diversity, 1 = max relevance)
 */

export interface MMROptions {
  lambda?: number;  // Balance between relevance and diversity (default: 0.5)
  k?: number;       // Number of results to return (default: 10)
  metric?: 'cosine' | 'euclidean' | 'dot';  // Similarity metric
}

export interface MMRCandidate {
  id: number;
  embedding: number[];
  similarity: number;  // Similarity to query
  [key: string]: any;  // Additional data
}

export class MMRDiversityRanker {
  /**
   * Select diverse results using MMR algorithm
   *
   * @param candidates - All candidate results with embeddings
   * @param queryEmbedding - Query vector
   * @param options - MMR configuration
   * @returns Diverse subset of candidates
   */
  static selectDiverse(
    candidates: MMRCandidate[],
    queryEmbedding: number[],
    options: MMROptions = {}
  ): MMRCandidate[] {
    const lambda = options.lambda ?? 0.5;
    const k = options.k ?? 10;
    const metric = options.metric ?? 'cosine';

    if (candidates.length === 0) {
      return [];
    }

    if (candidates.length <= k) {
      return candidates;
    }

    // Calculate initial similarities to query
    const candidatesWithSim = candidates.map(c => ({
      ...c,
      similarity: c.similarity ?? this.calculateSimilarity(
        queryEmbedding,
        c.embedding,
        metric
      ),
    }));

    const selected: MMRCandidate[] = [];
    const remaining = [...candidatesWithSim];

    // Select first item (highest relevance)
    remaining.sort((a, b) => b.similarity - a.similarity);
    selected.push(remaining.shift()!);

    // Iteratively select items with highest MMR score
    while (selected.length < k && remaining.length > 0) {
      let maxMMR = -Infinity;
      let maxIdx = 0;

      for (let i = 0; i < remaining.length; i++) {
        const candidate = remaining[i];

        // Calculate max similarity to already-selected items
        let maxSimToSelected = -Infinity;
        for (const selectedItem of selected) {
          const sim = this.calculateSimilarity(
            candidate.embedding,
            selectedItem.embedding,
            metric
          );
          maxSimToSelected = Math.max(maxSimToSelected, sim);
        }

        // Calculate MMR score
        const mmrScore = lambda * candidate.similarity - (1 - lambda) * maxSimToSelected;

        if (mmrScore > maxMMR) {
          maxMMR = mmrScore;
          maxIdx = i;
        }
      }

      // Add item with highest MMR score
      selected.push(remaining.splice(maxIdx, 1)[0]);
    }

    return selected;
  }

  /**
   * Calculate similarity between two vectors
   */
  private static calculateSimilarity(
    vec1: number[],
    vec2: number[],
    metric: 'cosine' | 'euclidean' | 'dot'
  ): number {
    if (vec1.length !== vec2.length) {
      throw new Error(`Vector dimension mismatch: ${vec1.length} vs ${vec2.length}`);
    }

    switch (metric) {
      case 'cosine': {
        let dot = 0, mag1 = 0, mag2 = 0;
        for (let i = 0; i < vec1.length; i++) {
          dot += vec1[i] * vec2[i];
          mag1 += vec1[i] * vec1[i];
          mag2 += vec2[i] * vec2[i];
        }
        return dot / (Math.sqrt(mag1) * Math.sqrt(mag2));
      }

      case 'euclidean': {
        let sum = 0;
        for (let i = 0; i < vec1.length; i++) {
          const diff = vec1[i] - vec2[i];
          sum += diff * diff;
        }
        return 1 / (1 + Math.sqrt(sum));  // Normalized to 0-1
      }

      case 'dot': {
        let dot = 0;
        for (let i = 0; i < vec1.length; i++) {
          dot += vec1[i] * vec2[i];
        }
        return dot;
      }

      default:
        throw new Error(`Unknown metric: ${metric}`);
    }
  }

  /**
   * Calculate diversity score for a set of results
   *
   * @param results - Results to analyze
   * @param metric - Similarity metric
   * @returns Average pairwise distance (higher = more diverse)
   */
  static calculateDiversityScore(
    results: MMRCandidate[],
    metric: 'cosine' | 'euclidean' | 'dot' = 'cosine'
  ): number {
    if (results.length < 2) {
      return 1.0;  // Single result is maximally diverse
    }

    let totalDistance = 0;
    let comparisons = 0;

    for (let i = 0; i < results.length; i++) {
      for (let j = i + 1; j < results.length; j++) {
        const similarity = this.calculateSimilarity(
          results[i].embedding,
          results[j].embedding,
          metric
        );
        // Convert similarity to distance
        const distance = metric === 'cosine' ? 1 - similarity : similarity;
        totalDistance += distance;
        comparisons++;
      }
    }

    return comparisons > 0 ? totalDistance / comparisons : 0;
  }
}
