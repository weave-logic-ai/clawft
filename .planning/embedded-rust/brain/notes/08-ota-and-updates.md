# 08 — OTA and firmware updates

> Trust tiers and source ids refer to `../sources.json`.

## What OTA buys, and what it requires

OTA enables "updating an application *without* the need of production flashing
tools."

Success depends on the **bootloader** supporting "switching, replacement and
rollback of OTA images." OTA is therefore a *bootloader* capability that the
application cooperates with — not an application feature you can add later
without touching boot.

`[src: rust-on-esp-book | upstream-official]`

## The support chain is narrow

- Only the **ESP-IDF bootloader** is supported as a second-stage bootloader.
- OTA is exposed through the **`esp-bootloader-esp-idf`** crate ("offers
  additional support for the ESP-IDF 2nd stage bootloader, including OTA").
- The esp-hal repository ships a **small OTA example** demonstrating the core
  building blocks, plus instructions for creating OTA binaries with `espflash`.

`[src: rust-on-esp-book | upstream-official]`

Current upstream version: `esp-bootloader-esp-idf` **0.5.0**.
`[src: esp-rust-docs-index | upstream-official]`

## Three preconditions to check before promising OTA in a plan

Derived from the above plus `03-boot-bootloader-partitions.md` — treat as a
planning checklist:

1. **You are running the second-stage ESP-IDF bootloader.** Skipping it costs
   you OTA *and* flash encryption / secure boot.
2. **Your partition table has the OTA slots.** Two app partitions plus an
   `otadata` partition. `espflash` applies **defaults** when no partition table
   is supplied, and the default layout is not necessarily an OTA layout —
   this is silent until an update fails.
3. **Your flash is big enough for two app images.** Two slots means the app
   budget is roughly half of flash, minus bootloader, partition table, NVS, and
   any filesystem.

**Concrete example of #3 biting**: `clawft-edge-pad` runs on an
ESP32-S3-WROOM-1 **N4R8 — 4 MB flash** — and is documented as
"**Flash app partition (4 MB) — single-app, no OTA**." That is a deliberate
call: at 4 MB, dual-slot OTA would roughly halve the available app space. If OTA
becomes a requirement for that device class, it is a **hardware** decision
(an N8R8 module or larger) rather than a firmware one.
`[src: in-repo-display-agent | in-repo-verified]`

## Rollback is the part people forget

The book names "switching, replacement and **rollback**" together. A rollback
story requires the application to *confirm* a new image is good; otherwise a
bad update that boots far enough to look alive but not far enough to be useful
will happily stay selected.

**⚠ Unverified detail**: the exact confirm/rollback API surface in
`esp-bootloader-esp-idf` 0.5.0 was **not** read during this corpus build. Before
implementing, read the crate docs and the esp-hal OTA example — do not infer the
API from ESP-IDF's C `esp_ota_mark_app_valid_cancel_rollback()` by analogy.

## Related security surface

Flash encryption and secure boot are listed alongside OTA as second-stage
bootloader capabilities. If firmware images are going to travel over a network,
image authenticity belongs in the same design conversation as the update
mechanism.

`[src: rust-on-esp-book | upstream-official]`

**WeftOS-specific note**: our devices already carry **ed25519 keypairs** and
sign their emissions (ADR-025 / ADR-057; both firmware crates pin
`ed25519-dalek` + `blake3` deliberately so the signing path is identical across
the std and no_std ports). Any OTA design should be examined for whether image
signing can reuse that identity infrastructure rather than introducing a second,
parallel trust root.
`[src: in-repo-edge-pad, in-repo-edge-pad-idf | in-repo-verified]`
That is a **design question flagged, not a decision made** — it needs an ADR.
