---
title: "Leaf Wetness Sensor (resistive)"
slug: "leaf-wetness"
category: "05-environmental"
part_numbers: ["generic leaf-wetness grid", "METER PHYTOS 31 (pro)"]
manufacturer: "generic / METER Group"
interface: ["ADC"]
voltage: "3.3V / 5V"
logic_level: "3.3V"
current_ma: "~5 mA"
price_usd_approx: "$8 – $80"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["leaf-wetness", "agriculture", "disease-pressure", "resistive"]
pairs_with:
  - "./bme280.md"
  - "./capacitive-soil-moisture.md"
  - "./anemometer.md"
  - "../10-storage-timing-power/ads1115-ads1015.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=leaf+wetness" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=leaf+wetness" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=leaf+wetness" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-leaf+wetness.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-leaf-wetness-sensor.html" }
libraries:
  - "no dedicated driver — ADC + threshold"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A leaf-wetness sensor is a flat PCB (or dielectric pad on pro units) that mimics a leaf's surface: when dew, rain, or irrigation wets the "leaf", surface conductivity rises and the sensor reports "wet". Duration of leaf wetness is the #1 predictor of fungal disease pressure (powdery mildew, downy mildew, late blight). For WeftOS agriculture deployments this is a small-dollar, high-impact sensor.

## Key specs

- Output: analog (resistance / voltage divider) or digital threshold.
- Active area: ~5 × 10 cm grid.
- Response: seconds to minutes (depends on surface-tension breakup).
- Voltage: 3.3 V or 5 V.
- Pro units (METER PHYTOS 31): dielectric-based, much more repeatable, SDI-12 or mV output — 10–30× the price.

## Interface & wiring

- `VCC`, `GND`, `AO`.
- Gate `VCC` through a MOSFET and only energize during sampling; continuous DC corrodes the exposed traces on hobby boards.
- Mount on top of a representative leaf surface, tilted slightly so water runs off.
- Feed ADC through an [ADS1115](../10-storage-timing-power/ads1115-ads1015.md) if you want consistent readings across boards.

## Benefits

- Cheap ($8 on AliExpress) for a metric that genuinely predicts plant disease.
- Simple analog read — no protocol.
- Combined with dew-point computed from T/RH, gives a very accurate "is the leaf wet?" signal.

## Limitations / gotchas

- **Corrosion on hobby resistive units** — trace galvanic action eats copper within weeks of near-continuous dew exposure. Coat with conformal lacquer, pulse VCC, and plan to replace yearly.
- Calibration is subjective: "wet" vs "dry" threshold depends on orientation, coating, and what the sensor is simulating (grape leaf ≠ tomato).
- Not all moisture is disease-relevant: irrigation water runs off quickly; dew sits for hours. Count only "wet for > N minutes" events.
- Pro METER units are far more accurate but require SDI-12 interface.

## Typical use cases

- Vineyard / orchard disease-pressure monitoring.
- Tomato / strawberry blight alerting.
- Greenhouse humidity + wetness combo for fungal risk.
- Turfgrass management (golf courses, sports fields).

## Pairs well with

- [`./bme280.md`](./bme280.md) — T/RH/P gives dew-point; dew-point + leaf wetness is the canonical pairing.
- [`./capacitive-soil-moisture.md`](./capacitive-soil-moisture.md) — irrigation context.
- [`./anemometer.md`](./anemometer.md) — wind dries leaves faster; factor into "expected drying time".
- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) — stable ADC at low current.

## Where to buy

- AliExpress / Amazon for hobby resistive boards.
- METER Group (Spectrum Technologies / Decagon) for lab/pro PHYTOS 31.

## Software / libraries

- None; ADC + threshold hysteresis + minute-accurate "duration wet" accumulator.

## Notes for WeftOS

- Publish `LeafWetnessEvent { wet, duration_minutes, rolling_24h_minutes }` from the HAL. The 24 h rolling figure is what disease models actually consume.
- Conform the sensor: seal non-active edges with epoxy to slow corrosion.
- Wire into WeftOS's agricultural app as a "disease-pressure" tile with orchard-crop-specific threshold curves.
