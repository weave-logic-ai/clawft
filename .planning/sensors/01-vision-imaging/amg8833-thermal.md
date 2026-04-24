---
title: "Panasonic AMG8833 Grid-EYE (8×8)"
slug: "amg8833-thermal"
category: "01-vision-imaging"
part_numbers: ["AMG8833", "Grid-EYE"]
manufacturer: "Panasonic"
interface: ["I2C"]
voltage: "3.3V / 5V (breakout)"
logic_level: "3.3V or 5V (per breakout)"
current_ma: "~4.5 mA"
price_usd_approx: "$20 – $40"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [thermal, ir, amg8833, grid-eye, low-res]
pairs_with:
  - "./mlx90640-thermal.md"
  - "../06-biometric-presence/index.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=AMG8833" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=AMG8833" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=AMG8833" }
  - { vendor: DigiKey, url: "https://www.digikey.com/en/products/search?keywords=AMG8833" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-AMG8833.html" }
datasheet: "https://industrial.panasonic.com/ww/products/pt/grid-eye"
libraries:
  - { name: "adafruit/Adafruit_AMG88xx", url: "https://github.com/adafruit/Adafruit_AMG88xx" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The AMG8833 "Grid-EYE" is an 8×8 (64 pixel) thermal sensor — the lowest-res thermal camera worth
buying, and often the cheapest. It's enough to see where a warm body is in a room, but not to
recognize what it is.

## Key specs

| Spec | Value |
|---|---|
| Resolution | 8 × 8 pixels (64) |
| FOV | ~60° |
| Temp range | 0 °C to 80 °C |
| Accuracy | ±2.5 °C typical |
| Frame rate | 1 or 10 Hz (configurable) |
| Interrupt | Programmable threshold, INT pin |

## Interface & wiring

I²C at 400 kHz is plenty — 64 pixels × 2 bytes per frame is tiny. Most breakouts include level
shifters and work at 3.3 V or 5 V. There's a dedicated INT pin that asserts when any pixel
crosses a threshold, enabling cheap motion-wake.

## Benefits

- Cheap, small, low power.
- Interrupt pin is great for deep-sleep motion wake.
- Bulletproof I²C driver — good first thermal project.

## Limitations / gotchas

- 8×8 means "blob detection," not imaging. Faces are ~1–2 pixels at normal desk distance.
- Limited temp range (0–80 °C); not a fire sensor.
- 10 Hz is the max; fast motion smears.

## Typical use cases

- Thermal presence / occupancy.
- Low-power wake-on-heat-blob.
- Desk "is someone there" indicators.

## Pairs well with

- [`./mlx90640-thermal.md`](./mlx90640-thermal.md) when you outgrow 8×8.
- [`../06-biometric-presence/index.md`](../06-biometric-presence/index.md) for PIR / mmWave occupancy.

## Where to buy

- Adafruit / SparkFun / Seeed / DigiKey.
- AliExpress for bare Grid-EYE modules.

## Software / libraries

- `adafruit/Adafruit_AMG88xx`.

## Notes for WeftOS

Speculative: the AMG8833 is a good fit for a low-power "presence surface" in WeftOS — a 64-float
tensor at 10 Hz is basically a signal, so it could pipe into the same effect chain as audio
feature extractors rather than images.
