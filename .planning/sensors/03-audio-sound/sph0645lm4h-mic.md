---
title: "Knowles SPH0645LM4H I²S MEMS Microphone"
slug: "sph0645lm4h-mic"
category: "03-audio-sound"
part_numbers: ["SPH0645LM4H", "SPH0645LM4H-B"]
manufacturer: "Knowles"
interface: ["I2S"]
voltage: "1.62–3.6V"
logic_level: "1.8–3.3V"
current_ma: "~0.6 mA"
price_usd_approx: "$5 – $8"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [mic, i2s, mems, sph0645, adafruit]
pairs_with:
  - "./inmp441-i2s-mic.md"
  - "./max98357a-amp.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/product/3421" }
  - { vendor: DigiKey, url: "https://www.digikey.com/en/products/search?keywords=SPH0645LM4H" }
  - { vendor: Mouser, url: "https://www.mouser.com/c/?q=SPH0645LM4H" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-SPH0645LM4H.html" }
libraries:
  - { name: "adafruit/Adafruit_ZeroI2S", url: "https://github.com/adafruit/Adafruit_ZeroI2S" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The SPH0645LM4H is Knowles' I²S MEMS mic — functionally the Adafruit-favored alternative to the
INMP441. Lower current, similar SNR, and identical I²S wiring make it a straight swap when
you want a second-source mic or you're already buying from Adafruit.

## Key specs

| Spec | Value |
|---|---|
| Output | 24-bit I²S PCM |
| Sensitivity | -26 dBFS |
| SNR | 65 dB A-weighted |
| Frequency response | 50 Hz – 15 kHz |
| Max SPL | 120 dB |

## Interface & wiring

Same three-wire I²S (BCLK, LRCL, DOUT) plus SEL for left/right select. Adafruit's breakout adds a
SEL pin, VDD, GND, and an internal regulator so you can run the board at 3.3 V directly.

## Benefits

- Well-supported Adafruit breakout (great docs).
- Low current (<1 mA) suits battery wearables.
- Identical pipeline to INMP441 — you can mix them in a single project.

## Limitations / gotchas

- Pricier than INMP441 clones.
- Same "top 18 bits valid" quirk as other 24-bit I²S MEMS mics.
- Only one per I²S data line unless you use two different L/R selects.

## Typical use cases

- Wearables / battery voice capture.
- Portable recorder / logger.
- Adafruit-ecosystem projects using CircuitPython.

## Pairs well with

- [`./inmp441-i2s-mic.md`](./inmp441-i2s-mic.md) — direct alternative.
- [`./max98357a-amp.md`](./max98357a-amp.md) as speaker output.

## Where to buy

- Adafruit direct (the canonical board).
- DigiKey / Mouser for raw SPH0645LM4H parts.
- AliExpress for clones.

## Software / libraries

- Adafruit Learn System examples for CircuitPython.
- Same `i2s_read` path as INMP441 under Arduino / ESP-IDF.

## Notes for WeftOS

Speculative: since SPH0645 and INMP441 present the same I²S surface, WeftOS should treat the
mic choice as a hardware descriptor detail, not a different driver class — same surface, same
effect chain, different provider.
