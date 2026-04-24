---
title: "MAX7219 8×8 LED Matrix + 7-Seg Driver"
slug: "max7219-matrix"
category: "04-light-display"
part_numbers: ["MAX7219", "MAX7221"]
manufacturer: "Analog Devices (formerly Maxim Integrated)"
interface: ["SPI"]
voltage: "5V"
logic_level: "5V (use level shifter from 3.3V MCU)"
current_ma: "~200 – 330 mA per 8x8 at full brightness"
price_usd_approx: "$2 – $6 per 8×8 module"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["display", "led-matrix", "7-segment", "spi", "daisy-chain"]
pairs_with:
  - "./tm1637-7seg.md"
  - "./ws2812b-neopixel.md"
  - "../07-control-actuators/mosfet-drivers.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=max7219" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=max7219" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=max7219" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-max7219.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-max7219-matrix.html" }
datasheet: "https://www.analog.com/en/products/max7219.html"
libraries:
  - "Rust: max7219 crate"
  - "Arduino: LedControl, MD_MAX72XX, MD_Parola"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The MAX7219 (and its EMI-optimized cousin MAX7221) is a serial LED driver that multiplexes up to 64 LEDs — sold most often as 8×8 dot-matrix modules and 8-digit 7-segment bars. Multiple modules daisy-chain over SPI with a shared `CS`/`LOAD` line, giving easy scrolling tickers. It's the retro-charm choice for WeftOS: zero per-pixel addressing cost and a distinctive "Radio Shack" aesthetic.

## Key specs

- Drives up to 64 LEDs (8 digits × 8 segments, or 8×8 matrix).
- 16-bit SPI-like serial interface with `DIN`, `CLK`, `LOAD/CS`.
- 16-step brightness (duty cycle) control.
- Built-in BCD decoder for 7-seg mode; raw bit mode for matrices.
- Voltage: 5 V logic; 3.3 V SPI may work on shorter runs but spec'd for 5 V.
- Daisy-chainable to 8+ modules; each frame shifts data through `DOUT`.

## Interface & wiring

- SPI: `DIN`, `CLK`, `LOAD` (CS) + `VCC` 5 V + `GND`.
- 3.3 V MCU → 5 V logic: use a 74HCT125 or dedicated level shifter on `DIN` and `CLK`.
- Per-module decoupling: 10 µF + 100 nF near the MAX7219's V+.
- Current budget: each 8×8 can draw 300+ mA at full brightness — dimension your 5 V rail for N × 300 mA.
- The "ISET" resistor (usually 10 kΩ on modules) sets max LED current; lower R = brighter but hotter.

## Benefits

- Handles all LED multiplexing internally — MCU just shifts frame bytes.
- Easy to chain — 4× 8×8 gives a 32×8 scrolling-text display for under $10.
- Datasheet-stable part from Analog Devices; has been in production for decades.
- Tolerant of slow SPI — 1 MHz is enough.

## Limitations / gotchas

- **5 V logic.** Driving with 3.3 V without a level shifter is "works on my bench, fails in heat" territory.
- Clones sometimes use MAX7219 ICs that EMI-clock aggressively; MAX7221 variant is better if you care about FCC/CE.
- Full brightness on a full matrix is eye-searing and hot — clamp brightness in firmware.
- Only monochrome (per-module color); no per-pixel color like WS2812.
- Daisy-chain order is physically module-dependent; label which module is "leftmost" to avoid confusion.

## Typical use cases

- Scrolling ticker (news, chat backlog, "now playing").
- Retro dot-clock.
- Big 7-seg counters (score, inventory, health-check heartbeats).
- Bar-graph style status of 8 subsystems.

## Pairs well with

- [`./tm1637-7seg.md`](./tm1637-7seg.md) — similar tier, but MAX7219 scales further with daisy-chaining.
- [`./ws2812b-neopixel.md`](./ws2812b-neopixel.md) — the color upgrade path; same "LED array" abstraction.
- [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) — gate the 5 V rail on battery nodes when the matrix isn't needed.

## Where to buy

- Adafruit (genuine Maxim/ADI parts) for certifiable builds.
- AliExpress "MAX7219 4-in-1" bundles for scrolling tickers (often with clone ICs — fine for dev).

## Software / libraries

- `max7219` crate (Rust) + `embedded-graphics` for WeftOS native.
- `MD_MAX72XX` + `MD_Parola` (Arduino) for rich scrolling/marquee effects.

## Notes for WeftOS

- Model each MAX7219 chain as one logical `1-bit bitmap surface` (e.g., 32×8) behind the same HAL as [NeoPixel matrices](./ws2812b-neopixel.md).
- Expose `brightness` as 0–15 in the HAL and map system-wide brightness policy to it.
- Daisy-chain order should be discovered via a config file, not burned into code.
