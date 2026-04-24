---
title: "PIR HC-SR501 – Passive Infrared Motion Sensor"
slug: "pir-hc-sr501"
category: "06-biometric-presence"
part_numbers: ["HC-SR501", "HC-SR505", "AM312"]
manufacturer: "various (BISS0001-based)"
interface: ["GPIO"]
voltage: "4.5V – 20V (HC-SR501); 2.7V – 12V (AM312)"
logic_level: "3.3V digital output"
current_ma: "~50 µA idle, ~1 mA active"
price_usd_approx: "$1 – $3"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [presence, motion, pir, low-power, gpio]
pairs_with:
  - "../01-vision-imaging/esp32-cam-ov2640.md"
  - "../07-control-actuators/relay-modules.md"
  - "../04-light-display/ws2812b-neopixel.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=PIR" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=PIR" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=PIR" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-PIR.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-HC-SR501.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The HC-SR501 is the canonical cheap **passive-IR motion sensor**: a pyroelectric
element under a Fresnel lens detects changes in infrared radiation (i.e. a warm
body moving across the field of view) and raises a digital pin HIGH for a
configurable hold time. The AM312 and HC-SR505 are smaller 3.3 V variants with
the same functional behavior.

It doesn't see heat. It sees *changes* in heat. A person sitting perfectly still
is invisible to a PIR after a few seconds.

## Key specs

| Spec | Value |
|------|-------|
| Range | ~3–7 m (HC-SR501), ~3 m (AM312) |
| Field of view | ~120° |
| Supply | 4.5–20 V (HC-SR501), 2.7–12 V (AM312) |
| Output | Digital HIGH while triggered |
| Hold time | Adjustable via trimpot (HC-SR501), fixed ~2 s (AM312) |
| Retrigger mode | Jumper selectable on HC-SR501 |

## Interface & wiring

Three wires: `VCC / GND / OUT`. `OUT` is a 3.3 V digital signal you wire to any
ESP32 GPIO and treat as a rising-edge interrupt. The HC-SR501 has two trimpots
(sensitivity, hold time) and a jumper for single-trigger vs retrigger mode.

## Benefits

- Cheapest reliable human-motion trigger.
- Microamp idle current → deep-sleep friendly.
- No MCU processing required; it's literally a digital pin.

## Limitations / gotchas

- **Only detects motion**, not presence. Seated humans vanish after a few seconds.
- Fooled by HVAC vents blowing warm air, sunlight creeping across a wall, and pets.
- HC-SR501 needs ~30–60 s warm-up after power-on; expect spurious trips early.
- Low-angle or glass-in-the-way installs drop range sharply.

## Typical use cases

- Wake-on-motion for battery-powered ESP32 cameras (deep sleep + `ext0` wake).
- Hallway / porch lights that don't need to know "is the person still here".
- Cheap occupancy heuristic layered *in front of* a smarter (mmWave) sensor to
  save its power budget.

## Pairs well with

- [`../01-vision-imaging/esp32-cam-ov2640.md`](../01-vision-imaging/esp32-cam-ov2640.md) —
  PIR triggers camera snapshot + upload.
- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) —
  PIR → lights / fan.
- [`ld2410-mmwave.md`](ld2410-mmwave.md) — PIR as a cheap first-stage gate for
  the more expensive mmWave.

## Where to buy

See `buy:`. These are sold in 10-packs on AliExpress for under $10 total;
Adafruit and Sparkfun carry higher-quality AM312 equivalents.

## Software / libraries

No library needed — it's a digital pin. ESPHome's `binary_sensor: pir` platform
or Arduino's `attachInterrupt()` is enough.

## Notes for WeftOS

PIR is the first rung of the presence ladder. WeftOS should tag PIR events as
**motion**, not **presence**, so downstream rules don't conflate the two. A
policy like "lights off after 30 s of no PIR" is correct; "nobody is home because
PIR is quiet" is the classic mistake.
