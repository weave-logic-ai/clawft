---
title: "EC / TDS Water Conductivity Sensors"
slug: "ec-tds"
category: "05-environmental"
part_numbers: ["DFR0300 EC", "SEN0244 TDS", "generic TDS probe"]
manufacturer: "DFRobot / generic"
interface: ["ADC"]
voltage: "3.3V / 5V"
logic_level: "3.3V"
current_ma: "~5 – 15 mA"
price_usd_approx: "$15 – $50"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["ec", "tds", "water-quality", "conductivity", "hydroponics"]
pairs_with:
  - "./ds18b20.md"
  - "./ph-sensor.md"
  - "./turbidity-sensor.md"
  - "../10-storage-timing-power/ads1115-ads1015.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=tds" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=tds" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=tds" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-ec.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-tds-sensor.html" }
libraries:
  - "no dedicated driver — AC-excited probe + ADC + temperature compensation"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

EC (electrical conductivity) and TDS (total dissolved solids) sensors measure how well water conducts current — a direct proxy for dissolved ions (nutrients, salts). In hydroponics, EC is the primary nutrient-strength indicator. TDS is just EC × 0.5 (NaCl scale) or × 0.67 (442 scale) — same measurement, different unit. DFRobot's SEN0244 (TDS) and DFR0300 (EC) are the canonical hobby probes; Atlas Scientific sells lab-grade equivalents.

## Key specs

- Range: 0–4000 µS/cm (EC); equivalent 0–2000 ppm TDS (NaCl scale).
- Accuracy: ± 5 % with fresh calibration.
- Temperature dependence: ~2 % / °C — **must temperature-compensate** against a [DS18B20](./ds18b20.md) in the same fluid.
- Excitation: AC (not DC — DC polarizes the electrodes).
- Probe life: ~1 year with regular cleaning.

## Interface & wiring

- Probe → signal board → analog out to ESP32 ADC (or [ADS1115](../10-storage-timing-power/ads1115-ads1015.md) for better precision).
- 5 V supply on the signal board; analog out in 0–3 V range after conditioning.
- Keep probe and BNC cable short; long cable lengths reduce the AC excitation amplitude.
- Don't mount near pH probe's ground reference — cross-talk lowers pH accuracy.

## Benefits

- Primary hydroponics nutrient-strength metric.
- Cheap enough for per-tank deployment.
- Temperature-compensated EC is a stable long-term metric.
- EC directly correlates with fertilizer dilution, so a "target EC" recipe is easy to automate.

## Limitations / gotchas

- **Temperature compensation is mandatory.** Without it, a 5 °C swing changes EC readings by ~10 %.
- **Calibration is per-solution.** One-point calibration at 1413 µS/cm standard solution is typical; two-point (84 + 1413) for wider range.
- Probes foul with biofilm / mineral scaling — clean monthly with vinegar or probe cleaner.
- TDS "PPM" depends on conversion factor (442 vs NaCl vs KCl); reporting "500 ppm" without saying which scale is meaningless.
- Cheap analog probes drift 5–10 % per month.
- Never measure EC in a nutrient solution currently being aerated with CO₂ injection — CO₂ dissolves and adds conductivity.

## Typical use cases

- Hydroponics nutrient tank strength.
- Aquaponics salinity monitoring.
- Saltwater aquarium salinity (with a marine-specific probe).
- Drinking water TDS monitoring (rough purity indicator).
- Industrial process water.

## Pairs well with

- [`./ds18b20.md`](./ds18b20.md) — **required** for temperature compensation.
- [`./ph-sensor.md`](./ph-sensor.md) — pH + EC is the canonical hydroponics duo.
- [`./turbidity-sensor.md`](./turbidity-sensor.md) — water-quality triplet.
- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) — stable ADC reads.

## Where to buy

- DFRobot Gravity Analog TDS / Analog EC.
- Atlas Scientific (K 0.1 / K 1.0 / K 10 probes, I²C interface, lab accuracy).
- AliExpress bulk TDS — fine for rough monitoring, drifts noticeably.

## Software / libraries

- None dedicated; ADC read + temperature-compensated linear fit.
- DFRobot sample code includes a typical TDS/EC conversion.

## Notes for WeftOS

- HAL exposes both `EC (µS/cm)` and `TDS_ppm (with scale)`; never drop the scale tag.
- Include a per-probe calibration wizard (1413 µS/cm step).
- Alarm on rising EC without user action → evaporation / over-dosing; alarm on falling EC → dilution leak or depletion.
- Pair with dosing-pump automation: route EC targets through the same [relay-module](../07-control-actuators/relay-modules.md) control surface as pH dosing.
