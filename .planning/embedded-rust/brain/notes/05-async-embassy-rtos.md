# 05 — Async: Embassy, esp-rtos, ArielOS, RTIC

> Trust tiers and source ids refer to `../sources.json`.
> The book is explicit that its async section "does not serve as an async
> tutorial" — it routes you to the Embassy book. This note carries the routing
> plus the Embassy semantics that actually bite.

## Three async/concurrency options, with sharply different maturity

| Option | What it is | Espressif support |
|---|---|---|
| **Embassy** | Async framework for embedded Rust; static task allocation, no heap | Integrated via **`esp-rtos`**. The default choice. |
| **ArielOS** | An OS for "secure, memory-safe, low-power IoT" — multicore scheduler, secure networking, portable drivers, unified build system | Built on the embedded-Rust ecosystem; integrates well with Embassy |
| **RTIC** | Real-Time Interrupt-driven Concurrency, community-supported | **Only ESP32-C3 and ESP32-C6** are supported |

`[src: rust-on-esp-book | upstream-official]`

**Decision shortcut**: on Xtensa (ESP32/S2/S3), RTIC is not available at all —
so the real choice is Embassy (or bare interrupt handlers). On C3/C6 all three
are live. This is another place the Xtensa/RISC-V split changes architecture,
not just tooling.

## Embassy on Espressif goes through esp-rtos — not esp-hal-embassy

`esp-rtos` provides:

- an **interrupt-mode executor**
- a **multicore-aware thread-mode embassy executor**
- the **embassy time driver**
- the **timer waiter queue**

The old `esp-hal-embassy` crate is gone; its functionality merged into
`esp-rtos`. Any tutorial adding `esp-hal-embassy` is stale.

`[src: rust-on-esp-book, esp-hal-1.0-release | upstream-official]`

`esp-rtos` is also the scheduler that **`esp-radio` needs** in order to run
Wi-Fi/BLE in the background — which is why our firmware enables
`esp-rtos = { features = ["esp32s3", "embassy", "esp-radio"] }` rather than just
`embassy`. Radio and async are not independent choices on this platform.
`[src: rust-on-esp-book, in-repo-edge-pad | upstream-official + in-repo-verified]`

## The executor model: cooperative, static, and fair

> "When a task is created, it is polled. The task attempts to make progress
> until it reaches a point where it would be blocked… the task yields execution
> by returning `Poll::Pending`."

Tasks are declared with `#[embassy_executor::task]` and are **statically
allocated at compile time — no heap required**. The executor manages a fixed
number of tasks allocated at startup, though more can be added later.

Fairness is guaranteed: "a task can't monopolize CPU time even if it's
constantly being woken."

`[src: embassy-book, rust-on-esp-book | ecosystem-canon + upstream-official]`

**The consequence people trip on**: cooperative means *a task that never
returns `Pending` starves everything else on that executor*. A busy-wait loop, a
long blocking computation, or a blocking-mode driver call inside an async task
all break the model. There is no preemption to save you — except across
executors (below).

## Interrupt executors give you real priority preemption

`InterruptExecutor` can be driven by an interrupt, so multiple executor
instances at different interrupt priorities let **higher-priority tasks preempt
lower-priority tasks**.

`[src: embassy-book | ecosystem-canon]`

**This is the architectural answer to the starvation problem**: put
latency-critical work (sensor capture, control loop) on a high-priority
interrupt executor and bulk work (networking, rendering, logging) on the
thread-mode executor. Do not solve it by sprinkling `yield_now()`.

## embassy-time: ticks and the per-platform driver

- `Timer::at(Instant)` — completes at an absolute instant
- `Timer::after(Duration)` — completes after a duration
- Default tick rate **32768 Hz**, selectable via `time-tick-<frequency>`
  features; **1000 Hz**, **32768 Hz**, and **1 MHz** are the documented options
- A **Timer Driver** must exist per platform so waits are interrupt-driven
  rather than busy-waited — on Espressif that driver comes from `esp-rtos`

`[src: embassy-book | ecosystem-canon]`

**Tick-rate selection is a real tradeoff**: 1 MHz gives fine-grained timing and
more timer interrupts (power + overhead); 1000 Hz is cheap but quantizes waits
to 1 ms. Pick deliberately if you have either a power budget or a sub-millisecond
requirement.

## embassy-sync primitives

- **`Mutex<RawMutexType, T>`** — mutual exclusion across tasks. The
  `RawMutexType` parameter chooses the criticality (thread-mode vs
  critical-section vs no-op), which is how you express "is this shared across
  interrupt priorities or not."
- **`Channel`** with `Sender` / `Receiver` — enqueue work for later processing
- **`Signal`** — latest-value notification
- **`PubSubChannel`** — multi-consumer broadcast

`[src: embassy-book | ecosystem-canon]`

**⚠ Xtensa PSRAM interaction**: these primitives contain atomics. Per
`04-memory-heap-psram.md`, atomics in PSRAM are broken on Xtensa. Anything
holding an embassy-sync primitive must be allocated from internal SRAM. The
capability-scoped allocator split enforces this structurally.

**In-repo pattern**: `clawft-edge-pad` accumulates touch points into a
`heapless::Vec<TouchPoint, N>` with a static bound, then **moves** (not clones)
the buffer to the publisher task over an `embassy-sync::Channel` on touch-up —
no allocator on the input path at all.
`[src: in-repo-display-agent | in-repo-verified]`

## embassy-net

Provides Ethernet, IP, TCP, UDP, ICMP and DHCP. Async "drastically simplifies"
timeout and concurrent-connection management compared with polled approaches.

`[src: embassy-book | ecosystem-canon]`

Our firmware pairs `embassy-net` (features `dhcpv4`, `medium-ethernet`, `tcp`,
`udp`, `dns`) with `esp-radio` for Wi-Fi and `embedded-io-async` for the byte
streams. `[src: in-repo-edge-pad | in-repo-verified]`

## ⚠ Embassy's own HAL list does not include ESP32 — deliberately

Embassy first-party HALs: `embassy-stm32`, `embassy-nrf`, `embassy-rp`,
`embassy-mspm0`. **ESP32 is listed as externally supported**, alongside WCH
CH32, Microchip PolarFire, and Puya PY32.

`[src: embassy-book | ecosystem-canon]`

**Read the Embassy book accordingly**: its executor, `embassy-time`,
`embassy-sync`, and `embassy-net` chapters apply to us directly. Its HAL and
bootloader chapters do **not** — our HAL is `esp-hal` and our bootloader is the
ESP-IDF one (see `03-boot-bootloader-partitions.md`). Mixing those up is the
most common source of bad advice in this area.

## Our in-repo Embassy pins

embassy-executor **0.9.0** · embassy-time **0.5.1** · embassy-sync **0.8** ·
embassy-futures **0.1** · embassy-net **0.9.0** · `static_cell` **2.1**.

`[src: in-repo-edge-pad | in-repo-verified]`

`static_cell` is the idiomatic companion for handing statically-allocated
resources to `#[embassy_executor::task]` functions, which require `'static`
arguments.
