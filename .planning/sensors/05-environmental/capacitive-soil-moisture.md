---
title: "Capacitive Soil Moisture Sensor (anti-corrosion)"
slug: "capacitive-soil-moisture"
category: "05-environmental"
part_numbers: ["DFR0114", "capacitive v1.2 generic"]
manufacturer: "DFRobot / generic"
interface: ["ADC"]
voltage: "3.3V / 5V"
logic_level: "3.3V"
current_ma: "~5 mA"
price_usd_approx: "$3 – $8"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["soil-moisture", "capacitive", "anti-corrosion", "hydroponics"]
pairs_with:
  - "./resistive-soil-moisture.md"
  - "./ds18b20.md"
  - "./ec-tds.md"
  - "../10-storage-timing-power/ads1115-ads1015.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=capacitive+soil" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=soil+moisture" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=soil+moisture+capacitive" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-capacitive+soil.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-capacitive-soil-moisture.html" }
libraries:
  - "no dedicated driver — read ADC + apply two-point (air, water) calibration"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A capacitive soil-moisture sensor is a PCB with two coplanar copper traces driven as a 1 MHz oscillator; wet soil has a higher dielectric constant than dry, shifting the oscillator frequency, which on-board electronics convert to an analog voltage. Unlike the resistive version ([`resistive-soil-moisture.md`](./resistive-soil-moisture.md)), **no current flows into the soil**, so the probe doesn't corrode. For WeftOS this is the right soil-moisture sensor — resistive variants exist for completeness but should rarely be deployed.

## Key specs

- Output: 0–3.3 V (or 0–5 V), **inverse** to moisture — dry reads high, wet reads low.
- Response time: ~1 second.
- Probe material: fiberglass PCB with conformal coating (quality varies).
- Typical raw ADC spread: ~1600 (fully wet) to ~3200 (bone dry) on a 12-bit ESP32 ADC — calibrate per unit.

## Interface & wiring

- 3 pins: `VCC`, `GND`, `AOUT`.
- VCC: 3.3 V or 5 V — **read the silkscreen**, some require 5 V and sag at 3.3.
- AOUT to ESP32 ADC or an [ADS1115](../10-storage-timing-power/ads1115-ads1015.md).
- Insert only up to the silk-screen line — the electronics on top must stay dry.
- Two-point calibration required: dry (in air) and wet (fully submerged in water). Store per-sensor calibration in NVS.

## Benefits

- **No corrosion** — lifetime in a pot is years, not weeks.
- Simple analog output, no protocol.
- Cheap ($3–8).
- Works in hydroponics / aquaponics (no galvanic contact with nutrient solution).

## Limitations / gotchas

- Many cheap AliExpress units have **poor conformal coating** — water wicks into the PCB, corrodes the exposed copper, and the "capacitive" sensor becomes resistive + dead. Inspect the coating; consider adding a layer of clear epoxy.
- Not equally sensitive across the full insertion depth — the effective sensing zone is roughly the bottom 2/3. Long root zones need multiple sensors.
- Reading depends on **soil type**, **density**, and **temperature**. A calibration for potting soil is not valid for clay.
- Some clones are missing a critical oscillator cap and give a flat output; verify with a known dry/wet test on first install.
- Readings drift with the air gap between electrode and soil — poor insertion technique gives false-dry.

## Typical use cases

- Individual plant pot moisture.
- Garden bed wilt-warning.
- Hydroponic root-zone monitoring.
- Greenhouse automation (drip irrigation trigger).

## Pairs well with

- [`./resistive-soil-moisture.md`](./resistive-soil-moisture.md) — the legacy alternative; documented for comparison.
- [`./ds18b20.md`](./ds18b20.md) — soil temperature in the same pot; drives temperature compensation on moisture readings.
- [`./ec-tds.md`](./ec-tds.md) — hydroponics combo with EC for nutrient strength.
- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) — 16-bit ADC for repeatable readings.

## Where to buy

- Adafruit (STEMMA-style with on-board microcontroller and I²C — better than raw analog).
- DFRobot Gravity capacitive moisture sensor (the canonical one).
- AliExpress "capacitive soil moisture v1.2" — cheap, coating varies. Seal with epoxy before use.

## Software / libraries

- No driver required — analog read + two-point calibration (dry = 100 %, wet = 0 %).
- Smooth with a median filter over 5–10 samples; single readings are jumpy.

## Notes for WeftOS

- HAL exposes `SoilMoisture { percent_wet, raw_adc }`; apps should consume `percent_wet` based on the per-node calibration curve.
- Re-seal the PCB top with clear epoxy before deployment — this doubles typical life in the ground.
- Trend is more useful than absolute: "has moisture dropped N % since last watering?" beats "is it 42 % wet?".
