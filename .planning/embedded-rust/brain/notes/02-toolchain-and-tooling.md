# 02 — Toolchain and tooling

> Trust tiers and source ids refer to `../sources.json`.

## Install Rust with rustup, not a system package manager

The book is explicit about using rustup rather than distro packages. On Windows
choose either the MSVC (recommended) or GNU ABI.
`[src: rust-on-esp-book | upstream-official]`

## RISC-V targets need no special toolchain

For ESP32-C2/C3/C6/H2 you need only stock Rust plus `rust-src` and the right
target triple:

```bash
rustup toolchain install stable --component rust-src
# or: rustup toolchain install nightly --component rust-src

rustup target add riscv32imc-unknown-none-elf     # ESP32-C2, ESP32-C3
rustup target add riscv32imac-unknown-none-elf    # ESP32-C6, ESP32-H2
```

If you are targeting RISC-V only, **you are done** — skip `espup` entirely.

`[src: rust-on-esp-book | upstream-official]`

**Note the `imc` vs `imac` distinction** — C2/C3 are `imc` (no atomics), C6/H2
are `imac` (with atomics). Getting this wrong produces link errors about
missing atomic intrinsics, not a clear "wrong target" message.

## Xtensa targets need espup, because LLVM has no upstream Xtensa backend

For ESP32 / ESP32-S2 / ESP32-S3:

```bash
cargo install espup --locked      # or cargo binstall espup, or a release binary
espup install
```

`espup install` lays down four things:

1. an **Espressif Rust fork** with support for Espressif targets
2. a **stable toolchain** for RISC-V targets
3. an **LLVM fork** with Xtensa support
4. a **GCC toolchain** used for linking the final binary

On Unix you must then source the environment file espup writes (see espup's
README for the options); on Windows no extra step is needed.

`[src: rust-on-esp-book | upstream-official]`

**Why this exists**: Rust has no official Xtensa support because LLVM does not
yet support Xtensa. `espup` is the bridge, and it is the reason Xtensa and
RISC-V workflows diverge at all.
`[src: rust-on-esp-book | upstream-official]`

## esp-generate — start projects from it, don't hand-roll Cargo.toml

```bash
cargo install esp-generate --locked
esp-generate                                    # interactive TUI
esp-generate --headless -o esp32 -o embassy -o unstable-hal -o alloc -o wifi my-project
```

Discovery commands, which are the authority on what options exist:

```bash
esp-generate list-options
esp-generate explain <option>
```

Chip options: `esp32`, `esp32c2`, `esp32c3`, `esp32c5`, `esp32c6`, `esp32c61`,
`esp32h2`, `esp32s2`, `esp32s3`. Feature options observed: `alloc`, `wifi`,
`embassy`, `unstable-hal`, `defmt`, `log`, `probe-rs`, `embedded-test`,
`stack-protector`. MSRV **1.86**.

`[src: esp-generate-repo | upstream-official]`

**What it is actually for**: it applies "a known set of crates and feature
combinations." The `esp-*` crates have a genuinely awkward
feature-flag matrix (chip selection, `unstable`, `log-04`, `coex`, per-crate
chip features that must all agree). `esp-generate` exists so you don't derive
that matrix by hand — and it can also write editor config for VS Code, Helix,
Neovim and Zed.
`[src: rust-on-esp-book, esp-generate-repo | upstream-official]`

Press `s` in the TUI to generate.

## espflash — the serial flash/monitor path

```bash
cargo install espflash --locked      # or cargo binstall espflash
export ESPFLASH_BAUD=460800          # default baud is conservative
```

Subcommands: `board-info`, `checksum-md5`, `completions`, `erase-flash`,
`erase-parts`, `erase-region`, `flash`, `hold-in-reset`, `list-ports`,
`monitor`, `partition-table`, `read-flash`, `reset`, `save-image`, `write-bin`.

Supported silicon: ESP32, ESP32-C2/C3/C5/C6/C61, ESP32-H2, ESP32-P4,
ESP32-S2/S3.

`-L` / `--log-format` selects `serial` or `defmt` decoding on `flash` and
`monitor`.

`[src: espflash-repo | upstream-official]`

**⚠ Flag spellings**: the fetched README enumerated subcommands and config, but
**not** the full per-subcommand flag tables (`--chip`, `--port`,
`--bootloader`, `--partition-table`, …). Confirm with
`espflash <subcommand> --help` before putting them in a script. Don't guess.

### espflash configuration

Config precedence: **env vars > local config > global config**.

- **Local**: `espflash.toml` in the current or any parent directory
- **Global**: `$HOME/.config/espflash/` (Linux),
  `$HOME/Library/Application Support/rs.esp.espflash/` (macOS),
  `%APPDATA%\esp\espflash\` (Windows) — files `espflash.toml` and
  `espflash_ports.toml`

```toml
baudrate = 460800

[idf]
bootloader = "path"
partition_table = "path"

[flash]
mode = "qio"
size = "8MB"
frequency = "80MHz"
```

Env vars: `ESPFLASH_PORT`, `ESPFLASH_BAUD`, `MONITOR_BAUD`.

`[src: espflash-repo | upstream-official]`

### cargo-espflash and `cargo run`

`cargo-espflash` is the cargo-extension sibling. Either way, the ergonomic
setup is a runner in `.cargo/config.toml`:

```toml
runner = "espflash flash --baud=921600 --monitor /dev/ttyUSB0"
```

…after which `cargo run --release` compiles, flashes, and opens the monitor in
one step. `[src: espflash-repo, rust-on-esp-book | upstream-official]`

## probe-rs — the debug/JTAG path (and the only path for HIL tests)

Optional for flashing, **mandatory for `embedded-test`** and for real
debugging. Install per <https://probe.rs/docs/getting-started/>, or
`cargo install probe-rs-tools` for the test runner.

**Built-in USB-JTAG-SERIAL** (no external probe needed): ESP32-C6, ESP32-H2,
ESP32-S3, and ESP32-C3 **rev 0.3+**. Everything else needs an external
programmer such as **ESP-Prog**.

`[src: rust-on-esp-book, embedded-test-docs | upstream-official + ecosystem-canon]`

**Pairing rule**: `defmt` + `probe-rs` is the combination the book recommends
for Espressif chips. `log` + `espflash monitor` is the other coherent pair. See
`06-logging-and-observability.md`.

## esp-config and its TUI

```bash
cargo install esp-config --features=tui --locked
```

`esp-config` handles build settings that don't fit cargo features. Full
mechanics in `03-boot-bootloader-partitions.md` §Configuration.
`[src: rust-on-esp-book | upstream-official]`

## Simulation and container tooling

- **Wokwi** (<https://wokwi.com/rust>) — browser simulator with Rust-on-ESP32
  support. Real option for CI/demo when hardware isn't attached.
- **`wokwi-server`** — bridges VS Code Remote Containers to Wokwi.
- **`esp-web-flash-server`** — flashing from inside a Remote Container.

`[src: awesome-esp-rust | ecosystem-canon]`

## Training material for hands-on ramp-up

- **Embedded Rust (no_std) on Espressif** —
  <https://docs.espressif.com/projects/rust/no_std-training/>. Targets
  ESP32-C3 (ESP32-C3-DevKit-RUST-1). Exercise ladder: Panic! → Blinky → button
  poll → button interrupt → **DMA over SPI** → HTTP client → defmt.
  **⚠ Self-declared out of date**, rewrite in progress on `feat/overhaul`.
  Use it for exercise *shape*, not for current API.
  `[src: no-std-training | upstream-official]`
- **Embedded Rust on Espressif (std)** — <https://esp-rs.github.io/std-training/>.
  The std-path course, on the community-maintained crates.
  `[src: std-training | ecosystem-canon]`
- **`awesome-esp-rust`** — CC0-licensed inventory of ~30 std and ~30 no_std
  example projects, tools, blogs, and video courses. The fastest way to find
  "someone already did this on this chip."
  `[src: awesome-esp-rust | ecosystem-canon]`

Community: Matrix room `#esp-rs:matrix.org`. `[src: awesome-esp-rust]`
