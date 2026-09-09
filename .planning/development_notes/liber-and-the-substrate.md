# Liber and the substrate — branches feeding the roots

**Date**: 2026-09-02
**Status**: **CONCEPT DROP — nothing here is built.** Dropped from the Sansone OS engagement
(`~/Clients/ctox/sansone`) for WeftOS to research further and follow along.
**Origin**: the engagement lead (Mathew Beane), 2026-09-02, in conversation.

---

## The concept, in his words

> *"You need to have a memorememberer agent that basically can manage all of the linking and storing,
> and when something has to be memorized it is handed to the memorememberer and it goes to the right
> spot, places it and updates any indexes. Perhaps we should call it **liber**, and it will be the
> **bark in our knowledge tree**."*

And then, the part this note exists for:

> *"The bark brings the water and nutrients up into the branches… It is interesting that we should
> consider the opposite direction of the flow and look at where the branches should feed the roots.
> This is the substrate that weftos is intended to be, so that these trees are planted somewhere."*

## ⚠ The botany makes his second thought the *literal* one, not the metaphorical one

Two tissues, two directions, and the distinction is the whole idea:

- **Xylem** (the wood) carries water and minerals **UP** — roots to leaves.
- **Phloem** (the inner bark — *liber*) carries sugars **DOWN** — from the leaves, where they are
  made, to the roots and everything else that cannot make its own.

**So `liber` is already the downward-feeding tissue.** The "opposite direction" he reached for
second is not an inversion of the metaphor — **it is what liber actually does.** The name he picked
carries the meaning before anyone argued for it. (*liber* = inner bark → the material written on →
"book" → *librarium* → "library". The word for knowledge is the word for bark.)

**The consequence for design, stated plainly: a memory-keeper that only files things where the
current project can find them is xylem — it moves what already exists upward into use. Liber's
distinctive job is the other direction: taking what was LEARNED at the leaves and moving it down
into something that outlives the tree.**

## ⚠ CORRECTED 2026-09-02, same day — WeftOS is a CONCEPT GARDEN, not the substrate

**This section first read "What WeftOS is being asked to be — the substrate the trees are planted
in." That was the note-writer's framing and the engagement lead rejected it hours later:**

> *"I am not suggesting we implement WeftOS, it is really more of a **concept garden**, that we can
> pull from at this time. Plus **it is hard to sell to the client**. But its parity with
> **ruvector/ruflo** and this project will be clear, and we can probably take some **inspiration**
> from it."*

**So: WeftOS is a place that has already thought about these problems. It is not a dependency, not a
runtime, and nothing in the Sansone engagement will run on it.** ⚠ **"Hard to sell to the client" is
the constraint with teeth** — anything client-facing that depends on WeftOS is a component the client
did not buy and cannot evaluate. **Whatever reaches that deliverable must stand on ruvector and ruflo,
which are already in that stack.**

**The useful artifact is therefore a PARITY MAPPING, not an integration plan:** WeftOS concept →
the ruvector/ruflo primitive that does the same job → what the engagement actually needs. **A concept
with no ruvector/ruflo equivalent is inspiration only, and should be labelled as such rather than
recommended.**

**The downward-flow QUESTION below stays real and unanswered — where a lesson goes when an engagement
ends. What changed is that WeftOS is not the answer to it.**

## What is trapped in one tree today

One engagement is one tree. Its lessons are currently trapped in it:

- ~100 session-memory files, ~524K, scoped to one project path
- a `.brain/` vector corpus of 2,588 records, gitignored, engagement-internal
- three separate AgentDB-shaped `memory.db` files nobody deliberately created
- a client-owned in-product memory table holding **17 demo/seed rows, last written 2026-07-11**

**Every one of those dies with the engagement.** The next client starts from an empty tree, and the
only thing that crosses is whatever a human remembered to carry.

**The question for WeftOS: what does a lesson look like when it is worth moving down into the
substrate, and what does the substrate owe a tree planted in it later?**

## ⚠ The hard part, and it is not storage

Storage is solved several times over. **The unsolved problems are:**

1. **What generalises.** Most of what a project learns is about that project. A lesson like *"a
   `state:` tag on this board is reliably stale"* is local. *"Derived metadata beats primary text in
   a careful reader's head, and that is a defect class"* is not. **Nothing currently separates them,
   and the second kind is the only kind worth carrying down.**
2. **Confidentiality is directional too.** The Sansone corpus is client-owned or engagement-internal
   and **must not** cross into a shared substrate. ⚠ **A lesson's SHAPE may generalise while its
   EVIDENCE cannot.** Any downward flow needs a stripping step that is auditable, and "we removed the
   client names" is not auditable.
3. **Provenance survives or the lesson is worthless.** A claim in the substrate with no traceable
   origin is folklore. RVF's witness-chain and AgentDB's explainable recall are both relevant here —
   **that is the property to preserve across the boundary, not the bytes.**
4. **Recall is the actual constraint, not capacity.** The originating complaint was *"it's hard to
   remember which memory to look at."* A substrate that accumulates well and surfaces badly has
   moved the problem down a level rather than solving it.

## Existing WeftOS material this should be read against — do not start from scratch

| path | lines | note |
|---|---|---|
| `.planning/development_notes/knowledge-graph-paper-survey.md` | 371 | papers mapped to graphify/causal/HNSW modules, **dated 2026-04-04** |
| `.planning/development_notes/knowledge-graph-paper-survey-phase2.md` | 526 | 7 arxiv papers (2604.x), **2026-04-04** |
| `.planning/weftos.weavelogic.ai/09-assessment-knowledge-model.md` | 303 | |
| `.planning/weftos.weavelogic.ai/07-rvf-knowledge-base-plan.md` | 311 | |
| `.planning/development_notes/08-memory-workspace/` | — | `h1-workspace`, `h2-vector-memory`, `h3-timestamps` |

⚠ **All five months old. Re-verify before relying on any of it** — the RuvNet stack ships fast and
these predate the current AgentDB/RVF division of roles.

## The division of roles, grounded rather than assumed

From the RuvNet corpus (`agentdb/capability-cards.md#agentdb`), verbatim:

> *"RVF is the binary vector/knowledge container and HNSW index; **AgentDB stores structured
> operational records, agent state, decisions, and memory**."*

**Session-memory files are structured records with typed metadata and `[[wikilinks]]` — AgentDB's
shape, not RVF's.** WeftOS already carries an `agentdb.rvf` at its root, so both halves are present.

## Linkage back to the tree

The Sansone side is producing **`AGENTS-MEMORY.md`** — a decision procedure for which store answers
which question, plus a chunked memory index (one general package every agent loads, task packages
loaded with the agent) and the Liber specification. **That document is the tree's own bark.** This
note is the request that WeftOS work out what the ground underneath it should be.

**Governing constraint carried over:** Liber is **portable, the tree is per-repo** — the house
pattern for every specialist agent in that engagement (`ctoxos-002`/`003`: the process is portable,
client facts stay in the repo, never in the agent). ⚠ **One tree per repo is the current scope. A
forest is explicitly not being built — but nothing should foreclose it.**

## Open, and owned by nobody yet

- Does a lesson move down **automatically**, on a rule, or only when a human promotes it?
- What is the unit — a memory file, a triple, an embedding, a signed claim?
- **What does the substrate refuse?** A store that accepts everything is a landfill, and this
  engagement has four of those already.
