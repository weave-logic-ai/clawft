# WEFT-668 result — edge-pad set-wise esp-* version bump

**Status:** partial — phase-1 peripherals done; radio-coupled set held (NO-GO)  
**Branch:** `wave0c/weft-668-esp-bump`  
**Base:** `release/0.8-staging`  
**Plane id:** `c116ed13-1dcc-4740-becf-423af4409bce`  
**Date:** 2026-07-30

## Upstream re-confirm (2026-07-30)

Canonical index: https://docs.espressif.com/projects/rust/  
crates.io `max_version` used where the docs index lags (esp-hal).

| Crate | Our pin after this change | Upstream | Action |
|---|---|---|---|
| `esp-hal` | `~1.0` (lock **1.0.0**) | 1.1.1 (docs 1.1.0) | **held** — radio-coupled |
| `esp-rtos` | **0.2.0** | 0.3.0 | **held** — radio-coupled |
| `esp-alloc` | **0.9.0** | 0.10.0 | **held** — radio-coupled |
| `esp-println` | **0.17.0** | 0.17.0 | **bumped** |
| `esp-backtrace` | **0.19.0** | 0.19.0 | **bumped** |
| `esp-bootloader-esp-idf` | **0.5** | 0.5.0 | **bumped** |
| `esp-radio` | `~0.17` (lock **0.17.0**) | 1.0.0-beta.0 (+ 0.18.0) | **NO-GO** phase 2 |

Tilde pins from WEFT-667 (`esp-hal ~1.0`, `esp-radio ~0.17`) **kept**.

## Embassy + esp-idf currency (first check)

| Crate | Pin | crates.io max |
|---|---|---|
| embassy-executor | 0.9.0 | 0.10.0 |
| embassy-time | 0.5.1 | 0.5.1 |
| embassy-sync | 0.8 | 0.8.0 |
| embassy-net | 0.9.0 | 0.9.1 |
| esp-idf-svc | 0.52 | 0.52.1 |
| esp-idf-hal | 0.46 | 0.46.2 |
| esp-idf-sys | 0.37 | 0.37.2 |
| embedded-svc | 0.29 | 0.29.0 |

Not bumped (out of WEFT-668 scope; embassy-executor 0.10 pairs with esp-rtos 0.3).

## esp-radio migration — summary + go/no-go

Sources read before any pin change:

- `esp-radio` CHANGELOG through `v1.0.0-beta.0` (2026-06-03)
- `MIGRATING-0.17.0.md` (0.17 → 0.18)
- `MIGRATING-0.18.0.md` (0.18 → 1.0.0-beta.0)

### Breaking highlights for our `net.rs` (0.17 API)

Our firmware still uses 0.17 shapes:

```rust
esp_radio::init()
esp_radio::Controller
esp_radio::wifi::new(radio, wifi, Config::default())
// → (WifiController, Interfaces)
interfaces.sta
WifiEvent::StaDisconnected
controller.start_async() / connect_async() / wait_for_event()
ModeConfig::Client(ClientConfig::…)
AuthMethod, WifiDevice
```

| From | To (0.18 / 1.0-beta) |
|---|---|
| `esp_radio::init()` + `Controller` | **removed** — wifi/BLE take peripherals directly |
| `wifi::new(...) → (controller, interfaces)` | `WifiController::new(...)` only; `Interface::station()` singleton |
| `interfaces.sta` / `.ap` | `Interface::station()` / `::access_point()` |
| `ModeConfig::Client` / `ClientConfig` | `Config::Station` / `StationConfig` |
| `start_async` / `stop_async` | **removed** — `set_config` starts; drop stops |
| `wait_for_event` / `wait_for_events` | specific `wait_for_disconnect_async` / `EventSubscriber` |
| non-async start/stop/scan/connect | **removed** — async-only |
| `is_connected() → Result` | `is_connected() → bool` (and unstable) |
| SSID as `String` | dedicated `Ssid` type |
| MAC getters on radio | `esp_hal::efuse::interface_mac_address(...)` |
| esp-hal feature `psram` | **gone** on 1.1 — PSRAM always-on module / `PsramMode` |

Also between 0.17 and 0.18: large module reshuffles (`wifi::sta`, `wifi::ap`,
`wifi::scan`), event system rewrite, protocol enum rename for 5 GHz.

### Decision: **NO-GO on 1.0.0-beta.0** (and on 0.18 this wave)

| Factor | Rationale |
|---|---|
| Beta risk | Jumping production CrowPanel firmware to beta is not a neutral update |
| API surface | `net.rs` needs a full rewrite, not a pin bump |
| Coupling | Moving radio forces esp-hal 1.1 + esp-rtos 0.3 + esp-alloc 0.10 + embassy-executor 0.10 |
| Ticket split | Ticket itself says phase radio separately with its own rollback |
| HW gate | Acceptance requires real-hardware flash; host cannot prove Wi-Fi |

**Follow-up:** spawn phase-2 item (suggested name: WEFT-668b / edge-pad
esp-radio 0.18-or-1.0-beta migration) after reading a stable 1.0 if it ships,
or accept 0.18 as interim if needed for security fixes.

## Why the six "one-minor" bumps are not all independent

`cargo +stable update` resolution experiments (esp toolchain not installed on
this host — resolution-only via temporarily bypassing `rust-toolchain.toml`):

| Attempt | Result |
|---|---|
| Full non-radio set (hal 1.1, rtos 0.3, alloc 0.10, println/backtrace/bootloader) + radio ~0.17 | **fail** — `xtensa-lx-rt` `links` conflict (0.22 vs 0.21); rtos-driver 0.3 vs 0.2; alloc `^0.9` excludes 0.10 |
| esp-hal ~1.1 alone + radio ~0.17 | **fail** — same `xtensa-lx-rt` `links` |
| println 0.17 + backtrace 0.19 + bootloader 0.5, rest held | **ok** — locked |

So phase 1 = the **maximum resolvable non-radio subset**. The rest is phase 2
with radio.

## What shipped

### `crates/clawft-edge-pad`

| Dep | Before | After |
|---|---|---|
| esp-println | 0.16.1 | **0.17.0** |
| esp-backtrace | 0.18.1 | **0.19.0** |
| esp-bootloader-esp-idf | 0.4 | **0.5** |
| esp-hal / rtos / alloc / radio | ~1.0 / 0.2 / 0.9 / ~0.17 | unchanged |

Comments document blockers + NO-GO.

### `crates/lgfx-bus-rgb-rs`

| Dep | Before | After |
|---|---|---|
| esp-hal | ~1.0 | ~1.0 (held, lockstep) |
| esp-alloc | 0.9.0 | 0.9.0 (held) |
| dev: esp-println / backtrace / bootloader | 0.16.1 / 0.18.1 / 0.4 | **0.17 / 0.19 / 0.5** |

### Locks

`Cargo.lock` updated in both standalone crates (resolution via host
`cargo +stable update` with esp toolchain override temporarily unset).

### Brain / docs

- `.planning/embedded-rust/brain/notes/12-weftos-anchors.md` §2 drift table + §2.1 embassy/idf + §3 marked resolved (WEFT-667)
- `.planning/embedded-rust/brain/sources.json` `crate_versions_at_fetch` refreshed (incl. embassy/idf)
- `.planning/embedded-rust/brain/coverage-map.md` migration gap marked READ / phase-2 still open

## Build / hardware

| Check | Status |
|---|---|
| Host workspace `scripts/build.sh` | N/A — these crates are **out-of-workspace** (`[workspace]` empty); not in host graph |
| Xtensa firmware build | **not run** — `channel = "esp"` toolchain not installed on this agent host |
| CrowPanel hardware flash | **not run** — required for full AC; phase-1 peripherals only (println/backtrace/bootloader) are low risk vs HAL/radio |

## Acceptance criteria

- [x] Upstream versions re-confirmed (docs index + crates.io)
- [x] Embassy-side and esp-idf-side versions established (§2.1)
- [x] esp-radio migration notes read + summarised; **explicit NO-GO on beta**
- [x] Non-radio crates bumped **as far as the resolver allows**; held crates documented with hard blockers
- [ ] Firmware verified on real hardware — **deferred** (no HW / no esp toolchain here); phase-1 risk low
- [x] Drift table + `sources.json` refreshed
- [x] No ADR required (stack pin change is documented in anchors + this result; no living ADR claimed a frozen minor set)

## Files

- `crates/clawft-edge-pad/Cargo.toml`
- `crates/clawft-edge-pad/Cargo.lock`
- `crates/lgfx-bus-rgb-rs/Cargo.toml`
- `crates/lgfx-bus-rgb-rs/Cargo.lock`
- `.planning/embedded-rust/brain/notes/12-weftos-anchors.md`
- `.planning/embedded-rust/brain/sources.json`
- `.planning/embedded-rust/brain/coverage-map.md`
- `docs/plans/wave-0c-WEFT-668-result.md`

## How to test (for tester)

1. Diff manifests: only println/backtrace/bootloader version bumps + comments.
2. `cd crates/clawft-edge-pad && cargo +stable metadata --no-deps` (or tree) — should resolve with locks above.
3. If esp toolchain available: `cd crates/clawft-edge-pad && cargo build --release` for `xtensa-esp32s3-none-elf` (or crate's default target).
4. Confirm `esp-radio` still `~0.17` / lock 0.17.0; no accidental 1.0-beta.
5. Optional HW: flash CrowPanel; serial should still show `[edge-pad]` / `[net]` lines via esp-println 0.17.
