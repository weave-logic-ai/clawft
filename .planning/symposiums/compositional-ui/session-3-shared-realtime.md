# Session 3 — Shared Real-time Surfaces (RQ3)

**Scope**: the ancestors of Mission Console, where many participants
see the same view and every action propagates in real time. The extra
twist is that in our world the **surface itself is being composed by
an agent at runtime**, which invalidates some assumptions these prior
systems relied on. This document teases apart which patterns survive
that change and which do not.

Read against Evans (DDD): every shared surface is a **bounded context
with multiple participants**, and the hardest problems — presence,
conflict, audit, permission — are really questions about *who is
allowed to mutate which aggregate under which ubiquitous language*
(Evans, Ch. 14, *Maintaining Model Integrity*). Mission Console is a
context-map surface in the DDD sense: it shows an aggregate boundary
and all the roles legitimately inside it.

---

## Teardowns

### 1. Figma multiplayer

1. **State-sync**: custom server-authoritative **multiplayer tree**
   (not OT, not off-the-shelf CRDT). Each document node has a
   server-assigned id; clients send operations against ids; the
   server is the canonical timeline and rebroadcasts reconciled
   state. Last-writer-wins on property conflicts, with tree-move
   semantics that avoid "orphan" children. This is documented in
   Figma's own engineering post "How Figma's multiplayer technology
   works" — closer to a custom CRDT tuned for tree mutation than to
   Google-style OT.
2. **Presence**: per-session avatar, live cursor, current
   selection, current tool. Presence is ephemeral state on the
   server, not persisted to the document.
3. **Conflict**: property-level LWW; structural (reparenting)
   conflicts resolved by giving precedence to the earliest
   server-serialised op. Deleted nodes are tombstoned so late writes
   are dropped cleanly.
4. **Audit**: version history is separate from the op-log; it's
   a snapshot-and-comment model, not a replay of every keystroke.
5. **Agents as participants**: Figma's plugin API is single-user
   scoped. The multiplayer primitive assumes humans, which shows up
   in cursor rendering and selection feedback, but the underlying
   op-log is perfectly serviceable for non-human actors — they
   would just have unusual presence signatures.
6. **Bounded contexts**: per-file permissions (view/comment/edit)
   are coarse; team-level contexts live a layer up. No expression of
   *which verbs* a participant may use — it's view vs edit.

### 2. Liveblocks

1. **State-sync**: a hosted Yjs + their own "Storage" layer. CRDT
   by default; rooms are the unit of replication. Opinionated about
   "presence vs storage": presence is ephemeral, storage is
   persistent, and the wire protocol separates them.
2. **Presence**: first-class — each client has a typed `presence`
   object (cursor, selection, idle/active, focused-id) that the
   server broadcasts without storing. They ship React hooks
   (`useOthers`, `useMyPresence`) that assume presence is a
   rendering input.
3. **Conflict**: CRDT-merge (Yjs for text; LiveList / LiveMap /
   LiveObject for structured data). No explicit conflict UI.
4. **Audit**: room history is retained but is replay of CRDT ops,
   not a human-readable timeline. Threads/comments are a separate
   product that sits *on top of* the room to give narrative audit.
5. **Agents as participants**: nothing in the model rejects them,
   but the SDK is React-first and the presence type is a blob —
   agents would need a typed presence schema to be legible to
   humans in the same room.
6. **Bounded contexts**: rooms are the bounded context. Row-level
   auth via a token signed by your own backend. Clean separation of
   "who can join" from "what they can do once in".

### 3. Yjs / Y-Protocols

1. **State-sync**: pure CRDT — YATA-family, op-based, commutative
   merges. Every client eventually converges without a server.
   y-protocol defines the sync handshake over any transport
   (websocket, webrtc, IPFS).
2. **Presence**: `y-awareness` — a sibling protocol, explicitly
   ephemeral, keyed by client id. Not persisted in the doc.
3. **Conflict**: CRDT merge; no conflict surfaces to the user for
   most data types. Intention-preservation only holds for the
   operations the type was designed for (text insert/delete is
   fine; moving a subtree is famously not).
4. **Audit**: full op-log is replayable; `y-undo` gives per-client
   undo stacks. But op-log is not semantic — you see CRDT deltas,
   not intent.
5. **Agents as participants**: fine in principle — Yjs doesn't
   care if you're human. The awareness schema is yours to define.
6. **Bounded contexts**: a Yjs document is flat; sub-docs are
   supported but the permission model is entirely offloaded to
   whatever provider hosts the sync layer. No native ACLs.

### 4. Google Docs (OT)

1. **State-sync**: Operational Transform with a central server.
   Clients send ops, server rebroadcasts transformed ops, everyone
   converges. Classic Jupiter/OT lineage.
2. **Presence**: per-user cursor, per-user selection, per-user
   avatar in the header. Cursor colour is persistent identity.
3. **Conflict**: OT transforms ensure intention preservation for
   text insert/delete; richer operations (comments, suggestions)
   are layered on a separate queue. Conflicts are almost never
   surfaced.
4. **Audit**: "version history" is a semantic diff, not a full op
   replay. "See edits by X" is the killer feature — someone who
   was away *can* review what the room did.
5. **Agents as participants**: Google's own Smart Canvas adds AI
   actors, but they appear as suggestion-layer artifacts, not as
   peers in the OT stream. The OT model does not distinguish actor
   class.
6. **Bounded contexts**: document-scoped; comments have their own
   thread context. Sharing ACLs are Drive-level and coarse.

### 5. Plan 9 Acme

1. **State-sync**: shared *filesystem*, not shared doc. Acme
   exposes the editor as files under `/mnt/acme/`. Any program
   (local, or remote over 9P) can read `body`, write `data`, list
   windows, post events. The editor is addressable the same way a
   disk is.
2. **Presence**: none, in the Figma sense. But an *action*
   (middle-click a verb; write to `ctl`) is universally visible
   because the filesystem sees it. Presence is whatever files you
   have open.
3. **Conflict**: file semantics — the last write wins on bytes.
   Intent is expressed by composing *verbs* in the chorded menu,
   and the verb's own program handles merging if it must.
4. **Audit**: 9P event stream can be tailed — every selection,
   every click on a tag, every `Edit`/`Put` is a line. A later
   participant can replay exactly what the room did.
5. **Agents as participants**: Acme was *built* for this. A shell
   script is a first-class peer; `win` lets a program drive a
   window; `plumber` routes typed intents across agents. The
   filesystem metaphor makes human and agent fully symmetric.
6. **Bounded contexts**: 9P authentication + mount namespaces.
   A remote user mounts only the part of the namespace they are
   entitled to. Cleanest model in this list.

Acme is the non-obvious ancestor that matters most for us. **The UI
is the substrate, and the substrate is addressable by agents and
humans through exactly the same protocol.** That is the predicate we
are calling "dual-operator" in `foundations.md`.

### 6. NORAD / Apollo mission-control consoles

1. **State-sync**: telemetry broadcast — central radar/comm feeds
   replicated to every console at the speed of the display hardware.
   Server-authoritative with zero client-side mutation of the
   shared state; consoles are *viewers* plus annotation surfaces.
2. **Presence**: organisational — FLIGHT, RETRO, FIDO, GUIDO etc.
   Each operator has a named role, a known station, a known
   comm-loop. Presence is not a cursor; it is a seat.
3. **Conflict**: avoided by role separation. Only FIDO touches the
   trajectory panel. Overlapping authority is resolved by FLIGHT.
   The entire model depends on **verbs being partitioned by role
   before the session starts**.
4. **Audit**: every loop is recorded; every decision is logged on
   paper; transcripts are preserved. A participant returning after
   a break reviews the recorded loop.
5. **Agents as participants**: historically yes — auto-sequencers,
   onboard computer — and they were treated as participants *with
   a seat*, not as invisible daemons. The Apollo Guidance Computer
   was on the comm loop.
6. **Bounded contexts**: each flight is a context. Cross-flight
   data is ported manually. The strictness is the feature.

The mission-control pattern's gift to us is **witnessed action**:
every non-trivial verb is both heard on the loop and logged, *by
design*. The agent equivalent is that every `invoke` on a shared
surface enters a visible, replayable channel.

### 7. Twitch / stream overlays

1. **State-sync**: single broadcast pushed one-way; interactions
   (chat, reactions, polls) are *side-channels* that the broadcaster
   merges into the visible surface via overlay software. The shared
   "view" is the broadcast; there is no shared mutable state.
2. **Presence**: viewer count + chat participants. Cursor /
   selection presence is absent.
3. **Conflict**: none at the primitive level; the broadcaster's
   machine is the authority.
4. **Audit**: VOD + chat replay together give a faithful record.
5. **Agents as participants**: bots are normal (chat bots, overlay
   bots, clip bots) and are often better citizens than humans —
   typed commands, rate limits, structured responses.
6. **Bounded contexts**: per-channel moderation; mods are
   delegated authority; extensions sandbox third-party code.

### 8. Incident / war-room dashboards (PagerDuty, Datadog, Grafana)

1. **State-sync**: mostly server-authoritative read-only with live
   refresh. Edits to dashboards themselves are rare and LWW. The
   *data* is streaming; the *surface* is almost static.
2. **Presence**: Datadog and Grafana have recent "shared cursor"
   features (Datadog Notebooks, Grafana Scenes) but they are
   bolted on. PagerDuty has role presence (on-call).
3. **Conflict**: avoided — users who edit a dashboard concurrently
   mostly clobber each other. Annotations are append-only.
4. **Audit**: incident timelines are first-class. Every paged
   user, every acknowledge, every escalation is on record.
5. **Agents as participants**: auto-remediation bots exist and
   *do* appear on the timeline as actors. This is closer to our
   model than Figma is.
6. **Bounded contexts**: service / team / environment. RBAC is
   mature; governance is a real product feature.

### 9. Slack / Discord channels

1. **State-sync**: event-sourced, server-authoritative, eventually
   consistent ordering. Each message has a server-assigned ts;
   clients sort locally.
2. **Presence**: typing indicators, online status, read receipts.
   Voice/stage presence is a separate subsystem.
3. **Conflict**: editing a message is a new event; deletion is a
   tombstone. No structural conflicts because the data shape is
   append-only log.
4. **Audit**: channel history is the audit trail, natively. Search
   + permalinks mean late joiners can reconstruct the room.
5. **Agents as participants**: bot users are first-class; slash
   commands and app mentions give typed verbs. Discord's
   interaction model (buttons, selects, modals) is arguably the
   most mature "agent-composed surface" already in production.
6. **Bounded contexts**: workspace / channel / thread. DMs are a
   separate bounded context. Guest accounts exist for cross-tenant.

### 10. Notion / Coda live cursors

1. **State-sync**: Notion uses a custom block-tree sync; Coda is
   closer to OT over a spreadsheet model. Both are server-
   authoritative with optimistic local.
2. **Presence**: live cursors, avatars-on-block, editing indicator.
3. **Conflict**: block-level LWW in Notion; cell-level in Coda.
4. **Audit**: version history per page; block-level history is
   coarser than Google Docs character-level.
5. **Agents as participants**: Notion AI and Coda AI run as
   server-side actors that produce blocks; they appear as authored
   edits, not as peers on the awareness channel.
6. **Bounded contexts**: workspace / page / database. Permissions
   inherit down the page tree.

---

## Agent-composition stress-test

The foundational twist: **our surface is not pre-drawn**. An agent
decides, mid-session, to insert a `Gauge`, reconfigure a `Panel`
into a `Split`, promote a `Chip` to a `Form`. What breaks?

1. **Property-level LWW (Figma, Notion)** — breaks when the
   *schema* of a primitive changes under it. "Last write wins on
   the `value` field" assumes the field keeps existing.
   Composition edits delete fields. Fix: every primitive carries a
   **schema version**, and writes that name a non-existent field
   are rejected, not silently dropped.

2. **Text/tree CRDT intention-preservation (Yjs, Liveblocks)** —
   well-posed only for the operation types the CRDT knows about.
   Moving a subtree, retyping a primitive from `Chip` to `Form`,
   swapping its ontology id — these are not text-ops. A pure CRDT
   cannot merge them safely. Fix: structural edits go through a
   **server-serialised composition log**, not the CRDT layer.
   Within a primitive (a `TextField`'s value, a `List`'s items),
   CRDT merge is fine and should be used.

3. **Awareness/presence blobs (Liveblocks, Yjs)** — assume the
   presence shape is stable. If the surface morphs, "selected id"
   may refer to a primitive that no longer exists. Fix: presence
   references are **typed handles** into the current composition,
   and the server garbage-collects stale ones on every structural
   commit.

4. **Cursor rendering as pixel coordinates (Figma, Notion)** —
   breaks immediately. Pixel coordinates mean nothing when the
   layout is recomposed. Fix: presence is **ontology-addressed**
   ("user U is focused on primitive `field://order.qty` in variant
   `v7`"), and the renderer resolves that to pixels locally. This
   is the mission-control "seat" pattern, not the Figma "cursor"
   pattern.

5. **Role separation (NORAD)** — survives and becomes *more*
   important. If any agent can compose, the composition authority
   must itself be a role with a seat, not an invisible daemon.
   Weaver-the-composer is a named participant with witnessed
   actions. Humans see "Weaver added a Gauge" on the loop.

6. **File-as-UI (Acme)** — survives completely. Acme was already
   dynamic; windows come and go. The lesson is that the protocol
   must address every primitive as if it were a file/resource with
   a stable name, and every verb as if it were a `write` to that
   resource's control channel. Dynamic composition is then just a
   sequence of `mkdir` and `rm`.

7. **Event-sourced append log (Slack, Discord, mission-control)**
   — survives and should be the spine. A surface's life is an
   append log of composition events + interaction events +
   presence events, interleaved. Late-joiners replay to a
   snapshot; the audit trail is free.

8. **CRDT-for-everything (Liveblocks, pure Yjs)** — does not
   survive as the *sole* model. CRDTs are great for opaque
   payloads (freeform text, user-authored annotations) and poor
   for schema-changing composition. The right architecture is
   **event-sourced composition + embedded CRDT leaves**, not
   CRDT-all-the-way-down.

9. **Presence as cosmetic (Figma cursor)** — does not survive.
   Presence is a first-class ontology fact: who is here, what role
   they have, what primitive they are focused on, what variant of
   the surface they see (remember GEPA variants from
   `foundations.md` — two participants may see different variants
   of the "same" surface, and presence must surface that without
   making the room feel schismatic).

10. **Broadcast-only (Twitch)** — does not survive for Mission
    Console, but is the right model for *witness* mode. A mission
    room may have active operators and passive witnesses; the
    witness channel can be pure broadcast with one-way reactions.

---

## Bounded contexts and cross-tenant collaboration (Evans)

Every shared surface in the system sits inside a bounded context in
Evans's sense (*DDD*, Ch. 14). The ubiquitous language inside a
context is not guaranteed to mean anything in another. When Mission
Console spans contexts — a regional manager joining a store's
incident room, a counsel agent joining a client's project — we are
doing a **context map**, and the literature is clear that without an
explicit translation layer, models corrupt each other.

Patterns that survive from Evans and match our substrate:

- **Anti-corruption layer**: when an external participant enters,
  their verbs are translated into the host context's language
  through an explicit adapter. Our version: a `ForeignParticipant`
  wrapper analogous to `ForeignSurface`, with its own affordance
  whitelist.
- **Shared kernel**: the base primitive set (Button, Field, etc.)
  is the shared kernel — stable across contexts. Domain primitives
  (`OrderRow`, `IncidentTimeline`) are context-specific.
- **Customer/Supplier**: governance layer declares that context A
  supplies data to context B's surface; changes require
  coordination.

The operational consequence: **presence is per-context**. A single
human may be in three rooms; they have three presences with three
handles, and the system does not leak a selection from one into
another. Cross-context presence is an explicit affordance a
governance rule must grant.

---

## Recommendations for canon

1. **Spine is event-sourced, not CRDT.** A shared surface is an
   append log of `Composition`, `Interaction`, `Presence`, and
   `Confidence` events, server-serialised. CRDT is used *inside*
   leaf primitives that carry freeform payloads (text, rich notes).
   This rejects the Liveblocks/pure-Yjs "CRDT for the whole
   document" stance.

2. **Presence is ontology-addressed, never pixel-addressed.**
   Presence records `(participant-id, role, primitive-id,
   affordance-in-progress, variant-id)`. Renderers locally resolve
   to pixels. Recomposition never invalidates presence semantics.

3. **Every participant has a seat.** Humans, agents, services, and
   foreign-surface shells all present in the same list, with role
   and affordance-set. Weaver-the-composer is visible as a
   participant whose verb is `compose`. Mission-control discipline.

4. **Every verb is witnessed.** No silent mutation. A `compose`
   that adds a `Gauge` is a log entry; a `invoke` that fires a
   workflow is a log entry; a read-only subscription is a log
   entry with a weaker tier. Audit is free because it is the
   primary channel, not a bolt-on (Slack/mission-control).

5. **Composition edits and payload edits travel on different
   channels with different merge rules.** Structural edits are
   serialised by the room server (LWW on schema version; reject on
   version mismatch). Payload edits inside a leaf can be CRDT-merged.
   This is the Figma-tree lesson plus the Yjs lesson, split cleanly.

6. **Bounded context is a first-class surface concept.** Every
   room declares its context id; participants carry a context
   membership list; cross-context affordances require explicit
   translation via an `AntiCorruption` adapter primitive. Evans,
   made executable.

7. **Variants are per-participant, convergent on consent.** GEPA
   variants (see `foundations.md`) mean two participants may see
   different renderings of the same event log. The presence record
   includes `variant-id`; the room maintains a "canonical" variant
   used for screenshots and audit replay, but each participant's
   local variant can drift for learning purposes. Variants must
   never diverge on safety or governance affordances.

8. **Late-joiner replay is a first-class affordance.** A
   participant returning from a break issues `replay-since(t)` on
   the room; the server returns the compacted event stream + a
   snapshot, rendered as a reviewable "what happened while you
   were away" surface. This is Google Docs version-history +
   mission-control loop recording + Slack backlog, unified.

9. **Agent-composition is itself a role with a seat and a budget.**
   Weaver speaks on the loop. Its mutations enter a preview
   channel, and — per the governance layer — may require human
   confirmation before promotion to the canonical log. Role
   separation (NORAD) applied to agents: just because Weaver *can*
   compose doesn't mean every composition is auto-committed.

10. **Tombstones, not deletes.** Removed primitives leave a
    tombstone carrying the last schema version and a reason. Late-
    arriving writes against them fail cleanly; replay renders them
    as struck-through historical facts. This is how Figma avoids
    orphans and how we keep audit honest.

---

## Open questions

- **OT for the composition spine vs event-sourced** — we assume
  event-sourced, but OT on a typed operation alphabet (compose,
  recompose, retype) is viable and gives cleaner intention-
  preservation. Worth a micro-prototype in RQ6 (protocol design).
- **Presence granularity for voice** — is "speaking" a presence
  flag or a stream-level fact? (Bleeds into RQ7, dev-panel.)
- **Cross-room migration** — a participant moving from room A to
  room B mid-task is common (an incident escalates to war-room).
  How does their state, focus, and authored content port? This is
  the Evans context-map question with a UX answer still unclear.
- **Variant reconciliation on audit** — if two participants saw
  different variants, which variant is "the" record? Proposal: the
  canonical variant is what audit replays; variant deltas are
  recorded but subordinate. Needs confirmation.
- **Agent presence that respects human attention** — an agent
  joining a room mid-task must not steal presence salience. NORAD
  solves this with comm-loop discipline; we need a protocol-level
  equivalent, probably an `attention-priority` field on every
  presence record consumed by the renderer.
- **Offline-first vs always-connected** — pure CRDT models are
  offline-first by design; event-sourced spines are not. Our mesh
  has periods of partition; does the room degrade to a local
  event log that merges on reconnect, and if so, who is
  authoritative when logs diverge? (Connects to mesh-runtime and
  HeartbeatTracker work already in-tree.)
- **Permissions-as-affordances vs permissions-as-filters** —
  Discord's model (interaction buttons appear only if allowed) vs
  Slack's model (everyone sees the button; server rejects if
  unauthorised). We lean toward the former because the foundations
  require affordances to be enumerable per-caller, but it raises
  the bar on real-time permission recomputation when roles change
  mid-session.

---

## Files referenced

- `/claw/root/weavelogic/projects/clawft/.planning/symposiums/compositional-ui/foundations.md`
- `/claw/root/weavelogic/projects/clawft/.planning/symposiums/compositional-ui/AGENDA.md`
- `/claw/root/weavelogic/projects/clawft/.planning/development_notes/eml-causal-collapse-research.md` (cross-ref, not required reading for this session)
