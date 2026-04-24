---
title: "MAX17048 LiPo Fuel Gauge"
slug: "max17048-fuel-gauge"
category: "10-storage-timing-power"
part_numbers: ["MAX17048", "MAX17049 (2S)"]
manufacturer: "Analog Devices (Maxim)"
interface: ["I2C"]
voltage: "2.5 – 4.5 V (from battery)"
logic_level: "3.3V"
current_ma: "~23 µA active, 1 µA hibernate"
price_usd_approx: "$3 – $8"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [power, fuel-gauge, lipo, battery, soc, max17048, modelgauge, i2c]
pairs_with:
  - "./tp4056-lipo-charger.md"
  - "./solar-charge-controllers.md"
  - "./ina219-ina260.md"
  - "./ds3231-rtc.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=max17048" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=fuel+gauge" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=max17048" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-fuel%20gauge.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-max17048.html" }
datasheet: "https://www.analog.com/en/products/max17048.html"
libraries:
  - { name: "Adafruit_MAX1704x", url: "https://github.com/adafruit/Adafruit_MAX1704X" }
  - { name: "SparkFun_MAX1704x", url: "https://github.com/sparkfun/SparkFun_MAX1704x_Fuel_Gauge_Arduino_Library" }
  - { name: "ESPHome max17043",  url: "https://esphome.io/components/sensor/max17043.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The MAX17048 is a Maxim **ModelGauge** battery fuel gauge: it estimates
state-of-charge (%) for a 1-cell LiPo / Li-ion cell by tracking open-circuit
voltage through an internal battery model — **without** a sense resistor. It
also reports battery voltage and a "charge/discharge rate" in %/h. The
MAX17049 is the 2-cell variant. It's the "just trust it" replacement for
voltage-only SoC tables.

## Key specs

| Spec | MAX17048 |
|------|----------|
| Cells | 1S LiPo / Li-ion |
| Voltage range | 2.5 – 4.5 V |
| Reported SoC | 0.0 – 100.0 % (1% resolution) |
| Voltage resolution | 1.25 mV |
| Quiescent | ~23 µA active, 1 µA hibernate |
| Interface | I²C @ 0x36 |

## Interface & wiring

- Tied **directly to battery positive** (VBAT pin), not to the regulated rail
  — it needs to see the cell voltage through load transients.
- I²C to MCU at 0x36.
- `ALRT` pin: configurable interrupt (SoC threshold, undervoltage).
- That's it — no shunt, no calibration capacitor, no external R/C.

## Benefits

- No sense resistor — smallest PCB footprint of any fuel gauge.
- Per-chip pre-programmed with a generic LiPo discharge curve; works fine for
  typical 3.7 V / 4.2 V LiPos out of the box.
- Low quiescent current — won't dominate sleep current on a battery node.
- Tiny microlite (2 × 0.88 mm) — or buy the Adafruit / SparkFun breakout.

## Limitations / gotchas

- The default battery model is a compromise. For **very high-capacity** (>5 Ah)
  or **very non-standard** cells (LiFePO4, silicon-anode), the SoC will be off
  by 5–10%. For precision, move to MAX17260 / BQ27441 with a learning cycle.
- Gauge needs **relaxed rest** periods (no load) to recalibrate from OCV —
  always-on loads can make SoC drift until you deep-sleep.
- I²C 0x36 is not super-common but check the rest of your bus.
- The chip does **not** report current — it infers rate from voltage slope.
  Pair with [`INA219`](./ina219-ina260.md) for actual amperes.

## Typical use cases

- Battery-powered ESP32 nodes that need to display / telemeter SoC.
- Solar / off-grid sensors that should deep-sleep earlier as SoC drops.
- Portable devices with "battery %" UIs (badges, handhelds).

## Pairs well with

- [`./tp4056-lipo-charger.md`](./tp4056-lipo-charger.md) — together they are the minimum "smart battery" stack.
- [`./solar-charge-controllers.md`](./solar-charge-controllers.md) — close the solar-state-of-charge loop.
- [`./ina219-ina260.md`](./ina219-ina260.md) — actual current for cross-check.
- [`./ds3231-rtc.md`](./ds3231-rtc.md) — timestamp discharge curves.

## Where to buy

Adafruit 5580 (MAX17048 Stemma QT) — the cleanest. SparkFun Qwiic version.
AliExpress clones exist but QC varies.

## Software / libraries

- `Adafruit_MAX1704x` — returns %, voltage, and "charge rate" in one call.
- `SparkFun_MAX1704x` — Qwiic-friendly, identical API.
- ESPHome `sensor: platform: max17043` — compatible register map.

## Notes for WeftOS

Model SoC as a **ScalarSource** (0.0–1.0) with a capability flag indicating
"non-learning model" — higher layers can choose to apply a correction curve if
the cell type is known. Good candidate to feed the WeftOS power-aware
scheduler's budget predictions.
