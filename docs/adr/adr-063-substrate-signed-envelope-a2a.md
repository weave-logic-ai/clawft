# ADR-063: Signed, content-addressed envelope for substrate A2A messaging

**Date**: 2026-07-03
**Status**: Proposed
**Deciders**: Substrate/mesh security review 2026-07-03 (AgentBBS pattern study)
**Depends-On**: ADR-025 (Ed25519 node identity), ADR-028 (ML-DSA dual signing),
ADR-057 (substrate per-path read ACLs), ADR-022 (ExoChain mandatory audit)
**Interlocks-With**: ADR-064 (anti-entropy reconciliation), ADR-065 (bridge
subkey identity), ADR-066 (capability tokens + human-join)

## Context

The substrate wire vocabulary today is [`StateDelta`]
(`crates/clawft-substrate/src/delta.rs`): a three-arm enum
(`Append` / `Replace` / `Remove`) carrying an absolute topic-rooted `path`
and a `serde_json::Value`. [`Substrate::apply`]
(`crates/clawft-substrate/src/snapshot.rs`) folds deltas into a flat
`BTreeMap<String, Value>` **with no authentication step**. Any code that can
hand a delta to `apply` — the daemon's own adapters today, but tomorrow any
A2A IPC peer or ESP32 edge publisher — is implicitly trusted to write any
path.

Two already-accepted decisions assume a write-side authentication that the
wire format does not yet carry:

- **ADR-057** (read ACLs) states that the write side "already has a per-path
  gate (the publish prefix `substrate/<publisher-node-id>/` is enforced by
  signature check per ADR-025 + JOURNALED-NODE-ESP32.md §3.5)." That
  enforcement is specified in the node journal but is **not expressible on
  `StateDelta`** — the delta has no author field and no signature, so the
  daemon cannot actually verify "this publish came from the node that owns
  this path prefix."
- **JOURNALED-NODE-ESP32.md §2–3** requires every ESP32 publish to (a) target
  a path under `substrate/<esp32-node-id>/` and (b) be signed by that node's
  Ed25519 key, with unsigned publishes rejected "at the substrate boundary."
  The ESP32 firmware already carries `ed25519-dalek` for chain signing
  (ADR-025), so the key material exists; the envelope to carry the signature
  does not.

The moment the substrate stops being a single-process, single-trust-domain
map — i.e. the moment ESP32 nodes publish StateDeltas over A2A IPC, or two
daemons sync substrate subtrees across a mesh link — an unauthenticated delta
is a forgery primitive: a compromised or buggy peer can `Replace`
`substrate/<other-node>/sensor/mic/summary` with fabricated data, or worse,
overwrite `substrate/<mesh-id>/acl/**` and unlock the read gate from ADR-057.

We need the substrate's unit of change to be **self-authenticating**: any
receiver, with no trusted intermediary, must be able to prove *who* authored a
delta and that it has *not been altered*, and must be able to reject a delta
whose author is not entitled to write its path.

## Decision

Introduce a **`SubstrateEnvelope`** as the authenticated unit of substrate
mutation. A bare `StateDelta` remains the in-process value type, but **every
delta that crosses a trust boundary** (A2A IPC, edge-node publish, cross-node
sync) MUST be wrapped in a signed, content-addressed envelope, and
`Substrate::apply` gains an authenticated sibling that only accepts verified
envelopes.

### Envelope shape

```
SubstrateEnvelope
  ├─ author:   NodeId | ActorId      (the Ed25519 identity claiming the write)
  ├─ delta:    StateDelta            (op + path + value, unchanged)
  ├─ seq:      u64                   (per-author monotonic; ordering/replay aid)
  ├─ ts_ms:    u64                   (author wall clock; skew-bounded on ingest)
  ├─ id:       EnvelopeId            (BLAKE3 of the canonical signing bytes)
  ├─ ed_sig:   Ed25519 signature over the canonical bytes
  └─ pq_sig:   Option<ML-DSA-65 signature>   (see "Dual signing" below)
```

### Canonical signing bytes (content addressing)

Following the content-addressing discipline we import from AgentBBS ADR-0003,
the bytes that are hashed and signed are a **fixed-field, length-prefixed**
encoding, not serialized JSON — JSON key ordering and whitespace are an
injection surface we refuse to depend on. The encoding is:

- a version tag line `weftos.substrate.env.v1\n`;
- author identity (hex), `seq`, `ts_ms`, and the delta `op` — each on its own
  newline-terminated line;
- the delta `path` **length-prefixed** (`"{byte_len}:" + path`);
- the delta `value` serialized to canonical bytes and **length-prefixed**
  likewise, so an embedded newline or delimiter in a path or value can never
  forge a field boundary.

`EnvelopeId = hex(BLAKE3(signing_bytes))`. The author signs exactly these
bytes. Verification **recomputes** the id from the content and rejects a
mismatch **before** checking the signature; then it verifies the Ed25519
signature under `author`'s public key. Both checks are re-run on every hop —
there is no "already verified upstream" shortcut.

### The path-ownership gate (this is the write ACL ADR-057 assumed)

Verification is necessary but not sufficient. After signature verification,
`apply_signed` MUST enforce **path ownership**:

1. Resolve `author` to its node-id / actor-id string form
   (`n-<hex>` per JOURNALED-NODE-ESP32.md, or `a-<hex>` for Actors).
2. A delta targeting `substrate/<owner-id>/**` is accepted only if
   `author`'s id equals `<owner-id>`.
3. Writes to the mesh-owned subtree (`substrate/<mesh-id>/acl/**`,
   `.../cluster/**`) are accepted only from the mesh bootstrap identity
   (`scope:admin`), reusing the ADR-057 rule that only the bootstrap Actor may
   write the ACL table.
4. Any other path resolves to deny-by-default; an explicit grant (capability
   token, ADR-066) is required to write outside one's own prefix.

A rejected write emits a `substrate.write.denied` ExoChain event (ADR-022),
mirroring ADR-057's `substrate.read.denied`, so write-policy violations are
forensically traceable.

### Idempotent apply (content addressing pays off twice)

Because `id` is a pure function of content, `apply_signed` is naturally
idempotent: an envelope whose `id` was already applied is a no-op. This is the
property ADR-064 relies on to make cross-node replication safe to replay — a
duplicate delivered by anti-entropy collides on id and does nothing.

### Dual signing (cross-node vs local)

Consistent with ADR-028's split for chain events:

- **Cross-node envelopes** (an envelope that crosses a node boundary during
  substrate sync, ADR-064) MUST carry **both** `ed_sig` and `pq_sig`
  (ML-DSA-65), and both MUST verify or the envelope is rejected.
- **Local / edge-publish envelopes** default to Ed25519-only. ESP32 nodes are
  performance- and memory-constrained; requiring a 2,420-byte ML-DSA signature
  on every `pcm_chunk` publish is not viable on an S3. Edge publishes are
  Ed25519-signed; if their deltas are later replicated cross-node, the
  receiving daemon re-seals them into dual-signed envelopes under its own
  identity (it vouches for faithful relay, not for the edge author's
  post-quantum resistance — see ADR-064).

### The no_std ESP32 publisher fit

ESP32 edge nodes already hold an `ed25519-dalek` `SigningKey` (ADR-025,
JOURNALED-NODE-ESP32.md §2.1). The envelope was chosen so the publisher side
is buildable under `no_std`:

- The canonical-bytes encoder is pure byte concatenation over `heapless`
  buffers — no `serde_json`, no allocation beyond a fixed scratch buffer.
- The publisher only needs Ed25519 **signing**; it never verifies. BLAKE3 has
  a `no_std` core. The `ed25519-dalek` + `blake3` pair is already in the edge
  build.
- The publisher only ever authors deltas under its own `substrate/<self>/`
  prefix, so it never needs the path-ownership table — it is structurally
  incapable of forging another node's path because the daemon rejects any
  envelope whose signed `author` does not match the path prefix.

This mirrors AgentBBS's "client holds the key, node only verifies"
split (their ADR-0016): the constrained device signs; the daemon is the
verifier and policy point.

## Consequences

### Positive

- Closes the write-forgery hole that exists the instant the substrate admits a
  second trust domain. The ACL-table-overwrite escalation against ADR-057 is
  structurally blocked.
- Makes the write gate that ADR-057 and JOURNALED-NODE-ESP32.md already assume
  *actually exist* on the wire, rather than being an unenforceable prose
  requirement.
- Content addressing gives idempotent apply for free, which is the
  precondition for safe anti-entropy replay (ADR-064).
- Edge publishers stay cheap (Ed25519-only, no_std-friendly); the expensive
  post-quantum signature is paid only where it matters (cross-node, long-lived
  audit trail), exactly as ADR-028 reasons for chain events.
- A delta is verifiable in isolation, so the A2A IPC transport and any relay
  never need to be trusted — only the author's key matters.

### Negative

- Adds a signing cost to every boundary-crossing publish and a verify +
  path-lookup cost to every accepted write. High-rate sensor streams
  (`pcm_chunk`) will feel this; the verify path must use the same path-trie
  structure ADR-057 mandates for reads, and batch verification where a peer
  delivers many envelopes at once.
- The canonical encoding is now a **wire contract**. Any change to field order
  or framing is a `v1 → v2` break that invalidates existing ids and
  signatures; the version tag makes the break explicit but migration is real
  work.
- `StateDelta` and `SubstrateEnvelope` are now two types with a "when do I need
  which" rule that is easy to get wrong. In-process adapter deltas stay bare;
  everything crossing a boundary must be wrapped. Lint/review discipline is
  required so an adapter author does not accidentally expose an `apply` (bare)
  path to a remote caller.
- Re-sealing edge Ed25519-only envelopes into dual-signed cross-node envelopes
  (see ADR-064) means the cross-node author identity is the *relaying daemon*,
  not the origin edge node. Consumers that need the original author must read
  it from the inner delta's origin metadata, not the envelope author — the
  same "bridge vouches for relay, not authorship" semantics ADR-065 uses.

### Neutral

- The envelope is a **complement** to the ADR-057 read gate, not a replacement:
  reads are gated by the ACL table, writes by signature + path ownership. A
  path can be publicly readable (`allow: public`) yet writable only by its
  owner. The two gates are independent.
- Identity-string derivation is not unified here: ADR-025 derives node ids as
  `hex(SHAKE-256(pubkey)[0..16])` while JOURNALED-NODE-ESP32.md uses
  `n-<blake3(pubkey)[0..3]>`. The envelope binds to whatever id form appears in
  the path segment and verifies the signature against the key; it does not
  require the two derivations to converge. Reconciling the two derivations is
  deferred to a dedicated identity ADR.

## Alternatives considered

- **Sign at the transport layer (per-connection), not per-delta.** A
  Noise/TLS session authenticates the *peer*, but the substrate's value is that
  a delta remains verifiable after it is stored and re-replicated by an
  untrusted relay (ADR-064). Transport auth dies at the socket; content
  signatures survive the hop. Rejected for the same reason AgentBBS re-verifies
  every message on ingest rather than trusting the relayer (their ADR-0007).
- **Sign the serialized JSON of the delta directly.** Avoids a separate
  canonical encoder but re-introduces JSON-canonicalization ambiguity (key
  order, number formatting, whitespace) as a forgery surface. Rejected in
  favor of explicit length-prefixed framing, matching AgentBBS ADR-0003.
- **Merkle-root the whole substrate and sign the root instead of per-delta.** A
  single signed root is cheap to verify but forces whole-tree snapshots and
  loses per-write attribution and per-write path-ownership enforcement.
  Rejected as the *primary* unit; a signed snapshot digest is still used for
  bootstrap in ADR-064, layered on top of per-delta envelopes.
- **Require ML-DSA on every publish for uniform post-quantum safety.** Correct
  in the abstract, but a ~2.4 KB signature per `pcm_chunk` is not affordable on
  an ESP32-S3 and would dominate the WiFi budget. Rejected in favor of the
  ADR-028 cross-node/local split.

## References

- **AgentBBS** (github.com/ruvnet/AgentBBS) — read for design patterns only.
  AgentBBS is **FSL-1.1-MIT** (Functional Source License, inherited from
  late.sh), *not* MIT or Apache. No code was copied; the patterns below were
  reimplemented from the design described in its ADRs.
  - ADR-0003 (Content-Addressed, Signed Messages) — the id = hash(canonical
    length-prefixed signing bytes) discipline and verify-recomputes-id-then-
    checks-signature order.
  - ADR-0002 (Anonymous Ed25519 Identity) — key-only, self-authenticating
    authorship.
  - ADR-0016 (Anonymous Client-Held Keys) — the constrained-client-signs /
    server-only-verifies split, applied here to the ESP32 publisher.
  - ADR-0007 (Zero-Trust Federation) — re-verify on every ingest, never trust
    the relayer.
- WeftOS internal: ADR-025, ADR-028, ADR-057, ADR-022;
  `crates/clawft-substrate/src/delta.rs`, `snapshot.rs`;
  `.planning/sensors/JOURNALED-NODE-ESP32.md`.
