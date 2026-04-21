# DESIGN-A — `/lewm-worldmodel-rs` · scroll pop-up book

**Target URL**: `https://weftos.weavelogic.ai/lewm-worldmodel-rs`
**Route**: `docs/src/app/lewm-worldmodel-rs/page.tsx` (client-rendered island, Fumadocs chrome hidden)
**Stack**: Next.js 16 · React 19 · Tailwind 4 · `motion` npm (adds ~25 KB gz)
**Posture**: dark-first, editorial, technically dense, no kitsch

Existing site vocabulary (`docs/src/app/page.tsx`, `clawft/`, `assess/`, `vowl-navigator/`) is Fumadocs-token-driven (`fd-foreground`, `fd-card`, `fd-primary`), mostly flat surfaces, rounded cards, Inter-ish via Fumadocs default. This page **extends** that vocabulary with one new theme slice (`--lewm-*`) scoped to the route — the rest of the site is unchanged. The one external dep we add is `motion` (successor to framer-motion); we do **not** add a 3D engine, not even for the hero — everything is CSS transform, mask, and SVG under `motion.dev`'s scroll primitives.

---

## 1. High-concept pitch

A **dark schematic that breathes**. The reader lands on a single luminous word — *LeWM under the DAG* — and as they scroll, the camera dollies *into* the WeftOS system diagram as if it were a folded technical drawing. Individual panels lift off the page, labels unfold, streams light up as data traces. The pop-up-book metaphor isn't whimsy — it's a way to **admit one subsystem at a time** to a reader who otherwise gets eleven ADRs at once. The aesthetic is Dieter Rams graph paper crossed with a subway map of a sensor network: cool blues and cyans on near-black, monospaced labels, one warm accent (sodium amber) reserved exclusively for the decoupling invariant and SIGReg-health warnings. Forward-looking, technically serious. Not flashy. The motion carries the argument; it doesn't decorate it.

---

## 2. Scroll storyboard (12 chapters)

Total scroll length ~ 900vh (12 chapters averaging 75vh each, some sticky). Progress ranges are fractions of total page scroll.

```
0.00 ┬─ HERO · title glyph                         [full viewport, static then dolly-in]
0.05 ┼─ TL;DR single paragraph                    [fade + typography reveal]
0.12 ┼─ THE INVERSION (decoupling invariant)      [flip-panel]
0.22 ┼─ SYSTEM PANEL · wide establishing shot     [sticky · zoom-in begins]
0.32 ┼─ PANEL ❶ SENSOR PLANE · opens              [pop-up, children stagger]
0.42 ┼─ PANEL ❷ OBSERVATION WIRE · streams light  [mask-path reveal]
0.50 ┼─ PANEL ❸+❹ CONSUMERS (split screen)        [crossfade, left/right]
0.58 ┼─ PANEL ❺ TRAINING · two-surface diagram    [layer shuffle]
0.66 ┼─ PANEL ❻ EXOCHAIN · undercurrent           [parallax, bottom-anchored]
0.72 ┼─ TRANSITION · pull camera back → diagram ②  [scale-out + rotate-z 3°]
0.80 ┼─ H-O-E-A CYCLE · animated loop              [sticky loop, replaces scrub]
0.88 ┼─ ADR TIMELINE · 048 → 058                   [horizontal scroll-jack]
0.94 ┼─ CTA · read-the-synthesis · closer          [settle]
```

### 2.1 Hero (0.00–0.05)

Full viewport. Near-black. Centered: `LeWM under the DAG` set in a large humanist sans, subtitle in mono: `JEPA as a sub-layer of CMVG · ADR-048 → ADR-058`. A single faint 192-dot grid pulses behind (the 192-dim latent space). Scroll indicator: a thin vertical rule at right that *is* the scroll scrubber the rest of the page uses.

**Motion**: `useScroll({ offset: ['start start', 'end start'] })` on a sticky hero; `useTransform` maps progress to `scale` (1.0 → 1.15) and `filter: blur()` (0 → 6px) as the reader scrolls past, so the hero recedes *into* the page rather than scrolling away.

### 2.2 TL;DR paragraph (0.05–0.12)

A single 60-word paragraph from `synthesis.md` §TL;DR, rendered large (text-2xl), letters revealing via a CSS `mask-image` gradient driven by `useTransform`. A pull-quote in mono hangs in the left margin: `The right move is not to replace ECC with LeWM — it is to put LeWM *under* ECC as a subsymbolic sensor substrate.` Reveal finishes at 0.09; the last three vh sit the block still so it can be read.

### 2.3 The Inversion (0.12–0.22) — **novel moment #1**

A two-state visual. State A: a lattice with observers attached (arrows pointing inward). State B: a sensor pipeline with a world-model orbiting it as one of many subscribers. A headline above: `we inverted it`. The diagram physically flips via a 3D `rotateY` driven by scroll progress, labels swapping at 50%. This is the decoupling invariant, stated visually before it's stated textually. Below the flip: the five formal rules of ADR-058 pop in sequentially with staggered children.

**Motion**: sticky container height 100vh, `useTransform` on `rotateY: [0, 180]` clamped with spring smoothing. `prefers-reduced-motion` replaces the flip with a simple crossfade plus an explanatory caption.

### 2.4 System panel establishing shot (0.22–0.32)

The ASCII-reference diagram, rendered not as ASCII but as a clean schematic SVG — six labeled bays (❶–❻) laid out in the same topology as `diagram.md` view ①. At entry, the full diagram fits the viewport. The camera sits still for ~2vh (user sees it as a single object), then starts zooming. The reader feels: *there's a whole system here; we're about to walk through it.*

**Motion**: sticky-parent technique. Wrapper is `h-[700vh]`, inner is `sticky top-0 h-screen`. `useScroll` on the wrapper maps to a `scale` transform on the inner SVG: 1.0 at 0.22 → 3.5 at 0.70. A translateX component tracks which bay is centered (❶ at 0.32, ❷ at 0.42, etc.) so the zoom feels like panning-plus-zooming. The SVG is a single DOM subtree — nothing unmounts — so grain and line quality stay consistent.

### 2.5–2.9 Panel pop-ups (0.32–0.66)

For each of five bays (SENSOR PLANE, OBSERVATION WIRE, LOCAL CONSUMERS + WORLDMODELSERVICE split-view, TRAINING LAYER, EXOCHAIN), the shared sticky frame translates/zooms to center that bay, then the bay's internal contents **unfold**: arrows trace on, labels appear, inner sub-boxes slide up a few px on a small `z` offset to mimic a popped page. Each bay stays sticky for ~75vh so the reader can absorb without feeling rushed.

**Per-bay contents**:

- **❶ Sensor Plane** — three-step pipeline (Collect → Aggregate → Encode). Each step has a tiny animated icon (transmit-gate: pulse dropouts, aggregate: three dots merging, encode: 192 points settling into a Gaussian bell curve).
- **❷ Observation Wire** — three topic streams light up in sequence (`encoded` always-on solid, `consensus` dashed/conditional, `control` bidirectional). Ed25519 signature glyph pulses at each frame.
- **❸+❹ Consumers split** — left half Local ECC (authoritative), right half WorldModelService (optional). The word "optional" appears larger than the word "WorldModelService" deliberately.
- **❺ Training** — two-surface card, edge-intelligence above, streaming-merge below, the four-condition AND gate rendered as an actual AND gate symbol.
- **❻ ExoChain** — a horizontal attestation rail that runs the full width, understated, anchored to the bottom third.

**Motion mechanics for each**: `motion.svg` child elements with `whileInView` stagger, `useTransform` for arrow-path `strokeDashoffset` from `pathLength` to 0 (line-draw). SIGReg-health meters are `useSpring`-smoothed bars that settle at a target value.

### 2.10 Pull-back transition (0.66–0.72)

Camera scales back to 1.0, the system diagram dims to 30% opacity, and with a light `rotateZ(3deg)` and `y: 40px`, the whole plane tucks under a new surface sliding in from below: diagram ②, the H-O-E-A cycle. The effect: *we've been shown the architecture; now we'll watch it think.*

### 2.11 H-O-E-A cycle (0.72–0.88) — **novel moment #2**

Once diagram ② is in frame, **scroll stops driving camera motion** — instead, scroll position becomes **cycle speed**. A closed loop diagram (HYPOTHESIZE → OBSERVE → EVALUATE → ADJUST → back to HYPOTHESIZE) runs as a looping SVG-path animation. Scroll *faster* and the loop accelerates; stop scrolling and the loop settles at 1 tick/sec. On the right, three live-updating timescale badges tick: `1 ms — DEMOCRITUS`, `10 Hz — planner`, `cont. — streaming merge`. This is the only chapter where the scroll ↔ time mapping inverts; it communicates that the world model trains *continuously*, never in batches.

**Motion**: `useMotionValue` + `useVelocity` from scroll, clamped and smoothed into the loop's `dur`. When scroll velocity is 0 for >500ms, a target baseline speed engages via a spring. Sidebar callouts swap on loop tick, showing the VoE-surprise and Δλ-Fiedler numbers from ADR-048/049.

### 2.12 ADR timeline (0.88–0.94) — **novel moment #3**

A horizontal scroll-jacked ribbon: eleven ADR cards (048 through 058) laid edge-to-edge, the ribbon moving right-to-left as the reader scrolls down. Each card is a flat schematic panel with ADR number, one-line claim, and a connecting wire to the adjacent card indicating dependency (an arrow from 048 → 050, 050 → 053, 058 → everything). At the right edge, the ribbon slows; 058 arrives last and **glows sodium-amber for 600 ms** — it's the constitutional invariant. Title above the ribbon: `eleven decisions, one spine`.

### 2.13 Closer (0.94–1.00)

Single full-viewport frame. Centered: a one-sentence summary: *The sensor network is a self-sufficient three-step learned pipeline whose emissions live on a SIGReg manifold so any consumer, including an optional world model, can subscribe uniformly.* Three CTAs: `Read the full synthesis` · `Inspect the ADRs (048–058)` · `Browse the workspace crates`. Footer: version + commit + date, in mono, small.

---

## 3. Visual system

### Palette (dark-first, all values are dark-mode; light-mode is an auto-inverted fallback)

```
--lewm-bg        #07090C   background (near-black, 2% warm)
--lewm-surface   #0D1117   panel surface (Fumadocs-adjacent)
--lewm-grid      #1A2230   schematic grid lines (2% opacity glow)
--lewm-ink       #E6EDF3   primary ink
--lewm-mute      #8B98A8   secondary ink / mono labels
--lewm-line      #3A4A60   default schematic line
--lewm-cyan      #6DD3FF   primary data / sensor plane
--lewm-mint      #7AE0C3   local ECC / authoritative
--lewm-violet    #B18CFF   world model / optional consumer
--lewm-amber     #FFB65A   DECOUPLING INVARIANT + sigreg_health warnings (reserved)
--lewm-white     #FAFBFF   SIGReg manifold highlight
```

The palette is budgeted: three cool hues for the three conceptual layers (sensor, ECC, world model) and one warm reserved for the constitutional invariant. Restraint is the aesthetic.

### Typography

- **Display & body**: `Inter` (already in Fumadocs chain) at weights 400/500/700.
- **Mono**: `JetBrains Mono` — added locally via `next/font/google`, scoped via `--font-mono-lewm`. Used for every label inside a diagram, every ADR code, every numeric figure, every quote from source material.
- **Humanist-sans for editorial copy**: `Fraunces` (one weight, 450) for the TL;DR pull-quote and the closer sentence — gives pages-of-a-book feel without being precious. No other serif on the page.

### Iconography

Custom, all built as inline SVG paths. Line weight 1.25px. Vocabulary: circle-with-inscribed-square (sensor), hexagon (node), double-arrow (stream), AND-gate (training gate), ring-plus-dot (latent), lightning-slash-circle (ServiceUnavailable), fingerprint (Ed25519). No Lucide/Heroicons. Every icon is in an `<svg>` sprite imported once — cheap, crisp at any zoom, themable via `currentColor`.

### Panel "look"

Not glassy. Not neon. **Blueprint-schematic**: 1px solid lines in `--lewm-line`, interior fills in `--lewm-surface` at 95% opacity over a faint isometric grid. Each panel has a thin number badge (❶, ❷) in mono, a caption rail at bottom with ADR references, and a subtle 1px inner glow on its primary colour when "active" (the bay the camera is centered on). Corners are 4px radius — sharp, not soft. Shadow is a single 30px soft inner shadow top-left so panels read as *lit from above-left by a drafting lamp*.

---

## 4. Pop-up-book mechanics

The illusion rests on three conventions held consistently.

**Z-layer discipline**. The page has exactly four z-planes:
- `z-0` grid background (never moves relative to camera)
- `z-10` system-diagram base plane (zooms/pans with scroll)
- `z-20` pop-up layer (children of the active bay; `y: 8px`, drop-shadow, `scale: 1.02`)
- `z-30` editorial overlays (TL;DR text, chapter labels — always legible)

A panel "opens" by promoting its inner contents from z-10 to z-20 with a 600ms spring: `y: 16 → 0`, `opacity: 0 → 1`, `scale: 0.97 → 1.00`, staggered across children by 40ms. Closing reverses with slightly faster tweening so the reader feels the page settle.

**Sticky containers**. The two big sticky sections are the system panel (0.22–0.72) and the H-O-E-A cycle (0.72–0.88). Both use the pattern:

```
<section className="relative h-[500vh]">
  <div className="sticky top-0 h-screen">... sticky content ...</div>
</section>
```

Scroll-progress within the section drives all camera transforms — nothing is `position: absolute` outside the sticky frame.

**Cross-diagram blend**. The transition between diagram ① and diagram ② (0.66–0.72) is **not** a crossfade. Diagram ① scales to 0.85, rotates 3° right, drops to 40% opacity; diagram ② rises from the bottom with `y: 60 → 0` under it. The two coexist for 3vh; then ① unmounts via IntersectionObserver. This costs a few DOM nodes for a moment but preserves the pop-up-book mental model: you don't dissolve pages, you turn them.

---

## 5. Hero and closer

**Hero (first viewport, before any scroll)**:

```
┌──────────────────────────────────────────────┐
│   weftos.weavelogic.ai         [docs] [gh]   │   ← minimal top chrome
│                                              │
│                                              │
│          L e W M   under the DAG             │   ← display, ~72px
│                                              │
│     JEPA as a sub-layer of CMVG              │   ← mono, mute
│     ADR-048 → ADR-058 · weaver A1 amended    │
│                                              │
│                                              │
│           ·  ·  ·  ·  ·  ·  ·  ·  ·          │   ← 192 faint dots
│           ·  ·  ·  ·  ·  ·  ·  ·  ·          │      (breath-pulse)
│                                              │
│                          scroll to descend ↓ │
└──────────────────────────────────────────────┘
```

The 192 dots (16×12) breathe on a 4-second cycle — opacity 0.2 ↔ 0.35, eased. They foreshadow the 192-dim latent manifold. A single line of metadata at the top-right: `v0.7.0 · feature/lewm-worldmodel · 11 ADRs`.

**Closer (last viewport)**:

The one-sentence summary in Fraunces 450, centered, limited to 68ch width. Below: the three CTA buttons in the existing site's button style (Fumadocs tokens — so this page lands you back into the normal site aesthetic). Under the CTAs, a low-contrast line: `ADR-058 — the constitutional invariant — is what makes every other decision on this page consistent.` Then the footer. Reader should leave with: *this is a shipped, decided, documented architecture, not a pitch.*

---

## 6. What to add around the diagram (supplemental sections)

The core spine is 12 chapters. Four supplemental insertions enrich without breaking the scroll:

1. **Decoupling invariant flip-card** (already chapter 2.3 — this is the most critical supplement, promoted to a spine chapter because ADR-058 *must* appear prominently).
2. **Performance target table** — inserted at 0.48 as a side-panel during the consumers split: four rows (Jetson Orin NX, AVX-512 laptop, Pi 5, Browser WASM), three columns (10Hz replan? latency, notes). Taken verbatim from `synthesis.md` §4. Renders as a quiet table — no motion — because some things deserve to just sit still.
3. **"What changes for robotics"** — inserted at 0.70 during the pull-back transition: a three-line editorial beat addressing the Sesame/DEMOCRITUS story. Single sentence each: *Sesame gets perception. DEMOCRITUS gains a learned planner. The 1ms tick stays inviolate.*
4. **TL;DR pull-quote callout** — already chapter 2.2, treated as a supplement because the language is lifted directly from `synthesis.md`. Attribution: mono subscript `— synthesis.md, Round 1`.

A fifth candidate — a **sigreg_health live-meter** — is worth prototyping: on the ❷ Observation Wire bay, show a Welford-style running mean settling to `N(0,1)` over 40 frames. It would be the only genuinely animated number on the page, and it would make the SIGReg contract viscerally concrete. Flagged as stretch.

---

## 7. Accessibility and reduced motion

The page has a hard branch on `prefers-reduced-motion: reduce`:

- **All scroll-driven transforms become snap sections**. Each chapter becomes a discrete viewport slide with `scroll-snap-type: y mandatory`. The diagrams are shown in their final, fully-revealed state rather than animating into existence. Zoom transitions become instant layout swaps.
- **The 3D flip (chapter 2.3) becomes a side-by-side comparison** with an explicit caption: *"Before: observers attached to a lattice. After: a self-sufficient sensor pipeline with the world model as one subscriber."*
- **The H-O-E-A loop (chapter 2.11) becomes a static four-state diagram** with arrows and a text explanation of the continuous-training claim.
- **Keyboard**: every chapter has an `id` and the vertical rule on the right side includes 12 clickable anchors, keyboard-focusable with Tab, producing smooth (or instant, under reduced-motion) scroll-into-view.
- **Screen reader narrative**: each chapter has a hidden `<h2>` and an `aria-describedby` section summary. Diagrams have `role="img"` with detailed `aria-label` copied from the ASCII source. SVG lines themselves are `aria-hidden`. We essentially ship two documents — the visual scroll page, and an SR-readable outline — from one source tree.
- **Text contrast**: all body copy targets WCAG AA on the dark surface (`--lewm-ink` on `--lewm-bg` is 14.7:1). Mono labels on coloured surfaces are validated per-combo; the amber-on-near-black (`--lewm-amber` on `--lewm-bg`) is 11.4:1.
- **Motion never replaces information**. Every chapter reads correctly with motion off; the animation is argumentative colour, not carrier.

---

## 8. Novel moments (why someone shares this page)

1. **The inversion flip (chapter 2.3)**. The decoupling invariant is the most important conceptual claim in the whole document, and we state it with a physical flip before we state it with words. Readers screenshot the A↔B snapshot pair.
2. **Scroll becomes cycle-speed (chapter 2.11)**. In a page of forward-scroll animations, one chapter *changes the rule*: your scroll position now means *how fast the mind thinks*. The conceptual payload (training happens continuously, not in batches) lands viscerally. This is the moment a technical reader messages a friend: *"look what they did with motion.dev scroll"*.
3. **The sodium-amber ADR-058 glow (chapter 2.12)**. Ten ADR cards in cool tones scroll past; one warm-amber card arrives last and stays lit. You walk away remembering which decision the other ten cite.

A fourth, ambitious candidate: **depth-of-field on hover over any bay** — hover or focus on a panel causes the other five to drop to 40% opacity and blur 2px, like a drafter's finger keeping their place on a drawing. Cheap, cheap to implement, high payoff. Ship it.

---

## Implementation notes (sized, not specified)

- One new route; one new theme slice (`.lewm-scope { ... }` scoped in CSS); one new component library (`components/lewm/`) with ~12 leaf components (Hero, Invert, SystemBay, StreamWire, ConsumerSplit, HoeaLoop, AdrCard, AdrRibbon, PerfTable, Closer, ScrollRail, LatentDots). Each under 200 LOC.
- `motion` dep is the only new npm dep. No Three.js, no lottie, no GSAP.
- SVG sprite lives in `public/lewm/sprite.svg`; all icons referenced by `<use>`.
- Total bundle budget for the route: ≤80 KB gz JS + ≤40 KB SVG. No images; everything vector.
- First paint target: ≤1.2s on 3G-Fast; LCP ≤2.5s on 4G.

The page is the argument. The motion is the arrangement. The invariant is the spine.
