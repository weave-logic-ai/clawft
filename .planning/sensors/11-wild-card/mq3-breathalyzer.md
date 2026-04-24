---
title: "MQ-3 Alcohol Sensor (Breathalyzer)"
slug: "mq3-breathalyzer"
category: "11-wild-card"
part_numbers: ["MQ-3"]
manufacturer: "Hanwei / Winsen"
interface: ["ADC", "GPIO (digital threshold)"]
voltage: "5 V (heater)"
logic_level: "5V analog; digital out is comparator"
current_ma: "~150 mA (heater)"
price_usd_approx: "$2 – $5"
esp32_compat: ["ESP32 via ADC + level safety"]
tags: [mq-3, alcohol, breathalyzer, gas, analog, niche]
pairs_with:
  - "../05-environmental/mq-series.md"
  - "../10-storage-timing-power/ads1115-ads1015.md"
  - "../07-control-actuators/relay-modules.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=mq-3" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=mq-3" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=mq-3" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-mq-3.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-mq-3.html" }
datasheet: "https://www.sparkfun.com/datasheets/Sensors/MQ-3.pdf"
libraries:
  - { name: "MQUnifiedsensor", url: "https://github.com/miguel5612/MQSensorsLib" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The MQ-3 is a tin-dioxide (SnO₂) electrochemical gas sensor from the MQ family
(see the full index at
[`../05-environmental/mq-series.md`](../05-environmental/mq-series.md)),
tuned for **ethanol vapour** in the 0.05 – 10 mg/L range. Paired with an
ESP32 analog input (preferably via an
[`ADS1115`](../10-storage-timing-power/ads1115-ads1015.md)), it makes a
proof-of-concept breath alcohol detector. Accuracy is not police-grade.

## Key specs

| Spec | MQ-3 |
|------|------|
| Target gas | Ethanol (C₂H₅OH) |
| Range | ~0.05 – 10 mg/L (0.025 – 5 BAC equivalent) |
| Heater supply | 5 V @ ~150 mA |
| Warm-up | 24 h initial; 5–10 min per cold-start |
| Output | Analog voltage from load-resistor divider; digital comparator on module |

## Interface & wiring

- Most modules expose VCC (5 V), GND, AOUT (analog), DOUT (digital
  comparator). A small pot on the board adjusts the digital threshold.
- AOUT scales with gas concentration but ranges to ~5 V — feed through a
  divider or (better) an ADS1115 in ±4 V mode. Do **not** wire 5 V AOUT
  directly into an ESP32 ADC pin.
- Heater draws ~150 mA continuously — budget accordingly, this is not a
  battery-friendly sensor.
- Allow **5+ minutes warm-up** after every power-on before trusting readings.

## Benefits

- Cheap, fast response (< 10 s).
- Dramatic demo — exhale, see the number spike.
- Part of a broader MQ family you can mix-and-match for multi-gas sniffing.

## Limitations / gotchas

- **Not a medical / legal breathalyzer.** No NIST calibration, no flow control,
  cross-sensitive to acetone/benzene/methane.
- Sensitivity drifts over months with use. Calibrate against known samples
  periodically (an R₀ calibration in clean air is mandatory).
- 150 mA heater kills battery projects.
- Cross-sensitivity: the MQ-3 also responds to CO, CH₄, H₂, benzene — not
  selective.
- Humidity shifts the baseline; calibrate in your deployed environment.

## Typical use cases

- Arcade-style "blow on the sensor" party game.
- Teaching demos for gas sensing.
- DIY "don't drink and drive" toy — **for fun, not for safety decisions**.

## Pairs well with

- [`../05-environmental/mq-series.md`](../05-environmental/mq-series.md) — context on the whole MQ family, calibration, gotchas.
- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) — clean readings without 5 V clipping.
- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) — trip a light / door mechanism above threshold.

## Where to buy

AliExpress ~$2 per module. SparkFun and DFRobot sell MQ-3 breakouts with
proper load-resistor documentation.

## Software / libraries

- `MQUnifiedsensor` — covers the whole MQ family including R₀ calibration.
- Or roll your own: log-linear regression between AOUT voltage and
  concentration per the datasheet's Rs/R₀ curve.

## Notes for WeftOS

Model as a **ScalarSource** (mg/L ethanol) with a mandatory `warmup_seconds`
gate, a `baseline_r0` calibration value, and a cross-sensitivity warning in
metadata. WeftOS should refuse to surface "BAC"-labeled readings from an MQ-3 —
only a "relative ethanol intensity" channel.
