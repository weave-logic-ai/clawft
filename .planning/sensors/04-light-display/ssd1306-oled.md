---
title: "SSD1306 0.96\" / 1.3\" Monochrome OLED"
slug: "ssd1306-oled"
category: "04-light-display"
part_numbers: ["SSD1306", "128x64 OLED", "128x32 OLED"]
manufacturer: "Solomon Systech"
interface: ["I2C", "SPI"]
voltage: "3.3V / 5V (module-level)"
logic_level: "3.3V"
current_ma: "~8 – 20 mA (content dependent)"
price_usd_approx: "$3 – $8"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["display", "oled", "monochrome", "hud", "i2c"]
pairs_with:
  - "../08-communication/tca9548a-i2c-mux.md"
  - "./sh1106-oled.md"
  - "./ili9341-tft.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=ssd1306" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=ssd1306" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=ssd1306" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-ssd1306.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-ssd1306-oled.html" }
libraries:
  - "Rust (embedded-graphics): ssd1306"
  - "Arduino: Adafruit_SSD1306 + Adafruit_GFX, U8g2"
  - "ESP-IDF: u8g2-hal-esp-idf"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The SSD1306 drives small passive-matrix monochrome OLED panels — overwhelmingly the 0.96" 128×64 and 128×32 breakouts found on every hobby bench. For WeftOS they are the go-to "status HUD": boot messages, Wi-Fi state, current task, a few lines of telemetry. OLED means no backlight, high contrast, and viewable from nearly any angle.

## Key specs

- Resolution: 128×64 (0.96" and 1.3") or 128×32 (0.91").
- Controller: SSD1306 (Solomon Systech). Internal 1 KB RAM framebuffer.
- Interface: I²C (default 0x3C, sometimes 0x3D via solder jumper) or SPI.
- Voltage: 3.3 V logic; modules include on-board charge pump for the 7–9 V OLED rail.
- Current: a few mA idle, up to ~20 mA all-white.

## Interface & wiring

- **I²C**: `SDA`, `SCL`, `VCC`, `GND`. Pull-ups are on the module (usually 4.7 kΩ). Use 400 kHz Fast-mode; 1 MHz Fast-mode+ works on most but not all clones.
- **SPI**: `MOSI`, `SCLK`, `CS`, `DC`, `RES`; faster full-frame refresh (>60 Hz) than I²C (~20 Hz at 400 kHz).
- OLEDs at 0x3C collide with many other sensors (e.g., some IMUs); use a [TCA9548A mux](../08-communication/tca9548a-i2c-mux.md) or switch to SPI if you need more than one.

## Benefits

- High contrast, wide viewing angle, zero backlight to manage.
- Cheap and ubiquitous; libraries are mature in every language.
- Low idle current — well suited to always-on status displays.

## Limitations / gotchas

- **Burn-in.** Leaving the same static layout on for weeks visibly etches pixels. Rotate layouts, invert periodically, or shift content by 1 px.
- Only 128×64 / 128×32 — not enough for dense UIs.
- I²C at 400 kHz means sub-20 Hz full-frame redraw; tear-sensitive animation needs SPI.
- Some clones ship with a corrupted reset sequence; always issue a controller reset at boot.
- No grayscale — dithering is the only way to fake it and looks noisy.

## Typical use cases

- Node boot / status / IP HUD.
- "Now playing" or "now listening" on a voice terminal.
- Menu navigation with a rotary encoder on a dev node.
- Small graph of one telemetry channel (e.g., temperature over last hour).

## Pairs well with

- [`./sh1106-oled.md`](./sh1106-oled.md) — direct alternative; don't mix drivers silently.
- [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md) — required if you want >1 at the default 0x3C.
- [`./ili9341-tft.md`](./ili9341-tft.md) — upgrade path when you need color and real fonts.

## Where to buy

- Adafruit / SparkFun / Seeed (clear panel QC, breakouts with level shifters).
- AliExpress bulk (roughly $2 each; expect 5–10 % with dim or dead columns).

## Software / libraries

- `ssd1306` Rust crate + `embedded-graphics` for the WeftOS native stack.
- `U8g2` is the reference C/C++ library; many fonts, very battle-tested.
- `Adafruit_SSD1306` in Arduino world — heavier, more convenient.

## Notes for WeftOS

- Build a 128×64 virtual-surface driver behind the same `present()` API as NeoPixels / TFTs; swap physical displays without touching apps.
- Include a screensaver / dimmer that drops to inverted low-contrast mode after N minutes of no updates to fight burn-in.
- Do not log secrets to the HUD — treat on-screen content as "over the shoulder" readable.
