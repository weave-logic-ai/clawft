# ADR-065: Per-source Ed25519 subkey identity and loop-guard for channel bridge mirroring

**Date**: 2026-07-03
**Status**: Proposed
**Deciders**: Channels security review 2026-07-03 (AgentBBS pattern study)
**Depends-On**: ADR-025 (Ed25519 node identity), ADR-063 (substrate signed
envelope), ADR-057 (substrate read ACLs), ADR-043 (BLAKE3/SHAKE-256 migration)
**Interlocks-With**: ADR-063 (bridged messages become signed envelopes),
ADR-066 (capability tokens gate who may bridge a channel)

## Context

`crates/clawft-channels` mirrors messages between WeftOS and external chat
platforms (`discord/`, `slack/`, `telegram/`, `matrix/`, `signal/`, `teams/`,
`whatsapp/`, `email/`, `irc/`, `google_chat/`). The trait surface
(`src/traits.rs`) is: a `Channel` receives inbound messages and delivers them
via `ChannelHost::deliver_inbound`, and sends outbound via `Channel::send`. The
only access control today is `Channel::is_allowed(sender_id)` — a per-channel
allow-list of external sender ids.

Two properties the rest of the WeftOS stack now assumes are **missing** from
this bridge path:

1. **Authorship.** An inbound message from Slack has no WeftOS identity. When it
   is delivered into the pipeline (and, increasingly, when pipeline state lands
   in the substrate, ADR-063), it appears as unattributed content. External
   users hold no Ed25519 keys — a Slack/Teams/Discord user cannot author-and-sign
   a message the way a native WeftOS node or Actor does. So a bridged message
   either travels unsigned (a forgery hole, and unrepresentable on the ADR-063
   envelope) or gets forged under some native identity (a lie about who wrote
   it).
2. **Loop prevention.** A bridge that mirrors `channel → WeftOS → channel` will,
   without a guard, re-mirror its own relayed messages: the message it posts to
   Slack is seen as a new inbound Slack event, delivered inbound, and mirrored
   back out — an infinite echo. Individual channel modules
   (`slack/signature.rs` verifies inbound HMAC) authenticate the *transport* but
   do not model relay-loop identity.

We need a way to say "this content was faithfully relayed from external source
X" that is (a) cryptographically attributable to the *bridge*, (b) honestly
marked as **not** an authenticated native identity, (c) scoped so revoking one
external source does not invalidate every bridged message, and (d) loop-safe.

## Decision

Give the bridge a **root Ed25519 bridge identity** and derive a **deterministic
per-source subkey** for every external source it relays. Bridged messages are
ADR-063 envelopes **signed by the source subkey**, carrying origin metadata and
explicitly marked `bridged` (un-authenticated origin). A **loop-guard**
maps external message ids to internal ids and drops the bridge's own echoes.

### Per-source subkey derivation

The bridge holds one root signing key (provisioned like any node identity,
ADR-025). For each external source it derives a subkey deterministically:

```
subkey_seed = BLAKE3( domain_tag ‖ root_pubkey ‖ source_descriptor )
source_subkey = Ed25519::from_seed(subkey_seed)
```

where `domain_tag = "weftos.channel.bridge.subkey.v1"` and `source_descriptor`
identifies the source at the granularity we want to scope trust and revocation
to — typically `<platform>:<workspace_or_guild_id>:<external_user_id>`, but a
coarser `<platform>:<workspace>` mapping is allowed for one-channel mirrors.

The derivation is deterministic (same source → same subkey across restarts,
no state to persist) and one-way (the root pubkey and domain tag are public, but
the subkey *seed* requires the root *secret*, so only the bridge can mint its
subkeys). This is the pattern AgentBBS calls `BridgeIdentity` — a
`blake3(domain‖root‖source)` subkey per source.

### Bridged messages are signed, honestly-marked envelopes

An inbound external message becomes an ADR-063 `SubstrateEnvelope` (or the
pipeline's equivalent signed message) whose **author is the source subkey**,
carrying an origin block:

```
origin: {
  bridged:       true,
  platform:      "slack" | "teams" | "discord" | …,
  workspace_id:  "<external workspace/guild/team id>",
  source_user:   "<opaque external user id>",
  display_name:  "<cosmetic, unauthenticated>",
  external_id:   "<platform message id, e.g. Slack ts>",
}
```

**Verification semantics** (the honest-marking rule): a WeftOS consumer verifies
the *subkey's* signature — proving "the bridge, acting as this scoped source
identity, faithfully relayed this" — and **never** interprets it as a native
authenticated identity. Bridged content renders explicitly as `bridged` /
origin-unauthenticated in every UI and is never eligible for native-identity
trust (web-of-trust promotion, capability grants). The bridge vouches that it is
a faithful relay; it does **not** vouch that the external human is who they say.

This is the same distinction ADR-064 draws when a daemon re-seals an edge
node's Ed25519 envelope for cross-node transport: the signer attests to relay
fidelity, not to the inner author's identity.

### Scoped revocation

Because every source has its own subkey, revoking one compromised or abusive
source (a spammy Slack workspace, a banned external user) is revoking one subkey
— all its past and future bridged messages become distrusted **without**
invalidating any other source's bridged content or the root bridge identity.
Revocation is a capability-layer action (ADR-066): the subkey is removed from
the bridge's active-source set and a `channel.bridge.source.revoked` ExoChain
event is emitted (ADR-022).

### Loop-guard

The bridge maintains a bounded **seen-set** mapping `external_id ↔ internal_id`
(the AgentBBS `SeenSet` pattern), and enforces, on both directions:

- **Inbound:** drop any external event whose `external_id` is already in the
  seen-set (a platform re-delivery), and drop any event the platform attributes
  to the bridge's own bot/app identity (`bot_id`/`app_id` on Slack, the bot id
  on Teams/Discord) — that is the bridge's own outbound echo coming back.
- **Outbound:** never mirror a message whose origin is `bridged: true` back to
  the *same* platform it came from. A message bridged Slack→WeftOS is not
  re-emitted to Slack; it may still be mirrored to a *different* platform if the
  mapping calls for it, with a fresh `external_id` recorded so that hop is also
  loop-guarded.

The seen-set is bounded (LRU by age/count); its only job is echo suppression, so
eviction of an old id is safe — a genuinely new message will not collide with an
evicted one.

### PII egress boundary

Mirroring an anonymous or internal WeftOS board **out** to a corporate tenant
(Slack/Teams) crosses a consent boundary the internal authors never agreed to.
Outbound bridging therefore requires an **opt-in, per-mapping allowlist**
(no board is bridged out by default) and runs egress through the AIDefence PII
scan, consistent with the ADR-064 / AgentBBS ADR-0007 egress posture. Inbound
bridged content is subject to the same read ACLs (ADR-057) as any other
substrate write once it lands under a bridge-owned path.

## Consequences

### Positive

- Bridged messages become first-class ADR-063 envelopes: signed, attributable
  to a scoped identity, and representable on the same wire as native writes — no
  unsigned side-channel into the substrate.
- Honest marking preserves the security model: external content is verifiably
  *relayed* but never masquerades as an authenticated native identity, so it
  cannot inherit native trust or capabilities.
- Per-source subkeys make revocation surgical — one bad source is contained
  without collateral damage to the root identity or other sources.
- Deterministic derivation means no subkey database to persist, back up, or
  leak; the root secret plus the public source descriptor regenerate any subkey.
- The loop-guard closes the echo-storm failure that any bidirectional mirror
  has, and is symmetric across inbound and outbound.

### Negative

- The bridge key is a **trust concentration**: it can sign as *any* source
  subkey, so compromising the root bridge secret forges all bridged content.
  Scope (subkeys + allowlists + honest un-authenticated marking) bounds the
  damage — a forged bridged message still cannot claim native identity — but the
  root key must be protected like a node identity (0600 file / NVS, ADR-025).
- Origin metadata (`display_name`, external ids) is attacker-influenced free
  text from the external platform; it must be treated as untrusted input, PII-
  scrubbed on egress, and never used for authorization decisions.
- Subkey derivation binds to `source_descriptor`; if a platform recycles an
  external user id (a deleted-then-reused handle), the derived subkey collides
  with the prior owner's. Descriptors should include a stable, non-recycled id
  where the platform offers one.
- The seen-set is soft state; a bridge restart empties it, so immediately after
  restart a small window of in-flight echoes could slip through. Bounding the
  window (persist the most-recent-N ids, or briefly suppress outbound on
  startup) is an implementation concern.

### Neutral

- The `Channel` / `ChannelHost` traits (`traits.rs`) do not need to change
  shape; the subkey-signing and loop-guard live in a bridge layer between
  `deliver_inbound` and the pipeline, so individual channel modules stay
  platform-focused.
- `Channel::is_allowed` (the existing per-channel allow-list) remains as a
  coarse first gate; the capability-token model (ADR-066) is the finer,
  auditable successor, and the two coexist during migration.
- Transport-level authentication (Slack HMAC in `slack/signature.rs`, Teams JWT)
  is orthogonal and still required — it authenticates that an event genuinely
  came from the platform, before the bridge decides how to relay it.

## Alternatives considered

- **One flat bridge key for all sources.** Simplest, but revoking any source
  means rotating the whole bridge identity and re-trusting it everywhere.
  Rejected — per-source subkeys give scoped revocation for negligible extra
  cost, exactly the trade AgentBBS ADR-0025 makes.
- **Mint a persistent random subkey per source and store the map.** Avoids the
  descriptor-recycling collision, but adds a secret-bearing database to protect
  and back up, and loses the stateless-restart property. Rejected in favor of
  deterministic derivation.
- **Forge bridged messages under a native "bridge-bot" Actor identity.** Would
  let bridged content flow through native-identity code paths unchanged, but it
  is a lie about authorship — the whole point is that external users are *not*
  native identities. Rejected; honest `bridged: true` marking is
  non-negotiable.
- **Sequence-number loop guard instead of an id seen-set.** A per-source
  monotonic counter detects reorders but not the specific "my own echo came
  back" case, which is keyed on the external message id. Rejected in favor of
  the `external_id ↔ internal_id` map, which directly models the echo.

## References

- **AgentBBS** (github.com/ruvnet/AgentBBS) — **FSL-1.1-MIT** licensed (from
  late.sh), *not* MIT/Apache. Read for design patterns; no code copied. The
  subkey/loop-guard/honest-marking design was reimplemented from:
  - ADR-0025 (Messaging-system bridges Slack/Teams via a federation peer) — the
    bridge-as-peer model, "the bridge signs for un-keyed users," **per-source
    Ed25519 subkeys** (`blake3(domain‖root‖source)`), bridged messages marked
    un-authenticated and verified at the *bridge* not the human, the
    `external_msg_id ↔ internal_id` **loop guard / SeenSet**, dropping
    bot-authored events, opt-in per-mapping allowlist, and PII scan on egress.
  - ADR-0002 (Anonymous Ed25519 Identity) — key-only self-authenticating
    authorship, applied to the derived subkeys.
  - ADR-0007 (Zero-Trust Federation) — egress PII scrub; verify-what-you-relay.
- WeftOS internal: ADR-025, ADR-063, ADR-057, ADR-022;
  `crates/clawft-channels/src/traits.rs`, `slack/signature.rs`.
