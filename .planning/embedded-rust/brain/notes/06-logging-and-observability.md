# 06 — Logging and observability

> Trust tiers and source ids refer to `../sources.json`.

## Two frameworks, and they pair with different transports

**`defmt`** — "a highly efficient logging framework designed for
resource-constrained environments." Compact, **binary-encoded** log messages,
which removes the overhead of string-based logging: format strings live on the
host side, so the device transmits an index plus arguments rather than text.

**`log`** — the widely adopted Rust logging **facade**: `info!`, `warn!`,
`error!`, etc. Espressif ships a logger implementation in **`esp-println`**,
though custom implementations are fine.

`[src: rust-on-esp-book | upstream-official]`

## The pairing rule

The book's recommendation: **pair `defmt` with `probe-rs`** for optimal results
on Espressif chips. `[src: rust-on-esp-book | upstream-official]`

The other coherent pairing is **`log` + `esp-println` + `espflash monitor`**.

`espflash` bridges both: `-L` / `--log-format` accepts `serial` or `defmt`,
so `espflash monitor -L defmt` will decode defmt frames off the serial link.
`[src: espflash-repo | upstream-official]`

**How to choose**, concretely:

- **Tight flash budget, or high log volume in a hot path** → `defmt`. The
  string-table-on-host property is the whole point; it also cuts the formatting
  code out of the binary.
- **Porting existing code, or depending on crates that log via the `log`
  facade** → `log`. Third-party driver crates overwhelmingly emit `log`, not
  `defmt`, and bridging is friction.
- **Doing HIL testing** → you are on `probe-rs` already
  (`07-testing-host-and-hil.md`), so `defmt` is nearly free.

## Let esp-generate wire it

Whichever you choose, "`esp-generate` will make sure that everything is set up
correctly" — logging is a generation-time option (`defmt` or `log`), which
avoids hand-assembling the feature flags.
`[src: rust-on-esp-book, esp-generate-repo | upstream-official]`

**Why that matters here specifically**: the `esp-*` crates encode the log
backend in *features*, and they must agree across crates. Our firmware carries
`esp-println = { features = ["esp32s3", "log-04"] }` and
`esp-radio = { features = [..., "log-04"] }` — the `log-04` feature (log 0.4
compatibility) has to be set on **every** crate that logs, or you get either
missing output or a duplicate-logger conflict.
`[src: in-repo-edge-pad | in-repo-verified]`

## esp-backtrace — panics that tell you something

`esp-backtrace` "provides backtraces support." In practice it is what turns a
silent reset into a diagnosable panic.

Our pin shows the intended feature shape:
`esp-backtrace = { version = "0.18.1", features = ["esp32s3", "panic-handler", "println"] }`
— it supplies the `#[panic_handler]` and routes output through `esp-println`.

`[src: rust-on-esp-book, in-repo-edge-pad | upstream-official + in-repo-verified]`

**Review check**: exactly one crate in the binary may provide `#[panic_handler]`.
If you see a duplicate-panic-handler link error, something else (often a second
`esp-backtrace` version, or `panic-halt`) is also claiming it.

## Logging is a binary-size lever, not just a debugging tool

The book lists "filter unnecessary log messages" among its binary-size
recommendations, and Embassy's optimization guidance goes further — building
`core` with `panic_immediate_abort` "eliminates fmt code," which is largely
*formatting machinery pulled in by logging*.

`[src: rust-on-esp-book, embassy-book | upstream-official + ecosystem-canon]`

See `10-optimization-size-and-memory.md`. The short version: `log` with runtime
formatting is one of the largest single contributors to firmware size, and
`defmt` exists substantially to remove it.

## ⚠ Observability has a hardware constraint on the HIL path

You **must** use only the `USB-Serial-JTAG` port for `probe-rs`-based work
(`07-testing-host-and-hil.md`). A board wired through a **CH340 USB-UART
bridge** — like our CrowPanel — gives you serial logging but **not** the JTAG
path, so `defmt`-over-probe-rs and HIL tests are unavailable without an
external `esp-prog`.

Additionally measured in-repo: the CH340 path **loses bytes under sustained
throughput** when the host is simultaneously under heavy CPU load (4096-byte
reads returning ~3900–4080 bytes). Don't trust a serial capture taken during a
release build.

`[src: rust-on-esp-book, in-repo-display-agent | upstream-official + in-repo-verified]`
