---
title: "Rain Detection Board & Water Level Sticks"
slug: "rain-water-level"
category: "05-environmental"
part_numbers: ["YL-83 rain board", "water-level stick"]
manufacturer: "generic"
interface: ["ADC", "GPIO (digital threshold)"]
voltage: "3.3V / 5V"
logic_level: "3.3V"
current_ma: "~10 mA"
price_usd_approx: "$1 – $4"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["rain", "water-level", "flood", "legacy", "corrodes"]
pairs_with:
  - "./capacitive-soil-moisture.md"
  - "./anemometer.md"
  - "./bme280.md"
  - "../07-control-actuators/mosfet-drivers.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=rain+sensor" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=rain+sensor" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=water+level" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-rain+sensor.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-yl-83-rain.html" }
libraries:
  - "no dedicated driver — read ADC or digital GPIO"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

Two commonly-bundled but different sensors in hobby kits:

- **Rain detection board** (YL-83) — a PCB with interleaved copper fingers; water bridges the fingers and conducts, giving a yes/no or analog signal.
- **Water-level stick** — a vertical PCB with rungs of exposed copper traces; the higher the water, the more rungs are shorted.

Both are resistive, both **corrode quickly**, and both are cheap. Useful for one-off "something is wet" or "the tank is full" logic, but not for long-term deployment. Rain events in WeftOS weather stations are better served by a **tipping-bucket rain gauge** (reed switch pulse counter), which is not the YL-83.

## Key specs

- Voltage: 3.3 V or 5 V (choose based on ADC range on your MCU).
- Output: analog (AO) or digital (DO, with trimpot threshold).
- Water-level stick: typically 0–40 mm sensing height.
- Rain board: ~5 × 4 cm PCB; detects water droplets within seconds.

## Interface & wiring

- `VCC`, `GND`, `AO`, `DO`.
- For battery / outdoor use: **gate VCC through a MOSFET** ([`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md)) and energize only during sampling. Continuous DC through the traces galvanically corrodes them.
- Tipping-bucket gauges (recommended for actual rainfall quantification) use a reed switch: one GPIO with interrupt-on-falling, count pulses, and convert to mm per hour using the bucket volume.

## Benefits

- Cheapest possible "is it raining?" / "tank is full" indicator.
- Immediate response to droplets (rain board).
- Water-level stick gives a rough "low / medium / high" without a dedicated float switch.

## Limitations / gotchas

- **Corrodes fast.** Exposed copper in wet conditions galvanizes away in weeks.
- No meaningful quantity — "is it raining" or "is the water above here", not "how much rain".
- Surface tension affects readings — a single water drop on the rain board can read as "heavy rain".
- Water-level stick can read stale until water actually touches a rung — coarse quantization.
- 5 V rain board outputs > 3.3 V; level-shift or divide before feeding ESP32 ADC.

## Typical use cases

- Retract awning / close window on first rain.
- Irrigation "stop pumping" if rain detected.
- Sump / tank overflow warning.
- Bilge-pump trigger.

## Pairs well with

- [`./capacitive-soil-moisture.md`](./capacitive-soil-moisture.md) — non-corroding moisture alternative for soil.
- [`./anemometer.md`](./anemometer.md) — full weather station pairing.
- [`./bme280.md`](./bme280.md) — pressure-drop forecast + rain detection covers 90 % of rain events.
- [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) — pulsed power to extend probe life.

## Where to buy

- AliExpress / Amazon under "YL-83 rain sensor" or "water level sensor" for a few dollars.
- SparkFun / Adafruit for nicer variants with protected PCBs.
- For real rainfall totals: any tipping-bucket rain gauge with a reed switch — Ambient Weather, Misol, or Davis.

## Software / libraries

- None — GPIO read or ADC + threshold.
- Tipping-bucket gauge: interrupt + debounce, count pulses, multiply by mm/tip.

## Notes for WeftOS

- Tag these sensors as "binary / coarse-level, corrodes". UI must warn on deployment age > N weeks.
- Prefer tipping-bucket rain gauges for any weather-station deployment.
- For water-tank level, the correct upgrade path is an ultrasonic distance sensor ([`../02-positioning-navigation/`](../02-positioning-navigation/)) pointed down from the tank lid — no corrosion, accurate depth.
