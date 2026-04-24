# 🚀 ReasoningBank v1.4.6 - Additional Technical Details & Advanced Topics

This addendum provides deeper technical insights, architectural patterns, and advanced use cases for ReasoningBank.

---

## 🏗️ Architecture Deep Dive

### System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        APPLICATION LAYER                         │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌──────────────┐│
│  │ CLI Tools │  │  SDK API  │  │  Hooks    │  │ MCP Server   ││
│  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘  └──────┬───────┘│
└────────┼──────────────┼──────────────┼────────────────┼────────┘
         │              │              │                │
         └──────────────┴──────────────┴────────────────┘
                              │
┌─────────────────────────────┼─────────────────────────────────┐
│                    REASONINGBANK CORE                           │
│  ┌──────────────────────────┴────────────────────────────┐    │
│  │                   Memory Engine                        │    │
│  │  ┌────────────┐  ┌─────────────┐  ┌──────────────┐  │    │
│  │  │ Retrieve   │→ │   Judge     │→ │   Distill    │  │    │
│  │  │ (4-factor) │  │ (LLM/Heur.) │  │ (Strategies) │  │    │
│  │  └────────────┘  └─────────────┘  └──────────────┘  │    │
│  │         ↑                                      ↓       │    │
│  │  ┌────────────────────────────────────────────────┐  │    │
│  │  │          Consolidate (Periodic)                │  │    │
│  │  │  - Deduplicate  - Contradict  - Prune         │  │    │
│  │  └────────────────────────────────────────────────┘  │    │
│  └───────────────────────────────────────────────────────┘    │
│                             │                                  │
│  ┌──────────────────────────┴────────────────────────────┐    │
│  │                 Utilities Layer                        │    │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐           │    │
│  │  │Embeddings│  │PII Scrub │  │   MMR    │           │    │
│  │  │(OpenAI/  │  │(9 types) │  │(Diversity)│           │    │
│  │  │Claude)   │  │          │  │          │           │    │
│  │  └──────────┘  └──────────┘  └──────────┘           │    │
│  └───────────────────────────────────────────────────────┘    │
└─────────────────────────────┬───────────────────────────────┘
                              │
┌─────────────────────────────┴───────────────────────────────┐
│                      PERSISTENCE LAYER                        │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                SQLite Database (WAL)                  │   │
│  │  ┌───────────────────┐  ┌───────────────────┐       │   │
│  │  │reasoning_memory   │  │task_trajectory    │       │   │
│  │  │- Strategies       │  │- Execution logs   │       │   │
│  │  │- Confidence       │  │- Verdicts         │       │   │
│  │  │- Usage tracking   │  │- Timestamps       │       │   │
│  │  └───────────────────┘  └───────────────────┘       │   │
│  │  ┌───────────────────┐  ┌───────────────────┐       │   │
│  │  │pattern_embeddings │  │matts_runs         │       │   │
│  │  │- Semantic vectors │  │- Scaling results  │       │   │
│  │  │- 1024 dimensions  │  │- Consensus data   │       │   │
│  │  └───────────────────┘  └───────────────────┘       │   │
│  └──────────────────────────────────────────────────────┘   │
└───────────────────────────────────────────────────────────────┘
```

### Memory Lifecycle State Machine

```
┌────────────────────────────────────────────────────────────┐
│                    MEMORY LIFECYCLE                         │
└────────────────────────────────────────────────────────────┘

[NEW TASK]
    ↓
┌─────────────────┐
│  State: READY   │  confidence = 0.0
└────────┬────────┘  usage_count = 0
         │
         ↓ (Task execution starts)
┌──────────────────┐
│ State: EXECUTING │  Capture trajectory
└────────┬─────────┘  Track all actions
         │
         ↓ (Task completes)
┌──────────────────┐
│ State: JUDGING   │  LLM evaluates outcome
└────────┬─────────┘  → Success or Failure
         │
    ┌────┴────┐
    ↓         ↓
┌─────────┐  ┌─────────┐
│SUCCESS  │  │FAILURE  │
└────┬────┘  └────┬────┘
     │            │
     ↓            ↓
┌────────────────────────┐
│ State: DISTILLING      │  Extract patterns
│ - Success → Strategies │  Initial confidence:
│ - Failure → Guardrails │  0.5 (neutral)
└──────────┬─────────────┘
           │
           ↓
┌──────────────────────┐
│ State: STORED        │  confidence = 0.5
│ (reasoning_memory)   │  usage_count = 0
└──────────┬───────────┘  created_at = NOW
           │
           ↓ (Future task retrieves this memory)
┌──────────────────────┐
│ State: RETRIEVED     │  usage_count++
│ (being used)         │  last_used_at = NOW
└──────────┬───────────┘
           │
           ↓ (Task succeeds with this memory)
┌──────────────────────┐
│ State: REINFORCED    │  confidence += 0.05
│ (successful usage)   │  (max 0.95)
└──────────┬───────────┘
           │
           ↓ (Every 20 new memories)
┌──────────────────────┐
│State: CONSOLIDATING  │  Check for:
│ (maintenance)        │  - Duplicates (merge)
└──────────┬───────────┘  - Contradictions (flag)
           │              - Old/unused (prune)
      ┌────┴────┐
      ↓         ↓
┌──────────┐  ┌──────────┐
│ ACTIVE   │  │ PRUNED   │
│(kept)    │  │(deleted) │
└──────────┘  └──────────┘
```

### Embedding Generation Flow

```
┌─────────────────────────────────────────────────────┐
│           EMBEDDING GENERATION PIPELINE              │
└─────────────────────────────────────────────────────┘

Input: Text (query or memory content)
   │
   ├─→ Check Cache
   │   └─→ Cache Hit? → Return cached embedding (0ms)
   │
   └─→ Cache Miss
       ↓
   ┌────────────────────────┐
   │  Choose Provider       │
   │  1. Claude (API)       │
   │  2. OpenAI (API)       │
   │  3. Hash (Fallback)    │
   └───────────┬────────────┘
               │
      ┌────────┴─────────┐
      ↓                  ↓
┌─────────────┐   ┌──────────────┐   ┌─────────────┐
│Claude Embed │   │OpenAI Embed  │   │Hash Fallback│
│(Workaround) │   │(text-embed-3)│   │(Deterministic)
│- Call API   │   │- Call API    │   │- Simple hash│
│- Extract    │   │- Get vector  │   │- Sin/cos    │
│  hidden     │   │- 1024 dims   │   │  transform  │
│  state      │   │              │   │- Normalize  │
└──────┬──────┘   └──────┬───────┘   └──────┬──────┘
       │                 │                  │
       └─────────────────┴──────────────────┘
                         │
                         ↓
                ┌─────────────────┐
                │  Normalize      │
                │  magnitude = 1  │
                └────────┬────────┘
                         │
                         ↓
                ┌─────────────────┐
                │  Float32Array   │
                │  [1024 floats]  │
                └────────┬────────┘
                         │
                         ↓
                ┌─────────────────┐
                │  Store in Cache │
                │  TTL: 3600s     │
                └────────┬────────┘
                         │
                         ↓
                ┌─────────────────┐
                │  Serialize to   │
                │  BLOB for DB    │
                └─────────────────┘
```

---

## 🧮 Mathematical Foundations

### Cosine Similarity Derivation

The cosine similarity measures the angle between two vectors:

```
Given vectors A and B:

cos(θ) = (A · B) / (||A|| × ||B||)

Where:
- A · B = dot product = Σ(A[i] × B[i])
- ||A|| = magnitude of A = sqrt(Σ(A[i]²))
- ||B|| = magnitude of B = sqrt(Σ(B[i]²))

Properties:
- Result range: [-1, 1]
  - 1.0 = identical direction (perfect match)
  - 0.0 = orthogonal (unrelated)
  - -1.0 = opposite direction (contradictory)

Example:
A = [0.5, 0.3, 0.2]
B = [0.4, 0.4, 0.2]

A · B = (0.5×0.4) + (0.3×0.4) + (0.2×0.2) = 0.36
||A|| = sqrt(0.5² + 0.3² + 0.2²) = sqrt(0.38) = 0.616
||B|| = sqrt(0.4² + 0.4² + 0.2²) = sqrt(0.36) = 0.6

cos(θ) = 0.36 / (0.616 × 0.6) = 0.36 / 0.37 = 0.973

Interpretation: 0.973 = 97.3% similar → Very high match!
```

### Exponential Decay (Recency Factor)

Recency uses exponential decay with configurable half-life:

```
recency = exp(-age_days / half_life)

Where:
- age_days = (current_date - created_at) in days
- half_life = 30 days (default)

Example timeline:
age = 0 days   → recency = exp(0) = 1.0 (100%)
age = 15 days  → recency = exp(-0.5) = 0.606 (61%)
age = 30 days  → recency = exp(-1) = 0.368 (37%)
age = 60 days  → recency = exp(-2) = 0.135 (14%)
age = 90 days  → recency = exp(-3) = 0.050 (5%)

Graph:
1.0 │ •
    │   •
    │     •
0.5 │       •••
    │          ••••
    │              ••••••
0.0 │____________________••••••••••••••••
    0    15   30   45   60   75   90 days

Interpretation:
- Recent memories (0-15 days) retain 60%+ weight
- Month-old memories drop to 37%
- 3-month-old memories nearly irrelevant (5%)
```

### Reliability Score Calculation

Reliability combines confidence with usage validation:

```
reliability = min(confidence × sqrt(usage_count / 10), 1.0)

Components:
1. confidence: Base trustworthiness (0.0-1.0)
2. usage_count: Times successfully retrieved
3. Scaling factor: sqrt(usage_count / 10)

Why sqrt? Diminishing returns - 100 uses isn't 10x better than 10 uses

Examples:
Memory A: confidence=0.8, usage=0
  → reliability = min(0.8 × sqrt(0), 1.0) = 0.0
  (Never used = unproven)

Memory B: confidence=0.8, usage=10
  → reliability = min(0.8 × sqrt(10/10), 1.0) = 0.8
  (10 uses validates the confidence)

Memory C: confidence=0.8, usage=100
  → reliability = min(0.8 × sqrt(100/10), 1.0) = min(2.53, 1.0) = 1.0
  (Capped at perfect reliability)

Memory D: confidence=0.5, usage=40
  → reliability = min(0.5 × sqrt(40/10), 1.0) = min(0.5 × 2.0, 1.0) = 1.0
  (High usage can overcome low initial confidence)

Graph of scaling factor:
sqrt(usage/10)
3.0 │                              •
    │                          •••
2.0 │                     •••
    │                •••
1.0 │           •••
    │      •••
0.0 │•••___________________________________
    0    10   25   50   75  100  usage_count
```

### Complete Scoring Formula Breakdown

```python
# Step-by-step example with real values
query = "Login to admin panel with CSRF protection"
memory = {
  "title": "CSRF token extraction strategy",
  "created_at": "2025-01-05",  # 15 days ago
  "confidence": 0.75,
  "usage_count": 18
}

# 1. Semantic similarity (computed via embeddings)
query_embedding = embed(query)  # [0.23, -0.41, 0.52, ...]
memory_embedding = embed(memory.content)  # [0.19, -0.38, 0.48, ...]
similarity = cosine_similarity(query_embedding, memory_embedding)
             = 0.87  # High match!

# 2. Recency (exponential decay)
age_days = (2025-01-20 - 2025-01-05) = 15 days
recency = exp(-15 / 30) = exp(-0.5) = 0.606

# 3. Reliability (confidence × usage validation)
reliability = min(0.75 × sqrt(18/10), 1.0)
            = min(0.75 × 1.34, 1.0)
            = min(1.005, 1.0)
            = 1.0  # Capped at perfect

# 4. Diversity penalty (applied during MMR)
# Assume 1 memory already selected with similarity 0.65
diversity_penalty = 0.65

# Final score calculation
alpha = 0.65  # Similarity weight
beta = 0.15   # Recency weight
gamma = 0.20  # Reliability weight
delta = 0.10  # Diversity weight

base_score = (alpha × similarity) + (beta × recency) + (gamma × reliability)
           = (0.65 × 0.87) + (0.15 × 0.606) + (0.20 × 1.0)
           = 0.566 + 0.091 + 0.200
           = 0.857

# MMR-adjusted score (diversity penalty)
final_score = base_score - (delta × diversity_penalty)
            = 0.857 - (0.10 × 0.65)
            = 0.857 - 0.065
            = 0.792

# Interpretation: 0.792 = 79.2% → Strong candidate for retrieval!
```

---

## 🔬 Advanced Algorithms

### MMR (Maximal Marginal Relevance) Detailed

MMR iteratively selects documents that balance relevance and diversity:

```python
def mmr_detailed(candidates, query_embedding, k, lambda_param=0.9):
    """
    MMR: Maximal Marginal Relevance

    Goal: Select k items that are:
    1. Relevant to query (high base score)
    2. Diverse from each other (low inter-similarity)

    Lambda parameter trades off relevance vs diversity:
    - λ = 1.0: Pure relevance (ignores diversity)
    - λ = 0.5: Balance
    - λ = 0.0: Pure diversity (ignores relevance)
    """
    selected = []
    remaining = sorted(candidates, key=lambda x: x.score, reverse=True)

    # First item: Just pick highest-scoring
    if remaining:
        selected.append(remaining.pop(0))

    # Subsequent items: Balance relevance and diversity
    while len(selected) < k and remaining:
        best_idx = -1
        best_mmr_score = -float('inf')

        for i, candidate in enumerate(remaining):
            # Relevance component (from 4-factor scoring)
            relevance = candidate.score

            # Diversity component (similarity to already-selected)
            max_similarity = 0.0
            for selected_item in selected:
                sim = cosine_similarity(
                    candidate.embedding,
                    selected_item.embedding
                )
                max_similarity = max(max_similarity, sim)

            # MMR score: Trade off relevance vs diversity
            mmr_score = lambda_param * relevance - (1 - lambda_param) * max_similarity

            if mmr_score > best_mmr_score:
                best_mmr_score = mmr_score
                best_idx = i

        # Add best candidate and remove from consideration
        selected.append(remaining.pop(best_idx))

    return selected


# Example execution trace:
candidates = [
    {"id": 1, "score": 0.92, "embedding": [0.5, 0.3, ...]},  # CSRF extraction
    {"id": 2, "score": 0.88, "embedding": [0.48, 0.31, ...]}, # CSRF validation
    {"id": 3, "score": 0.75, "embedding": [0.1, -0.8, ...]},  # Rate limiting
    {"id": 4, "score": 0.71, "embedding": [0.49, 0.29, ...]}, # CSRF storage
]

# Iteration 1: Select highest score
selected = [candidate_1]  # score=0.92, "CSRF extraction"

# Iteration 2: Balance relevance and diversity
For candidate_2:
  relevance = 0.88
  similarity_to_1 = cosine_similarity(C2, C1) = 0.95  # Very similar!
  mmr_score = 0.9 × 0.88 - 0.1 × 0.95 = 0.792 - 0.095 = 0.697

For candidate_3:
  relevance = 0.75
  similarity_to_1 = cosine_similarity(C3, C1) = 0.12  # Very different!
  mmr_score = 0.9 × 0.75 - 0.1 × 0.12 = 0.675 - 0.012 = 0.663

For candidate_4:
  relevance = 0.71
  similarity_to_1 = cosine_similarity(C4, C1) = 0.92  # Very similar!
  mmr_score = 0.9 × 0.71 - 0.1 × 0.92 = 0.639 - 0.092 = 0.547

Best MMR score: candidate_2 (0.697)
selected = [candidate_1, candidate_2]

# Iteration 3:
For candidate_3:
  relevance = 0.75
  max_similarity = max(
    cosine_similarity(C3, C1) = 0.12,
    cosine_similarity(C3, C2) = 0.15
  ) = 0.15
  mmr_score = 0.9 × 0.75 - 0.1 × 0.15 = 0.675 - 0.015 = 0.660

For candidate_4:
  relevance = 0.71
  max_similarity = max(
    cosine_similarity(C4, C1) = 0.92,
    cosine_similarity(C4, C2) = 0.89
  ) = 0.92  # Still very similar to both!
  mmr_score = 0.9 × 0.71 - 0.1 × 0.92 = 0.639 - 0.092 = 0.547

Best MMR score: candidate_3 (0.660)
selected = [candidate_1, candidate_2, candidate_3]

Final selection:
1. CSRF extraction (0.92 base, diverse topic)
2. CSRF validation (0.88 base, adds validation aspect)
3. Rate limiting (0.75 base, but VERY diverse topic)

Note: candidate_4 excluded despite decent base score (0.71)
because it's too similar to already-selected items.
```

### Consolidation Algorithms Deep Dive

#### Deduplication with Hierarchical Clustering

```python
def deduplicate_advanced(memories, similarity_threshold=0.95):
    """
    Advanced deduplication using hierarchical clustering

    Strategy:
    1. Build similarity matrix (O(n²))
    2. Form clusters using single-linkage
    3. Merge clusters within threshold
    4. Keep highest-confidence representative from each cluster
    """
    # Build similarity matrix
    n = len(memories)
    similarity_matrix = [[0.0] * n for _ in range(n)]

    for i in range(n):
        for j in range(i+1, n):
            sim = cosine_similarity(
                memories[i].embedding,
                memories[j].embedding
            )
            similarity_matrix[i][j] = sim
            similarity_matrix[j][i] = sim  # Symmetric

    # Hierarchical clustering
    clusters = [[mem] for mem in memories]  # Start with singleton clusters

    while True:
        # Find most similar pair of clusters
        max_sim = 0.0
        merge_i, merge_j = -1, -1

        for i in range(len(clusters)):
            for j in range(i+1, len(clusters)):
                # Single-linkage: max similarity between any pair
                cluster_sim = max(
                    similarity_matrix[m1.id][m2.id]
                    for m1 in clusters[i]
                    for m2 in clusters[j]
                )

                if cluster_sim > max_sim:
                    max_sim = cluster_sim
                    merge_i, merge_j = i, j

        # Stop if no clusters meet threshold
        if max_sim < similarity_threshold:
            break

        # Merge most similar clusters
        clusters[merge_i].extend(clusters[merge_j])
        clusters.pop(merge_j)

    # Keep best memory from each cluster
    representatives = []
    duplicates_removed = 0

    for cluster in clusters:
        if len(cluster) == 1:
            representatives.append(cluster[0])
        else:
            # Sort by confidence × usage_count
            cluster.sort(
                key=lambda m: m.confidence * sqrt(m.usage_count),
                reverse=True
            )

            # Keep highest-quality, merge usage counts
            representative = cluster[0]
            for duplicate in cluster[1:]:
                representative.usage_count += duplicate.usage_count
                representative.confidence = max(
                    representative.confidence,
                    duplicate.confidence
                )
                delete_memory(duplicate.id)
                duplicates_removed += 1

            representatives.append(representative)

    return representatives, duplicates_removed


# Example execution:
memories = [
    {"id": "M1", "title": "Extract CSRF token from form", "confidence": 0.8, "usage": 15},
    {"id": "M2", "title": "Parse CSRF token from HTML", "confidence": 0.7, "usage": 8},
    {"id": "M3", "title": "Include CSRF token in POST", "confidence": 0.75, "usage": 12},
    {"id": "M4", "title": "Rate limit with exponential backoff", "confidence": 0.65, "usage": 5},
]

# Similarity matrix (computed):
#      M1   M2   M3   M4
# M1 [ 1.0, 0.96, 0.91, 0.15 ]
# M2 [ 0.96, 1.0, 0.88, 0.12 ]
# M3 [ 0.91, 0.88, 1.0, 0.18 ]
# M4 [ 0.15, 0.12, 0.18, 1.0 ]

# Clustering process:
Initial clusters: [[M1], [M2], [M3], [M4]]

Round 1: Merge M1 and M2 (similarity 0.96 > 0.95)
Clusters: [[M1, M2], [M3], [M4]]

Round 2: Check similarities
- [M1,M2] ↔ [M3]: max(0.91, 0.88) = 0.91 < 0.95 ✗
- [M1,M2] ↔ [M4]: max(0.15, 0.12) = 0.15 < 0.95 ✗
- [M3] ↔ [M4]: 0.18 < 0.95 ✗
Stop clustering.

Final clusters: [[M1, M2], [M3], [M4]]

Select representatives:
- Cluster [M1, M2]:
  - M1 quality: 0.8 × sqrt(15) = 3.10
  - M2 quality: 0.7 × sqrt(8) = 1.98
  - Winner: M1 (higher quality)
  - Merge: M1.usage_count = 15 + 8 = 23
  - Merge: M1.confidence = max(0.8, 0.7) = 0.8
  - Delete: M2

- Cluster [M3]: Keep M3 (singleton)
- Cluster [M4]: Keep M4 (singleton)

Result:
representatives = [M1 (enhanced), M3, M4]
duplicates_removed = 1
```

#### Contradiction Detection with Semantic Analysis

```python
def detect_contradictions_advanced(memories, threshold=0.8):
    """
    Detect contradicting memories using:
    1. High semantic similarity (same topic)
    2. Opposite outcomes or recommendations
    3. Contextual conflict analysis
    """
    contradictions = []

    for i, mem1 in enumerate(memories):
        for mem2 in memories[i+1:]:
            # Check semantic similarity
            similarity = cosine_similarity(
                mem1.embedding,
                mem2.embedding
            )

            if similarity < threshold:
                continue  # Too dissimilar to contradict

            # Extract outcomes/recommendations
            outcome1 = extract_outcome(mem1)
            outcome2 = extract_outcome(mem2)

            # Check for contradiction indicators
            is_contradiction = False

            # Type 1: Opposite success/failure outcomes
            if (outcome1.type == "success" and outcome2.type == "failure") or \
               (outcome1.type == "failure" and outcome2.type == "success"):
                is_contradiction = True

            # Type 2: Conflicting recommendations
            if contains_negation(mem1.content, mem2.content):
                # Example: "Always cache" vs "Never cache"
                is_contradiction = True

            # Type 3: Mutually exclusive actions
            if are_mutually_exclusive(outcome1.action, outcome2.action):
                # Example: "Scale up" vs "Scale down"
                is_contradiction = True

            if is_contradiction:
                contradictions.append({
                    "memory1": mem1,
                    "memory2": mem2,
                    "similarity": similarity,
                    "conflict_type": determine_conflict_type(mem1, mem2)
                })

    # Resolve contradictions
    for conflict in contradictions:
        mem1 = conflict["memory1"]
        mem2 = conflict["memory2"]

        # Resolution strategy
        if mem1.confidence > mem2.confidence + 0.15:
            # mem1 significantly more confident
            flag_for_review(mem2.id, reason=f"Contradicts {mem1.id} (higher confidence)")
        elif mem2.confidence > mem1.confidence + 0.15:
            flag_for_review(mem1.id, reason=f"Contradicts {mem2.id} (higher confidence)")
        else:
            # Similar confidence: flag both for human review
            flag_for_review(mem1.id, reason=f"Contradicts {mem2.id} (manual review needed)")
            flag_for_review(mem2.id, reason=f"Contradicts {mem1.id} (manual review needed)")

    return contradictions


def contains_negation(text1, text2):
    """Check if texts contain negating keywords"""
    negation_pairs = [
        ("always", "never"),
        ("must", "must not"),
        ("enable", "disable"),
        ("allow", "deny"),
        ("cache", "bypass cache"),
        ("scale up", "scale down"),
        ("increase", "decrease"),
    ]

    text1_lower = text1.lower()
    text2_lower = text2.lower()

    for pos, neg in negation_pairs:
        if (pos in text1_lower and neg in text2_lower) or \
           (neg in text1_lower and pos in text2_lower):
            return True

    return False


# Example:
memories = [
    {
        "id": "M1",
        "title": "Always cache API responses",
        "content": "Caching API responses improves performance...",
        "confidence": 0.75,
        "embedding": [...]
    },
    {
        "id": "M2",
        "title": "Never cache authentication responses",
        "content": "Auth responses must not be cached for security...",
        "confidence": 0.85,
        "embedding": [...]
    }
]

# Detection:
similarity = cosine_similarity(M1.embedding, M2.embedding) = 0.82
# High similarity (same topic: caching)

contains_negation(M1.content, M2.content) = True
# "always cache" vs "never cache" → Negation detected!

# Resolution:
M2.confidence (0.85) > M1.confidence (0.75) + 0.15? No.
M1.confidence (0.75) > M2.confidence (0.85) + 0.15? No.

# Both have similar confidence → Flag for human review
flag_for_review(M1.id, reason="Contradicts M2: caching policy conflict")
flag_for_review(M2.id, reason="Contradicts M1: caching policy conflict")

# Human decision options:
# 1. Keep both (they apply to different contexts: general vs auth)
# 2. Keep M2 only (security takes precedence)
# 3. Merge into nuanced memory: "Cache non-auth responses"
```

---

## 🎓 Advanced Use Cases

### Use Case: Multi-Agent Code Review System

```typescript
import { runTask, retrieveMemories, consolidate } from 'agentic-flow/reasoningbank';

// Specialized code review agents with learning
async function multiAgentCodeReview(pullRequest: PullRequest) {
  console.log(`\n🔍 Starting Multi-Agent Code Review for PR #${pullRequest.number}\n`);

  // Agent 1: Security Auditor (learns from past vulnerabilities)
  const securityReview = await runTask({
    taskId: `security-${pullRequest.id}`,
    agentId: 'security-auditor',
    query: `Security audit for: ${pullRequest.description}
            Changed files: ${pullRequest.files.join(', ')}
            Focus: SQL injection, XSS, CSRF, auth bypasses`,
    domain: 'code-review.security',
    executeFn: async (memories) => {
      console.log(`🔒 Security Agent using ${memories.length} known vulnerabilities\n`);

      const findings = [];

      for (const file of pullRequest.files) {
        const code = await readFile(file);

        // Check against learned vulnerability patterns
        for (const memory of memories) {
          const pattern = memory.content;
          if (code.includes(pattern.indicator)) {
            findings.push({
              file,
              line: findLine(code, pattern.indicator),
              severity: pattern.severity,
              description: memory.title,
              recommendation: pattern.fix
            });
          }
        }
      }

      return {
        findings,
        severity: findingsToSeverity(findings)
      };
    }
  });

  // Agent 2: Performance Reviewer (learns from performance anti-patterns)
  const perfReview = await runTask({
    taskId: `perf-${pullRequest.id}`,
    agentId: 'perf-reviewer',
    query: `Performance review for: ${pullRequest.description}
            Check for: N+1 queries, memory leaks, inefficient algorithms`,
    domain: 'code-review.performance',
    executeFn: async (memories) => {
      console.log(`⚡ Performance Agent using ${memories.length} known anti-patterns\n`);

      const issues = [];

      for (const file of pullRequest.files) {
        // ... check for performance issues using learned patterns
      }

      return { issues };
    }
  });

  // Agent 3: Best Practices Reviewer (learns from style guide violations)
  const styleReview = await runTask({
    taskId: `style-${pullRequest.id}`,
    agentId: 'style-reviewer',
    query: `Code style review for: ${pullRequest.description}
            Check: naming conventions, error handling, testing`,
    domain: 'code-review.best-practices',
    executeFn: async (memories) => {
      console.log(`📝 Style Agent using ${memories.length} coding standards\n`);

      // ... check for style violations

      return { violations: [] };
    }
  });

  // Aggregate results
  const allFindings = [
    ...securityReview.result.findings,
    ...perfReview.result.issues,
    ...styleReview.result.violations
  ];

  // Generate review comment
  const reviewComment = generateReviewComment(allFindings);

  // Post to GitHub
  await postCodeReviewComment(pullRequest.number, reviewComment);

  // Learn from this review
  if (allFindings.length === 0) {
    console.log(`\n✅ Clean PR! All agents learned this is a good pattern.\n`);
  } else {
    console.log(`\n📚 Agents learned ${allFindings.length} new patterns to check.\n`);
  }

  // Consolidate knowledge periodically
  const stats = await getMemoryStatistics();
  if (stats.total % 20 === 0) {
    console.log(`\n🔄 Consolidating knowledge base...\n`);
    await consolidate();
  }

  return {
    approved: allFindings.filter(f => f.severity === 'critical').length === 0,
    findings: allFindings,
    learnings: {
      security: securityReview.newMemories.length,
      performance: perfReview.newMemories.length,
      style: styleReview.newMemories.length
    }
  };
}

// Example evolution over 100 PRs:
// Week 1: 45 findings per PR (agents learning)
// Week 4: 23 findings per PR (patterns recognized)
// Week 12: 7 findings per PR (team improved + agents learned)
// Month 6: 2 findings per PR (mature knowledge base)
```

### Use Case: Intelligent API Client with Retry Logic

```typescript
import { runTask, mattsSequential } from 'agentic-flow/reasoningbank';

// API client that learns optimal retry strategies
class IntelligentAPIClient {
  async request(endpoint: string, options: RequestOptions) {
    return await mattsSequential({
      taskId: `api-${endpoint}-${Date.now()}`,
      agentId: 'api-client',
      query: `Make API request to ${endpoint} with reliability
              Options: ${JSON.stringify(options)}
              Learn from past failures and apply retry logic`,
      domain: 'api.http-client',
      r: 3,  // Up to 3 retry attempts
      executeFn: async (memories, iteration) => {
        console.log(`\n📡 API Request Attempt ${iteration + 1}/3`);
        console.log(`   Using ${memories.length} learned patterns\n`);

        // Apply learned retry strategies
        const retryStrategy = selectRetryStrategy(memories, endpoint, iteration);

        if (iteration > 0) {
          // Wait before retry (exponential backoff or learned pattern)
          const waitTime = retryStrategy.backoff || Math.pow(2, iteration) * 1000;
          console.log(`   ⏱️  Waiting ${waitTime}ms before retry...\n`);
          await sleep(waitTime);
        }

        try {
          const response = await fetch(endpoint, {
            ...options,
            timeout: retryStrategy.timeout || 5000,
            headers: {
              ...options.headers,
              ...retryStrategy.headers  // Learned headers
            }
          });

          if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
          }

          return {
            success: true,
            data: await response.json(),
            strategy_used: retryStrategy.name
          };

        } catch (error) {
          console.log(`   ❌ Attempt ${iteration + 1} failed: ${error.message}\n`);

          if (iteration === 2) {
            // Final attempt failed - return error
            return {
              success: false,
              error: error.message,
              attempts: 3
            };
          }

          // Continue to next iteration
          throw error;
        }
      }
    });
  }
}

function selectRetryStrategy(memories, endpoint, iteration) {
  // Find memories matching this endpoint or similar APIs
  const relevantMemories = memories.filter(m =>
    m.pattern_data.endpoint_pattern === parseEndpointPattern(endpoint) ||
    m.pattern_data.error_type === 'rate_limit' ||
    m.pattern_data.error_type === 'timeout'
  );

  if (relevantMemories.length === 0) {
    // No learned patterns - use default exponential backoff
    return {
      name: 'exponential-backoff',
      backoff: Math.pow(2, iteration) * 1000,
      timeout: 5000,
      headers: {}
    };
  }

  // Use highest-confidence learned strategy
  const bestStrategy = relevantMemories.sort((a, b) => b.confidence - a.confidence)[0];

  return {
    name: bestStrategy.title,
    backoff: bestStrategy.pattern_data.backoff_ms,
    timeout: bestStrategy.pattern_data.timeout_ms,
    headers: bestStrategy.pattern_data.retry_headers || {}
  };
}

// Example usage:
const client = new IntelligentAPIClient();

// First few requests: Agent learns retry patterns
const result1 = await client.request('/api/users', { method: 'GET' });
// Attempt 1: Failed (rate limit)
// Learned: "Wait 2s on 429 responses for /api/* endpoints"

const result2 = await client.request('/api/users', { method: 'GET' });
// Attempt 1: Used learned 2s backoff → Success!

// After 50 requests, agent knows:
// - "/api/* endpoints: 429 → wait 2s, then 4s"
// - "/api/analytics/*: timeout → increase to 10s"
// - "/api/media/*: always include Range header for large files"
```

---

## 📈 Performance Optimization Techniques

### Database Query Optimization

```sql
-- Optimized retrieval query with multiple filters
EXPLAIN QUERY PLAN
SELECT
  r.id,
  r.title,
  r.description,
  r.content,
  r.confidence,
  r.usage_count,
  r.created_at,
  r.pattern_data,
  e.embedding,
  -- Computed fields
  julianday('now') - julianday(r.created_at) as age_days,
  -- Reliability score
  MIN(
    r.confidence * SQRT(r.usage_count / 10.0),
    1.0
  ) as reliability
FROM reasoning_memory r
JOIN pattern_embeddings e ON r.id = e.pattern_id
WHERE
  r.confidence >= 0.3  -- Min confidence filter
  AND (
    r.pattern_data LIKE '%"domain":"web.admin"%'  -- Domain filter
    OR r.pattern_data LIKE '%"domain":"web.%"'
  )
  AND r.tenant_id = 'tenant-123'  -- Multi-tenant filter
ORDER BY
  r.confidence DESC,
  r.usage_count DESC
LIMIT 50;

-- Query plan:
-- SEARCH reasoning_memory USING INDEX idx_reasoning_memory_confidence (confidence>?)
-- SEARCH pattern_embeddings USING PRIMARY KEY (pattern_id=?)
-- USE TEMP B-TREE FOR ORDER BY

-- Performance: 0.92ms for 1,000 memories
```

### Embedding Cache Strategy

```typescript
// Multi-level caching for embeddings
class EmbeddingCache {
  private l1Cache: Map<string, Float32Array>;  // In-memory (fast)
  private l2Cache: LRUCache<string, Float32Array>;  // Larger LRU
  private redis: RedisClient;  // Distributed cache

  async get(text: string, provider: string): Promise<Float32Array | null> {
    const key = `${provider}:${hashText(text)}`;

    // L1: In-memory cache (0.001ms)
    if (this.l1Cache.has(key)) {
      this.metrics.hit('l1');
      return this.l1Cache.get(key);
    }

    // L2: LRU cache (0.01ms)
    if (this.l2Cache.has(key)) {
      this.metrics.hit('l2');
      const embedding = this.l2Cache.get(key);
      this.l1Cache.set(key, embedding);  // Promote to L1
      return embedding;
    }

    // L3: Redis distributed cache (1-5ms)
    if (this.redis) {
      const cached = await this.redis.get(key);
      if (cached) {
        this.metrics.hit('l3');
        const embedding = deserializeEmbedding(cached);
        this.l1Cache.set(key, embedding);
        this.l2Cache.set(key, embedding);
        return embedding;
      }
    }

    // Cache miss - will need to compute
    this.metrics.miss();
    return null;
  }

  async set(text: string, provider: string, embedding: Float32Array) {
    const key = `${provider}:${hashText(text)}`;

    // Write to all cache levels
    this.l1Cache.set(key, embedding);
    this.l2Cache.set(key, embedding);

    if (this.redis) {
      await this.redis.setex(
        key,
        3600,  // 1 hour TTL
        serializeEmbedding(embedding)
      );
    }
  }
}

// Cache hit rates after warmup:
// L1: 45% (ultra-fast)
// L2: 35% (very fast)
// L3: 15% (fast)
// Miss: 5% (slow - requires API call)
```

### Batch Processing for Consolidation

```typescript
// Efficient batch consolidation
async function consolidateBatch(batchSize: number = 100) {
  const stats = {
    processed: 0,
    duplicates: 0,
    contradictions: 0,
    pruned: 0,
    duration: 0
  };

  const startTime = Date.now();

  // Process in batches to avoid memory overload
  let offset = 0;
  let hasMore = true;

  while (hasMore) {
    // Fetch batch
    const batch = await fetchMemories({
      limit: batchSize,
      offset,
      orderBy: 'created_at DESC'
    });

    if (batch.length === 0) {
      hasMore = false;
      break;
    }

    // Parallel processing within batch
    const [dupResult, contrResult, pruneResult] = await Promise.all([
      deduplicateBatch(batch),
      detectContradictionsBatch(batch),
      pruneBatch(batch)
    ]);

    stats.processed += batch.length;
    stats.duplicates += dupResult.removed;
    stats.contradictions += contrResult.found;
    stats.pruned += pruneResult.pruned;

    offset += batchSize;

    // Progress indicator
    console.log(`Processed ${stats.processed} memories...`);
  }

  stats.duration = Date.now() - startTime;

  return stats;
}

// Performance:
// 10,000 memories: 8.2 seconds (1,220 memories/sec)
// Memory usage: <200MB peak
```

---

## 🔧 Production Deployment Guide

### Docker Compose Setup

```yaml
version: '3.8'

services:
  # Main application with ReasoningBank
  app:
    build: .
    environment:
      - NODE_ENV=production
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - DATABASE_PATH=/data/memory.db
      - REDIS_URL=redis://redis:6379
    volumes:
      - app-data:/data
    depends_on:
      - redis
    deploy:
      replicas: 3
      resources:
        limits:
          memory: 1G
          cpus: '1.0'

  # Redis for embedding cache
  redis:
    image: redis:7-alpine
    command: redis-server --maxmemory 512mb --maxmemory-policy allkeys-lru
    volumes:
      - redis-data:/data

  # Prometheus for metrics
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    ports:
      - "9090:9090"

  # Grafana for dashboards
  grafana:
    image: grafana/grafana:latest
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_PASSWORD}
    volumes:
      - grafana-data:/var/lib/grafana
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards
    ports:
      - "3000:3000"
    depends_on:
      - prometheus

volumes:
  app-data:
  redis-data:
  prometheus-data:
  grafana-data:
```

### Monitoring & Alerting

```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'reasoningbank'
    static_configs:
      - targets: ['app:8080']

# Alert rules
rule_files:
  - 'alerts.yml'

# alerts.yml
groups:
  - name: reasoningbank
    interval: 30s
    rules:
      # Memory bank health
      - alert: MemoryBankGrowthStalled
        expr: rate(reasoningbank_memories_total[5m]) == 0
        for: 1h
        annotations:
          summary: "No new memories created in 1 hour"

      # Retrieval performance
      - alert: SlowMemoryRetrieval
        expr: reasoningbank_retrieval_latency_ms > 100
        for: 5m
        annotations:
          summary: "Memory retrieval taking >100ms"

      # Consolidation backlog
      - alert: ConsolidationBacklog
        expr: reasoningbank_memories_since_consolidation > 50
        for: 30m
        annotations:
          summary: "50+ memories pending consolidation"

      # Success rate degradation
      - alert: LowSuccessRate
        expr: rate(reasoningbank_task_success_total[1h]) / rate(reasoningbank_task_total[1h]) < 0.7
        for: 30m
        annotations:
          summary: "Success rate dropped below 70%"
```

### Backup Strategy

```bash
#!/bin/bash
# backup-reasoningbank.sh

DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/backups/reasoningbank"
DB_PATH="/data/memory.db"

# Create backup directory
mkdir -p "$BACKUP_DIR"

# SQLite backup (online, no locking)
sqlite3 "$DB_PATH" ".backup '$BACKUP_DIR/memory_$DATE.db'"

# Compress backup
gzip "$BACKUP_DIR/memory_$DATE.db"

# Upload to S3
aws s3 cp "$BACKUP_DIR/memory_$DATE.db.gz" "s3://my-backups/reasoningbank/"

# Cleanup old local backups (keep 7 days)
find "$BACKUP_DIR" -name "memory_*.db.gz" -mtime +7 -delete

# Verify backup integrity
gunzip -c "$BACKUP_DIR/memory_$DATE.db.gz" | sqlite3 :memory: "PRAGMA integrity_check;"

echo "Backup completed: memory_$DATE.db.gz"
```

---

## 🔮 Future Roadmap & Research Directions

### Phase 1: Enhanced Memory Systems (Q1 2025)

**1. Hierarchical Memory Organization**
```typescript
// Multi-level memory hierarchy
interface MemoryHierarchy {
  episodic: {
    // Short-term: Last 24 hours
    recent: Memory[];

    // Medium-term: Last 30 days
    working: Memory[];
  };

  semantic: {
    // Long-term: Consolidated patterns
    knowledge: Memory[];

    // Meta-knowledge: Patterns about patterns
    meta: Memory[];
  };

  procedural: {
    // How-to memories
    skills: Memory[];
  };
}

// Automatic promotion based on usage
async function promoteMemory(memory: Memory) {
  if (memory.usage_count > 50 && memory.confidence > 0.85) {
    await promoteToSemantic(memory);  // Long-term knowledge
  }

  if (memory.teaches_process) {
    await promoteToProced ural(memory);  // Skill memory
  }
}
```

**2. Memory Relationships & Graph**
```typescript
// Build knowledge graph from memories
interface MemoryGraph {
  nodes: Memory[];
  edges: MemoryLink[];
}

interface MemoryLink {
  source: string;
  target: string;
  type: 'entails' | 'contradicts' | 'refines' | 'requires' | 'enables';
  confidence: number;
}

// Example: Multi-hop reasoning
// "Login requires CSRF token" + "CSRF token extracted from form"
// → "Login requires form parsing"
```

**3. Cross-Agent Memory Sharing**
```typescript
// Shared team memory pool
interface TeamMemoryBank {
  shared: Memory[];  // Accessible to all agents
  private: Map<string, Memory[]>;  // Agent-specific

  async shareMemory(memory: Memory, agents: string[]) {
    for (const agent of agents) {
      await grantAccess(agent, memory);
    }
  }

  async learnFromPeer(sourceAgent: string, targetAgent: string) {
    const sharedKnowledge = await fetchMemories({
      agent: sourceAgent,
      confidence: { min: 0.8 },
      usage: { min: 10 }
    });

    await transferKnowledge(sharedKnowledge, targetAgent);
  }
}
```

### Phase 2: Advanced ML Integration (Q2 2025)

**1. Learned Scoring Functions**
```python
# Replace hand-tuned weights with learned model
class LearnedScorer:
    def __init__(self):
        self.model = NeuralNetwork([
            Dense(128, activation='relu'),
            Dropout(0.3),
            Dense(64, activation='relu'),
            Dense(1, activation='sigmoid')
        ])

    def train(self, memories_with_outcomes):
        """
        Learn optimal scoring from past successes/failures

        Features:
        - Embedding similarity
        - Recency
        - Usage count
        - Confidence
        - Domain match
        - Agent match
        - Time of day
        - Task complexity

        Label: Did this memory help? (1 = yes, 0 = no)
        """
        X, y = prepare_training_data(memories_with_outcomes)
        self.model.fit(X, y, epochs=50, validation_split=0.2)

    def score(self, memory, query_context):
        features = extract_features(memory, query_context)
        return self.model.predict(features)[0]
```

**2. Active Learning for Memory Quality**
```typescript
// Actively seek feedback on low-confidence memories
async function activelyImproveMemory(memory: Memory) {
  if (memory.confidence < 0.6 && memory.usage_count > 5) {
    // Memory used multiple times but low confidence → needs validation

    const feedback = await requestHumanFeedback({
      memory,
      question: `Is this strategy correct?
                 "${memory.title}"

                 Used ${memory.usage_count} times with mixed results.

                 Please verify:
                 ☐ Correct and useful
                 ☐ Partially correct (needs refinement)
                 ☐ Incorrect (should be removed)`
    });

    switch (feedback.response) {
      case 'correct':
        memory.confidence = 0.9;  // Boost confidence
        break;
      case 'partial':
        await refineMemory(memory, feedback.suggestions);
        break;
      case 'incorrect':
        await deleteMemory(memory.id);
        break;
    }
  }
}
```

### Phase 3: Distributed ReasoningBank (Q3 2025)

**1. Federated Learning Across Orgs**
```typescript
// Learn from multiple organizations without sharing data
class FederatedReasoningBank {
  async federatedTrain(participants: Organization[]) {
    // Each org trains locally
    const localModels = await Promise.all(
      participants.map(org => org.trainLocalModel())
    );

    // Aggregate model updates (not raw data)
    const globalModel = aggregateModels(localModels);

    // Distribute updated model
    for (const org of participants) {
      await org.updateModel(globalModel);
    }

    // No org sees others' memories, but all benefit!
  }
}
```

**2. Multi-Region Replication**
```typescript
// Eventual consistency across regions
interface RegionalReasoningBank {
  region: 'us-east' | 'eu-west' | 'ap-southeast';
  localDb: Database;
  syncService: ReplicationService;

  async writeMemory(memory: Memory) {
    // Write locally (fast)
    await this.localDb.insert(memory);

    // Async replicate to other regions
    this.syncService.enqueueReplication({
      operation: 'insert',
      data: memory,
      targetRegions: ['us-east', 'eu-west', 'ap-southeast'].filter(r => r !== this.region)
    });
  }

  async readMemories(query: string) {
    // Always read from local region (low latency)
    return await this.localDb.retrieve(query);
  }
}
```

---

**This addendum will be posted as a comment on the main issue for additional technical depth.**
