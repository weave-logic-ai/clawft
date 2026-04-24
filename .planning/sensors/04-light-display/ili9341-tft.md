---
title: "ILI9341 2.8\" 320×240 SPI TFT (with optional touch)"
slug: "ili9341-tft"
category: "04-light-display"
part_numbers: ["ILI9341", "ILI9341 + XPT2046 touch", "ILI9341 + FT6206"]
manufacturer: "Ilitek"
interface: ["SPI"]
voltage: "3.3V"
logic_level: "3.3V (many breakouts 5V tolerant via level shifter)"
current_ma: "~80 – 120 mA (backlight dominates)"
price_usd_approx: "$9 – $18"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["display", "tft", "spi", "color", "touch", "resistive", "capacitive"]
pairs_with:
  - "./st7789-st7735-tft.md"
  - "../10-storage-timing-power/microsd.md"
  - "./ssd1306-oled.md"
  - "../08-communication/tca9548a-i2c-mux.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=ili9341" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=tft+ili9341" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=ili9341" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-ili9341.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-ili9341-tft.html" }
libraries:
  - "Rust (embedded-graphics): mipidsi (with Ili9341 model)"
  - "Arduino: TFT_eSPI, Adafruit_ILI9341"
  - "ESP-IDF: LVGL + tft_espi port"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The ILI9341 is a 320×240 color TFT controller, most often found on 2.4" and 2.8" SPI breakouts — sometimes bundled with an **XPT2046** 4-wire resistive touch controller, or a capacitive panel with **FT6206/GT911**. For WeftOS it's the sweet spot for a small color UI: big enough for menus and thumbnails, cheap, well-driven by LVGL and `embedded-graphics`.

## Key specs

- Resolution: 320×240, 16-bit RGB565 color.
- Interface: SPI up to 40–80 MHz (module- and wiring-dependent).
- Backlight: white LEDs, `LED` pin — ideally on a PWM GPIO for brightness control.
- Touch variants:
  - Resistive (XPT2046 / ADS7843) over SPI — accurate, pressure-sensitive, needs calibration per unit.
  - Capacitive (FT6206 / GT911) over I²C — multi-touch, no calibration needed, glass feel.
- Voltage: 3.3 V. 5 V tolerant signals only if breakout has a level shifter; most "3.3V/5V" modules do.

## Interface & wiring

- SPI: `SCLK`, `MOSI`, `MISO`, `CS`, `DC`, `RES`; backlight on `LED`.
- On ESP32, use the dedicated HSPI/VSPI bus and DMA for smooth >30 fps updates.
- Touch on a separate `T_CS` (XPT2046 shares SPI bus) or I²C `SDA/SCL` (cap touch).
- Drive the backlight pin via a MOSFET if you want <10 % dim — GPIO PWM alone is usually fine.

## Benefits

- Good pixel count and color depth for small UIs.
- Optional touch means a full self-contained kiosk-grade control surface.
- LVGL, `embedded-graphics`, and Adafruit GFX all support it natively.
- Fast enough for video preview thumbnails at 10–20 fps.

## Limitations / gotchas

- Backlight dominates power; at full brightness a 2.8" draws 80–120 mA just for illumination.
- Resistive touch panels need a per-device calibration (3- or 5-point) — store the calibration matrix in NVS.
- Capacitive panel controllers use different I²C addresses (FT6206 @ 0x38, GT911 @ 0x5D/0x14); auto-detect rather than hard-code.
- Cheap clones sometimes substitute ILI9342 or R61505 silicon — same pinout, different init sequences.
- SPI wiring length matters above 40 MHz; keep leads short or derate the clock.

## Typical use cases

- Color dashboards and menu UIs on standalone nodes.
- Preview surface for a [Pi / ESP32 camera](../01-vision-imaging/) — thumbnail + status overlay.
- Small "remote control" touch panel for room-scale automation.

## Pairs well with

- [`./st7789-st7735-tft.md`](./st7789-st7735-tft.md) — smaller/cheaper sibling for the same driver stack.
- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md) — many ILI9341 breakouts include an SD slot on the same SPI bus for bitmap assets.
- [`./ssd1306-oled.md`](./ssd1306-oled.md) — the monochrome fallback when power/cost matters more than fidelity.
- [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md) — only relevant for capacitive touch controller address collisions.

## Where to buy

- Adafruit (well-QC'd, clear pinouts, level shifter included), SparkFun.
- AliExpress "2.8 inch TFT LCD touch" modules — cheapest but beware random controller substitutions.

## Software / libraries

- `mipidsi` crate (Rust) with the `Ili9341` model for `embedded-graphics` / `embedded-hal`.
- `LVGL` for anything approaching a real UI (widgets, themes, transitions).
- `TFT_eSPI` (Arduino) is still the fastest PlatformIO-friendly driver.

## Notes for WeftOS

- Abstract the display as a 320×240 RGB565 surface; all apps render into a framebuffer and the HAL pushes via DMA.
- Touch events should emit into the same input bus as keyboards / rotary encoders — do not expose raw ADC values.
- Provide a system-level brightness policy (dim after idle, off when ambient light `< x`) driven by [`../05-environmental/bh1750-lux.md`](../05-environmental/bh1750-lux.md).
