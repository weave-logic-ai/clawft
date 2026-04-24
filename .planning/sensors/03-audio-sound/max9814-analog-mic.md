---
title: "Maxim MAX9814 Analog Electret Mic + AGC"
slug: "max9814-analog-mic"
category: "03-audio-sound"
part_numbers: ["MAX9814"]
manufacturer: "Analog Devices (was Maxim)"
interface: ["ADC (analog)"]
voltage: "2.7–5.5V"
logic_level: "analog (DC-biased around VCC/2)"
current_ma: "~3 mA"
price_usd_approx: "$5 – $12"
esp32_compat: ["ESP32 (ADC1 only when Wi-Fi on)", "ESP32-S3", "ESP32-C3"]
tags: [mic, analog, electret, agc, max9814]
pairs_with:
  - "./electret-mic-breakouts.md"
  - "./inmp441-i2s-mic.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/product/1713" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=MAX9814" }
  - { vendor: DigiKey, url: "https://www.digikey.com/en/products/search?keywords=MAX9814" }
  - { vendor: Mouser, url: "https://www.mouser.com/c/?q=MAX9814" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-MAX9814.html" }
datasheet: "https://www.analog.com/en/products/max9814.html"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The MAX9814 is an electret-mic preamplifier with automatic gain control (AGC). Output is analog
line-level audio, not digital — it suits ESP32 ADC-based projects where I²S wiring is not
available (or the part is too expensive / tricky for an early prototype).

## Key specs

| Spec | Value |
|---|---|
| Gain | 40 / 50 / 60 dB (selectable via pin-strap) |
| AGC attack/release | Selectable via CT pin cap |
| THD | 1% at full output |
| Output bias | ~VCC/2 |
| Peak-to-peak output | ~2 V (AGC-managed) |

## Interface & wiring

VCC (3–5 V), GND, OUT → ESP32 ADC1 pin. If Wi-Fi is active, ADC2 pins are unusable; use ADC1.
Pick a sample rate consistent with useful audio (≥16 kHz); the ESP32 built-in ADC isn't great
above ~30 kHz / 12-bit linearity, so budget for noise.

## Benefits

- No I²S peripheral required.
- AGC keeps faint / loud sources both in range — good for clap sensors and hand claps.
- Tolerates a wide input range.

## Limitations / gotchas

- ESP32 ADC is noisy; don't expect CD-quality recordings.
- AGC is a feature, not a bug, but it's bad for amplitude-based detection (level moves).
- Analog routing picks up Wi-Fi / switching noise — keep traces short and filter VCC.

## Typical use cases

- Clap / sound-level sensor.
- Voice-triggered toys where accuracy isn't critical.
- Quick prototypes before upgrading to I²S.

## Pairs well with

- [`./electret-mic-breakouts.md`](./electret-mic-breakouts.md) — the cheaper fallback.
- [`./inmp441-i2s-mic.md`](./inmp441-i2s-mic.md) as the upgrade path.

## Where to buy

- Adafruit / SparkFun / DigiKey / Mouser for authentic parts.
- AliExpress for cheap boards — often rebranded clones.

## Software / libraries

- None required — plain `analogRead` / `adc_oneshot_read` with a simple RMS window.

## Notes for WeftOS

Speculative: the MAX9814 is a good example of an "analog audio source surface" that downstream
effects must treat differently from I²S (mainly because AGC makes absolute amplitude unreliable);
WeftOS should tag the surface with an "AGC=on" metadata flag.
