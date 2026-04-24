---
title: "Potentiometers (Rotary + Slide)"
slug: "potentiometers"
category: "09-input-hmi"
part_numbers: ["10kΩ rotary pot", "10kΩ slide pot", "B10K"]
manufacturer: "ALPS / Bourns / generic"
interface: ["ADC"]
voltage: "3.3V"
logic_level: "3.3V"
current_ma: "0.33 mA (10 kΩ across 3.3 V)"
price_usd_approx: "$0.30 – $3"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [input, potentiometer, analog, adc, slider]
pairs_with:
  - "./joysticks.md"
  - "../10-storage-timing-power/ads1115-ads1015.md"
  - "../07-control-actuators/hobby-servos.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=potentiometer" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=potentiometer" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=potentiometer" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-potentiometer.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-b10k-potentiometer.html" }
datasheet: ""
libraries:
  - { name: "ResponsiveAnalogRead", url: "https://github.com/dxinteractive/ResponsiveAnalogRead" }
  - { name: "ESPHome adc",          url: "https://esphome.io/components/sensor/adc.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A potentiometer is a three-terminal variable resistor: wire the outer two across
Vcc↔GND and read the wiper as an analog voltage. It's the simplest way to get
continuous control input (volume, brightness, trim). Rotary pots come as panel
mount (B10K most common); slide pots are 10–100 mm linear travel.

## Key specs

| Spec | Typical |
|------|---------|
| Value | 10 kΩ (B10K) is the default |
| Taper | `B` = linear, `A` = audio-log |
| Travel | 270° rotary / 10–100 mm slide |
| Tolerance | ±20% |
| Life | 10 k – 100 k cycles carbon, >1 M cycles conductive plastic |

## Interface & wiring

- 3 pins: End-A (3.3 V), End-B (GND), Wiper (ADC input).
- **Use 3.3 V, not 5 V.** ESP32 ADC full-scale is 3.3 V; going higher clips.
- Always use **ADC1** channels (GPIO 32–39) on ESP32 — ADC2 is blocked while
  Wi-Fi is active.
- A 10 kΩ pot presents ~5 kΩ source impedance at mid-travel; the ESP32 ADC's
  sample-and-hold likes < 10 kΩ, so you're fine. For a 100 kΩ pot, add a 1 nF
  cap from wiper→GND to lower effective source impedance.

## Benefits

- Cheap, simple, no driver software.
- Absolute position — no "where am I" ambiguity like a rotary encoder.
- Works for both rotary knobs and slide faders.

## Limitations / gotchas

- ESP32 ADC is noisy (±20–50 LSB typical) and non-linear — plan on software
  smoothing and dead-band logic.
- Wi-Fi radio activity visibly modulates ADC readings; use the
  [`ADS1115`](../10-storage-timing-power/ads1115-ads1015.md) for anything
  precise (audio faders, motor-position feedback).
- Mechanical wear causes "scratchy" output zones over time; prefer
  conductive-plastic or Hall-effect pots for long life.
- Finite travel (unlike an encoder) limits use as a "throttle" control that
  needs re-centering.

## Typical use cases

- Volume / brightness / speed control.
- Servo position setter (pot → servo map).
- DIY synth parameter knobs (a bank of 8 on ADS1115).
- Tuning parameters in the field without a screen.

## Pairs well with

- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) — clean 16-bit reads.
- [`../07-control-actuators/hobby-servos.md`](../07-control-actuators/hobby-servos.md) — manual servo jog.
- [`./joysticks.md`](./joysticks.md) — pots packaged as a 2-axis bundle.

## Where to buy

Panel-mount B10K on AliExpress for pennies; Bourns 3386 trimmer pots at
Mouser/Digi-Key for precision trim.

## Software / libraries

- `analogRead()` / `adc1_get_raw()`.
- `ResponsiveAnalogRead` — responsive-but-stable smoothing, great for faders.
- ESP-IDF ADC **oneshot** API with calibration fuses for linearized reads.

## Notes for WeftOS

Model as a **ScalarSource** normalized to [0, 1]. Always wrap with a
smoothing/hysteresis filter because raw ESP32 ADC is too noisy to surface
directly. If paired with an external ADC, record the transport chain in
sensor metadata so the pipeline can skip redundant filtering.
