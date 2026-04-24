---
title: "MQ Series Gas Sensors (MQ-2/3/4/5/7/8/9/135)"
slug: "mq-series"
category: "05-environmental"
part_numbers: ["MQ-2", "MQ-3", "MQ-4", "MQ-5", "MQ-7", "MQ-8", "MQ-9", "MQ-135"]
manufacturer: "Hanwei / Winsen"
interface: ["ADC", "GPIO (digital threshold)"]
voltage: "5V (heater)"
logic_level: "5V (level shifter / voltage divider for analog)"
current_ma: "~150 mA (heater current)"
price_usd_approx: "$3 – $8 per module"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["gas", "metal-oxide", "mq", "smoke", "co", "lpg", "air-quality"]
pairs_with:
  - "../10-storage-timing-power/ads1115-ads1015.md"
  - "./mh-z19-co2.md"
  - "./bme680-bme688.md"
  - "./pms5003.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=mq-2" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=mq" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=mq-2" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-mq.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-mq-2.html" }
libraries:
  - "no dedicated driver required — read ADC and apply curve from datasheet"
  - "Arduino: MQUnifiedsensor"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The MQ series from Hanwei / Winsen is a family of heated metal-oxide gas sensors. Each variant targets a specific gas or class of gases; the sensing element's resistance drops in presence of its target. All use the same physical package (a small heater + sensing element in a metal cap) and module layout (heater + load resistor + comparator for digital threshold + analog output). They are cheap, ubiquitous, and *remarkably* unreliable — but for triggering "something is wrong" in a low-cost node, they remain useful.

## Variants and target gases

| Part | Primary target | Secondary / notes |
|------|----------------|-------------------|
| MQ-2 | LPG, propane, hydrogen, smoke | broad combustible |
| MQ-3 | Alcohol / ethanol | breathalyzer toys |
| MQ-4 | Methane (CH₄), natural gas | |
| MQ-5 | LPG, natural gas | similar to MQ-4 |
| MQ-7 | Carbon monoxide (CO) | requires cycled heater (1.5 V / 5 V) for best selectivity |
| MQ-8 | Hydrogen (H₂) | |
| MQ-9 | CO + combustible gases | cycled heater |
| MQ-135 | Air quality / NH₃ / NOx / CO₂ proxy | the "general air quality" one |

## Key specs

- Heater voltage: 5 V (critical — 3.3 V will never reach operating temperature).
- Heater current: ~150 mA continuously (some dissipate ~0.9 W).
- Warm-up: 24 – 48 h **burn-in** on *first* power-up before readings stabilize; ~3 minutes pre-heat on every subsequent boot.
- Output: analog voltage (ADC) — calibrate against a known baseline.
- Sensitivity curves are logarithmic vs PPM.

## Interface & wiring

- Power heater from the 5 V rail; do **not** try to run from 3.3 V.
- Analog out: divide to ADC range of your MCU (ESP32 ADC maxes at 3.3 V). Feed through an [ADS1115](../10-storage-timing-power/ads1115-ads1015.md) for stable 16-bit readings.
- Digital out: on-board comparator with trimpot threshold — useful for "alarm" GPIO without firmware work.
- Keep MQ sensors away from T/RH sensors — heater self-warming corrupts ambient readings.

## Benefits

- Dollar-tier gas detection with a visible pin-bag part — good for learning and demos.
- Huge variety of targets; one kit covers most common combustibles.
- Analog curve gives a continuous signal, not just a threshold.

## Limitations / gotchas

- **Heater burn-in.** First-time power requires 24–48 hours of continuous heating before readings are meaningful. Agents frequently skip this and get nonsense.
- **Cross-sensitivity is enormous.** An MQ-135 responds to almost every polar organic — can't distinguish CO₂ from alcohol from steam. Treat as a *change detector*, not a concentration meter.
- **Temperature and humidity dependent.** Readings shift 10–30 % over normal T/RH swings; compensate using an ambient [BME280](./bme280.md) or accept wide error bars.
- **Heater-only duty cycles (MQ-7, MQ-9).** For best CO selectivity, alternate heater voltage (1.4 V / 5 V) in a specific duty cycle — most hobby modules don't implement this and give muddy data.
- **Short life (~2 years)** in continuous operation; heater ages the element.
- Not certified for life-safety use — do **not** use as a gas-leak alarm in place of a UL/CE detector.
- Zero calibration against a standard means absolute PPM is essentially guessed.

## Typical use cases

- "Did something change in the air?" trigger for a WeftOS event.
- Hobby breathalyzer (MQ-3).
- LPG / CH₄ early-warning indicator alongside a real UL-certified detector.
- Educational demos and science-fair projects.

## Pairs well with

- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) — stable 16-bit ADC for noisy MQ analog.
- [`./mh-z19-co2.md`](./mh-z19-co2.md) — real NDIR CO₂ to ground-truth MQ-135's "air quality".
- [`./bme680-bme688.md`](./bme680-bme688.md) — Bosch's VOC channel is a modern replacement for MQ-135.
- [`./pms5003.md`](./pms5003.md) — particulates complement gas.

## Where to buy

- Adafruit, SparkFun, Seeed, DFRobot.
- AliExpress for cheap kits (MQ-2/3/4/5/6/7/8/9/135 for a few dollars total).

## Software / libraries

- Pure ADC read + datasheet curve math; no driver IC.
- `MQUnifiedsensor` (Arduino) provides curve fits for all variants.

## Notes for WeftOS

- HAL should expose MQ sensors as `TrendGasSensor` with a "relative, uncalibrated" tag; UI must *not* display a PPM value without the tag.
- Enforce a cold-boot burn-in counter: the sensor is flagged "unreliable" for the first 48 hours of cumulative runtime in NVS.
- If life-safety is the goal, require a certified detector (wired through [relay modules](../07-control-actuators/relay-modules.md)) instead — MQ is a hint, not a guard.
