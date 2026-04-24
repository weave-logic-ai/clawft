# ExoChain Observability — Format Research (Symposium Seat 3)

**Scope.** External research on log storage and compression formats for WeftOS / ExoChain
network chatter. The adjacent seats cover chain schema / multi-channel fabric (seat 1),
kernel-side lifecycle events (seat 2), and governance-gate integration (seat 4). This
document is decision-support only; no code, no commits.

**Proposal under evaluation.** The user's framing:
> "a separate protocol and storage format is probably in order for most of this type
> of network chatter. It keeps it separate from the main bus, which should be more
> about governance and stuff like that… we want to create a separate RVF format, or
> even look at a different type of format that can strategically compress the logs,
> since most logs are repeating."

The conclusion, up front, is that the user's instinct is correct on two counts:
(a) logs must leave the governance bus, and (b) the repetition is so structural that
a dedicated format is worth the code. Below we rank the candidates against the
ExoChain workload profile and recommend a concrete path.

---

## 1. Workload Profile

Restating from the brief and sharpening it with order-of-magnitude math so the
comparisons below have a common denominator.

| Dimension | Value | Notes |
|---|---|---|
| Event rate (aggregate) | 10³ – 10⁵ events/sec | Cluster-wide, not per-node. |
| Event rate (per node) | ~10¹ – 10³ events/sec | A node emits life-cycle + traffic events. |
| Raw event size | 100 – 500 B (JSON) | `cluster`, `node`, `service`, `agent`, `kind`, `k_level`, `severity`, `msg`, `ts`. |
| Inter-event redundancy | ≥ 90 % of field bytes | Same cluster / node / service across runs of events. |
| Write pattern | Append-only, single writer per shard | No random updates, no deletes (only retention sweeps). |
| Read pattern A | Tail-and-filter (real-time) | Subscribe to tag expression, low-latency. |
| Read pattern B | Range-replay (audit / debug) | `[t0,t1] WHERE tag=…`, seconds-minutes of latency OK. |
| Durability bound | ≤ 1 s of loss on crash | fsync-per-frame is too expensive; group-commit is fine. |
| Anchor semantics | Periodic BLAKE3 digest → ExoChain | Proves "these shards existed and were not tampered with." |
| Byte budget (rough) | 10⁴ ev/s × 250 B × 86.4 ks/day = ~216 GB/day raw | **Before** compression. This is the number to beat. |

**Compression math as a sanity check.** If 90 % of field bytes repeat, a competent
columnar + dictionary scheme should land in the 15 × – 40 × ratio band. ClickHouse
hits 170 × on NGINX logs because the tail bodies are also templated; our bodies
are more varied, so 15 × – 25 × is the realistic target. That drops 216 GB/day raw
to ~8 – 14 GB/day, which is small enough to keep many days hot on a single node.

---

## 2. Format Pro/Con Matrix

Scoring legend (subjective, workload-specific):
- **Ratio**: expected compressed / raw. Higher × is better.
- **Append**: ms to commit one batch with fsync-group-commit.
- **Tail-read**: cost to emit a matching event to a subscriber.
- **Tag-filter**: cost per-candidate-event to evaluate a tag expression.
- **Crash durability**: does frame framing survive torn writes; granularity of loss.
- **Impl effort**: rough days of engineering to productionize in the WeftOS kernel daemon.
- **Anchors**: composes cleanly with BLAKE3-rollup to the main chain?

Scores are **A / B / C / D** (A best). Ratios are educated estimates based on
the references in §4.

### 2.1 Candidate A — Extend RVF with a log segment type

I inspected the vendored RVF sources at
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rvf-runtime-0.2.0/`
and `rvf-types-0.2.0/`. Findings:

- The format is segment-based (`Vec`, `Index`, `Manifest`, `Journal`, `Witness`,
  `Wasm`, `Cow*`, `TransferPrior`, …). Discriminants are a `u8` enum with gaps
  that could accept `Log = 0x40` without a breaking change.
- `rvf-types/src/compression.rs` declares a `CompressionAlgo` enum (`None=0`,
  `Lz4=1`, `Zstd=2`, `Custom=3`) — but the **runtime does not wire it in**.
  Every write in `write_path.rs` emits `compression: 0` and the only
  implementation shipped is `compress.rs`, a hand-rolled LZ77 ("SCF-1") with
  a 4 KB window and 3–10-byte matches, explicitly sized for QR-seed
  microkernels. Typical ratio: 1.4× – 2.5×. That is an order of magnitude
  below what we need.
- There is **no columnar / batched record path**. `ingest_batch` is a batch
  of `f32` vectors with a `VEC_SEG` header (`dim`, `count`, then `[id || f32*dim]`)
  — it is not a general record batch.
- **No zstd dictionary training, no real zstd at all.** The `CompressionAlgo::Zstd`
  slot is reserved but unused.
- Two-fsync commit (payload segment + new manifest) is already there — good for
  our durability story, but expensive per-frame. For 10⁵ ev/s we would have to
  amortize across a batch or use `Journal`-like micro-segments.

**If we stayed on RVF**, we would:
1. Add `SegmentType::Log = 0x40`.
2. Add a real zstd-via-crate implementation behind the existing `CompressionAlgo::Zstd`,
   plus a `CompressionAlgo::ZstdDict = 4` variant that references a dict-id in
   a new `DictSeg` segment (or in `Meta`).
3. Define the payload as a columnar batch (see Candidate G below for the columns).
4. Leverage the existing `Witness`/`Manifest` chain for the BLAKE3 anchor —
   this is the real argument for RVF.

| Score | Grade | Why |
|---|---|---|
| Ratio | B | 15× – 25× if we do columns + zstd-dict; matches industry. |
| Append | B | Two-fsync + batch amortizes to ~5 ms per batch of 10 k events. |
| Tail-read | C | Reader has to decode a whole segment; streaming a tail is unnatural. |
| Tag-filter | C | No secondary index unless we also emit `MetaIdx` sidecars. |
| Crash durability | A | Two-fsync protocol; at most the open batch is lost. |
| Impl effort | **C → 15–25 days** | We'd be building zstd-dict + columnar-batch + index-sidecar inside RVF. |
| Anchors | A | Native. `Witness` + `Manifest` are literally the feature. |

**Verdict:** Viable but we'd be building a second format *inside* RVF. RVF is
designed for mutable vector stores, not append-mostly telemetry tails.

---

### 2.2 Candidate B — Apache Parquet (row-group columnar)

- Best-in-class compression for repeating fields: dict + RLE + delta + zstd per
  column, exactly the workload profile we have.
- **Streaming is structurally painful.** Parquet buffers a full row-group before
  flushing (recommended row-group size is 128 MB – 1 GB for query efficiency).
  For a tail reader this means nothing is visible until the group flushes.
  Commonly solved by a "two-pass" write (small groups, then consolidate), which
  is the write-amplification cost the brief flags.
- 1 s durability bound forces tiny row groups (~10 k rows), which destroys
  the columnar compression benefit and produces metadata overhead per group.
- Reader ecosystem is huge (DuckDB, Polars, Arrow-RS, PyArrow). That is real
  value for post-hoc audit.

| Score | Grade | Why |
|---|---|---|
| Ratio | **A** | 20× – 50× realistic on repetitive logs. |
| Append | D | Row-group buffering fights our 1 s loss bound. |
| Tail-read | D | Readers must wait for flush or read in-memory Arrow. |
| Tag-filter | A | Predicate pushdown via column stats + page indexes. |
| Crash durability | C | Unflushed row-group = lost rows; footer corruption = lost file. |
| Impl effort | B (~10 days) | `parquet` crate is mature; the hard part is the streaming workaround. |
| Anchors | B | Hash footer bytes; or hash each shard file at rotation. |

**Verdict:** Excellent for the cold tier. A poor fit for the hot/live tier.

---

### 2.3 Candidate C — Apache Arrow IPC Streaming Format

- Arrow IPC is explicitly "for sending an arbitrary number of record batches"
  with no fixed count and no seek requirement — the append-friendly sibling
  of the Feather/file format.
- In-memory representation is columnar, so a tail reader with Arrow-RS can
  filter on dictionary-encoded tag columns cheaply.
- Per-batch compression is supported (zstd, lz4) but **per-batch dictionaries
  reset each batch** unless you use the dict-delta feature carefully — this
  is the single biggest trap in the literature.
- Does not define at-rest durability semantics. You layer that on (WAL +
  rotate-on-batch).

| Score | Grade | Why |
|---|---|---|
| Ratio | B | 8× – 15×; dict coding helps, but resets cost us. |
| Append | A | Designed for streaming. |
| Tail-read | A | Columnar in-memory; filtering is native. |
| Tag-filter | A | Dict columns + Arrow compute kernels. |
| Crash durability | B | Per-batch framing survives torn writes; recent batch lost. |
| Impl effort | B (~8 days) | `arrow-rs` is solid; one-time schema evolution work. |
| Anchors | A | Hash each batch; Merkle-roll at flush. |

**Verdict:** Strong contender for the **hot tier**, paired with Parquet for cold.

---

### 2.4 Candidate D — systemd journald binary format

- Purpose-built for exactly this workload. Dict-coded field names, arbitrary
  `FIELD=value` tags, built-in binary-safe encoding, `journalctl`-grade reader.
- Has `MESSAGE_ID`, `PRIORITY`, `_SYSTEMD_UNIT` conventions that mirror our
  `kind`, `severity`, `service` schema; we could piggy-back.
- **Fatally Linux-only and opinionated.** WeftOS targets native + WASM + browser;
  embedding journald's on-disk format across those platforms is a fight we
  don't want. Also: the format is not versioned as a public spec in the
  stability sense — Lennart has shipped breaking changes.
- Licensing: LGPL for the reference reader.

| Score | Grade | Why |
|---|---|---|
| Ratio | B | XZ-compressed chunks; comparable to zstd in practice. |
| Append | B | Designed for high-volume journal writes. |
| Tail-read | A | `journalctl -f` is the reference impl. |
| Tag-filter | A | Native. |
| Crash durability | A | Journal has hash-chains and rotation. |
| Impl effort | D | Parsing it portably in Rust is real work; we'd effectively fork. |
| Anchors | C | Possible (hash the sealed journal) but awkward. |

**Verdict:** Great design inspiration, wrong deployment target.

---

### 2.5 Candidate E — OpenTelemetry OTLP + disk sink (Tempo / Loki)

- OTLP is a protobuf wire schema, not a storage format; you still need a sink.
- Loki's approach is what we'd end up with: per-stream chunks, snappy or
  lz4 compression, TSDB-indexed tag labels. It gets ~5× – 10× on logs in
  practice (snappy is fast but not dense).
- Plus side: vast ecosystem; every Grafana dashboard in the world knows how
  to point at it. Cross-tenant routing, retention, LogQL.
- Minus side: you are now running an observability stack as part of the
  substrate. Contradicts the user's framing — this is fabric-internal chatter,
  not a product telemetry pipeline.
- OTLP → disk still leaves you needing a format for the disk part.

| Score | Grade | Why |
|---|---|---|
| Ratio | C | Snappy/lz4 chunks; 5× – 10×. |
| Append | A | Designed for it. |
| Tail-read | A | LogQL, done for us. |
| Tag-filter | A | TSDB index. |
| Crash durability | A | Loki has replication + WAL. |
| Impl effort | D | We become Loki operators, not kernel engineers. |
| Anchors | C | We'd anchor Loki chunk hashes; possible but external. |

**Verdict:** Over-solution. Reject for the substrate; keep as an optional export.

---

### 2.6 Candidate F — FlatBuffers / Cap'n Proto records

- Schema-evolving, zero-copy. Cap'n Proto has a built-in "packing" that
  deflates zero-runs cheaply; FlatBuffers aligns to the scalar and is slightly
  denser out-of-the-box.
- Neither is a *file* format; both are record formats. You'd still need
  framing + compression + index + a durability story — that's the thing
  we're trying to buy off-the-shelf, not assemble.
- Per-record size on our schema: ~100 B packed. zstd over a chunk of 10 k
  such records hits 15× easily because of field repetition, but at that
  point you've rebuilt half of Arrow IPC.

| Score | Grade | Why |
|---|---|---|
| Ratio | C | Needs an outer zstd frame. |
| Append | A | Records are trivial to emit. |
| Tail-read | A | Zero-copy reader. |
| Tag-filter | B | Manual; no native secondary index. |
| Crash durability | B | Per-record framing survives torn writes. |
| Impl effort | C (~12 days) | You build the file layer. |
| Anchors | A | Easy. |

**Verdict:** No advantage over Arrow IPC for our workload.

---

### 2.7 Candidate G — JSONL + zstd-with-trained-dict, rotated per-minute

The "dead simple" baseline. Worth scoring because it sets the minimum bar
anything else has to clear.

- JSONL is append-friendly, human-decodable after decompression.
- Trained zstd dicts on log-like JSON give 2× – 3× better ratios than raw
  zstd, per the zstd docs and the Firefox-omni.ja benchmark cited below.
- With a 64 KB dict trained on one day of logs: realistic ratio 10× – 14×.
- Tag-filter is a linear scan unless you write an external index (Tantivy,
  a sidecar B-tree on `(ts, tag_hash)`, or just `grep -z`).

| Score | Grade | Why |
|---|---|---|
| Ratio | B | 10× – 14×. |
| Append | A | Append bytes, fsync-on-rotate. |
| Tail-read | B | Tail the current uncompressed chunk; grep-class filters. |
| Tag-filter | D | Full scan per chunk. |
| Crash durability | A | Uncompressed tail file; rotate when sealed. |
| Impl effort | A (~3 days) | `zstd` crate + a rotator. |
| Anchors | A | `blake3 shard.zst` → main chain. |

**Verdict:** Excellent baseline. If we don't do (J) below, we fall back to this.

---

### 2.8 Candidate H — Custom Gorilla-style (delta-of-delta + dict + bitpack)

The Facebook Gorilla paper (VLDB 2015) reports 12× on telemetry, with 96 % of
timestamps compressing to a single bit and 51 % of floating-point values to a
single bit. Our workload doesn't have FP values (mostly), but it does have:

- monotonic timestamps → delta-of-delta crushes them,
- field values with extremely skewed cardinality → dict-coding crushes them,
- repeated identical events (retry storms) → RLE crushes them.

Realistic ratio: 10× – 20×. Latency: sub-ms per batch. Implementation cost:
2–4 weeks of careful Rust because of bit-level work and round-trip testing,
plus a reader that's not in anyone else's toolbox.

| Score | Grade | Why |
|---|---|---|
| Ratio | A | 10× – 20×; peers ClickHouse. |
| Append | A | Streaming-native; no row-group delay. |
| Tail-read | A | Bit-reader can stream the current chunk. |
| Tag-filter | B | Dict-coded tag columns; bitmap indexes viable. |
| Crash durability | A | Chunk-framed with a BLAKE3 footer. |
| Impl effort | **D (~25–40 days + audit)** | We own a novel format forever. |
| Anchors | A | Native. |

**Verdict:** Best theoretical fit, worst engineering-cost profile. Reject
*as a ground-up effort*; consider cribbing the encodings inside Candidate J.

---

### 2.9 Candidate I — ClickHouse / Vector / operational stacks

Not a format; a system. Mentioned for completeness. Where it slots in:

- **ClickHouse as the cold-tier query engine** over our Parquet shards, via
  `S3` table engine or direct file reads. This gives us the 170× compression
  story *only* if we adopt ClickHouse's codec recipe: `DoubleDelta + ZSTD(6)`
  on timestamps and numerics, `LowCardinality(String) + ZSTD(6)` on tags,
  plain `ZSTD(6)` on free-text bodies. If we emit Parquet with equivalent
  logical types, DuckDB / ClickHouse / Polars can all read it.
- **Vector.dev** as the side-car that ships substrate logs into external
  sinks. Useful for humans; not part of the substrate storage decision.

---

### 2.10 Candidate J — **Recommended hybrid: WELF (WeftOS Event Log Format)**

A purpose-built, thin format that is Arrow IPC at heart with three
substrate-specific extensions. Not a ground-up reinvention; a narrow
profile over battle-tested pieces.

**Shape:**
```
shard_file := header
            || frame+                 // streaming append
            || footer (on rotate)

header     := magic "WELF\0" || u16 version || u32 dict_id
            || blake3(prev_shard_footer)      // chain to previous shard

frame      := u32 frame_len
            || u8  frame_kind          // 0=records, 1=dict-update, 2=checkpoint
            || u8  codec               // 0=none, 1=zstd, 2=zstd-dict
            || arrow_ipc_message_body  // columnar batch, schema below
            || u32 blake3_low32        // truncated; full hash in checkpoint

footer     := u32 frame_count
            || u64 first_ts_ns || u64 last_ts_ns
            || [32]u8 blake3_of_all_frames
            || [32]u8 blake3_footer_self
```

**Columnar schema** (one Arrow record batch per frame, 1 k – 10 k rows):

| Column | Arrow type | Encoding |
|---|---|---|
| `ts_ns` | `timestamp[ns]` | delta-of-delta → zstd |
| `cluster_id` | `dictionary<u16, utf8>` | dict + RLE |
| `node_id` | `dictionary<u32, utf8>` | dict + RLE |
| `service` | `dictionary<u16, utf8>` | dict + RLE |
| `agent` | `dictionary<u32, utf8>` | dict + RLE |
| `kind` | `dictionary<u16, utf8>` | dict + RLE |
| `k_level` | `u8` | bitpack |
| `severity` | `u8` | bitpack |
| `span_id` | `fixed_size_binary[8]` | zstd |
| `trace_id` | `fixed_size_binary[16]` | zstd |
| `body` | `utf8` | zstd-with-trained-dict |

**Key design properties:**
- The outer container is Arrow IPC streaming — we get arrow-rs, DuckDB,
  Polars, and Python for free.
- `body` is compressed with a **pre-shared zstd dictionary** trained per
  deployment. That is the single highest-leverage lever in the stack; it
  doubles the ratio on our small, templated payloads.
- The dict is referenced by `dict_id` in the header; a `frame_kind=1`
  frame rotates dicts without breaking the stream.
- Tag columns are Arrow dictionary-encoded and end up as small integer
  columns with a shared string pool — this is what lets tag-filter be
  near-free.
- Shard files are rolled every N seconds or M bytes; footer emits a
  BLAKE3 Merkle root over frames; the kernel daemon anchors the root to
  ExoChain. The header carries the previous shard's footer hash so the
  whole tail is a hash-chain, independent of the main chain.
- Crash durability: group-commit per ~100 ms, fsync after frame write,
  loss bound = the open frame = well under 1 s.

| Score | Grade | Why |
|---|---|---|
| Ratio | **A** | 15× – 25× achievable; Arrow dict + zstd-dict + delta ts. |
| Append | **A** | Streaming by construction; no row-group buffering. |
| Tail-read | **A** | Each frame is a self-contained Arrow batch. |
| Tag-filter | **A** | Dict-encoded tag columns = integer comparisons. |
| Crash durability | **A** | Per-frame fsync; footer-chain across shards. |
| Impl effort | **B (~7–10 days)** | Thin wrapper over `arrow-rs` + `zstd`. |
| Anchors | **A** | Merkle root per shard → ExoChain, trivial. |

---

### 2.11 Summary table

| # | Candidate | Ratio | Append | Tail | Tag | Durable | Effort | Anchor |
|---|---|---|---|---|---|---|---|---|
| A | Extend RVF | B | B | C | C | A | C | A |
| B | Parquet | A | D | D | A | C | B | B |
| C | Arrow IPC | B | A | A | A | B | B | A |
| D | journald | B | B | A | A | A | D | C |
| E | OTLP / Loki | C | A | A | A | A | D | C |
| F | FlatBuf / Capnp | C | A | A | B | B | C | A |
| G | JSONL+zstd-dict | B | A | B | D | A | A | A |
| H | Gorilla custom | A | A | A | B | A | D | A |
| I | ClickHouse (sys) | — | — | — | — | — | — | — |
| **J** | **WELF (hybrid)** | **A** | **A** | **A** | **A** | **A** | **B** | **A** |

---

## 3. Compression Techniques Survey

The repetition pattern in substrate logs is different from generic web-log or
metric compression because every event is *both* a time-series point (monotonic
timestamp, numeric severity/level) and a structured record (high-repetition
string fields). The best ratios come from stacking techniques, not picking one.

### 3.1 Log template extraction — Drain (He et al., ICWS 2017)

Drain builds a fixed-depth parse tree over the whitespace-tokenised first few
tokens of each log line and clusters into templates like
`"connection refused from <*>"`. On five real-world datasets totalling >10 M
messages, Drain hit 51.8 % – 81.47 % speedup over the prior state of the art
while keeping or improving accuracy. The relevance to us is structural: the
`body` column is the low-hanging fruit, and extracting templates *at write time*
in the kernel daemon would cut body bytes by 2× – 4× on top of zstd-dict.

**Decision:** Do not adopt Drain in v1. It's an online clusterer with state,
and it introduces a new way to lose information on crash. Revisit for a v2
"template column" where the template id is emitted alongside the free-text
body; this would give ClickHouse-tier ratios (170×) on the truly repetitive
subset of lines.

### 3.2 zstd with trained dictionaries (Collet / Facebook)

Facebook's production experience: a 128 KiB trained dict on small correlated
samples improves ratio by 2× – 3× and, critically, **decompression speed by
~2.2× – 2.4×** (logs specifically: 240 MB/s → 530 MB/s, vs. ~140 MB/s for
zlib-6). The dict must be bundled with the data (or referenced by id from a
registry).

**Decision:** Adopt. Train once per deployment, rotate via `frame_kind=1`
dict-update frames, address by `dict_id` in the shard header.

### 3.3 Gorilla (Pelkonen et al., VLDB 2015)

12× on production telemetry; 96 % of timestamps to 1 bit via delta-of-delta;
XOR-delta for FP values gets 51 % to 1 bit. Beringei is the open-source
descendant. The timestamp encoding in particular is free money for us because
our events are monotonic and often near-periodic (heartbeats, polling).

**Decision:** Adopt the timestamp-delta-of-delta technique inside the
`ts_ns` column. Arrow has delta encoding for timestamps; zstd on top closes
the gap. Skip the FP XOR path — we have no FP hot columns.

### 3.4 ClickHouse codec composition

The canonical log schema published by ClickHouse:
```
event_time DateTime CODEC(DoubleDelta, ZSTD(6))
status     LowCardinality(String) CODEC(ZSTD(6))
uri        String  CODEC(ZSTD(6))
body       String  CODEC(ZSTD(6))
```
produces 170× on NGINX logs because of three stacked effects:
DoubleDelta on temporal columns, `LowCardinality` (= dictionary-encoded string
columns) on tags, and ZSTD on everything. This is the exact stack we're
proposing for WELF, just expressed in Arrow types instead of ClickHouse types.

### 3.5 LogReduce (Sumo Logic) — fuzzy-cluster signatures

Operational technique, not a format. Groups messages into signature clusters
by edit distance. Useful as a **query-time** UX ("show me the 20 patterns
behind these 2 M lines") but not a storage-time compressor.

**Decision:** Not in scope for v1. A later `welf-cli reduce` command could
use a Drain-style clusterer over the decoded columns.

### 3.6 BtrBlocks (TUM, 2023)

Columnar compression designed for data lakes; outperforms zstd block-mode
by up to 2 orders of magnitude in lookup speed while matching compression
ratio. Uses cascaded lightweight encodings (FOR, RLE, dict) with small
per-block decoders. This is the technique set that a future "WELF v2" would
adopt if we outgrow zstd-dict. Worth watching.

---

## 4. Recommendation

**Adopt Candidate J (WELF) as the substrate log format, with a two-tier
physical layout.**

**Hot tier (live):** WELF shards, 60–120 s rotation, ~32–128 MB each.
Written by the kernel daemon with `fsync`-group-commit at a 100 ms cadence.
Tail readers attach via a small IPC protocol and receive decoded Arrow
batches frame-by-frame.

**Cold tier (audit / replay):** On shard rotation, the daemon (or a side
compactor) transcodes completed WELF shards into Parquet with the
ClickHouse codec recipe. DuckDB / Polars / arrow-rs can query the result
directly. The BLAKE3 root of the original WELF shard is preserved in the
Parquet file's key-value metadata so chain-of-custody is maintained across
the transcode.

**Anchoring:** One BLAKE3 Merkle root per shard, one transaction per
rotation, on the main ExoChain. Rotation is ~every minute, so the main
chain sees log traffic at **1 tx/node/min** — governance-bus pressure is
negligible, which satisfies the user's original concern about keeping the
main bus for governance.

**Runner-up:** Candidate C (Arrow IPC streaming format) used directly,
without the WELF framing. This is WELF minus the header/footer chain and
minus the zstd-dict profile. It's already ~85 % of the win for ~50 % of
the code. If we need to ship in a week instead of two, we ship this and
promote to WELF later — the shards are forward-compatible.

**One-line reason the top choice wins for this workload:** WELF stacks
Arrow's dictionary-encoded tag columns and delta-encoded timestamps with
a trained-zstd body dictionary, hitting the 15× – 25× ratio the 90 %-repetition
profile deserves, while staying streaming-native (unlike Parquet) and
re-using arrow-rs (unlike a Gorilla-style custom format).

**Rejections, one line each:**
- Extend RVF: RVF's runtime ships LZ77, not zstd; adding log semantics is building a second format inside it.
- Parquet direct: row-group buffering fights the 1 s durability bound.
- journald: Linux-only; portability to WASM/browser substrate is a fight.
- OTLP/Loki: over-solution; makes WeftOS an observability-stack operator.
- FlatBuffers/Capnp: no file layer; we'd rebuild Arrow IPC around them.
- Gorilla custom: best ratio, but we own a novel format forever.
- JSONL + zstd-dict: excellent fallback; tag-filter is the dealbreaker at scale.

---

## 5. Open Questions for the Other Seats

- **Seat 1 (chain schema / multi-channel):** Does the ExoChain anchor tx
  have a `category=log-root` discriminant so governance tooling can filter
  it out of "real" governance traffic? The whole point of the split is that
  anchors should not mix with votes on the same channel.
- **Seat 2 (kernel lifecycle):** Where does the WELF writer sit — in
  `clawft-weave/src/daemon.rs` or a new `clawft-welf` crate? I'd argue a
  new crate, so the format can be used by non-weave processes (build tools,
  kernel probes) without pulling the full substrate.
- **Seat 4 (governance-gate):** The dict-id rotation frame is a privileged
  operation (changes how all subsequent bytes decode). Does dict-rotation
  need a governance signature, or is it a local admin operation? My
  default is local-admin; a compromise is "local-admin, but the new dict
  hash is pinned into the next anchor tx."

---

## 6. References

### Format and storage documentation
- [systemd Journal File Format](https://systemd.io/JOURNAL_FILE_FORMAT/) — authoritative spec for journald binary layout.
- [systemd Journal Export Formats](https://systemd.io/JOURNAL_EXPORT_FORMATS/) — field serialisation details.
- [Apache Arrow — Streaming, Serialization, and IPC](https://arrow.apache.org/docs/python/ipc.html) — IPC streaming vs. file format.
- [Apache Arrow C++ IPC](https://arrow.apache.org/docs/cpp/ipc.html) — record-batch framing, dictionary messages.
- [Grafana Loki — Storage Architecture](https://grafana.com/docs/loki/latest/configure/storage/) — chunk + TSDB index model.
- [Grafana Loki — Single Store TSDB](https://grafana.com/docs/loki/latest/operations/storage/tsdb/) — the recommended index.
- [Grafana Labs — How Loki Reduces Log Storage](https://grafana.com/blog/2020/02/19/how-loki-reduces-log-storage/) — compression discussion.

### Compression techniques
- [Zstandard homepage](http://facebook.github.io/zstd/) — dictionary training, ratios.
- [facebook/zstd on GitHub](https://github.com/facebook/zstd) — reference implementation.
- [Gregory Szorc — Better Compression with Zstandard](https://gregoryszorc.com/blog/2017/03/07/better-compression-with-zstandard/) — practical dict-training benchmarks (Firefox omni.ja).
- [Engineering at Meta — Smaller and faster data compression with Zstandard](https://engineering.fb.com/2016/08/31/core-infra/smaller-and-faster-data-compression-with-zstandard/) — production deployment discussion.
- [ClickHouse — Optimizing with Schemas and Codecs](https://clickhouse.com/blog/optimize-clickhouse-codecs-compression-schema) — DoubleDelta + ZSTD + LowCardinality recipe.
- [ClickHouse — Compressing nginx logs 170x with column storage](https://clickhouse.com/blog/log-compression-170x) — the 170× benchmark referenced above.
- [Altinity — New Encodings to Improve ClickHouse Efficiency](https://altinity.com/blog/2019-7-new-encodings-to-improve-clickhouse) — Delta/DoubleDelta explained with physics analogy.

### Papers
- He, Zhu, Zheng, Lyu — [Drain: An Online Log Parsing Approach with Fixed Depth Tree (ICWS 2017)](https://jiemingzhu.github.io/pub/pjhe_icws2017.pdf) — the canonical online template extractor.
- Pelkonen et al. — [Gorilla: A Fast, Scalable, In-Memory Time Series Database (VLDB 2015)](https://www.vldb.org/pvldb/vol8/p1816-teller.pdf) — delta-of-delta timestamps, XOR-delta values, 12× ratio.
- Kuschewski et al. — [BtrBlocks: Efficient Columnar Compression for Data Lakes (SIGMOD 2023)](https://www.cs.cit.tum.de/fileadmin/w00cfj/dis/papers/btrblocks.pdf) — cascaded lightweight encodings, zstd-class ratios at 10× – 100× faster lookup.
- Zhu et al. — [Tools and Benchmarks for Automated Log Parsing (ICSE-SEIP 2019)](https://arxiv.org/pdf/1811.03509) — comparative benchmark across 13 parsers including Drain.
- Ouyang et al. — [High-Ratio Compression for Machine-Generated Data (arXiv 2023)](https://arxiv.org/pdf/2311.13947) — survey of modern log-specific compression schemes.

### Streaming / Parquet caveats
- Cloudsqale — [How Parquet Files are Written](https://cloudsqale.com/2020/05/29/how-parquet-files-are-written-row-groups-pages-required-memory-and-flush-operations/) — row-group flush mechanics.
- Estuary — [Memory Efficient Data Streaming To Parquet Files](https://estuary.dev/blog/memory-efficient-streaming-parquet/) — the two-pass workaround for row-group buffering.

### Binary record formats (for completeness)
- Kenton Varda — [Cap'n Proto, FlatBuffers, and SBE](https://capnproto.org/news/2014-06-17-capnproto-flatbuffers-sbe.html) — design comparison, packing discussion.
- [FlatBuffers Benchmarks](https://flatbuffers.dev/benchmarks/) — size/speed numbers.

### Operational tools
- Sumo Logic — [LogReduce documentation](https://www.sumologic.com/help/docs/search/behavior-insights/logreduce/) — fuzzy signature clustering for query-time reduction.

### Local sources inspected
- `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rvf-runtime-0.2.0/src/compress.rs` — the hand-rolled LZ77 shipped as "SCF-1".
- `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rvf-runtime-0.2.0/src/write_path.rs` — two-fsync protocol, segment framing, all writes currently emit `compression: 0`.
- `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rvf-types-0.2.0/src/compression.rs` — `CompressionAlgo` enum (zstd slot reserved, not wired).
- `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rvf-types-0.2.0/src/segment_type.rs` — available discriminant gaps for a new `Log` segment type.
