# 09 — Embedded Rust idioms: typestate, singletons, traits, concurrency

> Trust tiers and source ids refer to `../sources.json`.
> Source is **The Embedded Rust Book** — `ecosystem-canon`. Its examples are
> Cortex-M / STM32 and its register names do **not** transfer to Espressif. The
> **idioms** do, and they are what `esp-hal`'s API shape is built from.

## The four-layer stack

```
Board Crate            (pre-configured for one dev kit)
    ↓ wraps
HAL Crate              (implements embedded-hal traits)   ← esp-hal
    ↓ wraps
PAC                    (raw registers, svd2rust-generated)
    ↓ uses
Micro-architecture     (cortex-m / xtensa-lx / riscv)
```

`[src: embedded-rust-book | ecosystem-canon]`

Knowing which layer you are at tells you what kind of bug you have. Register
bit-twiddling problems are PAC-layer; "the type won't let me" problems are
HAL-layer; portability problems are trait-layer.

## Peripherals as singletons — enforced by ownership, not convention

```rust
let p = tm4c123x::Peripherals::take().unwrap();
```

`take()` enforces that there is "only one `SYST` structure in our entire
program," preventing concurrent misuse. The HAL then **consumes** raw
peripherals via `constrain()` / `split()` and hands back higher-level
abstractions.

HAL functions require borrowed resources (e.g. `&clocks`,
`&sc.power_control`) which **statically prevents misconfiguration** — you cannot
configure a peripheral before the clock tree it depends on exists.

`[src: embedded-rust-book | ecosystem-canon]`

**esp-hal equivalent**: `esp_hal::init(config)` returns a `Peripherals` struct
and you move fields out of it. The same rule applies — a peripheral is an owned
value, and if you find yourself wanting two handles to one peripheral, the
design is wrong, not the API.

## Register access through closures, not read-modify-write by hand

```rust
pwm.ctl.write(|w| w.globalsync0().clear_bit());
pwm.ctl.modify(|r, w| w.globalsync0().clear_bit());
```

`write()` sets the whole register (unnamed fields reset); `modify()` does a
read-modify-write **atomically with respect to your own code**, which "avoids
C-style read-modify-write bugs" by making the composition a closure rather than
three statements you might interleave.

`[src: embedded-rust-book | ecosystem-canon]`

**Review heuristic**: raw `read().bits()` / `write().bits()` in a HAL-level
codebase is a smell — it discards the type-safe field accessors. It is
legitimate when porting register sequences from a C reference (which is exactly
what `crates/lgfx-bus-rgb-rs/` does), but it should be *labelled as such* with
the C source cited, because the compiler can no longer help.

## Typestate programming — invalid states don't compile

```rust
let mut porta = p.GPIO_PORTA.split(&sc.power_control);
porta.pa1.into_af_push_pull::<hal::gpio::AF1>(&mut porta.control)
```

The pin's **type changes** as it is configured; a pin must be transitioned to
alternate-function mode before a peripheral will accept it, and skipping that is
a compile error rather than a runtime surprise.

The payoff is stated plainly: "the Rust compiler can use it to perform a bunch
of checks… then generate machine-code which is pretty close to hand-written
assembler!" The type-level constraints **compile away entirely** — closures
monomorphize, `unsafe` register writes stay isolated and reviewable.

`[src: embedded-rust-book | ecosystem-canon]`

**Design instruction, not trivia**: when writing a driver, encode
configuration state in the type where the state has a *correctness*
consequence. When it doesn't, don't — typestate on incidental state produces
generic-parameter soup for no safety gain.

## Portability: write drivers against embedded-hal, not against esp-hal

`embedded-hal` **1.0.0** (MSRV **1.60**) is the seam that makes driver crates
reusable across microcontroller families, embedded Linux, and other platforms.

Five modules: **`delay`**, **`digital`**, **`i2c`**, **`pwm`**, **`spi`**.

Design rules the traits follow:

- **erase device-specific details**
- **be generic within a device and across devices**
- be **minimal**, hence easy to implement and **zero cost**, yet **highly
  composable**
- **all trait methods are fallible**, "so that they can be used in any possible
  situation" — an infallible implementation can still be provided per-platform

`[src: embedded-hal-docs | ecosystem-canon]`

Companion crates:

| Crate | Role |
|---|---|
| `embedded-hal-async` | async variants of the traits |
| `embedded-hal-nb` | polling variants via the `nb` crate |
| `embedded-hal-bus` | **SPI and I²C bus sharing** |
| `embedded-io` | byte-stream I/O — **replaces the old serial traits** |
| `embedded-can` | CAN bus |

`esp-hal` implements `embedded-hal`, `embedded-io`, and `rand_core` (for the
hardware RNG). `[src: rust-on-esp-book | upstream-official]`

**`embedded-hal-bus` deserves attention**: sharing one SPI bus across several
chip-selects, or one I²C bus across several devices, is where hand-rolled code
usually goes wrong. Our firmware pins `embedded-hal-bus = "0.3"`.
`[src: in-repo-edge-pad | in-repo-verified]`

**This is the note that makes host testing possible** — see
`07-testing-host-and-hil.md`. A driver generic over `embedded_hal::i2c::I2c`
can be unit-tested on x86 against a mock. A driver that names
`esp_hal::i2c::master::I2c` cannot. Our `weftos-leaf-touch-gt911` crate took the
first path and consequently serves both the std and no_std firmware ports
unchanged.

## Concurrency: the hardware gives you one guarantee, and only one

The Embedded Rust Book's model rests on: "exception handlers can **not** be
called by software… **reentrancy is not possible**." Therefore:

```rust
#[exception]
fn SysTick() {
    static mut COUNT: u32 = 0;
    *COUNT += 1;  // safe: the handler cannot be concurrently invoked
}
```

"The absence of concurrent invocations of the same handler ensures that there
are no reentrancy issues, even if the handler uses static mutable variables."

The same property enables lazy init inside a handler:

```rust
#[exception]
fn SysTick() {
    static mut STDOUT: Option<HostStream> = None;
    if STDOUT.is_none() {
        *STDOUT = hio::hstdout().ok();
    }
}
```

`[src: embedded-rust-book | ecosystem-canon]`

### ⚠ The guarantee breaks in exactly the case we ship

> "In a multicore system… proper synchronization mechanisms need to be
> employed… locks, semaphores, or atomic operations."

`[src: embedded-rust-book | ecosystem-canon]`

**ESP32 / ESP32-S3 are dual-core.** Our firmware deliberately splits work
across Core 0 (touch ISR) and Core 1 (embassy executor, display, network). So
the single-handler non-reentrancy guarantee is **not** sufficient for us, and
`static mut` in a handler is **not** automatically safe on this platform.

`[src: in-repo-display-agent | in-repo-verified]`

**And the synchronization we must reach for is atomics-based — which
`04-memory-heap-psram.md` says is broken in PSRAM on Xtensa.** Those two facts
compose into a hard constraint:

> On Xtensa multicore, cross-core synchronization requires atomics, and atomics
> require internal SRAM. Therefore every cross-core shared structure must be
> SRAM-allocated.

That is the load-bearing reason the capability-scoped allocator split in
`04-memory-heap-psram.md` is architecture, not micro-optimization.

### Interrupt priority and nesting

Interrupts have "programmable priorities which determine their handlers'
execution order"; higher-priority handlers preempt lower-priority ones. You must
"clear the reason causing the interrupt to trigger to prevent re-entering the
interrupt handler endlessly."

`[src: embedded-rust-book | ecosystem-canon]`

**Failing to clear the interrupt source** is the classic embedded lockup: the
device appears hung with no panic and no log, because it is servicing the same
interrupt forever. Check this first when firmware goes silent without resetting.

The Embassy-layer counterpart is `InterruptExecutor` for priority preemption
between async tasks — see `05-async-embassy-rtos.md`.
