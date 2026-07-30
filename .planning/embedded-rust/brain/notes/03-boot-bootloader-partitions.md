# 03 — Boot, bootloader, partitions, configuration

> Trust tiers and source ids refer to `../sources.json`.

## Espressif boot is two-stage

**First stage — the ROM bootloader.** Sets up architecture-specific registers,
checks the boot mode and reset reason, and loads the second stage bootloader.
It is **burned into ROM: immutable, unflashable**. When something fails here,
no amount of rebuilding your app helps.

**Second stage bootloader.** Loads your application and sets up memory (RAM,
PSRAM, or flash).

`[src: rust-on-esp-book | upstream-official]`

## The second stage is technically optional and practically required

You can run without it, but you lose:

- **OTA support**
- **flash encryption / secure boot**

Currently **only the ESP-IDF bootloader** is supported as the second stage. It
uses the ESP image format and consults a partition table to decide where
binaries live.

`[src: rust-on-esp-book | upstream-official]`

**Reading between the lines**: "only the ESP-IDF bootloader is supported" is
why `esp-bootloader-esp-idf` exists as a crate, and why OTA is coupled to it
(see `08-ota-and-updates.md`). A pure-Rust second-stage bootloader is not an
option you get to choose today.

## Partition tables

Flash holds more than one thing: multiple app images, calibration data,
filesystems, parameter storage. The partition table describes the layout.

Each entry has: a **name (label)**, a **type** (`app`, `data`, or other), a
**subtype**, and the **offset** in flash where the partition is loaded.

If you invoke `espflash` **without** supplying a custom bootloader or partition
table, it applies **defaults**. Custom tables follow the ESP-IDF partition-table
documentation.

`[src: rust-on-esp-book | upstream-official]`

**Practical trap**: defaults are silent. A project that outgrows the default app
partition, or that wants two OTA slots, fails in a way that looks like a flash
or link problem rather than a partitioning problem. If an image "suddenly won't
fit", check the partition table before optimizing the binary.

## Building a custom ESP-IDF bootloader

Ironically, customizing the bootloader for a Rust project requires the C
toolchain:

1. Install ESP-IDF.
2. Create or enter a project (the examples in `esp-idf/examples` are simplest).
3. Modify the bootloader via `idf.py menuconfig`, or by editing `sdkconfig`.
4. Build: `idf.py set-target <CHIP_TARGET> build bootloader`
5. Result lands at `build/bootloader/bootloader.bin`.
6. Deploy with `espflash` / `cargo-espflash` using the `--bootloader` flag, or
   via the `[idf] bootloader = "path"` key in `espflash.toml`.

`[src: rust-on-esp-book, espflash-repo | upstream-official]`

**Scope warning for planning**: this step drags a whole ESP-IDF installation
into an otherwise pure-Rust project. If a task requires a custom bootloader,
that is a real dependency and a real CI cost — budget it explicitly rather than
discovering it late.

## Configuration: esp-config, for settings that aren't cargo features

`esp-config` manages additional configuration for `esp-*` crates that doesn't
fit the cargo-feature model. Available options are listed in each crate's
documentation (they are per-chip, so the chip selector on
`docs.espressif.com/projects/rust/` matters).

`[src: rust-on-esp-book | upstream-official]`

### Two ways to set a config value

**Environment variable**, named after the config key:

```bash
ESP_HAL_CONFIG_PLACE_ANON_IN_RAM=true
```

**`.cargo/config.toml`**, which is the reproducible form:

```toml
# .cargo/config.toml
[env]
ESP_HAL_CONFIG_PLACE_ANON_IN_RAM = "true"
```

Command-line environment variables **take precedence** over
`.cargo/config.toml`.

`[src: rust-on-esp-book | upstream-official]`

### ⚠ Clean build required after changing `[env]`

The book recommends a **clean build** after modifying `.cargo/config.toml`'s
`[env]` section. This is a real footgun: config changes can otherwise appear to
have no effect, sending you off debugging the wrong layer.

`[src: rust-on-esp-book | upstream-official]`

### Multiple configurations (multi-board / multi-chip projects)

Recommended structure:

- a **baseline `.cargo/config.toml`** with the standard build flags — Cargo
  always reads this one
- **one config file per configuration** alongside it in `.cargo/`
- **cargo aliases** to select them:

```toml
run-config-a = "run --config=./.cargo/config_a.toml --release"
```

Reference implementation:
<https://github.com/bjoernQ/esp-hal-multiconfig-example/tree/main>

`[src: rust-on-esp-book | upstream-official]`

**Relevance to us**: WeftOS firmware is per-board today (`clawft-edge-pad` is
ESP32-S3-only, with `esp32s3` hard-coded across every `esp-*` feature list). If
a second board class appears, this alias pattern is the intended answer —
rather than cargo features or a second crate.

### Defining your own config options

Declare them in an `esp_config.yml` (options, defaults, validation). Details
live in the esp-config repo's "Defining Configuration Options" section.

`[src: rust-on-esp-book | upstream-official]`

## Download mode / boot mode selection (the recovery path)

On reset the ROM bootloader **samples strapping pins**:

- strapping pin **high** → **SPI Boot** mode (runs from flash)
- strapping pin **low** → **Download** mode (waits for firmware)

In download mode the chip prints `waiting for download` on serial.

**To exit download mode**: reset the target so the pins are resampled, or use
`--after watchdog-reset` with `espflash` / `esptool` in USB-Serial/JTAG mode.

`[src: rust-on-esp-book | upstream-official]`

**Cross-reference — this interacts with pin assignment.** The
[[esp32-s3-rgb-touch-display]] agent documents that on the CrowPanel the LCD
PCLK lands on **GPIO 0, which is also a strapping pin**: the peripheral drives
it fine *after* boot, and the board is safe only because it doesn't pull that
pin at reset. An external pull on GPIO 0 at reset bricks boot. Any design
review that assigns a signal to a strapping pin must check the reset-time
state, not just the runtime function.
`[src: in-repo-display-agent | in-repo-verified]`
