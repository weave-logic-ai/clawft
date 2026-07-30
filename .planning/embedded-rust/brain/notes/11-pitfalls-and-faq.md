# 11 — Pitfalls and FAQ

> Trust tiers and source ids refer to `../sources.json`.
> Ordered roughly by how much damage each one causes.

## 1. ⚠ Atomics in PSRAM are broken on Xtensa

The worst one, because it fails silently. Full treatment in
`04-memory-heap-psram.md`. Summary: on ESP32 / S2 / **S3**, atomics in PSRAM
"can cause data races and defeat their purpose." RISC-V chips are fine.

Every `Mutex`, `Arc`, channel, and most synchronization primitives contain
atomics — so this is a constraint on *where things are allocated*, not on which
APIs you call. Enforce it structurally with the capability-scoped allocator
split.

`[src: rust-on-esp-book | upstream-official]`

## 2. ⚠ Depending on `unstable` with a caret version — the pinning trap

esp-hal's own policy: **unstable changes ship in MINOR releases**. If you depend
on `unstable`, upstream recommends the **tilde** operator:

```toml
esp-hal = { version = "~1.1" }
```

`[src: esp-hal-repo | upstream-official]`

**This repo currently steps in it.** `crates/clawft-edge-pad/Cargo.toml`:

```toml
esp-hal = { version = "1.0.0", default-features = true, features = [
    "esp32s3", "unstable", "psram",
] }
```

`"1.0.0"` is a **caret** requirement (`^1.0.0`), which permits 1.1.0 — and
esp-hal 1.1.0 is already released. So `cargo update` may pull a minor bump that
is explicitly allowed to break the unstable API this firmware depends on.

**The fix is one character**: `version = "~1.0"` (or `~1.1` after a deliberate
upgrade). This is a genuine, actionable finding — see
`12-weftos-anchors.md` §3.

The same reasoning applies to every `esp-*` crate in that manifest whose
`unstable` feature is enabled: `esp-radio` also sets `unstable` and is also
caret-pinned.

## 3. ⚠ Don't `mem::forget` a driver

> "The `mem::forget` function should be avoided, as forgetting drivers may
> result in unintended consequences."

**Why**: drivers implement `Drop` to return peripherals to their default
unconfigured state **and to cancel any in-flight DMA transactions**. Forgetting
a driver risks leaving peripherals misconfigured and **DMA transactions running
indefinitely**.

`[src: rust-on-esp-book | upstream-official]`

**The nastier corollary**: this applies to anything with `mem::forget`-like
effects — `ManuallyDrop`, a leaked `Box`, or a `static` that outlives the
reconfiguration you intended. A DMA engine still writing into a buffer you have
logically released is memory corruption with no unsafe block in sight.

## 4. Clean build required after changing `.cargo/config.toml` `[env]`

Config changes via `esp-config` env vars may otherwise appear to have no effect.
The book recommends a clean build. Symptom: you set the option, nothing changes,
you conclude the option doesn't work.

`[src: rust-on-esp-book | upstream-official]`

## 5. RISC-V target triple: `imc` vs `imac`

`riscv32imc-unknown-none-elf` for ESP32-C2/C3; `riscv32imac-unknown-none-elf`
for ESP32-C6/H2. The `a` is atomics. Choosing wrong surfaces as link errors
about missing atomic intrinsics, not as a clear target mismatch.

`[src: rust-on-esp-book | upstream-official]`

## 6. Stale tutorials add `esp-hal-embassy`

That crate no longer exists as a separate dependency; it merged into
**`esp-rtos`**. If a guide tells you to add it, the guide predates esp-hal 1.0.

`[src: esp-hal-1.0-release | upstream-official]`

Companion staleness signal: guides that treat `esp-idf-hal` as the officially
supported default predate **February 2025**, when the std crates became
community projects (`01-ecosystem-and-stack-choice.md`).

## 7. Log backend features must agree across every esp-* crate

`log-04` (or the `defmt` equivalent) must be set consistently. Ours is on both
`esp-println` and `esp-radio`. Mismatches produce missing output or duplicate
logger conflicts rather than a helpful error.

Related: exactly **one** crate may provide `#[panic_handler]`.

`[src: in-repo-edge-pad | in-repo-verified]`

## 8. Partition-table defaults are silent

`espflash` applies defaults when no partition table is supplied. An app that
outgrows the default partition, or a project that needs OTA slots, fails in a
way that looks like a link/flash problem. Check the table before optimizing the
binary.

`[src: rust-on-esp-book | upstream-official]`

## 9. Strapping pins double as peripheral pins

The ROM bootloader samples strapping pins at reset to pick SPI-Boot vs Download
mode. A peripheral may drive such a pin happily *after* boot while an external
pull on it *at reset* bricks boot. Our CrowPanel uses **GPIO 0 as LCD PCLK** for
exactly this reason — it works because the board doesn't pull that pin.

`[src: rust-on-esp-book, in-repo-display-agent | upstream-official + in-repo-verified]`

**To exit download mode**: reset to resample the pins, or
`--after watchdog-reset` in USB-Serial/JTAG mode. The chip prints
`waiting for download` on serial while it's stuck there.

## 10. HIL testing needs USB-Serial-JTAG, and a CH340 board doesn't have it

`embedded-test` / `probe-rs` require the **USB-Serial-JTAG port only**. Boards
behind a CH340 USB-UART bridge need an external `esp-prog`. Discover this at
planning time, not at test-writing time.

Also measured: the CH340 path **drops bytes** under sustained throughput while
the host is under heavy CPU load. Don't trust a serial capture taken during a
build.

`[src: rust-on-esp-book, in-repo-display-agent | upstream-official + in-repo-verified]`

## 11. Custom bootloader drags in the whole ESP-IDF toolchain

Building one requires installing ESP-IDF and running `idf.py`. In an otherwise
pure-Rust project this is a real dependency with real CI cost. Budget it
explicitly.

`[src: rust-on-esp-book | upstream-official]`

## 12. Cooperative scheduling means one bad task starves the executor

Embassy tasks that never return `Poll::Pending` — busy-wait loops, long blocking
computation, blocking-mode driver calls inside async tasks — monopolize their
executor. The fix is an `InterruptExecutor` at a higher priority, not scattered
`yield_now()` calls.

`[src: embassy-book | ecosystem-canon]`

## 13. Not clearing the interrupt source = silent lockup

The device appears hung with no panic and no log because it re-enters the same
handler forever. First thing to check when firmware goes quiet without
resetting.

`[src: embedded-rust-book | ecosystem-canon]`

## 14. Using crates from git

The book points at the Cargo Book for git dependencies, `[patch]`, and switching
to a main branch, **without providing TOML examples**. Recorded here honestly:
there is no Espressif-specific guidance, just standard Cargo mechanics.

`[src: rust-on-esp-book | upstream-official]`

Practically relevant because the `esp-*` crates move fast enough that a fix you
need may only be on `main`. `[patch.crates-io]` is the usual lever, and it must
patch **every** crate in the esp-hal family together, since they are
version-coupled.
**⚠ Flagged as synthesis** — this coupling claim follows from the crates sharing
a repo and release cadence, but was not read from a document.

## 15. The no_std training book is self-declared out of date

It says so itself; a rewrite is in progress on `feat/overhaul`. Use it for
exercise structure, not for current API.

`[src: no-std-training | upstream-official]`
