# 07 — Testing: host-first, then hardware-in-loop

> Trust tiers and source ids refer to `../sources.json`.

## The book's position: test on the host wherever possible

> Test "as much as possible on your host machine, not on the target device."

Three stated reasons: it is **easier to test in CI**, it is **faster**, and it
**won't waste flash write cycles** on your device.

`[src: rust-on-esp-book | upstream-official]`

**This is the highest-leverage architectural instruction in the whole book**,
and it is a *design* instruction disguised as a testing instruction. Host-testable
firmware requires that logic be separated from peripheral access — which means
writing against `embedded-hal` traits rather than concrete `esp-hal` types, so
the logic can be exercised with a mock implementation on x86.

See `09-embedded-rust-idioms.md` §Portability. The trait-generic driver is what
makes host testing possible; it is not merely a portability nicety.

**In-repo proof that this works**: `crates/weftos-leaf-touch-gt911/` is written
generic over any `embedded-hal` I²C implementation and explicitly depends on
"any esp-hal / esp-rtos crate" — *not*. That is why the identical GT911 driver
serves both the `no_std` (`clawft-edge-pad`) and `std`
(`clawft-edge-pad-idf`) firmware ports. Trait-generic drivers bought us both
host-testability and a free std/no_std port.
`[src: in-repo-edge-pad, in-repo-edge-pad-idf | in-repo-verified]`

## Hardware-in-loop testing: embedded-test + probe-rs

The HIL framework is **`embedded-test`** (currently **0.7.1**) driven by
`probe-rs`. Tests use a `#[test]`-like macro, so they read like ordinary Rust
tests but execute on the target.

Install the runner:

```bash
cargo install probe-rs-tools
```

`[src: embedded-test-docs | ecosystem-canon]`

### ⚠ Hard hardware requirement

> You **must** use **only** the `USB-Serial-JTAG` port on your DevKit.

Boards without that port need an **`esp-prog`** or similar programmer. Per
`02-toolchain-and-tooling.md`, built-in USB-JTAG-SERIAL exists on ESP32-C6,
ESP32-H2, ESP32-S3, and ESP32-C3 rev 0.3+.

`[src: rust-on-esp-book | upstream-official]`

**Applies to us**: the CrowPanel routes through a **CH340 USB-UART bridge**, so
HIL testing on that board needs an external probe. Plan for it rather than
discovering it. `[src: in-repo-display-agent | in-repo-verified]`

### Cargo wiring

```toml
[dependencies]
embedded-test = { version = "0.7.0" }

[lib]
harness = false

[[test]]
name = "example_integration_test"
harness = false

[[bin]]
name = "example_binary"
test = false
```

`harness = false` is required — `embedded-test` supplies its own harness in
place of libtest.

`[src: embedded-test-docs | ecosystem-canon]`

### build.rs

```rust
fn main() {
    println!("cargo::rustc-link-arg=-Tembedded-test.x");
}
```

### .cargo/config.toml runner

```toml
[target.thumbv7em-none-eabihf]
runner = "probe-rs run --chip STM32F767ZITx"
```

Substitute your own target triple and chip — for us that is
`xtensa-esp32s3-none-elf` with `--chip esp32s3`, or a
`riscv32imac-unknown-none-elf` / `--chip esp32c6` pair.

`[src: embedded-test-docs | ecosystem-canon]`

**Note the collision**: the runner key is the same one `espflash` uses
(`runner = "espflash flash …"`, per `02-toolchain-and-tooling.md`). A project
doing both flashing-via-espflash and HIL-testing-via-probe-rs needs the
multiple-configuration pattern from `03-boot-bootloader-partitions.md`
§"Multiple configurations" — separate `.cargo/config_*.toml` files plus cargo
aliases — because one `[target.*] runner` cannot serve both.

### Test attributes

- **`#[init]`** — optional init function, called before each test
- **`#[test]`** — marks a test
- **`#[should_panic]`** — passes when the test panics
- **`#[timeout(seconds)]`** — custom timeout; **default is 60 s**
- **`#[ignore]`** — skips
- **`#[cfg(...)]`** — conditional compilation

Async tests are supported with the **`embassy-09`** or **`embassy-010`**
features enabled — match this to your `embassy-executor` version.

`[src: embedded-test-docs | ecosystem-canon]`

Then simply:

```bash
cargo test
```

### Generating a project with it pre-wired

`esp-generate` offers `embedded-test` as an option under the `probe-rs` choice,
which writes all of the above for you.
`[src: rust-on-esp-book, esp-generate-repo | upstream-official]`

## A realistic three-tier testing strategy

Synthesized from the sources above plus what this repo actually does — flagged
as synthesis, not as a documented recommendation:

1. **Host unit tests** (`cargo test` on x86) over trait-generic logic: protocol
   encoding, state machines, hit-testing, geometry, parsing. Fast, runs in CI,
   no hardware. This is where the bulk of coverage should live.
2. **HIL tests** (`embedded-test` + `probe-rs`) for things only real silicon
   proves: peripheral register sequences, DMA descriptor chains, timing,
   interrupt latency, driver bring-up against a real chip on the bus.
3. **Manual hardware verification with a written journal** for what neither
   tier catches — analog behaviour, visual artifacts, EMI, thermal. This repo
   already does this: the `.planning/actors/inkpad-snapshots/` captures and the
   [[esp32-s3-rgb-touch-display]] session-learnings section are exactly tier 3,
   and they caught things (the floating-LSB "Fallout glitch") that no automated
   test could.
   `[src: in-repo-display-agent | in-repo-verified]`

**The tier-3 discipline matters**: an eleven-iteration bring-up that isn't
journaled is an eleven-iteration bring-up you will repeat.
