---
title: "Maxim MAX98357A I²S Class-D Mono Amp"
slug: "max98357a-amp"
category: "03-audio-sound"
part_numbers: ["MAX98357A"]
manufacturer: "Analog Devices (was Maxim)"
interface: ["I2S"]
voltage: "2.5–5.5V"
logic_level: "3.3V"
current_ma: "idle ~3 mA, peak 1 A+ (speaker dependent)"
price_usd_approx: "$4 – $9"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [amp, class-d, mono, max98357a, i2s, speaker]
pairs_with:
  - "./inmp441-i2s-mic.md"
  - "./pam8403-amp.md"
  - "./pcm5102-pcm5122-dac.md"
  - "../10-storage-timing-power/index.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/product/3006" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=MAX98357" }
  - { vendor: DigiKey, url: "https://www.digikey.com/en/products/search?keywords=MAX98357A" }
  - { vendor: Mouser, url: "https://www.mouser.com/c/?q=MAX98357A" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-MAX98357A.html" }
datasheet: "https://www.analog.com/en/products/max98357a.html"
libraries:
  - { name: "schreibfaul1/ESP32-audioI2S", url: "https://github.com/schreibfaul1/ESP32-audioI2S" }
  - { name: "espressif/esp-adf", url: "https://github.com/espressif/esp-adf" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The MAX98357A is the ESP32 default "put audio out of a small speaker" chip: it takes I²S in,
integrates a DAC + 3 W Class-D amp, and drives a 4–8 Ω speaker directly. No analog wiring, no
external DAC, no amp tuning — three data wires and a speaker.

## Key specs

| Spec | Value |
|---|---|
| Output | Class-D, 3.2 W into 4 Ω at 5 V |
| Input | I²S 16/24/32-bit, 8–96 kHz |
| SNR | 92 dB |
| Efficiency | ~92% |
| Gain | 3 / 6 / 9 / 12 / 15 dB (pin-strapped) |

## Interface & wiring

Four-wire I²S (BCLK, LRC, DIN, GND) plus VDD and a SD/mode pin. Speaker wires go straight to the
chip's OUT+ / OUT-. Keep the speaker wires short and twisted; Class-D switching noise radiates.

## Benefits

- Everything-in-one: DAC + amp + speaker driver.
- Efficient — battery-friendly.
- Easy ESP32 I²S driver compatibility.

## Limitations / gotchas

- Mono only. For stereo use two boards (opposite L/R selects) or a stereo amp.
- Peak current on kicks/bass can brown out weak 5 V rails — budget a decent LDO or DC-DC and a
  bulk cap ≥220 µF.
- No volume register — volume is digital, in software. Gain pin is coarse.
- The chip gets warm at sustained full output — allow airflow.

## Typical use cases

- Talking kiosks / toys.
- Notification speakers / doorbells.
- MP3 / stream playback on ESP32.

## Pairs well with

- [`./inmp441-i2s-mic.md`](./inmp441-i2s-mic.md) for duplex voice.
- [`./pam8403-amp.md`](./pam8403-amp.md) — cheap analog stereo alternative.
- [`./pcm5102-pcm5122-dac.md`](./pcm5102-pcm5122-dac.md) when you want line-out into a bigger amp.
- [`../10-storage-timing-power/index.md`](../10-storage-timing-power/index.md) — plan current capacity.

## Where to buy

- Adafruit / SparkFun / DigiKey / Mouser.
- AliExpress for clone breakouts (quality varies, but mostly fine).

## Software / libraries

- `schreibfaul1/ESP32-audioI2S` — widely used ESP32 audio playback lib.
- `espressif/esp-adf` for pipelined audio (TTS, decode, output).

## Notes for WeftOS

Speculative: treat MAX98357A as a canonical I²S "audio sink surface" — pairs symmetrically with
the mic surface; effects (ducking, EQ, limiter) can be inserted without the application caring
about the specific amp.
