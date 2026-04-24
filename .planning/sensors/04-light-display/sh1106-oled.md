---
title: "SH1106 1.3\" Monochrome OLED (SSD1306 alternative)"
slug: "sh1106-oled"
category: "04-light-display"
part_numbers: ["SH1106", "128x64 1.3\" OLED"]
manufacturer: "Sino Wealth"
interface: ["I2C", "SPI"]
voltage: "3.3V / 5V (module-level)"
logic_level: "3.3V"
current_ma: "~8 – 20 mA"
price_usd_approx: "$4 – $9"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["display", "oled", "monochrome", "hud", "sh1106", "i2c"]
pairs_with:
  - "./ssd1306-oled.md"
  - "../08-communication/tca9548a-i2c-mux.md"
  - "./ili9341-tft.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=sh1106" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=sh1106" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=sh1106" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-sh1106.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-sh1106-oled.html" }
libraries:
  - "Rust (embedded-graphics): sh1106"
  - "Arduino: U8g2 (sh1106 driver), Adafruit_SH1106"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The SH1106 is a common drop-in alternative to the [SSD1306](./ssd1306-oled.md) controller, typically found on the slightly larger 1.3" 128×64 OLED breakouts. Pin-compatible with SSD1306 modules, but **not** register-compatible — its internal framebuffer is 132 columns wide, with visible pixels starting at column 2. Using an SSD1306 library on an SH1106 display gives a characteristic "content shifted 2 px right, garbage at edges" effect.

## Key specs

- Resolution: 128×64 visible in a 132×64 internal RAM (the 2-column offset).
- Interface: I²C (0x3C / 0x3D) or SPI.
- Voltage: 3.3 V logic; on-board charge pump drives OLED rail.
- Panel size: usually 1.3", slightly larger pixels than SSD1306 0.96".

## Interface & wiring

- Identical pinouts to SSD1306 boards (VCC/GND/SCL/SDA or the SPI variant).
- **Critical:** use the SH1106 driver, not SSD1306. Easy tell at runtime: a column-bar pattern is shifted 2 px and there's a garbage strip at the right.
- I²C speed ceiling tends to be 400 kHz on clones; try 100 kHz first when debugging.

## Benefits

- Larger 1.3" pixels can read from across a bench.
- Cheap and widely available; same form factor as SSD1306.
- Same low power / wide viewing angle as SSD1306.

## Limitations / gotchas

- **Not register-compatible with SSD1306.** This is the #1 "why is my OLED broken" thread on every forum.
- Addressing mode is page-based only (no horizontal auto-increment like SSD1306), which changes how bulk writes work in lower-level drivers.
- Still susceptible to burn-in on static layouts.
- Some clones ship with inverted polarity silk — check VCC/GND before hooking to 3.3 V.

## Typical use cases

- Same as SSD1306 — status HUD, node boot log — when 1.3" readability is preferred.
- Bench instruments where the viewer is farther away.

## Pairs well with

- [`./ssd1306-oled.md`](./ssd1306-oled.md) — know which controller you have; drivers differ.
- [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md) — address clash at 0x3C otherwise.
- [`./ili9341-tft.md`](./ili9341-tft.md) — upgrade path to color.

## Where to buy

- Adafruit (1.3" 128×64 SH1106 breakout), Seeed, SparkFun, AliExpress.

## Software / libraries

- `sh1106` Rust crate + `embedded-graphics` for WeftOS native.
- `U8g2` supports both SSD1306 and SH1106 — pick the right constructor.
- `Adafruit_SH1106` for Arduino-world drop-in.

## Notes for WeftOS

- Auto-detect at boot: send an SH1106-specific command and check for a known response, or expose the controller as a config field. Misconfig shows up as the 2-px shift immediately.
- Treat SSD1306 and SH1106 as the same HAL device class — only the driver byte differs.
