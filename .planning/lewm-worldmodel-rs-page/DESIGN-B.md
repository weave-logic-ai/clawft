# DESIGN-B — "Pop-up book, but it ships Friday"

Pragmatic counter-proposal for `/lewm-worldmodel-rs`. Motion-first, motion-cheap.
Spine is `diagram.md`: ①  full-system panel diagram, then ②  the H-O-E-A cycle.
Two "spreads" plus a coda. Everything else is scaffolding.

Stack: Next.js 16 RSC, React 19, Tailwind 4, Fumadocs 16, `motion` v12. Page
shell is an RSC; motion lives in a handful of `'use client'` islands.

---

## 1. Thesis — one primitive

**`useScroll({ target })` driving `useTransform` against a progress scalar
in `[0, 1]`, composed into `scale`, `translateY`, `opacity`, and `clipPath:
inset(...)`. That's the whole vocabulary.**

No parallax layers. No Lenis. No `framer-motion-3d`. No Canvas unless the
artifact is already canvas-shaped. Every beat is a `motion.div` whose three
or four style channels are piped from one or two per-section `useScroll`
progress values. Flat mental model, bounded main-thread work, SSR'd DOM
readable before any JS loads. `transform` + `opacity` are the only
properties that composite off the main thread on every modern browser;
`clipPath` is the one exception we accept because it's the only cheap way
to get real "flaps open" feel.

---

## 2. Pop-up-book mechanic — concrete

Each "panel" is a `<section>` with a `ref` passed as `target` to `useScroll`;
progress scalar piped through `useTransform` with ease `[0.16, 1, 0.3, 1]`
(expressive-out). No `useSpring`-on-scroll — springs fight trackpad velocity.

### 2a. Sticky-zoom-in (hero → system overview)

```tsx
// HeroZoomFrame.tsx — wraps the ASCII ①  panel diagram
const ref = useRef<HTMLDivElement>(null);
const { scrollYProgress } = useScroll({
  target: ref,
  offset: ['start start', 'end start'],  // sticky for 100vh
});

const scale     = useTransform(scrollYProgress, [0.00, 0.35], [1.00, 1.60], { ease: EASE });
const translate = useTransform(scrollYProgress, [0.00, 0.35], ['0vh', '-12vh']);
const opacity   = useTransform(scrollYProgress, [0.25, 0.45], [1, 0]);

return (
  <section ref={ref} className="relative h-[180vh]">
    <div className="sticky top-0 h-screen flex items-center justify-center">
      <motion.div style={{ scale, y: translate, opacity, willChange: 'transform, opacity' }}>
        <SystemPanelSvg />  {/* diagram.md ①  as inline SVG, SSR'd */}
      </motion.div>
    </div>
  </section>
);
```

Hero zooms 1.0 → 1.6 and fades at 25–45% progress while the next panel slides
up under it. `180vh` gives ~80vh of slack to hold the zoom before release.

### 2b. Panel unfurl (diagram panel expands as its segment passes center)

Each of the six numbered panels in `diagram.md` (❶  sensor · ❷  wire · ❸
consumers · ❹  WorldModelService · ❺  training · ❻  ExoChain) gets one. The
"pop-up" is a `clipPath` inset collapsing from the edges plus scale-from-0.92.

```tsx
// PanelPopup.tsx
const { scrollYProgress } = useScroll({
  target: ref,
  offset: ['start 0.8', 'center 0.5'],  // starts entering at 80% viewport, peaks at center
});

const scale   = useTransform(scrollYProgress, [0, 1], [0.92, 1.00], { ease: EASE });
const y       = useTransform(scrollYProgress, [0, 1], [40, 0]);
const opacity = useTransform(scrollYProgress, [0, 0.4], [0, 1]);
// clipPath opens from top+bottom — the "book flaps"
const clip    = useTransform(
  scrollYProgress,
  [0, 1],
  ['inset(45% 8% 45% 8% round 12px)', 'inset(0% 0% 0% 0% round 12px)'],
);

return (
  <motion.article
    ref={ref}
    style={{ scale, y, opacity, clipPath: clip, willChange: 'transform, opacity, clip-path' }}
    className="panel"
  >
    {children}
  </motion.article>
);
```

### 2c. Cross-view dissolve (full-system → H-O-E-A cycle)

Two stacked `sticky` containers in one parent. System panel fades+shrinks;
H-O-E-A loop fades in with a counter-rotation. One scroll source drives both,
so they're locked.

```tsx
// CrossViewDissolve.tsx
const { scrollYProgress } = useScroll({
  target: ref,
  offset: ['start start', 'end end'],
});

const panelOpacity = useTransform(scrollYProgress, [0.0, 0.4, 0.6], [1, 1, 0]);
const panelScale   = useTransform(scrollYProgress, [0.4, 0.8], [1.00, 0.72], { ease: EASE });

const loopOpacity  = useTransform(scrollYProgress, [0.45, 0.75], [0, 1]);
const loopScale    = useTransform(scrollYProgress, [0.45, 0.75], [1.18, 1.00], { ease: EASE });
const loopRotate   = useTransform(scrollYProgress, [0.45, 0.85], [-6, 0]);  // degrees

return (
  <section ref={ref} className="h-[240vh] relative">
    <div className="sticky top-0 h-screen grid place-items-center">
      <motion.div style={{ opacity: panelOpacity, scale: panelScale }}>
        <SystemPanelSvg compact />
      </motion.div>
      <motion.div
        className="absolute inset-0 grid place-items-center"
        style={{ opacity: loopOpacity, scale: loopScale, rotate: loopRotate }}
      >
        <HoeaLoop />
      </motion.div>
    </div>
  </section>
);
```

240vh gives ~60vh of pure overlap where both are on-screen at low opacity —
the "page turn."

---

## 3. Performance budget

Bar: Pixel 6a / iPhone 13 throttled 4× CPU. Targets: LCP < 2.0 s, CLS = 0,
scroll INP < 120 ms, 60 fps during zoom beats.

- **`will-change` toggled, never static.** Set `willChange: 'transform,
  opacity'` only while the segment is on-screen; add/remove via
  `useMotionValueEvent(scrollYProgress, 'change')` at boundaries. Global
  `will-change` balloons layer count and kicks GPU memory.
- **Transforms only.** No `top/left/width/height/margin/padding/box-shadow`
  on scroll. `filter: blur()` banned — #1 mobile frame-drop cause. Soft
  look = baked SVG `<feGaussianBlur>` on static assets.
- **Hard cap 6 animating elements per moment.** Two sticky overlays at a
  transition = 4 animating + 2 decorative.
- **`LazyMotion` + `domAnimation` (not `domMax`)** saves ~18 KB vs full
  `motion`. No layout animations on this page, so `domAnimation` is enough.
- **Hand off to CSS** for the sticky progress bar (`animation-timeline:
  scroll()` where supported, no-op otherwise — not load-bearing).
- **Breaks on low-end mobile, so banned:** `backdrop-filter`, SVG filters
  on animating nodes, `motion.svg` with >200 paths zooming, synchronous
  layout reads in `onScroll`. None appear in this design. `useScroll` uses
  IntersectionObserver + rAF; we never add `window.addEventListener('scroll')`.

Budget: < 25 KB gzipped motion JS on first paint. Full `motion` only
lazy-loads past beat 4.

---

## 4. Scroll storyboard — 12 beats, real numbers

One `<main>`, `scroll-snap-type: none` (snap ruins zoom). Each beat is a
`<section>` whose sticky container drives local `scrollYProgress ∈ [0, 1]`.
Ease everywhere: `const EASE = [0.16, 1, 0.3, 1] as const`.

| # | Beat                          | Section height | Local progress range | Transform                                                                 |
|---|-------------------------------|----------------|----------------------|---------------------------------------------------------------------------|
| 1 | Title / eyebrow fade-in       | 100vh          | [0.00, 0.18]         | `opacity [0,1]`, `y ['16px','0']`                                         |
| 2 | Hero ASCII → SVG zoom         | 180vh          | [0.02, 0.35]         | `scale [1.00, 1.60]`, `y ['0vh','-12vh']`                                 |
| 3 | Decoupling invariant pullquote| 80vh           | [0.10, 0.55]         | `opacity [0,1,1,0]` at stops [.10,.30,.45,.55], no transform              |
| 4 | Panel ❶  Sensor plane unfurl  | 140vh          | [0.00, 0.50]         | `clipPath inset(45% 8%) → inset(0)`, `scale 0.92 → 1.00`, `y 40 → 0`      |
| 5 | Panel ❷  Observation wire     | 140vh          | [0.00, 0.50]         | same as #4; plus packet-flow CSS animation keyed off an `inView` flag     |
| 6 | Panel ❸+❹  consumers split    | 180vh          | [0.00, 0.55]         | two-column unfurl, left panel leads right by ~0.08 progress               |
| 7 | Panel ❺  Training layer       | 140vh          | [0.00, 0.50]         | unfurl + SIGReg-health gauge pulse (CSS `@keyframes`, not JS)             |
| 8 | Panel ❻  ExoChain spine       | 120vh          | [0.00, 0.40]         | unfurl; links scroll-highlight as they enter (`color` via `useTransform`) |
| 9 | Cross-view dissolve           | 240vh          | [0.00, 1.00]         | see §2c; panel fades 0.40→0.60, loop enters 0.45→0.75                     |
| 10| H-O-E-A loop animate-in       | 160vh          | [0.05, 0.55]         | `rotate [-6deg, 0]`, `scale [1.18, 1.00]`, stroke-dashoffset reveal arcs  |
| 11| Timescales triptych           | 120vh          | [0.10, 0.80]         | three cards stagger `y [30,0]` with lag 0.08                              |
| 12| ADR index / call to action    | 100vh          | [0.20, 0.60]         | `opacity [0,1]`, citation cards `y [24, 0]`                               |

**Total ≈ 17.4 screens** (~15 s of brisk-flick scroll on laptop, plenty of
reading time on mobile). If long, cut 3/7/11 (§8).

Spelled-out cases:

- **Beat 2 (zoom):** `useTransform(p, [0.02, 0.10, 0.28, 0.35], [1.00, 1.22,
  1.55, 1.60], { ease: EASE })`. Four stops so the last 5% "hangs" near
  max — hangs make zooms feel intentional, not whiplash.
- **Beat 9 (dissolve):** 0.40 / 0.45 split gives a 5% window where both
  views are ≥ 90% opacity — where the "same system" merge happens.
- **Beat 10 (loop):** H-O-E-A arcs reveal via `strokeDashoffset` —
  `useTransform(p, [0.08, 0.22], [1, 0])` for HYPOTHESIZE, `[0.15, 0.30]`
  OBSERVE, `[0.22, 0.38]` EVALUATE, `[0.30, 0.48]` ADJUST. Arrows turn on
  after each arc completes.

---

## 5. `prefers-reduced-motion`

Respect via `useReducedMotion()` at page root. One branch.

- **Degrade:** all `scale`/`y`/`rotate`/`clipPath` → `opacity [0, 1]` only
  over the same progress range.
- **Stay:** fades, color transitions, SIGReg-health gauge pulse
  (informational), H-O-E-A arc `strokeDashoffset` reveal (shows sequence).
- **Sticky stays**; `scroll-behavior: auto`. The cross-view dissolve
  becomes a hard cut at 0.50 progress.
- **Packet-flow CSS animation (beat 5):** disabled, replaced with a static
  arrow labelled "→  10 Hz".

```tsx
const reduce = useReducedMotion();
const scale = useTransform(p, [0, 0.35], reduce ? [1, 1] : [1.0, 1.6]);
```

---

## 6. Component inventory

RSC by default. "Client" = `'use client'`. "Client (dyn)" = dynamically
imported `ssr:false` (needs `window` or Canvas).

| Component                 | Purpose                                                       | Render   |
|---------------------------|---------------------------------------------------------------|----------|
| `<PageShell>`             | `<main>` wrapper, eyebrow, footer, ADR cite index             | RSC      |
| `<HeroZoomFrame>`         | Sticky hero → zoom (§2a); wraps `<SystemPanelSvg>`            | Client   |
| `<SystemPanelSvg>`        | Inline SVG of diagram.md ①; fully SSR'd, zero JS needed       | RSC      |
| `<PanelPopup>`            | Generic "unfurl as segment crosses center" wrapper (§2b)      | Client   |
| `<CrossViewDissolve>`     | Two-sticky overlay transition (§2c)                           | Client   |
| `<HoeaLoop>`              | Canvas or SVG of diagram.md ②  with stroke-dashoffset reveal  | Client (dyn) |
| `<StickyProgressBar>`     | Top-of-viewport chapter indicator; `animation-timeline:scroll` with JS fallback | Client |
| `<SigRegHealthGauge>`     | CSS-only pulsing gauge (decorative; informational label RSC)  | RSC      |
| `<AdrCitation>`           | Small inline link card (`ADR-050`, `ADR-058`); static         | RSC      |
| `<ReducedMotionGate>`     | Reads `useReducedMotion()`, provides context                  | Client   |

Ten components, six server-renderable, four client islands. Page loads with
SSR'd content visible; JS hydrates progressively.

---

## 7. Graceful degrade on first paint

If motion JS is slow/fails:

- All SVG + prose is in SSR'd HTML. No placeholders.
- No transforms → every panel sits at its final state. Page reads as a
  long-form article with diagrams (which it is).
- Sticky still sticks (pure CSS). Hero holds for a screen, just doesn't zoom.
- H-O-E-A loop SSR'd with all four arcs at full stroke. Info preserved.
- `<StickyProgressBar>` falls back to `animation-timeline: scroll(root
  block)`; absent on Safari <17.4 / old Firefox, not load-bearing.

Acceptance: page loads with `Content-Security-Policy: script-src 'none'` and
reads completely. CI check via Playwright `page.route` blocking JS.

---

## 8. "Ship it in a weekend" MVP — 40% cut

Keep: beats 1, 2, 4, 6, 9, 10, 12 (seven beats, ~11 screens).

Drop: beat 3 (pullquote — redundant with hero copy); beat 5 (fold into 6 as
sub-column; packet-flow was decoration); beat 7 (link to ADR-055/057 from
beat 12); beat 8 (footnote under 6); beat 11 (caption under the loop covers
"1 ms servo · 10 Hz planner · ≪1 Hz retrain").

MVP keeps the money shots: hero zoom into system diagram, cross-view
dissolve to H-O-E-A. Everything else is elaboration. Scope: Sat =
`<HeroZoomFrame>` + `<SystemPanelSvg>` + beats 1+2+4. Sun =
`<CrossViewDissolve>` + `<HoeaLoop>` + beats 6+9+10+12. Ship.

---

## 9. Where I differ from Designer A

A will probably propose:

- **WebGL/Three.js 3D parallax** "flying through the system" — ~400 KB of
  JS, unreadable on mobile, and the diagram is fundamentally 2D box-and-arrow.
- **Physics springs on every reveal** — springs fight scroll velocity and
  feel wrong on trackpads; fixed cubic-bezier tracks scroll cleanly.
- **Lenis / Locomotive smooth-scroll** — breaks `scrollIntoView`, breaks
  find-on-page, breaks iOS rubber-banding, adds ~30 KB. Native scroll is fine.
- **Animated gradient/noise shader backgrounds** — repaint every frame,
  eat 10–20% mobile GPU budget for zero information gain.
- **Typewriter text reveal** — never faster than reading pre-rendered copy;
  opacity + translateY fade-up is the ceiling.

My cut: no Canvas if SVG works (H-O-E-A loop = SVG with `strokeDashoffset`).
No Three.js. No smooth-scroll library. No shader backgrounds. One `<main>`,
one scroll context, one motion primitive. The *diagram* is the spectacle.

---

## 10. Risk register

1. **Accidental `.get()` in render body causes per-frame re-renders.**
   *Impact:* 20-fps scroll on mid-range Android.
   *Mitigation:* all motion values pass through `useTransform` and are
   consumed only via `style={{...}}` (subscribes in motion layer, not
   React render). Review gate: no `.get()` in component body, no
   `useMotionValueEvent` unless necessary. Consider an ESLint rule.

2. **Safari sticky + `scale` creates stacking context, breaks z-index on
   the pullquote (beat 3).**
   *Impact:* pullquote vanishes under hero SVG on iOS.
   *Mitigation:* every sticky container gets explicit `isolation: isolate`
   + z-index contract. Playwright + WebKit screenshot test for the hero
   handoff, plus real-iPhone verification (Safari RDM differs on layer
   promotion).

3. **Scroll-linked animations feel rubber-bandy on low-end Chromium
   Android when scroll container ≠ layout viewport.**
   *Impact:* transforms lag scroll 1–2 frames; looks broken.
   *Mitigation:* never wrap content in `overflow:auto`. `useScroll` with
   no `container` arg uses document scroll (optimized path). If lag
   persists, consolidate `scale` + `y` into one `translate3d(0, y, 0)
   scale(s)` string via `useMotionTemplate` to reduce transform updates.

---

## One-line summary

Two sticky sections, one ease curve, `transform` + `opacity` + `clipPath`,
ten components, 25 KB of motion JS, everything SSR'd, `prefers-reduced-motion`
is a one-line branch. The pop-up book is the diagram; we just scroll into it.
