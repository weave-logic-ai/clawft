---
title: "TI PCM5102 / PCM5122 I²S DAC"
slug: "pcm5102-pcm5122-dac"
category: "03-audio-sound"
part_numbers: ["PCM5102A", "PCM5122"]
manufacturer: "Texas Instruments"
interface: ["I2S (audio)", "I2C (PCM5122 control)"]
voltage: "3.3V"
logic_level: "3.3V"
current_ma: "~25 mA"
price_usd_approx: "$5 – $15"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [dac, i2s, pcm5102, pcm5122, ti, audio]
pairs_with:
  - "./pam8403-amp.md"
  - "./max98357a-amp.md"
  - "./tas5805m-amp.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=PCM5102" }
  - { vendor: DigiKey, url: "https://www.digikey.com/en/products/search?keywords=PCM5102A" }
  - { vendor: Mouser, url: "https://www.mouser.com/c/?q=PCM5102A" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-PCM5102.html" }
datasheet: "https://www.ti.com/product/PCM5102A"
libraries:
  - { name: "schreibfaul1/ESP32-audioI2S", url: "https://github.com/schreibfaul1/ESP32-audioI2S" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The PCM5102 and PCM5122 are TI's 32-bit, 384 kHz-capable stereo I²S DACs. Pair one with an
external amp (PAM8403, TAS5805M) for cleaner audio than MAX98357A offers. PCM5102 is fixed-function;
PCM5122 adds I²C control, DSP, and internal charge pump for ground-centered output.

## Key specs

| Spec | PCM5102 | PCM5122 |
|---|---|---|
| Resolution | Up to 32-bit | Up to 32-bit |
| Sample rate | Up to 384 kHz | Up to 384 kHz |
| SNR | 112 dB A-weighted | 112 dB A-weighted |
| Output | AC-coupled line | Ground-centered line |
| Control | Pin-strapped | I²C |
| DSP | No | Yes (miniDSP-style) |

## Interface & wiring

Four-wire I²S (BCLK, LRCLK/WS, DIN, optional MCLK). PCM5102 modules have jumpers for de-emphasis
and format selection; PCM5102A derivatives often accept no MCLK (internal PLL). PCM5122 adds I²C
(SDA/SCL). Output is line-level — always feed into an amp, not directly to headphones or
speakers (except the PCM5122 can drive headphones directly at low power).

## Benefits

- Great audio quality — the bottleneck becomes the amp and speakers.
- PCM5122's built-in DSP gives you EQ/mixers without a dedicated DSP chip.
- Widely stocked, well-documented.

## Limitations / gotchas

- Needs an external amp — not a speaker driver on its own.
- PCM5102 module de-emphasis jumper positions vary by clone; check docs.
- Cheap AliExpress "PCM5102" boards sometimes have poor analog layout and hum more than they should.
- MCLK requirement depends on chip revision; get the A variant if you can.

## Typical use cases

- Hi-fi DIY streamer (ESP32 + PCM5102 + PAM8403/TAS5805M + speakers).
- Headphone amp front-end (PCM5122).
- Signal-processing experiments with clean reference playback.

## Pairs well with

- [`./pam8403-amp.md`](./pam8403-amp.md) for cheap stereo.
- [`./max98357a-amp.md`](./max98357a-amp.md) as the all-in-one alternative.
- [`./tas5805m-amp.md`](./tas5805m-amp.md) for the hi-fi chain.

## Where to buy

- Adafruit (PCM5102A breakout).
- DigiKey / Mouser for raw parts.
- AliExpress for clones (quality varies; HiFiBerry-style boards are generally OK).

## Software / libraries

- `schreibfaul1/ESP32-audioI2S` handles PCM5102 as an I²S output.
- ESP-IDF `i2s_std` driver — plain config.

## Notes for WeftOS

Speculative: PCM5122's DSP is interesting — WeftOS could map EQ bands as first-class audio
effect params that happen to be executed on the DAC's silicon rather than the host CPU.
