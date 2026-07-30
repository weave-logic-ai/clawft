# Coverage map — topic ↔ note ↔ agent ↔ in-repo anchor

Read with `README.md` (trust tiers) and `sources.json` (provenance).

## 1. Topic → note → source → in-repo anchor

| Topic | Note | Primary source(s) | In-repo anchor |
|---|---|---|---|
| no_std vs std; official vs community; crate inventory; esp-hal 1.0 stable surface; chip/arch split | `01-ecosystem-and-stack-choice.md` | rust-on-esp-book, esp-hal-1.0-release, esp-hal-repo, esp-idf-hal-repo, esp-rust-docs-index | both firmware crates (the repo ships both stacks) |
| espup, rustup targets, esp-generate, espflash, probe-rs, esp-config, Wokwi, training material | `02-toolchain-and-tooling.md` | rust-on-esp-book, esp-generate-repo, espflash-repo, awesome-esp-rust, no-std-training | `rust-toolchain.toml` per firmware crate |
| Two-stage boot, ESP-IDF 2nd stage, partition tables, custom bootloader, esp-config mechanics, download mode | `03-boot-bootloader-partitions.md` | rust-on-esp-book, espflash-repo | Inkpad partition layout (single-app, no OTA) |
| Heap costs, reclaimed RAM, PSRAM, **Xtensa atomics landmine**, allocator regions + capabilities | `04-memory-heap-psram.md` | rust-on-esp-book, in-repo-display-agent | the measured fix-B heap split in `clawft-edge-pad/src/main.rs` |
| Embassy executor/time/sync/net, esp-rtos integration, ArielOS, RTIC | `05-async-embassy-rtos.md` | rust-on-esp-book, embassy-book | `clawft-edge-pad` embassy pins + Core0/Core1 split |
| defmt vs log, esp-println, esp-backtrace, log-04 feature agreement | `06-logging-and-observability.md` | rust-on-esp-book, espflash-repo, in-repo-edge-pad | `esp-println`/`esp-radio` `log-04` features |
| Host-first testing, embedded-test + probe-rs HIL, three-tier strategy | `07-testing-host-and-hil.md` | rust-on-esp-book, embedded-test-docs | `weftos-leaf-touch-gt911` (trait-generic ⇒ host-testable) |
| OTA preconditions, esp-bootloader-esp-idf, rollback, flash budget | `08-ota-and-updates.md` | rust-on-esp-book, esp-rust-docs-index | N4R8 4 MB ⇒ single-app, no OTA |
| Typestate, singletons, PAC/HAL layering, embedded-hal family, concurrency + multicore | `09-embedded-rust-idioms.md` | embedded-rust-book, embedded-hal-docs | `weftos-leaf-touch-gt911`, `lgfx-bus-rgb-rs` (raw-register exception) |
| Size levers, RAM levers, IRAM/ISR latency, bandwidth, hardware floors | `10-optimization-size-and-memory.md` | rust-on-esp-book, embassy-book, in-repo-display-agent | both crates' `[profile.*]` |
| 15 ranked pitfalls incl. the `unstable` caret trap and `mem::forget` | `11-pitfalls-and-faq.md` | rust-on-esp-book, esp-hal-repo, embassy-book, embedded-rust-book | §2 finding on `clawft-edge-pad` pins |
| The repo's own firmware, drift table, hard-won hardware facts, domain boundaries, repo conventions | `12-weftos-anchors.md` | in-repo-* sources | everything |

## 2. Agent → the notes each one leans on

| Agent | Primary notes | Why |
|---|---|---|
| [[embedded-rust-expert]] | all 12 | It answers "should we" and "why", so it needs the whole corpus, especially trust-tier conflicts (04) and boundaries (12 §5) |
| [[embedded-rust-planner]] | 01, 02, 03, 07, 08, 12 | Stack choice, toolchain cost, bootloader/OTA preconditions, testability-by-design, and existing repo state are what make a plan survive contact |
| [[embedded-rust-implementer]] | 02, 04, 05, 06, 09, 12 | The daily-driver layer: commands, allocator, async, logging, idioms, in-repo pins |
| [[embedded-rust-reviewer]] | 04, 09, 10, 11, 12 | The pitfall list *is* the review checklist; 11 and 12 §3 carry concrete open findings |

## 3. Deliberate coverage gaps — what this brain does NOT know

Recorded so agents say "I don't know" instead of improvising.

- **esp-hal MSRV** and a per-chip **architecture/status matrix** — neither the
  book nor the fetched README published them. Read `rust-version` in the crate
  manifest.
- **`esp-radio` 0.17 → 1.0.0-beta migration** — changelog unread. The one bump
  in the drift table that is not routine.
- **`esp-bootloader-esp-idf` OTA API surface** (confirm/rollback calls) —
  unread. Do not infer from ESP-IDF C by analogy.
- **Per-subcommand `espflash` flags** (`--chip`, `--port`, `--bootloader`,
  `--partition-table`, `--flash-size`, `--after`, …) — subcommands and config
  captured, exact flag spellings not. Use `espflash <subcmd> --help`.
- **`esp-generate` full option list** — README's list is not guaranteed
  exhaustive. `esp-generate list-options` is the authority.
- **Embassy and esp-idf-* upstream currency** — not published on the Espressif
  index, so not version-checked. Embassy/std-side drift is unknown, not zero.
- **`embedded-hal` 1.0.0 release date** — the docs.rs render's date looked like a
  build artifact; deliberately not recorded.
- **Peripherals we have never touched**: ADC/I²S capture depth,
  LEDC/MCPWM, RMT, TWAI/CAN, USB device, low-power cores (`esp-lp-hal`),
  ULP coprocessor, touch sensor, temperature sensor. `esp-lp-hal` is *named*
  in the crate inventory but nothing beyond that was read.
- **ArielOS and RTIC in practice** — book-level description only; neither has
  been used in this repo.
- **Secure boot / flash encryption** — named as bootloader capabilities, not
  studied.
- **ESP32-P4, C5, C61** — listed as supported silicon; no specifics.

## 4. Boundary with the existing hardware agents

The full routing table lives in `12-weftos-anchors.md` §5. The one-line version:

> **This brain owns the language, toolchain, and crate ecosystem. It does not own
> a peripheral, a panel, or an acoustic budget.**

[[esp32-s3-rgb-touch-display]] and [[embedded-acoustic-firmware]] predate this
brain and remain authoritative in their domains. Where a question straddles —
PSRAM is the canonical example, where allocator *strategy* is ours and panel
*timing flags* are theirs — the charter is to name the split and defer the rest
rather than answer past the edge of what is cited.

## 5. Refresh triggers

Re-run the `README.md` refresh procedure when any of these happens:

1. A firmware crate is about to have its `esp-*` pins bumped.
2. `https://docs.espressif.com/projects/rust/` shows a version newer than
   `sources.json` → `crate_versions_at_fetch`.
3. A new firmware crate or a new board class enters the repo.
4. An agent hits a gap in §3 and someone actually goes and reads the source —
   fold the answer in and delete the gap entry.
5. Quarterly, as a floor.
