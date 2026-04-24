---
title: "MLX90640 Thermal Array (32×24)"
slug: "mlx90640-thermal"
category: "01-vision-imaging"
part_numbers: ["MLX90640BAA", "MLX90640BAB"]
manufacturer: "Melexis"
interface: ["I2C"]
voltage: "3.3V"
logic_level: "3.3V"
current_ma: "~23 mA"
price_usd_approx: "$35 – $70"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [thermal, ir, mlx90640, far-ir, imaging]
pairs_with:
  - "./amg8833-thermal.md"
  - "./esp32-s3-ir-thermal.md"
  - "../05-environmental/index.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=MLX90640" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=MLX90640" }
  - { vendor: Pimoroni, url: "https://shop.pimoroni.com/search?q=mlx90640" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=MLX90640" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-MLX90640.html" }
datasheet: "https://www.melexis.com/en/product/MLX90640/Far-Infrared-Thermal-Sensor-Array"
libraries:
  - { name: "melexis/mlx90640-library", url: "https://github.com/melexis/mlx90640-library" }
  - { name: "adafruit/Adafruit_MLX90640", url: "https://github.com/adafruit/Adafruit_MLX90640" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The MLX90640 is a 32×24 far-IR thermal sensor array (768 pixels) from Melexis, sold in 55° (BAA)
and 110° (BAB) FOV variants. It's the go-to "real thermal camera" option for hobby ESP32 builds
— enough resolution to recognize people, pets, and heat leaks without the cost of a FLIR Lepton.

## Key specs

| Spec | Value |
|---|---|
| Resolution | 32 × 24 pixels (768) |
| FOV | 55° × 35° (BAA) or 110° × 75° (BAB) |
| Temp range | -40 °C to +300 °C |
| Accuracy | ±1 °C typical |
| Max frame rate | 64 Hz (I²C bandwidth-limited in practice) |
| Bus | I²C, up to 1 MHz (Fast-mode Plus) |

## Interface & wiring

I²C only, but you *want* 1 MHz bus speed — at 400 kHz you cap out around 8 fps. Power 3.3 V,
pull-ups appropriate for Fm+. Leave a few mm clearance above the sensor for thermal accuracy (no
local heat sources). ESP32 I²C supports Fm+ via driver config.

## Benefits

- Real thermal imagery — enough for heat maps, fever screening, leak detection.
- I²C-only wiring is simple.
- Well-supported libraries and calibration data stored on-chip.

## Limitations / gotchas

- 32×24 is low res — pixel count is ~8× a smartwatch icon. Bilinear upscaling is essential.
- Per-frame calibration math is heavy for ESP32 (~10–20 ms @ 240 MHz).
- Must run bus at 1 MHz to hit useful frame rates; crappy pull-ups will kill you.
- Absolute accuracy drifts with ambient; recalibrate if the enclosure self-heats.

## Typical use cases

- Heat-mapping tools (attics, HVAC, brewing).
- Human/pet presence detection.
- Fire / overheat watchdog on electronics.

## Pairs well with

- [`./amg8833-thermal.md`](./amg8833-thermal.md) as the cheaper 8×8 sibling.
- [`./esp32-s3-ir-thermal.md`](./esp32-s3-ir-thermal.md) as a higher-res alternative.
- [`../05-environmental/index.md`](../05-environmental/index.md) — pair with BME680 for "why is it hot *and* humid".

## Where to buy

- Adafruit / SparkFun / Pimoroni — quality breakouts.
- Seeed / AliExpress — cheaper bare modules.

## Software / libraries

- `melexis/mlx90640-library` (reference C driver).
- `adafruit/Adafruit_MLX90640` (Arduino/CircuitPython).

## Notes for WeftOS

Speculative: thermal frames fit naturally as a "low-bandwidth image surface" type in WeftOS —
768 float values at 8–32 Hz is essentially an audio-frame-rate image, so it could share
infrastructure with signal streams rather than the camera pipeline.
