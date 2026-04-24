---
title: "Generic Electret Mic Breakouts"
slug: "electret-mic-breakouts"
category: "03-audio-sound"
part_numbers: ["KY-037", "KY-038", "MAX4466 breakout", "LM386 breakout"]
manufacturer: "various"
interface: ["ADC (analog)", "digital comparator"]
voltage: "3.3V / 5V"
logic_level: "analog + comparator digital"
current_ma: "~1–5 mA"
price_usd_approx: "$1 – $4"
esp32_compat: ["ESP32 (ADC1)", "ESP32-S3", "ESP32-C3"]
tags: [mic, electret, analog, ky-037, ky-038, max4466, fallback]
pairs_with:
  - "./max9814-analog-mic.md"
  - "./inmp441-i2s-mic.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=electret+mic" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=electret" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-KY-037.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

"Sound sensor" breakouts — KY-037, KY-038, MAX4466 boards, or LM386 "microphone amplifier"
modules — are the $1 fallback when you need *some* audio signal and don't care about fidelity.
They're fine for clap detection, sound-triggered LEDs, or teaching. For anything voice-quality,
skip to the I²S parts in this category.

## Key specs

| Breakout | Output | Quality |
|---|---|---|
| KY-037 / KY-038 | Analog + digital threshold comparator | Poor SNR, good for clap detection |
| MAX4466 | Analog, gain pot | Better than KY-03x; ~50 Hz–20 kHz usable |
| LM386 | Analog, high gain | Noisy; fine for triggers |

## Interface & wiring

Three or four pins: VCC, GND, AOUT, and (on -038-style boards) DOUT, a digital signal that
asserts when a threshold is crossed (trimmed via onboard pot). ESP32-side: AOUT → ADC1 pin, or
DOUT → GPIO interrupt.

## Benefits

- Pennies per module.
- DOUT gives a zero-code "loud noise detected" interrupt.
- Great for classroom / teaching.

## Limitations / gotchas

- Noisy. Don't pretend you have an audio recorder.
- No AGC; loud sources clip instantly.
- Pot-tuned threshold drifts with temperature and mechanical jostling.
- Quality varies massively across AliExpress lots.

## Typical use cases

- Clap-on / clap-off switches.
- Ambient-noise LED visualizers (very crude).
- Workshop learning projects.

## Pairs well with

- [`./max9814-analog-mic.md`](./max9814-analog-mic.md) — the slightly-better analog option.
- [`./inmp441-i2s-mic.md`](./inmp441-i2s-mic.md) as the proper upgrade.

## Where to buy

- Adafruit / SparkFun for MAX4466.
- AliExpress for KY-037 / KY-038 in 10-packs.

## Software / libraries

- `analogRead` + moving-average RMS is usually all you need.

## Notes for WeftOS

Speculative: this class should exist in WeftOS mostly as a "trigger input" surface — a boolean
or low-rate scalar, not a PCM stream — to avoid pretending the audio is any good.
