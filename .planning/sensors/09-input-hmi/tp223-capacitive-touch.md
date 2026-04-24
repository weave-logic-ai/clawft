---
title: "TP223 Capacitive Touch Switch"
slug: "tp223-capacitive-touch"
category: "09-input-hmi"
part_numbers: ["TP223", "TTP223"]
manufacturer: "Tontek"
interface: ["GPIO"]
voltage: "2.0 – 5.5 V"
logic_level: "matches supply (use 3.3 V for ESP32)"
current_ma: "~1.5 µA idle, ~2 mA active"
price_usd_approx: "$0.30 – $1"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [input, touch, capacitive, tp223, ttp223, low-power]
pairs_with:
  - "./momentary-buttons.md"
  - "../04-light-display/ws2812b-neopixel.md"
  - "../06-biometric-presence/pir-hc-sr501.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=ttp223" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=capacitive+touch" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=ttp223" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-ttp223.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-ttp223.html" }
datasheet: "https://datasheet.lcsc.com/lcsc/1810191813_TonTek-Design-Technology-TTP223-BA6_C80757.pdf"
libraries:
  - { name: "ESPHome binary_sensor (gpio)", url: "https://esphome.io/components/binary_sensor/gpio.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The TP223 (aka TTP223) is a single-pad capacitive-touch IC on a tiny PCB. It
turns a copper pad (or solder jumper to an external conductive surface) into a
touch-to-trigger input. Output is a clean logic signal — no debounce needed.
Two jumpers on the back configure **active-low vs active-high** output and
**momentary vs toggle** latching.

## Key specs

| Spec | Typical |
|------|---------|
| Supply | 2.0 – 5.5 V |
| Output | Push-pull CMOS |
| Standby current | ~1.5 µA |
| Response time | ~60 ms fast mode, ~220 ms low-power mode |
| Sensitivity | Adjustable via capacitor to GND |

## Interface & wiring

- 3 pins: Vcc (3.3 V), GND, OUT (to any GPIO).
- Default: OUT is **low**; touching the pad drives it **high** for as long as
  you touch (momentary mode). Flip the `A` jumper for active-low; flip `B` for
  toggle mode (each touch flips the output).
- Works through thin non-conductive covers (acrylic, glass up to ~5 mm).
- If you solder a wire to the trace near the pad, you can extend the touch
  surface into an external conductor (foil, tape).

## Benefits

- Dirt cheap (~$0.30).
- No moving parts — survives millions of touches, works behind waterproof covers.
- Ultra-low standby current — great for battery devices.
- No debounce logic needed in firmware.

## Limitations / gotchas

- **Self-calibrates at power-on.** If the touch surface is covered or a finger
  is on it when booting, it will latch the wrong reference. Always power up
  with no contact on the pad, or add a delay before first read.
- Sensitive to nearby switching (Wi-Fi radio, PWM LEDs) — keep ground planes
  clean and avoid running next to noisy traces.
- Single-pad only — for multi-pad you need TTP229 / MPR121 / ESP32's built-in
  touch peripheral.
- ESP32-S2/S3 have their own touch-sense peripheral that makes the TP223
  redundant for onboard pads.

## Typical use cases

- Wake-from-sleep button behind a glass enclosure.
- Hidden capacitive switch on a finished wooden panel.
- Lamp on/off with toggle jumper.
- Accessibility switch — huge foil pad → TP223 → GPIO.

## Pairs well with

- [`../04-light-display/ws2812b-neopixel.md`](../04-light-display/ws2812b-neopixel.md) — touch-to-toggle LED strip.
- [`../06-biometric-presence/pir-hc-sr501.md`](../06-biometric-presence/pir-hc-sr501.md) — redundant wake inputs.

## Where to buy

Ubiquitous on AliExpress in 10-packs for a few dollars. Adafruit and SparkFun
carry the MPR121 breakout if you outgrow single-pad.

## Software / libraries

- Nothing needed — it's a clean logic output. Use `INPUT` (no pull) on the GPIO.
- ESPHome `binary_sensor: platform: gpio` with `mode: INPUT`.

## Notes for WeftOS

Model as a **DiscreteEventSource** with `source: capacitive_touch`. No debounce
filter needed (the IC handles it). Toggle-mode configuration is a **stateful**
sensor, so expose the jumper choice as sensor metadata so the pipeline knows
not to add edge-detection.
