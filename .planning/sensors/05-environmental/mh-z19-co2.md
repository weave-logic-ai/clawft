---
title: "Winsen MH-Z19B / MH-Z19C NDIR CO₂ Sensor"
slug: "mh-z19-co2"
category: "05-environmental"
part_numbers: ["MH-Z19B", "MH-Z19C", "MH-Z19E"]
manufacturer: "Winsen"
interface: ["UART", "PWM"]
voltage: "5V"
logic_level: "3.3V (Winsen TX is 3.3V-level; RX tolerates 3.3V)"
current_ma: "~75 mA avg; ~150 mA peak during heater cycles"
price_usd_approx: "$20 – $35"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["co2", "ndir", "air-quality", "uart", "pwm"]
pairs_with:
  - "./bme680-bme688.md"
  - "./pms5003.md"
  - "./ccs811-ens160.md"
  - "./bme280.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=mh-z19" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=mh-z19" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=mh-z19" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-mh-z19.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-mh-z19b.html" }
datasheet: "https://www.winsen-sensor.com/sensors/ndir-co2-sensor/mh-z19b.html"
libraries:
  - "Rust: mh-z19 crate (community)"
  - "Arduino: MH-Z19 (by WifWaf), ErriezMHZ19B"
  - "ESPHome: mhz19 component"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The MH-Z19B/C is a low-cost **NDIR** (non-dispersive infrared) CO₂ sensor from Winsen — a real CO₂ measurement, not an eCO₂ estimate like CCS811/ENS160. For WeftOS it's the ventilation-ground-truth sensor: "Is air actually getting exchanged in this room?" No other sensor in this price class answers that question honestly.

## Key specs

- Range: 0–5000 ppm (commonly; 0–10000 ppm variants exist).
- Accuracy: ± (50 ppm + 5 % of reading).
- Response time (T90): ~60 s.
- Interfaces: UART (9600 8N1), PWM (1004 ms period, duty ∝ concentration), analog (some revs).
- Warm-up: ~3 minutes.
- Life: ~5 years typical.

## Interface & wiring

- UART: TX / RX to an ESP32 hardware UART. TX is 3.3 V logic; RX tolerates 3.3 V.
- PWM: simpler, jitter-sensitive — one GPIO with input-capture or frequency-counter mode.
- Power: 5 V, with decent current capacity (150 mA peaks on heater cycles) — undersized LDOs cause intermittent bad reads.
- Don't breathe directly on the sensor for testing; exhaled CO₂ can be > 40000 ppm and temporarily saturates the element.

## Benefits

- Real NDIR measurement — genuinely tracks ventilation and occupancy.
- Factory calibrated; field re-calibrates automatically via ABC (see below).
- UART + PWM + analog gives flexibility for almost any MCU.
- Long-life, industrial-grade sensor at hobby prices.

## Limitations / gotchas

- **ABC (Automatic Baseline Correction).** By default the sensor assumes the lowest CO₂ it sees in a 24 h window is 400 ppm outdoor air and recalibrates to that. In a continuously occupied room (never dips to 400 ppm), ABC will drift the baseline **upward** over days, under-reporting true CO₂ by hundreds of ppm. **Disable ABC** via UART command for always-occupied rooms, and do manual re-cal monthly against fresh outdoor air.
- 5 V supply with unexpectedly high peak current; powering from a weak USB port gives random bad reads.
- PWM mode reports a trimmed range (starts at 400 ppm); UART gives the full range.
- Some cheap clones are labeled MH-Z19B but are actually older MH-Z14 — compare the silver heater cap and UART command ids.
- CO₂ at > 40000 ppm (direct breath) saturates the element for ~2 minutes.
- Sunlight / IR heat through the enclosure window can confuse NDIR — mount away from direct IR sources.

## Typical use cases

- Office / classroom CO₂ monitoring for ventilation control.
- "Open the window" trigger in a smart home.
- Commercial HVAC demand-control ventilation.
- Full air-quality node (MH-Z19 + BME680 + PMS5003).

## Pairs well with

- [`./bme680-bme688.md`](./bme680-bme688.md) — BME680/688 for VOC; MH-Z19 for real CO₂.
- [`./pms5003.md`](./pms5003.md) — particulates complement gas.
- [`./ccs811-ens160.md`](./ccs811-ens160.md) — cross-check eCO₂ against real CO₂.
- [`./bme280.md`](./bme280.md) — ambient T/RH/P alongside.

## Where to buy

- Adafruit, SparkFun, Seeed, DFRobot.
- Winsen direct for large quantities.
- AliExpress — MH-Z19B is among the more consistent clones, but still verify.

## Software / libraries

- `mh-z19` Rust crate (community, UART-based).
- `MH-Z19` Arduino library (WifWaf) is the reference.
- ESPHome `mhz19` component is a turnkey solution.

## Notes for WeftOS

- HAL should expose ABC as a configurable policy; default to "enabled for rooms that are unoccupied overnight, disabled for always-occupied rooms".
- Record time since last manual calibration; UI should prompt the user to take the node outside for 20 min once per month.
- CO₂ is the canonical "ventilation working?" metric — wire it into the WeftOS comfort/health dashboard as a first-class indicator, not buried under "air quality".
