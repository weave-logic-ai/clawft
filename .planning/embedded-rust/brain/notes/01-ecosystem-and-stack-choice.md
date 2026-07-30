# 01 — Ecosystem and stack choice

> Trust tiers and source ids refer to `../sources.json`.

## The two stacks, and the naming rule that tells them apart

There are exactly two ways to write Rust for Espressif silicon, and the crate
name prefix is the tell:

- **`esp-idf-*` prefix ⇒ the `std` path.** Rust bound over the ESP-IDF C
  framework. You get `std`, threads, `println!`, sockets, and everything
  ESP-IDF supports — because it *is* ESP-IDF underneath.
- **`esp-*` prefix (no `idf`) ⇒ the `no_std` / bare-metal path.** Pure Rust,
  no C framework, a plain `cargo build`.

`[src: esp-idf-hal-repo | ecosystem-canon]`

## Espressif officially supports the no_std path; the std path is community-maintained

This is the single most important orientation fact, and it is recent enough
that most tutorials predate it. In **February 2025**, `esp-idf-sys`,
`esp-idf-hal`, and `esp-idf-svc` became **community projects**. `esp-hal` and
the surrounding `no_std` crates are the **officially supported** environment.
Espressif still maintains the upstream *compiler targets* so the std path keeps
building, but the maintenance burden moved to the community.

`[src: esp-hal-beta-announcement | upstream-official]`

**Practical consequence for a new project**: default to `esp-hal` / `no_std`
unless you have a concrete reason. "Official support" here means bug reports
land with the vendor, and the 1.0 API-stability promise applies.

## The honest tradeoff (not a value judgement)

- **Feature completeness favours std.** The std ecosystem supports whatever
  ESP-IDF supports. `no_std` was written from scratch, more recently and by
  fewer people, so it is less complete — though drivers for most common
  peripherals now exist. `[src: esp-idf-hal-repo | ecosystem-canon]`
- **Language purity favours no_std.** On the std path, sooner or later you
  read ESP-IDF C. `esp-hal` is a regular cargo build and all the code is Rust.
  `[src: esp-idf-hal-repo | ecosystem-canon]`
- **The decision is often taste** — except where a specific peripheral driver
  exists on one side only. That exception is not hypothetical for us; see below.

## Our own repo is the best case study of that exception

WeftOS ships **both**, on purpose, for the same board:

- `crates/clawft-edge-pad/` — `no_std`, esp-hal 1.0 + esp-rtos + embassy.
- `crates/clawft-edge-pad-idf/` — `std`, esp-idf-hal/svc, existing specifically
  to use **Espressif's official `esp_lcd_panel_rgb`** driver, which has bounce
  buffers, frame sync, and hardware-erratum compensation.

The forcing function: **esp-hal 1.0 has no RGB-DPI bounce buffer** (upstream
esp-hal issue #5262, open at the time it was assessed). The `no_std` port had
to hand-port LovyanGFX's `Bus_RGB.cpp` register banging into
`crates/lgfx-bus-rgb-rs/` to get equivalent behaviour.

`[src: in-repo-edge-pad-idf, in-repo-display-agent | in-repo-verified]`

**Reusable lesson**: "which stack" is decided by *the hardest peripheral in the
design*, not by general preference. Find the peripheral with the weakest
`no_std` support and let it choose.

## What esp-hal 1.0 actually stabilized

The 1.0 stable surface is deliberately small:

- `esp_hal::init` and its configuration
- **Four drivers only: GPIO, UART, SPI, I2C** — in both `Async` and `Blocking` modes
- the `time` module: `Instant`, `Duration`, `Rate`
- SoC reset and misc system functions
- the `#[main]` macro
- `esp-config` (configuration beyond cargo features)

**Everything else is feature-gated behind the `unstable` feature.**

`[src: esp-hal-1.0-release | upstream-official]`

## "Unstable" means API-unstable, not broken

Espressif's own framing: *"Unstable in this case refers to API stability. There
is varying levels of functionality for unstable drivers, however, they are
suitable for most common use cases."*

So `unstable` is not a warning against use — it is a warning against
*unpinned* use. See `11-pitfalls-and-faq.md` for the pinning trap this creates
(and which our own firmware currently steps in).

`[src: esp-hal-1.0-release | upstream-official]`

## Chip support and the architecture split

- **Xtensa**: ESP32, ESP32-S2, ESP32-S3. Rust has **no official Xtensa
  support** because LLVM does not yet support Xtensa; Espressif maintains
  compiler forks to bridge it. This is why Xtensa needs `espup` and RISC-V does
  not.
- **RISC-V**: ESP32-C2, C3, C5, C6, C61, and ESP32-H2 (plus ESP32-P4).
  Officially supported by the upstream Rust toolchain.

esp-hal's supported set: ESP32; ESP32-C2/C3/C5/C6/C61; ESP32-H2; ESP32-P4;
ESP32-S2/S3. `esp-lp-hal` additionally targets the **low-power RISC-V cores**
found on ESP32-C6, ESP32-S2 and ESP32-S3.

`[src: rust-on-esp-book, esp-hal-repo | upstream-official]`

**The Xtensa/RISC-V split has teeth beyond toolchain setup** — it changes PSRAM
atomics behaviour (see `04-memory-heap-psram.md`) and which async frameworks
are available (see `05-async-embassy-rtos.md`).

## The esp-* crate inventory

Stable status as published; everything except `esp-hal` itself is marked
unstable.

| Crate | Role |
|---|---|
| `esp-hal` | The bare-metal (`no_std`) HAL. Peripheral drivers + chip init. **Stable\*** (per-driver caveats) |
| `esp-rtos` | Scheduler for `esp-radio`; **embassy support for esp-hal**. Interrupt-mode executor, multicore-aware thread-mode embassy executor, embassy time driver, timer waiter queue |
| `esp-radio` | Wi-Fi, BLE, IEEE 802.15.4, ESP-NOW. Needs `esp-radio-rtos-driver` for background operation |
| `esp-alloc` | Heap allocation, enabling `Vec`/`Box` under `no_std` |
| `esp-backtrace` | Backtraces on panic |
| `esp-println` | Print + logging; implements the `log` facade |
| `esp-storage` | Storage utilities |
| `esp-bootloader-esp-idf` | ESP-IDF 2nd-stage bootloader support, **including OTA** |
| `esp-config` | Build-time configuration beyond cargo features |
| `esp-hal-procmacros` | Proc macros for the esp-hal family |
| `esp-lp-hal` | HAL for the low-power / ultra-low-power cores |
| `esp-build` | Build-script utilities |
| `esp-preempt` | Threading + thread-aware sync primitives for the radio stacks |
| `esp-sync` | Synchronization primitives |
| `esp-metadata` / `-generated` | Device metadata, mainly for build scripts |
| `esp-rom-sys` | ROM code support |
| `esp-phy` | PHY support |
| `esp-riscv-rt` | Minimal startup/runtime for RISC-V CPUs |
| `xtensa-lx-rt` / `xtensa-lx` | Minimal startup/runtime, and low-level access, for Xtensa LX |

`[src: rust-on-esp-book, esp-rust-docs-index | upstream-official]`

## esp-hal-embassy is gone — it merged into esp-rtos

Anything you read that adds an `esp-hal-embassy` dependency is out of date.
Its functionality moved into **`esp-rtos`**, where development continues. Our
`clawft-edge-pad` already reflects this (`esp-rtos` with the `embassy` feature,
no `esp-hal-embassy`).

`[src: esp-hal-1.0-release, in-repo-edge-pad | upstream-official + in-repo-verified]`

## Where esp-hal meets the portable embedded-Rust ecosystem

`esp-hal` implements the community trait crates, which is what makes
third-party driver crates work:

- **`embedded-hal`** — the portable peripheral traits (see `09-embedded-rust-idioms.md`)
- **`embedded-io`** — `no_std` analogues of `std::io`
- **`rand_core`** — implemented for the hardware RNG peripheral

`[src: rust-on-esp-book | upstream-official]`

## Upstream current versions (2026-07-29)

esp-hal **1.1.0** · esp-rtos **0.3.0** · esp-radio **1.0.0-beta.0** ·
esp-alloc **0.10.0** · esp-println **0.17.0** · esp-backtrace **0.19.0** ·
esp-config **0.7.0** · esp-storage **0.9.0** · esp-sync **0.2.1** ·
esp-bootloader-esp-idf **0.5.0** · esp-lp-hal **0.3.0** ·
esp-riscv-rt **0.14.0** · xtensa-lx-rt **0.22.0**.

Read from the crate-doc index, which is the **refresh signal** for this whole
brain. `[src: esp-rust-docs-index | upstream-official]`

Our pins are all one minor behind this — see `12-weftos-anchors.md` §2 for the
drift table and what is safe to bump.
