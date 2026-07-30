# 04 — Memory, heap, and PSRAM

> Trust tiers and source ids refer to `../sources.json`.
> **This note contains the corpus's canonical trust-tier conflict.** Read §"The
> documented API panics on our hardware" before writing allocator code.

## Why the book argues against a heap

Two costs, both real on a microcontroller:

- **Fragmentation** — "over time, dynamic allocation can cause *fragmentation*:
  small, scattered allocations may prevent large ones even if total memory is
  available." On a device that runs for months without reboot, this is a
  latent, time-delayed failure.
- **Runtime overhead** — allocation and freeing cost cycles, plus whatever
  overhead the chosen allocator adds.

`alloc` is nevertheless available under `no_std`, giving you `Vec` and `Box`.

`[src: rust-on-esp-book | upstream-official]`

**The idiomatic middle ground** (and what our firmware does): use `heapless`
for anything on a hot or interrupt path — `heapless::Vec<T, N>` with a static
`N` — and reserve the heap for large, long-lived, allocate-once buffers like
framebuffers and network stacks. `[src: in-repo-edge-pad | in-repo-verified]`

## Reclaimed RAM — free memory hiding in the bootloader's leftovers

Espressif chips have **non-contiguous** memory layouts, and the second-stage
bootloader's working memory is dead space once boot is done. You can reclaim it:

```rust
// Use 64kB in dram2_seg for the heap, which is otherwise unused.
heap_allocator!(#[ram(reclaimed)] size: 64000);
```

`[src: rust-on-esp-book | upstream-official]`

This is nearly free RAM. On a chip with a few hundred KiB total, 64 KiB is not
a rounding error — it is worth reaching for before adding PSRAM to a design.

## PSRAM, and the Xtensa atomics landmine

External Pseudostatic RAM extends memory beyond the "few hundred kilobytes of
internal RAM." But:

> **On Xtensa chips, atomics in PSRAM do not work correctly — they can cause
> data races and defeat their purpose.**
>
> On RISC-V chips, PSRAM works correctly with atomics.

`[src: rust-on-esp-book | upstream-official]`

**This is the single most dangerous fact in the corpus**, because the failure
mode is a silent data race rather than a crash or a compile error — and because
it lands squarely on ESP32 / S2 / **S3**, which is what WeftOS ships.

Consequences to enforce in review:

- Never place an `AtomicUsize`, a lock, or any structure whose synchronization
  relies on atomics into PSRAM on an Xtensa target.
- Be suspicious of *transitively* PSRAM-allocated types: a `Vec` or `Box` of
  something containing an atomic, allocated from a PSRAM region, is the bug.
  A `Mutex`, an `Arc`, and most channel implementations contain atomics.
- The safe pattern is the capability-scoped allocation described below:
  synchronization primitives live in internal SRAM, bulk data lives in PSRAM.

## One global allocator, multiple regions

> "You can only have **one global allocator**" — but it may use **multiple
> regions**.

For genuinely multiple allocators, the book points at the `allocator_api`
nightly feature or the `allocator_api2` crate alongside `esp-alloc`.

`[src: rust-on-esp-book | upstream-official]`

## ⚠ The documented API panics on our hardware — the measured fix

**Trust-tier conflict, resolved in favour of `in-repo-verified`.**

`esp_alloc::psram_allocator!` — the macro that works in the
`infinition/waveshare-watch-rs` reference firmware — **panics** on our
ESP32-S3-WROOM-1 **N4R8 / AP_3v3** board, inside
`linked_list_allocator-0.10.6/src/hole.rs:331`
(`hole_size >= size_of::<Hole>()`).

`[src: in-repo-display-agent | in-repo-verified]`

The working, hardware-verified pattern ("fix-B heap split") — **internal SRAM
first, PSRAM added as a capability-tagged region**:

```rust
esp_alloc::heap_allocator!(size: 160 * 1024);              // Internal SRAM FIRST
let (psram_ptr, psram_len) =
    esp_hal::psram::psram_raw_parts(&peripherals.PSRAM);
unsafe {
    esp_alloc::HEAP.add_region(esp_alloc::HeapRegion::new(
        psram_ptr,
        psram_len,
        esp_alloc::MemoryCapability::External.into(),
    ));
}
```

Then allocate only the big buffer from PSRAM, explicitly:

```rust
esp_alloc::HEAP.alloc_caps(MemoryCapability::External.into(), layout)
```

**Why this shape is right, not just a workaround**: capability-less `alloc`
(WiFi, embassy, `Vec`, `heapless`) is served from SRAM and *never* touches
PSRAM. Only the framebuffer requests `External`. That gives you the Xtensa
atomics safety property above **structurally** rather than by discipline — the
synchronization-bearing allocations physically cannot land in PSRAM.

160 KiB of SRAM heap held WiFi + embassy-net + mesh comfortably on the tested
workload.

Mechanical detail: `psram_raw_parts(&PSRAM)` needs the peripheral handle, so it
must live in `main.rs`, not inside a driver constructor.

`[src: in-repo-display-agent | in-repo-verified]`

## PSRAM bandwidth is a shared resource, and contention looks like a logic bug

Measured on our hardware: with the heap living in PSRAM alongside a
framebuffer, WiFi/embassy/mesh allocations contend with GDMA's continuous
~24 MB/s framebuffer read. The symptom was "blocks shifting / blinking in
patches all over the screen" — indistinguishable at first glance from a
rendering bug.

The decisive diagnostic was a **static-grid test**: draw once, disable mesh,
WiFi, and touch. A write-once framebuffer that *still* glitches proves
contention; one that stays steady proves the cause is downstream.

`[src: in-repo-display-agent | in-repo-verified]`

**Generalizable lesson**: when a DMA-fed peripheral and the allocator share a
memory bus, "intermittent visual/data corruption" is a *bandwidth* hypothesis
before it is a *logic* hypothesis. Build the write-once diagnostic first.

## Configurable memory placement

`esp-config` exposes placement options — the book's example is
`ESP_HAL_CONFIG_PLACE_ANON_IN_RAM`. See `03-boot-bootloader-partitions.md`
§Configuration for the mechanics, including the **clean-build requirement**
after editing `.cargo/config.toml`'s `[env]`.
`[src: rust-on-esp-book | upstream-official]`

Related: `#[esp_hal::ram]` places code in IRAM. That is a *latency* tool rather
than a capacity tool, and it matters for ISRs —
see `10-optimization-size-and-memory.md`.
