---
title: "Rohm BH1750 Digital Ambient Light Sensor (lux)"
slug: "bh1750-lux"
category: "05-environmental"
part_numbers: ["BH1750", "BH1750FVI", "GY-30"]
manufacturer: "Rohm"
interface: ["I2C"]
voltage: "3.3V"
logic_level: "3.3V"
current_ma: "~120 µA measuring"
price_usd_approx: "$1 – $4"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["lux", "ambient-light", "i2c"]
pairs_with:
  - "./tsl2561.md"
  - "./apds9960.md"
  - "../04-light-display/eink-epaper.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=bh1750" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=bh1750" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=bh1750" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-bh1750.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-bh1750.html" }
datasheet: "https://www.rohm.com/products/sensors-mems/ambient-light-sensor-ics/bh1750fvi-product"
libraries:
  - "Rust: bh1750 crate"
  - "Arduino: BH1750, hp_BH1750"
  - "ESPHome: bh1750"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The BH1750 is Rohm's single-chip I²C ambient-light sensor — outputs lux directly in 16-bit counts. Dollar-tier, human-eye spectral response, good enough for "is the room lit?". For WeftOS, it's the default ambient-light source: informs display brightness policy, night-mode triggers, e-paper vs TFT selection, and plant-light logging.

## Key specs

- Range: 1–65535 lux.
- Resolution: 0.5, 1, or 4 lux modes.
- Spectral response: tuned to human eye (photopic).
- Interface: I²C, address 0x23 (ADDR low) or 0x5C (ADDR high).
- Measurement time: 16 ms (low-res) or 120 ms (high-res).

## Interface & wiring

- I²C: `SDA`, `SCL`, `VCC`, `GND`, `ADDR` (tie to GND or VCC).
- 3.3 V; 5 V will damage the die.
- Mount behind an IR-blocking window if used outdoors — direct sun IR inflates readings slightly.

## Benefits

- Direct lux output — no curve fitting, no IR/visible ratio math.
- Tiny and cheap; fits in any node.
- Human-eye-weighted, so "dim" looks dim and "bright" looks bright.

## Limitations / gotchas

- Only one address alternative (0x23 / 0x5C); two max per bus without a mux.
- Not IR-immune — direct sunlight through a cheap window reads high.
- No gain control beyond the three measurement modes — in very dim (< 1 lux) it's just noise.
- Some clone boards label ADDR inverted; verify by scanning both 0x23 and 0x5C.

## Typical use cases

- Display auto-brightness policy for [SSD1306 / ILI9341 / e-paper](../04-light-display/).
- Night / day mode for WeftOS shell UI.
- Plant-light logging under a grow lamp.
- Occupancy hint (lights on ⇒ someone's in the room).

## Pairs well with

- [`./tsl2561.md`](./tsl2561.md) — higher-range, separate IR + visible channels; both can cross-check.
- [`./apds9960.md`](./apds9960.md) — if you also want gesture / RGB / proximity on the same board.
- [`../04-light-display/eink-epaper.md`](../04-light-display/eink-epaper.md) — e-paper is readable in direct sun but not in the dark; BH1750 decides which display to wake.

## Where to buy

- Adafruit, SparkFun, Seeed for clean breakouts.
- AliExpress GY-30 boards for $1 each; perfectly fine for indoor use.

## Software / libraries

- `bh1750` Rust crate (embedded-hal).
- `BH1750` / `hp_BH1750` for Arduino (the latter supports all measurement modes).
- ESPHome `bh1750` component.

## Notes for WeftOS

- Expose as a system-wide "ambient_lux" channel; all display / LED drivers subscribe to it for brightness policy.
- Use hysteresis (e.g., night-mode triggers at < 20 lux for 30 s; day-mode at > 100 lux for 30 s) to avoid flicker when clouds pass.
- Mount behind a frosted window — specular sun spots through a clear lens cause spikes.
