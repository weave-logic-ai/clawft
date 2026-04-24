---
title: "Resistive Soil Moisture Sensor (YL-69 / HL-69)"
slug: "resistive-soil-moisture"
category: "05-environmental"
part_numbers: ["YL-69", "HL-69"]
manufacturer: "generic"
interface: ["ADC", "GPIO (digital threshold)"]
voltage: "3.3V / 5V"
logic_level: "3.3V"
current_ma: "~20 mA while powered"
price_usd_approx: "$1 – $3"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["soil-moisture", "resistive", "legacy", "corrodes"]
pairs_with:
  - "./capacitive-soil-moisture.md"
  - "../07-control-actuators/mosfet-drivers.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=soil+moisture" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=soil+moisture" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=soil+moisture" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-soil+moisture.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-yl-69.html" }
libraries:
  - "no dedicated driver — read ADC with active-low inversion"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The YL-69 / HL-69 style resistive soil moisture sensor is two bare metal probes that you stick in the soil; wet soil conducts better, dry doesn't, and a comparator + analog output give you a reading. It's the cheapest possible moisture sensor — and the worst. **It corrodes within weeks** because DC current flows through wet soil. Documented here for completeness, but [capacitive soil moisture](./capacitive-soil-moisture.md) is the correct default.

## Key specs

- Output: analog (AO) inverse to moisture + digital (DO) with trimpot threshold.
- Resistance swing: ~1 kΩ (fully wet) to open-circuit (dry).
- Voltage: 3.3 V or 5 V.
- Probe material: tinned copper or sometimes bare copper (dies fast either way).

## Interface & wiring

- `VCC`, `GND`, `AO`, `DO`.
- AO to ADC for analog, DO for a yes/no threshold (set by the trimpot on the board).
- **Gate VCC via a MOSFET** and only energize the probes when sampling (e.g., 100 ms per hour) — continuous DC is what kills the probe.

## Benefits

- Cheapest moisture sensor available — cents on AliExpress.
- Easy to breadboard and understand.
- Digital-threshold output avoids ADC work if you just want "dry alarm".

## Limitations / gotchas

- **Corrodes within weeks** in wet soil — the probe literally plates away.
- Readings drift heavily as the probes age.
- DC current into the soil disturbs the sample (electrolysis near the probe tip).
- No repeatability between two probes; even from the same batch, calibration varies wildly.
- Tinned probes corrode slower than bare, but neither lasts.

## Typical use cases

- One-off demo or classroom experiment.
- Short-term (days) experiments where long life doesn't matter.
- Legacy retrofits where changing the BOM isn't feasible.

## Pairs well with

- [`./capacitive-soil-moisture.md`](./capacitive-soil-moisture.md) — the non-corroding replacement; migrate to this ASAP.
- [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) — gate VCC so the probe is only powered during a read.

## Where to buy

- AliExpress / Amazon under "YL-69 soil moisture" for pennies.
- SparkFun / Seeed for branded (slightly better coated) variants.

## Software / libraries

- None — raw analog read + per-sensor calibration.

## Notes for WeftOS

- Refuse to register this as a persistent sensor in the WeftOS device DB — surface a one-time "demo-only" warning.
- If the user still insists, enforce the "power-only-during-read" pattern at the HAL level: the driver wraps the analog read in a MOSFET pulse.
- Flag "probe age" in telemetry so the user is reminded to replace it — most installations die silently at the 4–8 week mark.
