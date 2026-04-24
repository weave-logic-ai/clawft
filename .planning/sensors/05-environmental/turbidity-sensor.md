---
title: "Analog Turbidity Sensor (NTU)"
slug: "turbidity-sensor"
category: "05-environmental"
part_numbers: ["SEN0189", "DFRobot Turbidity"]
manufacturer: "DFRobot / generic"
interface: ["ADC"]
voltage: "5V"
logic_level: "3.3V"
current_ma: "~30 mA (IR LED)"
price_usd_approx: "$8 – $20"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["turbidity", "water-quality", "ntu", "optical"]
pairs_with:
  - "./ph-sensor.md"
  - "./ec-tds.md"
  - "./ds18b20.md"
  - "../10-storage-timing-power/ads1115-ads1015.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=turbidity" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=turbidity" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=turbidity" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-turbidity.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-turbidity-sensor.html" }
libraries:
  - "no dedicated driver — read analog + compare against calibration curve"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

Turbidity sensors measure how much light is scattered by suspended particles in water. An IR LED shines across a short water gap into a photodiode; clear water = full signal, cloudy water = attenuated signal. The canonical hobby part is the DFRobot SEN0189. For WeftOS this sits alongside pH and EC in a water-quality node — useful for filter-clog detection, algae bloom warnings, and dishwasher / washing-machine diagnostics.

## Key specs

- Range: 0–3000 NTU (nephelometric turbidity units) typical; hobby probes are better treated as 0–100 % relative.
- Output: analog voltage (DFRobot board is linear to clarity).
- IR LED + photodiode pair; ~1 cm sample path.
- Voltage: 5 V supply, 0–4.5 V analog output (divide for 3.3 V ADC, or use [ADS1115](../10-storage-timing-power/ads1115-ads1015.md)).

## Interface & wiring

- `VCC`, `GND`, `A_OUT`. Signal board can switch between "analog" and "digital threshold" (trimpot on the PCB).
- Gate VCC via MOSFET if battery-powered — the IR LED draws non-trivial current.
- Mount so flow passes **through** the probe's slot, not around it.
- Keep probe window clean; biofilm quickly depresses readings.

## Benefits

- Cheap way to catch "the water got visibly dirty".
- No wet electrical contact (the LED and photodiode are on opposite sides of the water gap).
- Analog output easily thresholded for go / no-go alerts.

## Limitations / gotchas

- **Hobby probes are not NTU-calibrated.** The "NTU" numbers in sample code are rough; for regulatory work use a lab turbidimeter.
- Probe window fouls over days in real water (biofilm, mineral deposits); clean weekly.
- Temperature affects LED output slightly; compensate if precision matters.
- Bubbles in the water path cause giant spikes — settle the water before sampling.
- Strong ambient IR (direct sun near a clear tank) washes out the photodiode.

## Typical use cases

- Hydroponic water cleanliness check.
- Pool / hot tub haze warning.
- Washing-machine / dishwasher water-quality diagnostics.
- Well water sediment detection.
- Stream monitoring for sediment events.

## Pairs well with

- [`./ph-sensor.md`](./ph-sensor.md) — pH + turbidity together often catches algae blooms early.
- [`./ec-tds.md`](./ec-tds.md) — complete water-quality triplet with EC.
- [`./ds18b20.md`](./ds18b20.md) — temperature logging alongside turbidity in the same tank.
- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) — clean ADC for small deltas.

## Where to buy

- DFRobot SEN0189 Gravity.
- SparkFun / AliExpress clones (same IR LED pair, variable signal-board quality).

## Software / libraries

- None dedicated; two-point calibration (clear water = 100 %, opaque = 0 %) and linear interpolation is sufficient.

## Notes for WeftOS

- Expose as `WaterClarity { percent_clear, raw_adc }`; treat "NTU" as a computed value only when calibration was done against a known-NTU standard.
- Flag "sensor foul" when readings drift downward with no input changes over weeks — prompts a cleaning reminder.
- Combine with [pH](./ph-sensor.md) and [EC](./ec-tds.md) into a single `WaterQualitySnapshot` emitted by the HAL.
