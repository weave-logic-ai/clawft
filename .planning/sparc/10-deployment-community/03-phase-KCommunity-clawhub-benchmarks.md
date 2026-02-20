# Phase K-Community: ClawHub Skill Registry & OpenClaw Benchmark Suite

| Field | Value |
|-------|-------|
| **Element** | 10 -- Deployment & Community |
| **Phase** | K-Community: ClawHub Registry (K4) + Benchmarks (K5) |
| **Items** | K4 (ClawHub registry, vector search, signing, community), K5 (Benchmark suite, 3 MVP skills) |
| **Timeline** | Weeks 10-12 |
| **Priority** | P1 |
| **Crates** | `clawft-services` (ClawHub client), `clawft-core` (security/signing), `clawft-cli` (skill subcommands) |
| **Dependencies** | 04/C3-C4 (skill loader + hot-reload), 08/H2 (HNSW vector search + Embedder trait), 10/K3a (security scan on install) |
| **Blocks** | None (final sprint element) |
| **Status** | Planning |

---

## 1. Overview

Phase K-Community delivers the final two items of the sprint: a skill registry (ClawHub) and a competitive benchmark suite against OpenClaw. Together they establish the community and performance story for Clawft.

**K4 -- ClawHub Skill Registry** provides:
- A REST API client for searching, publishing, and installing skills from a central registry.
- Vector-based semantic search using the H2 `HnswVectorStore` and `Embedder` trait, with keyword fallback.
- Mandatory code signing for published skills (content hash verification on install).
- Star/comment community system for skill ratings and reviews.
- Agent auto-search: when a local skill lookup fails, the agent transparently queries ClawHub.
- CLI integration: `weft skill install`, `weft skill publish`, `weft skill search`, `weft skill update`.

**K5 -- Benchmark Suite** provides:
- Micro-benchmarks via `criterion` and E2E timing via `hyperfine`.
- Four metrics compared head-to-head against OpenClaw: binary size, cold start, peak RSS, throughput.
- Regression detection in CI with OpenClaw as the comparison baseline.
- Three MVP skills (`coding-agent`, `web-search`, `file-management`) ported to prove feature parity.

---

## 2. Specification

### 2.1 K4: ClawHub REST API (Contract #20)

The ClawHub client communicates with a remote registry server. The API contract is defined here and consumed by the `clawft-services/src/clawhub/` module.

#### Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/v1/skills/search?q=&limit=&offset=` | Optional | Search skills by query string |
| `GET` | `/api/v1/skills/{id}` | Optional | Get skill details by ID |
| `POST` | `/api/v1/skills/publish` | Required (signed) | Publish a new skill version |
| `POST` | `/api/v1/skills/install` | Required (Bearer) | Record an install event |
| `POST` | `/api/v1/skills/{id}/star` | Required (Bearer) | Star a skill |
| `DELETE` | `/api/v1/skills/{id}/star` | Required (Bearer) | Unstar a skill |
| `POST` | `/api/v1/skills/{id}/comments` | Required (Bearer) | Post a comment |
| `GET` | `/api/v1/skills/{id}/comments?limit=&offset=` | Optional | List comments |

#### Authentication

- **Read operations**: API key via `Authorization: Bearer <token>` header (optional for search/listing).
- **Publish**: Requires a signed key pair. The request body includes a detached signature over the skill content hash.
- **Community actions** (star, comment): Require Bearer token.

#### Response Schema

All endpoints return:

```rust
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub pagination: Option<Pagination>,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}
```

#### Skill Metadata Schema (Returned by Search/Get)

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct RegistrySkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub content_hash: String,
    pub signature: Option<String>,
    pub stars: u64,
    pub downloads: u64,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

### 2.2 K4: Vector Search Integration

ClawHub search uses the H2 `HnswVectorStore` and `Embedder` trait to provide semantic skill matching:

1. **Index build**: When the ClawHub client starts, it fetches the skill catalog (paginated) and embeds each skill's `name + description` into the local HNSW index.
2. **Search flow**: User query is embedded, then searched against the local HNSW index. Results are merged with server-side keyword results for comprehensive coverage.
3. **Fallback**: If H2 vector search is not available (feature flag disabled, embedder not configured), falls back to keyword-only search via the REST API `q=` parameter.

```rust
pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<RegistrySkill>> {
    // 1. Try vector search locally
    let vector_results = if let Some(ref store) = self.vector_store {
        let embedder = store.read().await.embedder().clone();
        let query_vec = embedder.embed(query).await?;
        let hits = store.read().await.search(&query_vec, limit);
        hits.into_iter().map(|(id, score)| (id, score)).collect()
    } else {
        Vec::new()
    };

    // 2. Keyword search via API
    let api_results = self.client.search_api(query, limit).await?;

    // 3. Merge: deduplicate by skill ID, prefer higher-scored vector hits
    merge_results(vector_results, api_results, limit)
}
```

#### Cache Refresh

The local HNSW index is refreshed on a configurable interval (default: 1 hour). A lightweight `GET /api/v1/skills/catalog?since=<timestamp>` endpoint returns only skills updated since the last sync, keeping incremental updates efficient.

### 2.3 K4: Skill Signing

Skill signing uses Ed25519 key pairs (via the `ed25519-dalek` crate) for content authentication.

#### Publication Flow

1. Author generates a key pair: `weft skill keygen` creates `~/.clawft/keys/skill-signing.pub` and `~/.clawft/keys/skill-signing.key`.
2. Author runs `weft skill publish ./my-skill/`.
3. CLI computes SHA-256 hash of the SKILL.md content + all files in the skill directory.
4. CLI signs the content hash with the private key.
5. CLI POSTs to `/api/v1/skills/publish` with the skill content, content hash, public key, and detached signature.
6. Server verifies the signature before accepting.

#### Install Verification Flow

1. User runs `weft skill install my-skill`.
2. CLI downloads skill content from ClawHub.
3. CLI verifies the content hash matches the published hash.
4. CLI verifies the Ed25519 signature over the content hash using the author's public key.
5. If verification fails, install is rejected with an error.
6. If `--allow-unsigned` is passed, unsigned skills are accepted but a warning is logged.

#### Key Data Structures

```rust
/// Content hash of a skill package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContentHash {
    /// SHA-256 hex digest of concatenated file contents.
    pub sha256: String,
    /// Ordered list of files included in the hash computation.
    pub files: Vec<String>,
}

/// Signature over a skill content hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSignature {
    /// Ed25519 signature (hex-encoded).
    pub signature: String,
    /// Public key (hex-encoded).
    pub public_key: String,
    /// Algorithm identifier.
    pub algorithm: String, // "ed25519"
}
```

#### Security Constraints

- Unsigned skills are **rejected by default** on install.
- `--allow-unsigned` flag is available for local development only. It logs a warning: `"WARNING: Installing unsigned skill '{name}'. This bypasses content verification."`.
- On `weft skill install`, the K3a security scan runs automatically on the downloaded content before placing files in `~/.clawft/skills/`.

### 2.4 K4: Star/Comment System

```rust
/// A star event on a skill.
#[derive(Debug, Serialize, Deserialize)]
pub struct SkillStar {
    pub skill_id: String,
    pub user_id: String,
    pub created_at: String,
}

/// A comment on a skill.
#[derive(Debug, Serialize, Deserialize)]
pub struct SkillComment {
    pub id: String,
    pub skill_id: String,
    pub user_id: String,
    pub body: String,
    pub created_at: String,
}
```

The client exposes:
- `star(skill_id)` / `unstar(skill_id)` -- toggle star status.
- `comment(skill_id, body)` -- post a comment.
- `list_comments(skill_id, limit, offset)` -- paginated comment listing.

Moderation is server-side. The client passes content through; the server enforces moderation rules.

### 2.5 K4: Agent Auto-Search

When an agent receives a user request that references a skill not found locally, the agent loop queries ClawHub:

```rust
// In agent loop (crates/clawft-core/src/agent/loop_core.rs):
async fn resolve_skill(&self, name: &str) -> Result<Option<SkillDefinition>> {
    // 1. Check local registry first
    if let Some(skill) = self.skill_registry.get(name) {
        return Ok(Some(skill.clone()));
    }

    // 2. Query ClawHub if configured
    if let Some(ref clawhub) = self.clawhub_client {
        let results = clawhub.search(name, 1).await?;
        if let Some(top_hit) = results.first() {
            // Prompt user for confirmation before installing
            let msg = format!(
                "Skill '{}' not found locally. Found '{}' v{} on ClawHub ({} stars). Install?",
                name, top_hit.name, top_hit.version, top_hit.stars
            );
            if self.confirm_action(&msg).await? {
                clawhub.install(&top_hit.id, false).await?;
                // Reload registry after install
                self.skill_registry.reload().await?;
                return Ok(self.skill_registry.get(name).cloned());
            }
        }
    }

    Ok(None)
}
```

The auto-search is gated behind user confirmation to prevent supply chain attacks via typosquatting.

### 2.6 K4: Crate Structure

#### `clawft-services/src/clawhub/` (new module)

| File | Purpose |
|------|---------|
| `mod.rs` | Module root, `ClawHubClient` struct, configuration |
| `api.rs` | HTTP client for REST API calls (reqwest-based) |
| `search.rs` | Vector + keyword search, result merging |
| `community.rs` | Star/comment operations |
| `types.rs` | Shared types (`RegistrySkill`, `ApiResponse`, etc.) |

#### `clawft-core/src/security/signing.rs` (new file)

| Function | Purpose |
|----------|---------|
| `generate_keypair()` | Create Ed25519 key pair, save to `~/.clawft/keys/` |
| `compute_content_hash(dir: &Path)` | SHA-256 hash of skill directory contents |
| `sign_content(hash: &str, private_key: &[u8])` | Ed25519 sign the content hash |
| `verify_signature(hash: &str, sig: &str, pub_key: &str)` | Verify Ed25519 signature |
| `verify_skill_package(dir: &Path, sig: &SkillSignature)` | Full verification pipeline |

#### `clawft-cli/src/commands/skills_cmd.rs` (extend existing)

New subcommands added to the existing `SkillsAction` enum:

```rust
#[derive(Subcommand)]
pub enum SkillsAction {
    // ... existing: List, Show, Install ...

    /// Publish a skill to ClawHub.
    Publish {
        /// Path to the skill directory.
        path: String,
    },

    /// Search ClawHub for skills.
    Search {
        /// Search query.
        query: String,
        /// Maximum results to return.
        #[arg(long, default_value = "10")]
        limit: usize,
    },

    /// Update an installed skill to the latest version.
    Update {
        /// Skill name to update.
        name: String,
    },

    /// Generate a signing key pair.
    Keygen,

    /// Install a skill from ClawHub (extends existing local Install).
    RemoteInstall {
        /// Skill name or ID on ClawHub.
        name: String,
        /// Allow unsigned skills (local dev only).
        #[arg(long)]
        allow_unsigned: bool,
    },
}
```

### 2.7 K5: Benchmark Methodology

#### Metrics

| # | Metric | Tool | Direction | Unit |
|---|--------|------|-----------|------|
| 1 | Binary size | `stat` on stripped release binary | Lower is better | KB |
| 2 | Cold start | `hyperfine --warmup 0 --min-runs 20` | Lower is better | ms |
| 3 | Peak RSS | `/usr/bin/time -v` or `/proc/self/status` | Lower is better | MB |
| 4 | Throughput | `criterion` sustained 60s run | Higher is better | msgs/sec |

#### Baseline

- **Clawft baseline**: Current values from `scripts/bench/baseline.json` (startup: 3.5ms, binary: 4710KB, throughput: 418/s).
- **OpenClaw baseline**: Latest release tag at sprint start. Measured on the same CI runner hardware.

#### Reproducibility

- All benchmarks run on dedicated CI runner (`runs-on: ubuntu-latest` with pinned runner image).
- Hardware specs documented in results JSON (`cpu_model`, `ram_gb`, `os_version`).
- Both Clawft and OpenClaw measured in the same CI job to eliminate hardware variance.
- Minimum 20 runs per measurement, outlier trimming at p5/p95.

### 2.8 K5: Criterion Micro-Benchmarks

New file: `benches/clawft_bench.rs`

```rust
use criterion::{Criterion, criterion_group, criterion_main, black_box};

fn bench_skill_parse(c: &mut Criterion) {
    let skill_md = include_str!("../fixtures/bench_skill.md");
    c.bench_function("skill_parse_md", |b| {
        b.iter(|| {
            clawft_core::agent::skills_v2::parse_skill_md(black_box(skill_md), None)
        })
    });
}

fn bench_message_routing(c: &mut Criterion) {
    // Benchmark message routing throughput
    c.bench_function("message_route_simple", |b| {
        b.iter(|| {
            // Route a message through the pipeline
        })
    });
}

fn bench_embedding_hash(c: &mut Criterion) {
    let embedder = clawft_core::embeddings::hash_embedder::HashEmbedder::default_dimension();
    c.bench_function("embed_hash_384d", |b| {
        b.iter(|| {
            embedder.compute_embedding(black_box("benchmark test string for embedding"))
        })
    });
}

fn bench_hnsw_search(c: &mut Criterion) {
    // Build a store with 1000 entries, then benchmark search
    c.bench_function("hnsw_search_1k", |b| {
        // Setup: insert 1000 random vectors
        b.iter(|| {
            // Search for top-10
        })
    });
}

criterion_group!(
    benches,
    bench_skill_parse,
    bench_message_routing,
    bench_embedding_hash,
    bench_hnsw_search,
);
criterion_main!(benches);
```

### 2.9 K5: E2E Comparison Script

New file: `scripts/bench/compare-openclaw.sh`

```bash
#!/usr/bin/env bash
# Compare Clawft and OpenClaw performance head-to-head.
#
# Usage: compare-openclaw.sh <clawft-binary> <openclaw-binary> [output.json]
set -euo pipefail

CLAWFT_BIN="${1:?Usage: compare-openclaw.sh <clawft> <openclaw> [output.json]}"
OPENCLAW_BIN="${2:?Usage: compare-openclaw.sh <clawft> <openclaw> [output.json]}"
OUTPUT="${3:-compare-results.json}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== Head-to-Head: Clawft vs OpenClaw ==="

# Binary size
CLAWFT_SIZE=$(stat -c%s "$CLAWFT_BIN" 2>/dev/null || stat -f%z "$CLAWFT_BIN")
OPENCLAW_SIZE=$(stat -c%s "$OPENCLAW_BIN" 2>/dev/null || stat -f%z "$OPENCLAW_BIN")

# Cold start (hyperfine)
CLAWFT_STARTUP=$(hyperfine --warmup 0 --min-runs 20 --export-json /tmp/hf-clawft.json \
    "$CLAWFT_BIN --version" 2>/dev/null && \
    python3 -c "import json; print(json.load(open('/tmp/hf-clawft.json'))['results'][0]['median']*1000)")

OPENCLAW_STARTUP=$(hyperfine --warmup 0 --min-runs 20 --export-json /tmp/hf-openclaw.json \
    "$OPENCLAW_BIN --version" 2>/dev/null && \
    python3 -c "import json; print(json.load(open('/tmp/hf-openclaw.json'))['results'][0]['median']*1000)")

# Peak RSS
CLAWFT_RSS=$(/usr/bin/time -v "$CLAWFT_BIN" --help 2>&1 | \
    grep "Maximum resident" | awk '{print $NF}')

OPENCLAW_RSS=$(/usr/bin/time -v "$OPENCLAW_BIN" --help 2>&1 | \
    grep "Maximum resident" | awk '{print $NF}')

# Output JSON
python3 -c "
import json
results = {
    'clawft': {
        'binary_size_kb': $CLAWFT_SIZE // 1024,
        'cold_start_ms': $CLAWFT_STARTUP,
        'peak_rss_kb': $CLAWFT_RSS,
    },
    'openclaw': {
        'binary_size_kb': $OPENCLAW_SIZE // 1024,
        'cold_start_ms': $OPENCLAW_STARTUP,
        'peak_rss_kb': $OPENCLAW_RSS,
    }
}
print(json.dumps(results, indent=2))
with open('$OUTPUT', 'w') as f:
    json.dump(results, f, indent=2)
"
```

### 2.10 K5: 3 MVP OpenClaw Skills

Three skills are ported to demonstrate feature parity and serve as benchmark targets.

#### `skills/coding-agent/SKILL.md`

```yaml
---
name: coding-agent
description: |
  AI-powered coding assistant that can read, write, and modify source code
  files. Supports multiple languages via tree-sitter parsing.
version: "1.0.0"
variables: [language, task, file_path]
allowed-tools: [Read, Write, Edit, Glob, Grep, Bash]
user-invocable: true
openclaw-category: development
---
```

Depends on: C3 skill loader for SKILL.md parsing, F-Core tools (Read, Write, Edit) for file operations.

#### `skills/web-search/SKILL.md`

```yaml
---
name: web-search
description: |
  Search the web for information and return structured results.
  Supports query refinement and source extraction.
version: "1.0.0"
variables: [query, max_results]
allowed-tools: [WebSearch, WebFetch]
user-invocable: true
openclaw-category: research
---
```

Depends on: F-Core tools (WebSearch, WebFetch) for web access.

#### `skills/file-management/SKILL.md`

```yaml
---
name: file-management
description: |
  Manage files and directories: create, move, copy, delete, list,
  and search. Supports glob patterns and recursive operations.
version: "1.0.0"
variables: [operation, path, destination]
allowed-tools: [Read, Write, Glob, Bash]
user-invocable: true
openclaw-category: utilities
---
```

Depends on: F-Core tools for filesystem operations.

---

## 3. Pseudocode

### 3.1 ClawHub Client Initialization

```
FUNCTION init_clawhub_client(config: ClawHubConfig) -> ClawHubClient
    // Build HTTP client with auth headers
    http_client = reqwest::Client::builder()
        .default_headers(auth_headers(config.api_key))
        .timeout(config.timeout)
        .build()

    // Initialize local vector store if H2 is available
    vector_store = None
    IF feature_enabled("vector-memory")
        embedder = HashEmbedder::default_dimension()  // or ApiEmbedder if configured
        store = HnswVectorStore::new(Arc::new(embedder))
        vector_store = Some(Arc::new(RwLock::new(store)))

        // Background task: sync skill catalog into HNSW
        spawn_catalog_sync(http_client.clone(), vector_store.clone(), config.sync_interval)
    END IF

    RETURN ClawHubClient {
        http: http_client,
        base_url: config.registry_url,
        vector_store,
        signing_key: load_signing_key(),
    }
END FUNCTION
```

### 3.2 Catalog Sync (Background)

```
FUNCTION catalog_sync_loop(client, store, interval)
    last_sync = None

    LOOP every interval
        // Fetch skills updated since last sync
        url = "/api/v1/skills/catalog"
        IF last_sync IS NOT None
            url += "?since=" + last_sync
        END IF

        skills = client.GET(url).await
        IF skills.ok
            FOR EACH skill IN skills.data
                text = skill.name + " " + skill.description
                embedding = embedder.embed(text).await
                store.write().await.insert_vector(skill.id, embedding)
            END FOR

            IF store.read().await.needs_reindex()
                store.write().await.rebuild_index()
            END IF

            last_sync = now()
        END IF
    END LOOP
END FUNCTION
```

### 3.3 Search Pipeline (Vector + Keyword Merge)

```
FUNCTION search(query, limit) -> Vec<RegistrySkill>
    // Phase 1: Vector search (local HNSW)
    vector_hits = []
    IF vector_store IS SOME
        query_vec = embedder.embed(query).await
        raw_hits = vector_store.read().await.search(query_vec, limit * 2)
        // raw_hits: Vec<(skill_id, similarity_score)>
        vector_hits = raw_hits.filter(|(_, score)| score > 0.3)
    END IF

    // Phase 2: Keyword search (remote API)
    api_response = http.GET("/api/v1/skills/search?q={query}&limit={limit}").await
    keyword_hits = api_response.data  // Vec<RegistrySkill>

    // Phase 3: Merge and deduplicate
    merged = HashMap::new()
    FOR (id, score) IN vector_hits
        merged.insert(id, (score, None))  // vector score, no detail yet
    END FOR
    FOR skill IN keyword_hits
        IF merged.contains(skill.id)
            merged[skill.id].1 = Some(skill)  // attach detail
        ELSE
            merged.insert(skill.id, (0.0, Some(skill)))  // keyword-only hit
        END IF
    END FOR

    // Fetch details for vector-only hits
    FOR (id, (score, detail)) IN merged WHERE detail IS None
        detail_response = http.GET("/api/v1/skills/{id}").await
        IF detail_response.ok
            merged[id].1 = detail_response.data
        END IF
    END FOR

    // Sort by combined score (vector similarity * 0.7 + download_count_normalized * 0.3)
    results = merged.values()
        .filter(|(_, detail)| detail.is_some())
        .sort_by(combined_score)
        .take(limit)
        .collect()

    RETURN results
END FUNCTION
```

### 3.4 Skill Publication Flow

```
FUNCTION publish_skill(dir: Path, allow_unsigned: bool) -> Result<()>
    // 1. Validate skill directory
    skill_md = dir.join("SKILL.md")
    IF NOT skill_md.exists()
        RETURN Err("No SKILL.md found in directory")
    END IF

    // 2. Parse and validate
    content = read_file(skill_md)
    skill = parse_skill_md(content)

    // 3. Run security scan (K3a)
    scan_result = security_scan(dir)
    IF scan_result.has_critical_issues()
        RETURN Err("Security scan failed: {issues}")
    END IF

    // 4. Compute content hash
    hash = compute_content_hash(dir)
    // hash.sha256 = SHA-256 of sorted, concatenated file contents
    // hash.files = ordered list of included files

    // 5. Sign the hash
    signing_key = load_private_key("~/.clawft/keys/skill-signing.key")
    IF signing_key IS None
        IF allow_unsigned
            warn!("Publishing unsigned skill. Run 'weft skill keygen' for signing.")
            signature = None
        ELSE
            RETURN Err("No signing key found. Run 'weft skill keygen' or use --allow-unsigned")
        END IF
    ELSE
        signature = sign_content(hash.sha256, signing_key)
    END IF

    // 6. Package and upload
    payload = SkillPublishRequest {
        name: skill.name,
        description: skill.description,
        version: skill.version,
        content: read_all_files(dir),
        content_hash: hash,
        signature: signature,
    }

    response = http.POST("/api/v1/skills/publish", payload).await
    IF NOT response.ok
        RETURN Err(response.error)
    END IF

    println!("Published {} v{} to ClawHub", skill.name, skill.version)
END FUNCTION
```

### 3.5 Skill Install with Verification

```
FUNCTION install_skill(name: str, allow_unsigned: bool) -> Result<()>
    // 1. Search for the skill
    results = search(name, 1).await
    skill = results.first()
        .ok_or("Skill not found on ClawHub")?

    // 2. Download content
    content = http.GET("/api/v1/skills/{skill.id}/download").await
    IF NOT content.ok
        RETURN Err("Download failed")
    END IF

    // 3. Verify content hash
    computed_hash = sha256(content.data.raw_bytes)
    IF computed_hash != skill.content_hash
        RETURN Err("Content hash mismatch -- possible tampering")
    END IF

    // 4. Verify signature
    IF skill.signature IS SOME
        valid = verify_signature(computed_hash, skill.signature, skill.public_key)
        IF NOT valid
            RETURN Err("Invalid signature -- possible tampering")
        END IF
    ELSE
        IF NOT allow_unsigned
            RETURN Err("Skill is unsigned. Use --allow-unsigned to install (not recommended)")
        ELSE
            warn!("WARNING: Installing unsigned skill '{}'. This bypasses content verification.", name)
        END IF
    END IF

    // 5. Run security scan (K3a)
    temp_dir = extract_to_temp(content.data)
    scan_result = security_scan(temp_dir)
    IF scan_result.has_critical_issues()
        cleanup(temp_dir)
        RETURN Err("Security scan failed on downloaded skill: {issues}")
    END IF

    // 6. Install to user skills directory
    install_dir = "~/.clawft/skills/{name}"
    move_dir(temp_dir, install_dir)

    println!("Installed {} v{} from ClawHub", name, skill.version)
END FUNCTION
```

### 3.6 Benchmark Comparison Pipeline

```
FUNCTION run_benchmark_comparison(clawft_bin, openclaw_bin) -> ComparisonReport
    report = ComparisonReport::new()

    // Metric 1: Binary size
    report.clawft.binary_size = file_size(clawft_bin)
    report.openclaw.binary_size = file_size(openclaw_bin)

    // Metric 2: Cold start
    FOR i IN 1..=20
        report.clawft.cold_starts.push(time_command(clawft_bin, "--version"))
        report.openclaw.cold_starts.push(time_command(openclaw_bin, "--version"))
    END FOR
    report.clawft.cold_start_median = median(report.clawft.cold_starts)
    report.openclaw.cold_start_median = median(report.openclaw.cold_starts)

    // Metric 3: Peak RSS
    report.clawft.peak_rss = measure_rss(clawft_bin, "--help")
    report.openclaw.peak_rss = measure_rss(openclaw_bin, "--help")

    // Metric 4: Throughput (criterion handles this)
    // criterion outputs to target/criterion/ automatically
    run_criterion_benchmarks()

    // Generate comparison table
    report.generate_markdown()
    report.generate_json()

    RETURN report
END FUNCTION
```

---

## 4. Architecture

### 4.1 ClawHub Component Diagram

```
+-------------------+     +------------------+     +------------------+
|   clawft-cli      |     | clawft-services  |     | clawft-core      |
|                   |     |                  |     |                  |
| skills_cmd.rs     |     | clawhub/         |     | security/        |
|  - search         |---->|  mod.rs          |     |  signing.rs      |
|  - publish        |     |  api.rs          |---->|  - keygen        |
|  - install        |     |  search.rs       |     |  - sign          |
|  - update         |     |  community.rs    |     |  - verify        |
|  - keygen         |     |  types.rs        |     |                  |
+-------------------+     +------------------+     | embeddings/      |
                               |      |            |  hnsw_store.rs   |
                               |      +----------->|  hash_embedder.rs|
                               |                   |  (H2 vector)     |
                               v                   +------------------+
                     +-------------------+
                     | ClawHub Server    |
                     | (REST API)        |
                     |                   |
                     | /api/v1/skills/*  |
                     +-------------------+
```

### 4.2 Skill Lifecycle

```
Author                      ClawHub                      Consumer
  |                            |                             |
  |  weft skill keygen         |                             |
  |  (generates Ed25519 pair)  |                             |
  |                            |                             |
  |  weft skill publish ./     |                             |
  |  1. parse SKILL.md         |                             |
  |  2. security scan (K3a)    |                             |
  |  3. compute content hash   |                             |
  |  4. sign with private key  |                             |
  |  5. POST /publish -------->|  verify signature           |
  |                            |  store skill + metadata     |
  |                            |  index in vector store      |
  |                            |                             |
  |                            |  GET /search?q=... <--------|  weft skill search
  |                            |  return ranked results ---->|
  |                            |                             |
  |                            |  POST /install <------------|  weft skill install
  |                            |  return skill content ----->|
  |                            |                             |  1. verify hash
  |                            |                             |  2. verify signature
  |                            |                             |  3. security scan (K3a)
  |                            |                             |  4. install to ~/.clawft/skills/
```

### 4.3 Benchmark Pipeline

```
CI Job: benchmark-comparison
  |
  +-- Build Clawft (cargo build --release)
  +-- Download OpenClaw (latest release)
  |
  +-- Run scripts/bench/run-all.sh (Clawft)
  |     +-- startup-time.sh
  |     +-- memory-usage.sh
  |     +-- throughput.sh
  |     +-- save-results.sh -> clawft-results.json
  |
  +-- Run scripts/bench/run-all.sh (OpenClaw)
  |     +-- startup-time.sh
  |     +-- memory-usage.sh
  |     +-- throughput.sh
  |     +-- save-results.sh -> openclaw-results.json
  |
  +-- Run scripts/bench/compare-openclaw.sh
  |     +-- Merge results into compare-results.json
  |     +-- Generate markdown comparison table
  |
  +-- Run cargo bench (criterion micro-benchmarks)
  |     +-- bench_skill_parse
  |     +-- bench_message_routing
  |     +-- bench_embedding_hash
  |     +-- bench_hnsw_search
  |
  +-- Run regression-check.sh (existing)
  |     +-- Check Clawft vs baseline.json (10% threshold)
  |
  +-- Comment on PR with comparison table
  +-- Upload artifacts
```

### 4.4 Vector Search Architecture (K4 + H2 Integration)

```
User Query: "code review tool"
       |
       v
+-- ClawHub Search Pipeline --+
|                              |
|  1. Embed query (H2)        |
|     HashEmbedder.embed()     |
|     -> [0.12, -0.34, ...]   |
|                              |
|  2. HNSW search (local)     |
|     HnswVectorStore.search() |
|     -> [(skill_id, 0.87),   |
|         (skill_id, 0.72)]   |
|                              |
|  3. Keyword search (remote)  |
|     GET /search?q=code+review|
|     -> [RegistrySkill, ...]  |
|                              |
|  4. Merge + deduplicate      |
|     Rank by combined score   |
|     -> Top N results         |
+------------------------------+
```

---

## 5. Refinement

### 5.1 ClawHub Scaling Considerations

**Catalog size**: The initial deployment targets <10K skills. At this scale:
- HNSW index with 384-dimension HashEmbedder vectors: ~15MB RAM.
- Full catalog sync on startup: ~2-5 seconds.
- Incremental sync: <100ms for typical deltas.

**Future scaling path** (post-sprint):
- For 100K+ skills: switch to server-side vector search; local index becomes a cache of recently used skills.
- CDN-backed skill downloads for geographic distribution.
- Distributed registry mirrors.

### 5.2 Search Accuracy

**Vector search quality**: HashEmbedder produces 384-dimensional SimHash vectors. These provide reasonable semantic similarity for short text (skill names + descriptions) but are not competitive with transformer-based embeddings for long or nuanced queries.

**Hybrid scoring**: The combined score formula `vector_similarity * 0.7 + normalized_popularity * 0.3` balances semantic relevance with community signal (downloads, stars). The weights are configurable via `ClawHubConfig`.

**Fallback behavior**: When H2 vector search is unavailable:
- Search degrades gracefully to keyword-only via the REST API.
- CLI displays a note: `"Note: Vector search unavailable. Using keyword search only."`
- No functionality is lost; only search relevance quality is reduced.

### 5.3 Benchmark Reproducibility

**Environment pinning**:
- CI runner: `ubuntu-latest` with specific cargo version pinned via `rust-toolchain.toml`.
- OpenClaw version: pinned to a specific git tag (e.g., `v0.5.0`) at sprint start, recorded in `scripts/bench/openclaw-version.txt`.
- System-level dependencies installed via `apt-get` with pinned versions where feasible.

**Statistical rigor**:
- Minimum 20 iterations per measurement (`hyperfine --min-runs 20`).
- `criterion` handles warm-up, outlier detection, and confidence intervals automatically.
- Regression threshold: 10% (matching existing `baseline.json` checks).

**Noise reduction**:
- CPU governor set to `performance` mode in CI.
- No parallel jobs during benchmark execution.
- Benchmarks run as a separate CI workflow to avoid contention.

### 5.4 Signing Key Management

**Key storage**: Keys are stored in `~/.clawft/keys/` with restrictive permissions (0600).
- `skill-signing.key` -- Ed25519 private key (PEM format).
- `skill-signing.pub` -- Ed25519 public key (PEM format).

**Key rotation**: Not required for MVP. Post-sprint: support multiple key pairs with key ID in signature metadata.

**Trust on first use (TOFU)**: When a user installs a skill from a new author, the public key is recorded. Subsequent versions from the same author must use the same key. Key changes require explicit user approval.

### 5.5 Security Hardening

**Supply chain protections**:
- Mandatory code signing prevents tampering in transit.
- Content hash verification detects bit-rot or CDN cache poisoning.
- K3a security scan runs on every `weft skill install`, checking for prompt injection, credential literals, and other attack patterns.
- Agent auto-search requires user confirmation before installing unknown skills.

**Rate limiting**: The ClawHub client respects `429 Too Many Requests` responses with `Retry-After` header. Default backoff: exponential with base 1s, max 60s.

---

## 6. Implementation Tasks

### Task K4-1: Add Dependencies

**File**: `crates/clawft-services/Cargo.toml`

```toml
[dependencies]
# ... existing ...
ed25519-dalek = { version = "2", optional = true }
sha2 = { version = "0.10", optional = true }

[features]
clawhub = ["dep:ed25519-dalek", "dep:sha2"]
```

**File**: `crates/clawft-core/Cargo.toml`

```toml
[dependencies]
# ... existing ...
ed25519-dalek = { version = "2", optional = true }

[features]
signing = ["dep:ed25519-dalek", "dep:sha2"]
```

### Task K4-2: Create ClawHub Client Module

**Files**:
- `crates/clawft-services/src/clawhub/mod.rs`
- `crates/clawft-services/src/clawhub/api.rs`
- `crates/clawft-services/src/clawhub/search.rs`
- `crates/clawft-services/src/clawhub/community.rs`
- `crates/clawft-services/src/clawhub/types.rs`

Implement `ClawHubClient` with:
- `new(config: ClawHubConfig) -> Self`
- `search(query: &str, limit: usize) -> Result<Vec<RegistrySkill>>`
- `publish(dir: &Path) -> Result<()>`
- `install(skill_id: &str, allow_unsigned: bool) -> Result<PathBuf>`
- `star(skill_id: &str) -> Result<()>`
- `unstar(skill_id: &str) -> Result<()>`
- `comment(skill_id: &str, body: &str) -> Result<()>`

### Task K4-3: Create Signing Module

**File**: `crates/clawft-core/src/security/signing.rs`

Implement:
- `generate_keypair(output_dir: &Path) -> Result<()>`
- `compute_content_hash(dir: &Path) -> Result<SkillContentHash>`
- `sign_content(hash: &str, private_key: &[u8]) -> Result<SkillSignature>`
- `verify_signature(hash: &str, sig: &SkillSignature) -> Result<bool>`
- `verify_skill_package(dir: &Path, sig: &SkillSignature) -> Result<bool>`

### Task K4-4: Implement Vector Search Integration

**File**: `crates/clawft-services/src/clawhub/search.rs`

Implement:
- Local HNSW index backed by H2 `HnswVectorStore`.
- Background catalog sync task.
- Hybrid search (vector + keyword) with result merging.
- Graceful fallback to keyword-only when H2 is unavailable.

### Task K4-5: Extend CLI Skill Commands

**File**: `crates/clawft-cli/src/commands/skills_cmd.rs`

Add subcommands: `Search`, `Publish`, `RemoteInstall`, `Update`, `Keygen`.

Each subcommand should:
- Parse arguments via `clap`.
- Instantiate `ClawHubClient` from config.
- Call the appropriate client method.
- Display results in a table (using `comfy_table` as in existing commands).

### Task K4-6: Integrate Agent Auto-Search

**File**: `crates/clawft-core/src/agent/loop_core.rs`

Add `resolve_skill()` method to the agent loop that:
1. Checks local registry.
2. Falls back to ClawHub search.
3. Prompts for user confirmation before installing.
4. Reloads registry after install.

### Task K5-1: Create Criterion Benchmark Suite

**File**: `benches/clawft_bench.rs`
**File**: `Cargo.toml` (workspace root, add `[[bench]]` section)

```toml
[[bench]]
name = "clawft_bench"
harness = false
```

Add `criterion` as a dev-dependency:
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
```

Implement benchmarks for: skill parsing, message routing, hash embedding, HNSW search.

### Task K5-2: Create OpenClaw Comparison Script

**File**: `scripts/bench/compare-openclaw.sh`

Measures all 4 metrics for both Clawft and OpenClaw binaries. Outputs comparison JSON and markdown table.

### Task K5-3: Extend Baseline with OpenClaw Data

**File**: `scripts/bench/baseline.json`

Add OpenClaw comparison fields:

```json
{
    "startup_time_ms": 3.5,
    "binary_size_kb": 4710,
    "throughput_invocations_per_sec": 418,
    "timestamp": "2026-02-17",
    "wasm_size_raw_kb": 57.9,
    "wasm_size_gzip_kb": 24.3,
    "openclaw_version": "v0.5.0",
    "openclaw_startup_time_ms": null,
    "openclaw_binary_size_kb": null,
    "openclaw_throughput_invocations_per_sec": null,
    "openclaw_peak_rss_kb": null
}
```

### Task K5-4: Extend CI Workflow

**File**: `.github/workflows/benchmarks.yml`

Add steps:
1. Download OpenClaw release binary.
2. Run comparison script.
3. Run criterion benchmarks.
4. Include comparison table in PR comment.

### Task K5-5: Create 3 MVP Skills

**Files**:
- `skills/coding-agent/SKILL.md`
- `skills/web-search/SKILL.md`
- `skills/file-management/SKILL.md`

Each skill directory contains a `SKILL.md` with proper frontmatter and instructions body. The instructions are ported from OpenClaw's equivalent skills with clawft-specific adaptations.

### Task K5-6: Update Regression Check for OpenClaw

**File**: `scripts/bench/regression-check.sh`

Extend to optionally compare against OpenClaw baseline in addition to self-baseline:

```bash
# New flag: --compare-openclaw <openclaw-results.json>
compare_metric "clawft_vs_openclaw_startup"  "lower"  "$R_STARTUP"  "$OC_STARTUP"
compare_metric "clawft_vs_openclaw_size"     "lower"  "$R_SIZE"     "$OC_SIZE"
compare_metric "clawft_vs_openclaw_throughput" "higher" "$R_THROUGHPUT" "$OC_THROUGHPUT"
```

---

## 7. Dependencies

### Crate Dependencies

| Dependency | Version | Purpose | New? | Feature Gate |
|-----------|---------|---------|------|-------------|
| `ed25519-dalek` | 2.x | Ed25519 signing/verification | Yes | `signing` / `clawhub` |
| `sha2` | 0.10 | SHA-256 content hashing | Exists in `rvf` | `signing` / `clawhub` |
| `reqwest` | existing | HTTP client for ClawHub API | No | -- |
| `serde` / `serde_json` | existing | (De)serialization | No | -- |
| `criterion` | 0.5 | Micro-benchmarks | Yes (dev-dep) | -- |
| `instant-distance` | 0.6 | HNSW vector search (via H2) | Exists via H2 | `vector-memory` |

### Internal Dependencies

| Dependency | Element | Status | Required For |
|-----------|---------|--------|-------------|
| C3 (Skill Loader) | 04 | Planning | ClawHub install places skills into registry |
| C4 (Hot-Reload) | 04 | Planning | Registry reload after ClawHub install |
| H2 (HNSW + Embedder) | 08 | Planning | Vector-based semantic search in ClawHub |
| K3a (Security Plugin) | 10 | Planning | Security scan on `weft skill install` |
| F-Core (Git, Cargo tools) | 07 | Planning | MVP skills require tool access |

### Cross-Element Integration Points

| Integration | Direction | Mechanism |
|------------|-----------|-----------|
| K4 -> H2 | K4 consumes H2 | `HnswVectorStore` + `Embedder` trait for semantic search |
| K4 -> K3a | K4 triggers K3a | Security scan runs on downloaded skill content |
| K4 -> C3 | K4 depends on C3 | Installed skills must be parseable by C3 loader |
| K4 -> Agent Loop | K4 integrates into loop | Auto-search in `loop_core.rs` |
| K5 -> Scripts | K5 extends existing | New scripts in `scripts/bench/` |
| K5 -> CI | K5 extends workflow | Additional steps in `benchmarks.yml` |

---

## 8. Tests Required

### 8.1 ClawHub Client Tests

| Test | Module | Description |
|------|--------|-------------|
| `clawhub_search_returns_results` | `clawhub/api` | Mock server returns skill list for search query |
| `clawhub_search_empty_query` | `clawhub/api` | Empty query returns all skills (paginated) |
| `clawhub_search_pagination` | `clawhub/api` | Limit/offset parameters work correctly |
| `clawhub_publish_success` | `clawhub/api` | Publish with valid signature succeeds |
| `clawhub_publish_no_key` | `clawhub/api` | Publish without signing key fails |
| `clawhub_install_verified` | `clawhub/api` | Install with valid signature and hash succeeds |
| `clawhub_install_bad_hash` | `clawhub/api` | Install with mismatched hash fails |
| `clawhub_install_bad_signature` | `clawhub/api` | Install with invalid signature fails |
| `clawhub_install_unsigned_rejected` | `clawhub/api` | Unsigned skill rejected without flag |
| `clawhub_install_unsigned_allowed` | `clawhub/api` | Unsigned skill accepted with `--allow-unsigned` |
| `clawhub_star_toggle` | `clawhub/community` | Star and unstar operations work |
| `clawhub_comment_post` | `clawhub/community` | Post comment succeeds |
| `clawhub_comment_list` | `clawhub/community` | List comments returns paginated results |
| `clawhub_api_error_handling` | `clawhub/api` | Non-200 responses produce descriptive errors |
| `clawhub_rate_limit_retry` | `clawhub/api` | 429 response triggers backoff and retry |

### 8.2 Signing Tests

| Test | Module | Description |
|------|--------|-------------|
| `keygen_creates_keypair` | `security/signing` | Key pair files created with correct permissions |
| `keygen_does_not_overwrite` | `security/signing` | Existing keys are not overwritten without confirmation |
| `content_hash_deterministic` | `security/signing` | Same directory produces same hash |
| `content_hash_detects_changes` | `security/signing` | Modified file produces different hash |
| `content_hash_ignores_metadata` | `security/signing` | File timestamps do not affect hash |
| `sign_and_verify_roundtrip` | `security/signing` | Sign then verify succeeds |
| `verify_wrong_key_fails` | `security/signing` | Verify with wrong public key fails |
| `verify_tampered_hash_fails` | `security/signing` | Modified hash fails verification |
| `verify_skill_package_full` | `security/signing` | Full package verification pipeline works |

### 8.3 Vector Search Tests

| Test | Module | Description |
|------|--------|-------------|
| `vector_search_finds_semantic_match` | `clawhub/search` | Query "code review" finds "code-reviewer" skill |
| `vector_search_fallback_keyword` | `clawhub/search` | When H2 unavailable, keyword search still works |
| `merge_results_dedup` | `clawhub/search` | Skills found by both vector and keyword are not duplicated |
| `merge_results_ranking` | `clawhub/search` | Vector hits rank above keyword-only hits |
| `catalog_sync_incremental` | `clawhub/search` | Only new/updated skills are synced |

### 8.4 CLI Tests

| Test | Module | Description |
|------|--------|-------------|
| `cli_skill_search_output` | `skills_cmd` | Search results displayed in table format |
| `cli_skill_publish_flow` | `skills_cmd` | Publish command calls signing and API correctly |
| `cli_skill_install_flow` | `skills_cmd` | Install command verifies and places files correctly |
| `cli_skill_keygen_flow` | `skills_cmd` | Keygen creates files in correct location |

### 8.5 Benchmark Tests

| Test | Description |
|------|-------------|
| `criterion_benchmarks_compile` | `cargo bench --no-run` succeeds |
| `compare_script_runs` | `compare-openclaw.sh` produces valid JSON output (with mock binaries) |
| `regression_check_pass` | Results within threshold produce exit code 0 |
| `regression_check_fail` | Results exceeding threshold produce exit code 1 |
| `mvp_skills_parse` | All 3 MVP SKILL.md files parse successfully via C3 loader |
| `mvp_skills_valid_frontmatter` | All 3 MVP skills have required fields (name, description, version) |

### 8.6 Integration Tests

| Test | Description |
|------|-------------|
| `clawhub_end_to_end` | Publish a skill, search for it, install it, verify signature |
| `agent_auto_search` | Agent resolves unknown skill via mock ClawHub server |
| `security_scan_on_install` | K3a scan runs during install flow and rejects malicious skill |

---

## 9. Completion

### 9.1 Acceptance Criteria

#### K4: ClawHub Registry

- [ ] `ClawHubClient` struct created in `clawft-services/src/clawhub/mod.rs`
- [ ] REST API client implements all Contract #20 endpoints (search, publish, install, star, comment)
- [ ] `ApiResponse<T>` and `Pagination` types match Contract #20 schema
- [ ] Vector search uses H2 `HnswVectorStore` for semantic skill matching
- [ ] Keyword fallback works when `vector-memory` feature is disabled
- [ ] Hybrid search merges vector and keyword results with deduplication
- [ ] Background catalog sync keeps local HNSW index current
- [ ] Ed25519 key pair generation via `weft skill keygen`
- [ ] `compute_content_hash()` produces deterministic SHA-256 over skill directory
- [ ] `sign_content()` produces valid Ed25519 signature
- [ ] `verify_signature()` validates signature against public key
- [ ] `weft skill publish` signs and uploads skill to ClawHub
- [ ] `weft skill install` verifies hash + signature before installing
- [ ] Unsigned skills rejected by default; `--allow-unsigned` logs warning and proceeds
- [ ] K3a security scan runs on every `weft skill install`
- [ ] Star/comment system exposed via CLI (`weft skill star`, `weft skill comment`)
- [ ] Agent auto-search queries ClawHub when local skill not found
- [ ] Agent auto-search requires user confirmation before installing
- [ ] All 15 ClawHub client tests pass
- [ ] All 9 signing tests pass
- [ ] All 5 vector search tests pass
- [ ] `cargo check --features clawhub` succeeds
- [ ] `cargo test --features clawhub` passes

#### K5: Benchmark Suite

- [ ] `criterion` dev-dependency added, `benches/clawft_bench.rs` created
- [ ] `[[bench]]` section added to workspace `Cargo.toml`
- [ ] 4 criterion micro-benchmarks implemented (skill parse, message route, embedding, HNSW search)
- [ ] `scripts/bench/compare-openclaw.sh` created and executable
- [ ] Comparison script measures all 4 metrics for both binaries
- [ ] Comparison script outputs JSON and markdown table
- [ ] `scripts/bench/baseline.json` extended with OpenClaw fields
- [ ] `regression-check.sh` extended with optional OpenClaw comparison
- [ ] `.github/workflows/benchmarks.yml` extended with comparison steps
- [ ] 3 MVP skills created in `skills/` directory with valid SKILL.md
- [ ] All 3 MVP skills parse via C3 skill loader
- [ ] `cargo bench --no-run` compiles successfully
- [ ] All benchmark tests pass
- [ ] Comparison report generated successfully with mock data

### 9.2 Test Plan

**Phase 1: Unit tests (local, no network)**
1. Run signing unit tests: `cargo test -p clawft-core --features signing -- signing`
2. Run ClawHub client tests with `mockito`: `cargo test -p clawft-services --features clawhub -- clawhub`
3. Run CLI tests: `cargo test -p clawft-cli -- skills`
4. Run benchmark compilation: `cargo bench --no-run`

**Phase 2: Integration tests (mock server)**
1. Spin up mock ClawHub server (mockito or similar).
2. Run end-to-end publish/search/install flow.
3. Verify agent auto-search with mock server.
4. Verify security scan integration on install.

**Phase 3: Benchmark validation**
1. Build release binary: `cargo build --release`.
2. Run `scripts/bench/run-all.sh target/release/weft`.
3. Run `scripts/bench/compare-openclaw.sh` with mock OpenClaw binary.
4. Verify `regression-check.sh` produces correct pass/fail.
5. Run `cargo bench` and verify criterion output.

**Phase 4: MVP skill validation**
1. Parse each MVP skill: `weft skills show coding-agent`, `weft skills show web-search`, `weft skills show file-management`.
2. Verify all required frontmatter fields are present.
3. If C3 is landed, verify skill execution flow.

### 9.3 Exit Criteria

- All acceptance criteria checked off.
- `cargo clippy --workspace` produces no new warnings.
- `cargo test --workspace` passes.
- `cargo bench --no-run` compiles.
- No secrets or credentials in committed code.
- All new code has appropriate error handling (no `unwrap()` on user-facing paths).
- Feature gates (`clawhub`, `signing`) isolate new dependencies from default build.

---

## 10. Risk Notes

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| H2 vector search not ready by Week 10 | Medium | Medium | 6 | Keyword-only fallback is fully functional. Vector search is an enhancement, not a blocker. |
| ClawHub server not deployed in time | Medium | High | 8 | Client code is testable against mocks. Server deployment is a separate infrastructure task. Client ships regardless. |
| ed25519-dalek API changes | Low | Medium | 3 | Pin to `2.x`. The crate is well-maintained by the Rust crypto working group. |
| OpenClaw binary not available for comparison | Low | High | 4 | Pin version in `openclaw-version.txt`. If binary unavailable, skip comparison and run Clawft-only benchmarks. |
| Criterion benchmark noise on CI | Medium | Low | 4 | Use dedicated runner, minimum 20 iterations, criterion's built-in outlier detection. |
| Typosquatting on ClawHub | Medium | Critical | 8 | Agent auto-search requires user confirmation. K3a scan catches malicious patterns. Future: namespace ownership. |
| Key management complexity for users | Medium | Medium | 6 | `weft skill keygen` automates key pair creation. Default location is well-known. TOFU model for trust. |
| 3 MVP skills incomplete due to C3 dependency | Medium | Medium | 6 | Skills can be authored as SKILL.md files even without C3. Execution testing deferred until C3 lands. |
| Content hash computation differs across platforms | Low | High | 4 | Sort files alphabetically, normalize line endings to LF, use canonical path ordering. |

---

## 11. File Summary

| File | Action | Description |
|------|--------|-------------|
| `crates/clawft-services/Cargo.toml` | EDIT | Add `ed25519-dalek`, `sha2`, `clawhub` feature |
| `crates/clawft-services/src/lib.rs` | EDIT | Add `pub mod clawhub` under `clawhub` feature |
| `crates/clawft-services/src/clawhub/mod.rs` | NEW | ClawHub client, config, initialization |
| `crates/clawft-services/src/clawhub/api.rs` | NEW | HTTP client for REST API |
| `crates/clawft-services/src/clawhub/search.rs` | NEW | Vector + keyword hybrid search |
| `crates/clawft-services/src/clawhub/community.rs` | NEW | Star/comment operations |
| `crates/clawft-services/src/clawhub/types.rs` | NEW | Shared types (RegistrySkill, ApiResponse, etc.) |
| `crates/clawft-core/Cargo.toml` | EDIT | Add `ed25519-dalek`, `signing` feature |
| `crates/clawft-core/src/security/signing.rs` | NEW | Key generation, signing, verification |
| `crates/clawft-cli/src/commands/skills_cmd.rs` | EDIT | Add Search, Publish, RemoteInstall, Update, Keygen subcommands |
| `crates/clawft-core/src/agent/loop_core.rs` | EDIT | Add `resolve_skill()` with ClawHub auto-search |
| `benches/clawft_bench.rs` | NEW | Criterion micro-benchmarks |
| `Cargo.toml` (workspace) | EDIT | Add `[[bench]]` section, `criterion` dev-dep |
| `scripts/bench/compare-openclaw.sh` | NEW | Head-to-head comparison script |
| `scripts/bench/baseline.json` | EDIT | Add OpenClaw comparison fields |
| `scripts/bench/regression-check.sh` | EDIT | Add optional OpenClaw comparison flag |
| `.github/workflows/benchmarks.yml` | EDIT | Add comparison and criterion steps |
| `skills/coding-agent/SKILL.md` | NEW | MVP coding agent skill |
| `skills/web-search/SKILL.md` | NEW | MVP web search skill |
| `skills/file-management/SKILL.md` | NEW | MVP file management skill |

---

## 12. Dependency Graph

```
08/H2 (HNSW + Embedder) ──┐
                            ├──> K4.4 (Vector Search Integration)
04/C3 (Skill Loader) ──────┤
                            ├──> K4.2 (ClawHub Client)
                            │         |
                            │         v
10/K3a (Security Scan) ────┼──> K4.5 (CLI: install/publish)
                            │         |
                            │         v
                            ├──> K4.6 (Agent Auto-Search)
                            │
                            └──> K5.5 (MVP Skills)

K4.3 (Signing) ──> K4.2 (ClawHub Client)
K4.3 (Signing) ──> K4.5 (CLI)

K5.1 (Criterion) ──> K5.4 (CI Workflow)
K5.2 (Compare Script) ──> K5.4 (CI Workflow)
K5.3 (Baseline Update) ──> K5.6 (Regression Check Update)
```
