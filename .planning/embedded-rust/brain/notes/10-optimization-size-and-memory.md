# 10 — Optimization: binary size, RAM, and latency

> Trust tiers and source ids refer to `../sources.json`.
> Levers are labelled **measured** (we or a cited source observed the effect) or
> **recommended** (documented advice, effect not measured here). Don't present
> recommended levers as guaranteed wins.

## Measure before optimizing — the tools

- **`cargo size`** — inspect `.data` and `.bss` to see static RAM use.
- **`cargo-call-stack`** — **static worst-case stack** analysis. Valuable
  precisely because stack overflow on embedded is silent corruption rather than
  a clean abort.
- **Idle-loop timing** for CPU headroom: measure the duration of an
  idle/low-priority loop and infer utilization from how much it shrinks.

`[src: embassy-book | ecosystem-canon]`

The book defers memory-optimization detail to the Embassy docs' "How can I
measure resource usage (CPU, RAM, etc.)?" section.
`[src: rust-on-esp-book | upstream-official]`

## Binary size levers

**Recommended** `[src: rust-on-esp-book | upstream-official]`:

- Build in the **release profile** — it optimizes and removes debug symbols.
- Tune the cargo **profile** settings; they "can make a difference in the
  resulting size."
- Be **cautious with dependencies** — each one grows the artifact.
- **Filter unnecessary log messages** (see `06-logging-and-observability.md`;
  logging is one of the biggest single contributors).
- Consult **min-sized-rust** (<https://github.com/johnthagen/min-sized-rust>).

**Recommended, more aggressive** `[src: embassy-book | ecosystem-canon]`:

```toml
[profile.release]
opt-level = "s"
lto = "fat"
```

plus `build-std = ["core"]` with **`panic_immediate_abort`**, which "eliminates
fmt code" — the formatting machinery dragged in by panics and logging.

**⚠ `panic_immediate_abort` is a real tradeoff**: it removes panic messages
entirely. You get a smaller binary and a mute device. Do not enable it on
firmware you are still bringing up, and reconsider it if
`esp-backtrace` is load-bearing for your field diagnostics.

**Our current profile** `[src: in-repo-edge-pad | in-repo-verified]`:

```toml
[profile.dev]
opt-level = "s"     # note: NOT the default 0 — embedded needs opt even in dev

[profile.release]
opt-level = "s"
lto = true
```

Two observations: `opt-level = "s"` in **dev** is deliberate and correct for
embedded (unoptimized embedded builds are frequently too slow or too large to
run at all). And `lto = true` is "thin-ish" LTO — the Embassy guidance
recommends `lto = "fat"`, which is an untried, low-risk experiment for us.
The sibling `clawft-edge-pad-idf` uses `opt-level = "z"` in dev with
`debug = true`, a different and equally defensible balance.
`[src: in-repo-edge-pad-idf | in-repo-verified]`

## RAM levers

- **Reclaimed RAM** — `heap_allocator!(#[ram(reclaimed)] size: 64000)` recovers
  bootloader working memory (`04-memory-heap-psram.md`). **Recommended**, and
  close to free.
- **PSRAM** for bulk buffers, with the capability-scoped split so
  synchronization primitives stay in SRAM. **Measured** in-repo.
- **`heapless` over `alloc`** on hot paths — removes both fragmentation risk and
  allocator overhead, at the cost of a static upper bound. **Measured** in-repo:
  the touch path uses `heapless::Vec<TouchPoint, N>` and never allocates.
- **Right-size the SRAM heap.** 160 KiB held WiFi + embassy-net + mesh
  comfortably on our tested workload — a **measured** data point, and a useful
  starting estimate for a similar stack.

`[src: rust-on-esp-book, in-repo-display-agent | upstream-official + in-repo-verified]`

## Latency lever: IRAM placement for interrupt handlers

**Measured, and quantified** `[src: in-repo-display-agent | in-repo-verified]`:

A VSYNC ISR firing every ~33–40 ms gets **evicted from L1 icache between
fires**. Without IRAM placement, every fire eats a **20–30 µs flash-fetch
stall** — which at a 15 MHz pixel clock is **250–360 pixels** of visible
re-arm jitter.

The fix is `#[esp_hal::ram]` on the handler, plus raising its priority to 3+.

**Generalize this**: any ISR that fires infrequently but must respond fast is a
cache-miss victim. The instinct "it's a small function, it'll be fast" is wrong
— *infrequent* is precisely what makes it slow. If an ISR has a latency budget
in the tens of microseconds, place it in IRAM.

## Bandwidth is an optimization axis people forget

**Measured** `[src: in-repo-display-agent | in-repo-verified]`: GDMA reading a
framebuffer continuously at ~24 MB/s from PSRAM contends with every other PSRAM
access. The fix was not "make the code faster" but "move the contending
allocations off that bus."

**Diagnostic to reuse**: the write-once static-target test. If a buffer written
exactly once still exhibits corruption, you have a bandwidth/contention problem,
not a logic problem. See `04-memory-heap-psram.md`.

## Hardware limits are not optimizable — recognize them

**Measured** `[src: in-repo-display-agent | in-repo-verified]`: on the CrowPanel
DIS08070H the panel's low colour bits (R0–R2, G0–G1, B0–B2) are not wired to the
ESP32-S3, which drives only RGB565 into a true-RGB888 panel. If those lines
float they pick up EMI from the active data lines, producing a fine colour
sparkle that **no software can fix on that board revision**.

**The lesson worth keeping**: part of optimization competence is correctly
identifying the floor. Time spent optimizing against a hardware limit is time
lost twice — once to the effort, once to the wrong mental model it leaves
behind. Establish whether a limit is silicon, board, or software *before*
committing an optimization budget.
