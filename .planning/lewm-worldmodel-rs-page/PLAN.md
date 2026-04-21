# /lewm-worldmodel-rs — Build Plan

**Target**: `https://weftos.weavelogic.ai/lewm-worldmodel-rs`
**Route file**: `docs/src/app/lewm-worldmodel-rs/page.tsx`
**Branch**: `feature/lewm-worldmodel-rs-page` (cut from **master**, not from `feature/lewm-worldmodel`, so the page can ship independently of the ADR batch)
**PR posture**: **draft, do not merge**

Synthesis of `DESIGN-A.md` (visual spine, 12-chapter narrative) and `DESIGN-B.md` (motion mechanics, perf discipline). **We take A's ambition and B's engineering posture.** Where they disagreed, the pragmatist won on mechanics and the visual maximalist won on storyboard.

---

## 1. What we're building

A single scroll-driven page that walks a reader through the LeWM × ECC × weftos architecture (ADRs 048–058) as a "pop-up book": sticky zoom-ins, panel unfurls, and one cross-view dissolve between the full-system diagram and the real-time H-O-E-A training cycle. **Sensor-primary + decoupling-invariant** is the narrative spine, not a footnote.

The page is a Next.js 16 RSC shell with a small set of client islands running the `motion` v12 package. SSR'd DOM is readable and the argument is complete with JS disabled; motion is arrangement, not carrier.

---

## 2. Reconciled design decisions (where A and B disagreed)

| Tension | A's call | B's call | Resolution |
|---|---|---|---|
| 3D flip at "The Inversion" | rotateY 3D flip | flat, no 3D | **CSS 3D `rotateY` on a 2D plane** — cheap, no WebGL, lands the moment. Flat visual, 3D mechanic. |
| Scroll = cycle speed at H-O-E-A | yes, novel moment #2 | reveal via `strokeDashoffset`, no velocity play | **Both** — arcs reveal on entry (B), then scroll velocity modulates loop speed once fully revealed (A). |
| Depth-of-field on hover | yes, `filter: blur()` | no blur on mobile | **Opacity drop only** (other bays → 40%). No blur. Keep the "finger on the drawing" effect cheaply. |
| ADR timeline | horizontal scroll-jack ribbon | not budgeted | **Vertical stack, per-card unfurl.** Skip scroll-jack (hostile on mobile). Keep ADR-058 sodium-amber glow. |
| TL;DR pullquote | standalone chapter 2.2 | fold into hero | **Keep standalone**, MVP-cuttable. |
| Smooth-scroll library | silent | banned | **Native scroll only.** No Lenis / Locomotive. |
| Springs on scroll | silent | banned | **Cubic-bezier `[0.16, 1, 0.3, 1]` everywhere.** No springs-on-scroll. |
| Backdrop filters / shader bg | silent | banned | **Banned.** No `backdrop-filter`, no animated shaders. |

---

## 3. Final storyboard (12 beats)

Ease everywhere: `const EASE = [0.16, 1, 0.3, 1] as const`.
Section heights and local progress ranges follow DESIGN-B §4. Visual content and copy follow DESIGN-A §2.

| # | Beat | Section height | Transform | Source copy |
|---|---|---|---|---|
| 1 | Hero — "LeWM under the DAG" | 100vh | `opacity [0,1]`, `y [16px,0]`; 192-dot breath grid (CSS) | DESIGN-A §5 |
| 2 | Hero → system panel zoom | 180vh sticky | `scale [1.00, 1.22, 1.55, 1.60]` with hang at end; `y [0, -12vh]`; fade 0.25→0.45 | DESIGN-B §2a |
| 3 | TL;DR pullquote | 80vh | `opacity` stops `[.10, .30, .45, .55] → [0,1,1,0]`; no transform | synthesis §TL;DR |
| 4 | The Inversion (ADR-058 flip) | 120vh sticky | CSS 3D `rotateY [0, 180]` on flat plane; labels crossfade at 50% | ADR-058 |
| 5 | Panel ❶ Sensor plane unfurl | 140vh | `clipPath inset(45% 8%) → inset(0)`, `scale 0.92→1.00`, `y 40→0` | diagram ① ❶ |
| 6 | Panel ❷ Observation wire | 140vh | unfurl + 10 Hz packet-flow CSS animation | diagram ① ❷ |
| 7 | Panel ❸+❹ Consumers split | 180vh | two-column unfurl, left leads right by ~0.08 progress | diagram ① ❸+❹ |
| 8 | Panel ❺ Training layer | 140vh | unfurl + SIGReg-health CSS gauge pulse | diagram ① ❺ |
| 9 | Panel ❻ ExoChain spine | 120vh | bottom-anchored unfurl | diagram ① ❻ |
| 10 | Cross-view dissolve → H-O-E-A | 240vh | §2c of DESIGN-B: panel fades 0.40→0.60, loop enters 0.45→0.75 | diagram ② |
| 11 | H-O-E-A loop + scroll=speed | 160vh sticky | arcs reveal via `strokeDashoffset` per ADR tuple, then `useVelocity` → loop speed | diagram ② |
| 12 | ADR index (vertical) + closer | 140vh | 11 cards unfurl; ADR-058 amber glow; 3 CTAs | ADR README |

Total ≈ **17.4 screens** — ~15 s brisk-flick on laptop, ample on mobile.

**MVP cut** (if we need to ship 40% faster): beats 1 · 2 · 5 · 7 · 10 · 11 · 12. Drop 3 (TL;DR becomes hero copy), 4 (Inversion folds into hero copy), 6/8/9 (fold into 7 as footnote pills).

---

## 4. Visual system (locked from DESIGN-A §3)

**Palette** (dark-first, all scoped under `.lewm-scope`):
```
--lewm-bg       #07090C   background
--lewm-surface  #0D1117   panel surface
--lewm-grid     #1A2230   schematic grid lines
--lewm-ink      #E6EDF3   primary ink
--lewm-mute     #8B98A8   secondary / mono labels
--lewm-line     #3A4A60   default schematic line
--lewm-cyan     #6DD3FF   sensor plane
--lewm-mint     #7AE0C3   local ECC / authoritative
--lewm-violet   #B18CFF   world model / optional
--lewm-amber    #FFB65A   RESERVED: decoupling invariant + sigreg_health
--lewm-white    #FAFBFF   SIGReg highlight
```

**Type**: Inter 400/500/700 (Fumadocs default) · JetBrains Mono (added via `next/font/google`) · Fraunces 450 for the two editorial beats (TL;DR + closer).

**Panel look**: blueprint-schematic. 1px solid `--lewm-line`, 95% `--lewm-surface` fill over isometric grid, 4px corner radius, single 30px inner shadow top-left (drafting lamp). Sharp, not soft.

**Icons**: custom inline SVG, 1.25px stroke, one sprite at `public/lewm/sprite.svg`. No Lucide. No emoji.

---

## 5. Component inventory (locked from DESIGN-B §6, extended)

| Component | Purpose | Render |
|---|---|---|
| `<PageShell>` | `<main>` wrapper, route-scoped class, footer | RSC |
| `<HeroZoomFrame>` | Beat 1+2 sticky zoom; wraps `<SystemPanelSvg>` | Client |
| `<InversionFlip>` | Beat 4, CSS 3D rotateY | Client |
| `<Pullquote>` | Beat 3 editorial fade | Client (minimal) |
| `<SystemPanelSvg>` | Inline SVG of diagram ①; SSR, zero JS | RSC |
| `<PanelPopup>` | Generic unfurl wrapper (beats 5–9) | Client |
| `<ConsumersSplit>` | Beat 7 two-column layout | Client |
| `<TrainingLayer>` | Beat 8 with CSS gauge pulse | Client (minimal) |
| `<CrossViewDissolve>` | Beat 10 two-sticky overlay | Client |
| `<HoeaLoop>` | Beat 11 SVG with arc reveal + velocity-driven loop | Client (dyn, ssr:false) |
| `<AdrCard>` | Single ADR-NNN card | RSC |
| `<AdrIndex>` | Beat 12 stack of 11 cards; ADR-058 amber | Client (minimal — only for amber pulse) |
| `<ReducedMotionGate>` | `useReducedMotion()` context | Client |
| `<LatentDots>` | Hero 192-dot breath grid (CSS-only) | RSC |
| `<SigRegGauge>` | CSS-only pulsing gauge (stretch) | RSC |

**14 components · 7 RSC · 7 client.** First paint is SSR'd DOM; motion JS hydrates progressively. `LazyMotion` + `domAnimation` preset (not `domMax`) — ~25 KB gz.

---

## 6. Performance contract (from DESIGN-B §3)

- LCP < 2.0 s · CLS = 0 · scroll INP < 120 ms · 60 fps during zoom beats (Pixel 6a / iPhone 13 throttled 4× CPU).
- `transform` + `opacity` + `clipPath` only on scroll. **No** `filter: blur()`, **no** `backdrop-filter`, **no** `margin/padding/width/height` animations.
- `willChange` toggled on entry/exit via `useMotionValueEvent`, never static.
- Hard cap 6 animating elements per moment.
- `useScroll` with no `container` arg (document scroll, optimized path).
- Motion JS ≤ 25 KB gz on first paint. `motion` full-tree lazy-loads past beat 4.
- Total route budget: ≤ 80 KB gz JS + ≤ 40 KB SVG. No raster images.

---

## 7. Accessibility & reduced motion (from DESIGN-A §7)

- `useReducedMotion()` at page root → one top-level branch.
- Reduced-motion path: all `scale`/`y`/`rotate`/`clipPath` collapse to `opacity [0,1]` over the same progress range. Inversion flip becomes static side-by-side + caption. H-O-E-A becomes static 4-state diagram.
- Sticky stays in both modes.
- Scroll snap in reduced-motion.
- Every chapter has an `id`; right-side rule doubles as keyboard-focusable anchor set.
- Diagrams: `role="img"` + detailed `aria-label` from the ASCII source. SVG internals `aria-hidden`.
- Hidden `<h2>` per chapter + `aria-describedby` summaries — the page is a second, SR-readable document from the same tree.
- WCAG AA on all text; amber-on-near-black validated 11.4:1.

---

## 8. File layout

```
docs/src/app/lewm-worldmodel-rs/
├── page.tsx                   RSC shell
├── lewm.css                   scoped palette + fonts
├── copy.ts                    all prose, headings, ADR cites
└── components/
    ├── PageShell.tsx
    ├── HeroZoomFrame.tsx
    ├── InversionFlip.tsx
    ├── Pullquote.tsx
    ├── SystemPanelSvg.tsx     SSR SVG of diagram ①
    ├── PanelPopup.tsx
    ├── ConsumersSplit.tsx
    ├── TrainingLayer.tsx
    ├── CrossViewDissolve.tsx
    ├── HoeaLoop.tsx           SSR SVG + hydrated motion (dyn ssr:false not needed — SVG can SSR)
    ├── AdrCard.tsx
    ├── AdrIndex.tsx
    ├── ReducedMotionGate.tsx
    ├── LatentDots.tsx
    ├── SigRegGauge.tsx
    └── motion.ts              shared ease, helpers

docs/src/public/lewm/
└── sprite.svg                 icon sprite
```

Page addition is **self-contained**: one route directory, one CSS slice (scoped via `.lewm-scope`), one new npm dep (`motion`). No changes to existing routes, nav, or config.

---

## 9. Branch + PR plan

1. **Stash / verify clean tree** on `feature/lewm-worldmodel` (current branch has the ADR batch committed).
2. `git checkout master` · `git pull` if remote ahead.
3. `git checkout -b feature/lewm-worldmodel-rs-page`.
4. **Copy forward** only the two files needed for citations: `docs/adr/adr-048*…adr-058*.md` and `diagram.md` — or simpler, cherry-pick the two symposium commits (`5ffe3e1`, `9b00c2f`) so the page can link to live ADRs. **Actually, simplest: just reference the raw URLs on the other branch for now, and the cites become live when the ADRs merge.** Do not cherry-pick; keep this branch page-only.
5. `cd docs/src && npm install motion` · commit `package.json` + `package-lock.json` separately.
6. Build page files in order per §8, starting with SSR'd shell so the page is visible at any checkpoint.
7. `npm run build` local smoke test. Playwright smoke test if time permits.
8. `git push -u origin feature/lewm-worldmodel-rs-page`.
9. `gh pr create --draft --base master` with PR body explicitly marked **DO NOT MERGE — awaiting visual confirmation from @aepod**.
10. Return PR URL and preview URL (Vercel auto-deploys on push if wired).

---

## 10. Acceptance

For this PR alone:
- [ ] Page builds clean (`npm run build`)
- [ ] Renders at `/lewm-worldmodel-rs` on the preview deploy
- [ ] All 12 beats present OR MVP cut with clear TODOs
- [ ] Motion works (zoom, unfurl, cross-view dissolve, H-O-E-A reveal)
- [ ] `prefers-reduced-motion` path works (test via browser toggle)
- [ ] No changes to any existing route or config beyond `docs/src/package.json`
- [ ] PR status: **draft**
- [ ] PR body explicitly says **DO NOT MERGE**

---

## One-line summary

> We take DESIGN-A's 12-chapter visual narrative and ship it on DESIGN-B's motion mechanics and perf discipline: one primitive (`useScroll → useTransform` into transform/opacity/clipPath), one ease curve, SSR-first, ≤25 KB motion JS, reduced-motion as a one-line branch, draft PR to master that does not merge until you've seen it.
