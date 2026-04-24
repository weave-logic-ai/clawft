---
title: "AD8232 – Single-Lead ECG Analog Front-End"
slug: "ecg-modules"
category: "06-biometric-presence"
part_numbers: ["AD8232"]
manufacturer: "Analog Devices"
interface: ["ADC", "GPIO"]
voltage: "2.0V – 3.5V"
logic_level: "3.3V"
current_ma: "~170 µA typ"
price_usd_approx: "$15 – $25"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [biometric, ecg, biopotential, analog, adc]
pairs_with:
  - "../10-storage-timing-power/ads1115-ads1015.md"
  - "../04-light-display/ws2812b-neopixel.md"
  - "../10-storage-timing-power/microsd.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=AD8232" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=AD8232" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=AD8232" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-ECG.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-AD8232.html" }
datasheet: "https://www.analog.com/en/products/ad8232.html"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The AD8232 is a single-lead **ECG analog front-end**: a low-power instrumentation
amplifier with integrated filtering and lead-off detection that turns three
skin-contact electrodes (RA, LA, RL/reference) into a clean ~1 V ECG waveform
ready for an ADC. The SparkFun / Sparkfun-clone breakouts are the most common
packaging. For WeftOS-style projects it's the canonical "let's actually look at
a heart waveform" entry point, sitting one level above the [MAX30102 PPG
sensor](max30102-max30105.md).

## Key specs

| Spec | Value |
|------|-------|
| Gain | 100 (fixed) plus configurable high-pass filter |
| Bandwidth | ~0.5 – 40 Hz typical cardiac monitor mode |
| Supply | 2.0 – 3.5 V |
| Current | ~170 µA |
| Output | Analog voltage, ~mid-rail ± few hundred mV |
| Leads | 3-lead (RA / LA / RL) |

## Interface & wiring

ESP32 ADC pin to the module's `OUTPUT`, 3.3 V and GND for power. `LO+` / `LO-`
are digital "lead-off" flags — tie them to GPIOs to detect electrode disconnect.
The three electrode leads plug into a 3.5 mm TRS socket; use disposable ECG
electrodes for real-looking traces.

## Benefits

- Gives you an actual ECG waveform, not just a derived HR number.
- Microamp supply → easily battery-powered.
- Lead-off detection prevents "is that a real trace or a loose electrode?" bugs.

## Limitations / gotchas

- **Medical disclaimer:** This is a hobby / learning front-end. Not a medical
  device, not FDA-cleared, not safe for anyone who needs real cardiac monitoring.
- The ESP32's built-in ADC is noisy and nonlinear. For anything beyond a demo,
  sample through an external ADC (see pairings) or filter aggressively.
- Mains-hum pickup on long leads is brutal. Notch-filter at 50 / 60 Hz in
  firmware.
- Biopotential traces are biometric data in most jurisdictions.

## Typical use cases

- Maker / educational ECG visualizer on an OLED or web UI.
- Biofeedback / relaxation-training hack that derives HRV.
- Biopotential-research prototypes (sweat / EMG variants use the same chip family).

## Pairs well with

- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) —
  upgrade from the ESP32's built-in ADC for cleaner traces.
- [`../04-light-display/ws2812b-neopixel.md`](../04-light-display/ws2812b-neopixel.md) —
  pulse-matched visualization.
- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md) —
  log raw waveforms for offline analysis.

## Where to buy

See `buy:`. SparkFun and Adafruit both stock the canonical breakout; clones on
AliExpress are identical reference designs.

## Software / libraries

- SparkFun publishes an Arduino example sketch and a detection-algorithm
  write-up.
- For HR / HRV extraction, the open `Pan-Tompkins` QRS-detector ports to ESP32
  within a handful of kB.

## Notes for WeftOS

ECG traces are **biometric** by default. WeftOS must never stream raw waveforms
to shared namespaces; derived features (HR, HRV) can be opt-in shared. Always
power-gate the AD8232 when not actively sampling.
