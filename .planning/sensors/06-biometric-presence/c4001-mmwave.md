---
title: "C4001 – mmWave Breathing / Heart-Rate / Fall Detection Radar"
slug: "c4001-mmwave"
category: "06-biometric-presence"
part_numbers: ["C4001", "SEN0609"]
manufacturer: "DFRobot (C4001 radar core)"
interface: ["UART", "I2C"]
voltage: "3.3V – 5V"
logic_level: "3.3V"
current_ma: "~100 mA active"
price_usd_approx: "$20 – $40"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [presence, mmwave, radar, breathing, heart-rate, fall-detection]
pairs_with:
  - "../06-biometric-presence/ld2410-mmwave.md"
  - "../07-control-actuators/relay-modules.md"
  - "../04-light-display/ws2812b-neopixel.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=mmwave+radar" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=mmwave" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=mmwave+radar" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-C4001.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-C4001-radar.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The C4001 is a higher-end 24 GHz mmWave radar module oriented at **health and
safety** rather than plain presence. In addition to occupancy it estimates
breathing rate, heart rate, and in some firmware variants flags falls. DFRobot
sells the consumer-friendly packaging (with Gravity connectors and example
Arduino libs); Seeed carries similar modules under their own SKUs.

Think of it as the next rung of the presence ladder after [LD2410](ld2410-mmwave.md):
same physics, richer extracted features, higher cost.

## Key specs

| Spec | Value |
|------|-------|
| Frequency | 24 GHz FMCW |
| Presence range | ~8 m |
| Breathing / HR range | ~0.5–2 m (subject must be reasonably still) |
| Output | UART (primary) + I²C on Gravity variant |
| Supply | 3.3–5 V |
| Current | ~100 mA |

## Interface & wiring

Four wires on the Gravity variant: `VCC / GND / SDA / SCL` or `VCC / GND / TX /
RX`. UART at 115200 8N1 is the flexible path; I²C is fine for simple occupancy.
Keep the module **aimed at the subject** (seated chair, bed) and avoid metal
reflectors within ~30 cm.

## Benefits

- Extracts respiration and heart rate without any contact — no wearable needed.
- Fall-detection firmware turns one sensor into an elder-care trigger.
- Same physics as LD2410 but with a richer feature extractor baked in.

## Limitations / gotchas

- **Privacy:** This is the most invasive "no camera" sensor in the catalog. It
  infers sleep patterns, breathing rate, heart rate, and presence. Require
  explicit user consent and never stream raw waveforms to cloud.
- HR/breathing accuracy degrades fast with distance, motion, and multi-person
  scenes. Do not market health claims.
- Fall-detection firmware has notable false-positive / false-negative rates;
  treat as an assist, not a medical alarm.
- Draws ~100 mA continuously; plan a [MOSFET power gate](../07-control-actuators/mosfet-drivers.md)
  if on battery.

## Typical use cases

- Bedroom sleep monitor (pair with MQTT + HA).
- Elder-care fall flag (not a replacement for certified alarms).
- Focus / stress biofeedback station at a desk.

## Pairs well with

- [`ld2410-mmwave.md`](ld2410-mmwave.md) — use LD2410 for whole-room presence
  and the C4001 only when someone is close to the bed/chair.
- [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) —
  duty-cycle the radar to manage power and heat.
- [`../04-light-display/ws2812b-neopixel.md`](../04-light-display/ws2812b-neopixel.md) —
  ambient breathing-rate visualizer.

## Where to buy

See `buy:`. DFRobot's Gravity C4001 is the easiest turnkey option; Seeed sells
related mmWave-health modules under their mmWave catalog.

## Software / libraries

- DFRobot publishes an Arduino library and example sketches for the Gravity
  C4001; search for `DFRobot_C4001` on GitHub.
- Community ESPHome components exist but are less mature than the LD2410 one.

## Notes for WeftOS

Classify C4001 outputs as **biometric** (`hr`, `breathing`) or **presence-advanced**
(`occupancy`, `fall`). The biometric channels must be scoped to the device's
local memory by default and only exposed to rules the user explicitly authorizes.
