---
title: "GUVA-S12SD UV Index Sensor"
slug: "guva-s12sd-uv"
category: "01-vision-imaging"
part_numbers: ["GUVA-S12SD"]
manufacturer: "Genicom / Roithner"
interface: ["ADC (analog)"]
voltage: "3.3V / 5V"
logic_level: "analog 0–1 V typical"
current_ma: "<0.1 mA"
price_usd_approx: "$3 – $8"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [uv, analog, photodiode, uv-index]
pairs_with:
  - "./uv-illuminators.md"
  - "../05-environmental/index.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=GUVA-S12SD" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=UV+sensor" }
  - { vendor: DFRobot, url: "https://www.dfrobot.com/search-GUVA.html" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=UV+sensor" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-GUVA-S12SD.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The GUVA-S12SD is a cheap UV photodiode + op-amp breakout that outputs an analog voltage roughly
proportional to UVA/UVB intensity (240–370 nm). It's not a camera — it's a single-pixel UV meter
— but paired with a UV light source it gives you basic UV presence/intensity for fluorescence,
sun-index, or leak-detection projects.

## Key specs

| Spec | Value |
|---|---|
| Spectral range | ~240–370 nm |
| Output | 0–~1 V analog (scales with intensity) |
| Response time | ~10 ms |
| Operating voltage | 2.7–5.5 V |
| Current | <100 µA |

## Interface & wiring

Three pins: VCC, GND, OUT → ESP32 ADC pin. ESP32 ADCs are noisy; average 32–128 samples per
reading and use ADC1 channels (ADC2 is unavailable when Wi-Fi is active). Add a 100 nF decoupling
cap on VCC close to the module.

## Benefits

- Dirt cheap.
- No library needed — it's just an ADC read.
- Works for sun-index estimation outdoors.

## Limitations / gotchas

- Single pixel, not imaging.
- Very rough calibration; treat as "relative" unless you calibrate against a reference meter.
- ESP32 ADC nonlinearity matters at low voltages — calibrate with `esp_adc_cal`.

## Typical use cases

- UV index estimation for wearables / garden nodes.
- Confirming a UV illuminator is actually lit (safety interlock).
- Fluorescence experiments (blacklight on, GUVA reads).

## Pairs well with

- [`./uv-illuminators.md`](./uv-illuminators.md) — the active source.
- [`../05-environmental/index.md`](../05-environmental/index.md) — combine UV with T/RH/pressure for a weather node.

## Where to buy

- Adafruit / SparkFun / DFRobot / Seeed — usually as "UV sensor breakout".
- AliExpress — dirt cheap clones.

## Software / libraries

- None required; use ESP-IDF `adc_oneshot` / `adc_continuous` or Arduino `analogRead`.

## Notes for WeftOS

Speculative: this belongs in the same "scalar sensor" surface class as air-quality sensors —
emit a UV-index float plus a confidence/validity flag; effects subscribe without any image
machinery.
