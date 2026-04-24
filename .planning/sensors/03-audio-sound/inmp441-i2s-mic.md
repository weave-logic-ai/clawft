---
title: "InvenSense INMP441 I²S MEMS Microphone"
slug: "inmp441-i2s-mic"
category: "03-audio-sound"
part_numbers: ["INMP441"]
manufacturer: "InvenSense (TDK)"
interface: ["I2S"]
voltage: "1.8–3.3V"
logic_level: "1.8–3.3V"
current_ma: "~1.4 mA"
price_usd_approx: "$2 – $6"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [mic, i2s, mems, inmp441, digital-audio]
pairs_with:
  - "./sph0645lm4h-mic.md"
  - "./max98357a-amp.md"
  - "./dual-mic-arrays.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=INMP441" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=INMP441" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=INMP441" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-INMP441.html" }
datasheet: "https://invensense.tdk.com/products/digital/inmp441/"
libraries:
  - { name: "espressif/esp-adf", url: "https://github.com/espressif/esp-adf" }
  - { name: "atomic14/esp32-i2s-mic-test", url: "https://github.com/atomic14/esp32-i2s-mic-test" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The INMP441 is the default "ESP32 I²S microphone" — an omnidirectional digital MEMS mic with 24-bit
audio, flat response, and no analog anti-aliasing wiring to get wrong. It's usually sold on a
$3 6-pin breakout; drop it on a breadboard, hook it to an ESP32 I²S peripheral, and you have
clean 16 kHz / 44.1 kHz audio in minutes.

## Key specs

| Spec | Value |
|---|---|
| Output | 24-bit I²S PCM |
| Sensitivity | -26 dBFS |
| SNR | 61 dB A-weighted |
| Frequency response | 60 Hz – 15 kHz (usable) |
| Max SPL | 120 dB |
| Interface | I²S (SD, SCK, WS/LRCL, L/R select) |

## Interface & wiring

Pins: VDD, GND, L/R (channel select — tie to GND for left, VDD for right), WS (LRCL), SCK (BCLK),
SD (data). ESP32 I²S peripheral drives SCK+WS; INMP441 clocks data out on SD. Typical BCLK is
1.536 MHz for 48 kHz @ 32-bit. Keep wires short (or use shielded cable on longer runs).

## Benefits

- Clean I²S audio with no analog fuss.
- Well-documented, widely copied — every ESP32 voice tutorial uses it.
- Small enough to embed in wearables.

## Limitations / gotchas

- **Only the top 18 bits** of the 24-bit word are valid audio; the bottom 6 are noise. Shift or
  mask on the ESP32 side.
- Single-chip only — for stereo you need two on the same bus with L/R set opposite.
- TDK has put the INMP441 on "not recommended for new designs" status; stock is still fine as of
  early 2026 but plan a migration (SPH0645LM4H or ICS-43434).
- The PCB pads are small — the breakout is easier than soldering the bare MEMS.

## Typical use cases

- Wake-word / voice assistant input.
- Audio streaming over Wi-Fi / BT.
- Smart-speaker style intercom.
- Noise / sound-pressure-level logging.

## Pairs well with

- [`./max98357a-amp.md`](./max98357a-amp.md) as the speaker-side counterpart.
- [`./sph0645lm4h-mic.md`](./sph0645lm4h-mic.md) — drop-in alternative.
- [`./dual-mic-arrays.md`](./dual-mic-arrays.md) for stereo / beamforming.

## Where to buy

- Adafruit / SparkFun / Seeed — verified stock.
- AliExpress for $2 breakouts by the 5-pack.

## Software / libraries

- `espressif/esp-adf` — the ADF pipeline handles I²S capture end-to-end.
- `atomic14/esp32-i2s-mic-test` — minimal reference sketch.
- Arduino: `i2s_read` in the ESP32 core.

## Notes for WeftOS

Speculative: this is the canonical "I²S audio source surface" in WeftOS — a streaming PCM
producer whose sample rate, bit depth, and channel map are declared up front, and whose effects
(VAD, wake-word, AEC) are pluggable.
