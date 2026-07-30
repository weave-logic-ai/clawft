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

## 2. Drift table — our pins vs upstream current (2026-07-29)

Upstream column read from `https://docs.espressif.com/projects/rust/`.
`[src: esp-rust-docs-index | upstream-official]`

| Crate | `clawft-edge-pad` pin | Upstream current | Delta |
|---|---|---|---|
| `esp-hal` | `1.0.0` | **1.1.0** | one minor |
| `esp-rtos` | `0.2.0` | **0.3.0** | one minor |
| `esp-alloc` | `0.9.0` | **0.10.0** | one minor |
| `esp-println` | `0.16.1` | **0.17.0** | one minor |
| `esp-backtrace` | `0.18.1` | **0.19.0** | one minor |
| `esp-bootloader-esp-idf` | `0.4` | **0.5.0** | one minor |
| `esp-radio` | `0.17.0` | **1.0.0-beta.0** | ⚠ **major line** |
| `esp-config` | not pinned | 0.7.0 | unused |
| `esp-storage` | not used | 0.9.0 | unused |
| `esp-sync` | not used | 0.2.1 | unused |

**Reading**: every `esp-*` pin is *exactly* one minor behind. That is the
signature of a coherent set-wise pin taken at one upstream moment — a good
practice — and it means the upgrade should be evaluated as **one atomic set**,
not crate by crate.

**⚠ The `esp-radio` exception**: 0.17.0 → 1.0.0-beta.0 is a major-line move,
not a routine bump. Its changelog was **not read** during this corpus build.
Anyone advising "just bump them all" must read the esp-radio migration notes
first. Also note that jumping *to a beta* is a deliberate risk decision, not a
neutral update.

Embassy side (not published on the Espressif index, so **not** version-checked
during this build — treat as unknown-drift): embassy-executor `0.9.0`,
embassy-time `0.5.1`, embassy-sync `0.8`, embassy-net `0.9.0`.

std side: esp-idf-svc `0.52` / esp-idf-hal `0.46` / esp-idf-sys `0.37` /
embedded-svc `0.29`, with the coherent matrix documented in the manifest
(`svc 0.52 ⇒ hal ^0.46 ⇒ sys ^0.37`). Also not version-checked upstream.

## 3. Open finding: caret-pinned `unstable` dependencies

`clawft-edge-pad` enables the `unstable` feature on both `esp-hal` and
`esp-radio` while caret-pinning them (`"1.0.0"`, `"0.17.0"`). esp-hal's own
policy is that **unstable changes ship in minor releases**, and upstream
therefore recommends tilde pinning (`~1.0`) for `unstable` consumers.

Since esp-hal 1.1.0 exists, a `cargo update` is currently permitted to pull a
minor bump that may break the unstable API this firmware relies on.

`[src: esp-hal-repo | upstream-official]` + `[src: in-repo-edge-pad]`

**Recommended remediation** (one-character-class change, no behaviour change):

```toml
esp-hal  = { version = "~1.0", ... }
esp-radio = { version = "~0.17", ... }
```

This has **not** been applied — it is a finding, and it belongs in a Plane work
item per the repo's tracker rule. Note also that a `Cargo.lock` is what actually
protects the build today; the tilde pin protects it from a careless `cargo
update`.

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
