# 12 — WeftOS in-repo anchors and drift

> Trust tier `in-repo-verified` throughout unless marked otherwise.
> **This is the note that makes the brain useful rather than generic.** Read it
> before advising on anything in this repo.

## 1. What Espressif-Rust code already exists here

| Path | Stack | Purpose |
|---|---|---|
| `crates/clawft-edge-pad/` | **no_std** — esp-hal 1.0 + esp-rtos 0.2 + embassy | Inkpad Actor firmware, ESP32-S3 + Elecrow CrowPanel DIS08070H. The bare-metal spike. |
| `crates/clawft-edge-pad-idf/` | **std** — esp-idf-svc/hal/sys | Same board, ESP-IDF port, existing to use Espressif's official `esp_lcd_panel_rgb` (bounce buffers + frame sync). |
| `crates/clawft-edge-bench/` | **std** — esp-idf-svc 0.51 / hal 0.46 / sys 0.36 | The earlier IDF-on-Rust precedent in this repo. |
| `crates/lgfx-bus-rgb-rs/` | **no_std** — esp-hal 1.0 | Faithful Rust port of LovyanGFX `Bus_RGB.cpp`: PSRAM framebuffer, circular GDMA descriptor ring, FIFO-skip restart descriptor, VSYNC ISR. Hardware-verified 2026-05-15. |
| `crates/weftos-leaf-touch-gt911/` | **no_std**, HAL-agnostic | GT911 driver written against `embedded-hal` traits — explicitly **not** depending on any esp-hal/esp-rtos crate, so it serves both ports. |

### The out-of-workspace pattern (important, and easy to break)

All three firmware crates carry an **empty `[workspace]` table**:

```toml
# Standalone crate — explicitly NOT part of the host workspace.
[workspace]
```

This stops cargo walking up the directory tree and claiming the crate for the
host workspace, which would apply host-side toolchain settings (stable,
x86_64) instead of the per-crate `rust-toolchain.toml` esp/Xtensa settings.

**Do not "tidy" these into the workspace.** It looks like an omission; it is
load-bearing. `[src: in-repo-edge-pad, in-repo-edge-pad-idf]`

Minor note: those header comments reference a stale absolute path
(`/home/aepod/dev/clawft/Cargo.toml`) from a different machine. Harmless, but
don't trust paths in comments.

## 2. Drift table — our pins vs upstream current (2026-07-30, WEFT-668)

Upstream column re-confirmed from `https://docs.espressif.com/projects/rust/`
plus crates.io max_version where the index lags (esp-hal docs index = 1.1.0;
crates.io max = **1.1.1**).
`[src: esp-rust-docs-index | crates.io | upstream-official]`

| Crate | `clawft-edge-pad` pin | Upstream current | Delta |
|---|---|---|---|
| `esp-hal` | `~1.0` (lock 1.0.0) | **1.1.1** (docs 1.1.0) | one minor — **held** |
| `esp-rtos` | `0.2.0` | **0.3.0** | one minor — **held** |
| `esp-alloc` | `0.9.0` | **0.10.0** | one minor — **held** |
| `esp-println` | **0.17.0** | **0.17.0** | current ✅ (WEFT-668) |
| `esp-backtrace` | **0.19.0** | **0.19.0** | current ✅ (WEFT-668) |
| `esp-bootloader-esp-idf` | **0.5** | **0.5.0** | current ✅ (WEFT-668) |
| `esp-radio` | `~0.17` (lock 0.17.0) | **1.0.0-beta.0** (+ 0.18.0) | ⚠ **major line — NO-GO** |
| `esp-config` | transitive (0.6.1 + 0.7.0) | 0.7.0 | dual after peripheral bump |
| `esp-storage` | not used | 0.9.0 | unused |
| `esp-sync` | transitive (0.1.1 + 0.2.1) | 0.2.1 | dual after peripheral bump |

**Reading (post WEFT-668)**: the "all one minor behind" uniformity is broken
intentionally. Three peripheral crates (`esp-println` / `esp-backtrace` /
`esp-bootloader-esp-idf`) were bumped to upstream. The remaining four
(`esp-hal`, `esp-rtos`, `esp-alloc`, `esp-radio`) form a **radio-coupled set**
that cannot move without co-moving `esp-radio` past 0.17:

| Blocker | Why phase-1 cannot take them |
|---|---|
| `esp-hal` 1.1 | pulls `xtensa-lx-rt ^0.22`; radio 0.17 needs `^0.21` — `links` conflict |
| `esp-rtos` 0.3 | needs `esp-radio-rtos-driver ^0.3`; radio 0.17 needs `^0.2` |
| `esp-alloc` 0.10 | outside radio 0.17 default-feature `esp-alloc ^0.9.0` (`<0.10`) |
| `esp-radio` 1.0-beta | major API rewrite; beta risk — see §2.1 |

**⚠ The `esp-radio` exception (resolved as NO-GO for this wave)**:
0.17.0 → 0.18.0 → 1.0.0-beta.0. Migration guides read 2026-07-30
(`MIGRATING-0.17.0.md`, `MIGRATING-0.18.0.md`, CHANGELOG). Decision: **do not
adopt the beta** (or even 0.18) without a dedicated phase-2 ticket that
rewrites `net.rs` and flashes CrowPanel. Summary in
`docs/plans/wave-0c-WEFT-668-result.md`.

### 2.1 Embassy + esp-idf currency (first check, 2026-07-30)

| Crate | Our pin | crates.io max | Delta |
|---|---|---|---|
| `embassy-executor` | `0.9.0` | **0.10.0** | one minor (rtos 0.3 wants `^0.10`) |
| `embassy-time` | `0.5.1` | **0.5.1** | current |
| `embassy-sync` | `0.8` | **0.8.0** | current |
| `embassy-net` | `0.9.0` | **0.9.1** | patch behind |
| `esp-idf-svc` | `0.52` | **0.52.1** | patch behind |
| `esp-idf-hal` | `0.46` | **0.46.2** | patch behind |
| `esp-idf-sys` | `0.37` | **0.37.2** | patch behind |
| `embedded-svc` | `0.29` | **0.29.0** | current |

std-side matrix in `clawft-edge-pad-idf` remains coherent
(`svc 0.52 ⇒ hal ^0.46 ⇒ sys ^0.37`). Not bumped in WEFT-668.

## 3. Caret-pinned `unstable` — **resolved (WEFT-667)**

WEFT-667 applied tilde pins:

```toml
esp-hal  = { version = "~1.0", ... }
esp-radio = { version = "~0.17", ... }
```

(same for `lgfx-bus-rgb-rs` esp-hal). Keep `~` while `unstable` is enabled.
Do **not** flip to `~1.1` until the radio-coupled set moves (see §2).

## 4. Hard-won hardware facts (do not re-derive these)

Carried from the [[esp32-s3-rgb-touch-display]] agent's 2026-05-15
session-learnings, after **eleven config iterations**. Full detail lives in that
agent; the load-bearing generalizations for *Rust/allocator/toolchain* purposes:

1. **`esp_alloc::psram_allocator!` panics** on ESP32-S3-WROOM-1 N4R8 / AP_3v3
   in `linked_list_allocator-0.10.6/src/hole.rs:331`. The working pattern is the
   SRAM-first + capability-tagged-PSRAM-region split
   (`04-memory-heap-psram.md`).
2. **esp-hal 1.0 has no RGB-DPI bounce buffer** — upstream esp-hal issue #5262,
   open at assessment. This is *the* reason `clawft-edge-pad-idf` exists and the
   single clearest illustration in this repo of the std-vs-no_std tradeoff.
3. **`#[esp_hal::ram]` on infrequent ISRs is mandatory**, not an optimization:
   a ~33–40 ms-period VSYNC ISR eats a 20–30 µs flash-fetch stall per fire
   without it.
4. **PSRAM bandwidth contention presents as a rendering bug.** The write-once
   static-grid diagnostic separates contention from logic.
5. **Two independent failure modes look like one.** Patchy shifting blocks =
   bandwidth contention. Steady diagonal drift = frame-lock failure. Diagnosing
   them together costs days.
6. **The CH340 bridge loses bytes** under sustained throughput during heavy host
   CPU load — and gives no USB-Serial-JTAG, so no `probe-rs` HIL.

`[src: in-repo-display-agent | in-repo-verified]`

## 5. Domain boundaries — who answers what

This brain owns the **language / toolchain / crate-ecosystem** layer. It does
**not** own hardware or application domains, which already have owners:

| Question type | Owner |
|---|---|
| Which HAL, `unstable` semantics, allocator strategy, async model, testing, toolchain, crate versions, `embedded-hal` idiom | **this brain** → [[embedded-rust-expert]] |
| RGB LCD bring-up, GT911 touch, LCD_CAM DPI timings, PSRAM init on *this panel*, CrowPanel pin maps, Inkpad Actor contract | [[esp32-s3-rgb-touch-display]] |
| I²S DMA ADC capture, matched filters on LX7, ESP-DSP SIMD, pulser timing, LoRa/ESP-NOW backhaul, sonobuoy power budgets | [[embedded-acoustic-firmware]] |
| Sonar equation, propagation, dB budgets | [[marine-acoustician]] |
| Substrate publish protocol, ADR-025/057 identity + ACL | [[clawft-kernel-specialist]] / the ADRs themselves |

**Overlap is real and must be handled by deferring, not guessing.** PSRAM is the
clearest case: the *allocator strategy* is this brain's; the *panel's PSRAM
timing feature flags* are the display agent's. When a question straddles, say
which part you own and route the rest.

## 6. Repo conventions that apply to any firmware work here

From `CLAUDE.md` — these are enforced, not advisory:

- **`scripts/build.sh` for ALL build/test/check/lint.** Not raw cargo. If a
  firmware build needs flags the script lacks, **extend the script**.
  (Note: the firmware crates are out-of-workspace, so confirm how — or whether —
  `build.sh` reaches them before assuming `scripts/build.sh test` covers them.)
- **`scripts/build.sh gate`** — the 11-check phase gate, before committing.
- **Plane is the authoritative work tracker.** New TODOs become work items; use
  the `plane-workflow` skill. The §3 finding above needs one.
- **ADRs in `docs/adr/` are living plans** — if a change alters what an accepted
  ADR describes, update the ADR in the same piece of work.
- **Never commit to `master`.** Files never go in the repo root or `/tmp`.
