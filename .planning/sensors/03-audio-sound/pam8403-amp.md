---
title: "PAM8403 Stereo Class-D Amplifier"
slug: "pam8403-amp"
category: "03-audio-sound"
part_numbers: ["PAM8403", "PAM8403 mini board"]
manufacturer: "Diodes Incorporated"
interface: ["analog line-in"]
voltage: "2.5–5.5V"
logic_level: "analog"
current_ma: "~6 mA idle, ~1 A peak per channel"
price_usd_approx: "$1 – $4"
esp32_compat: ["ESP32 (via external DAC)", "ESP32-S3", "ESP32-C3"]
tags: [amp, class-d, stereo, pam8403, analog]
pairs_with:
  - "./pcm5102-pcm5122-dac.md"
  - "./max98357a-amp.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/product/2130" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=PAM8403" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=PAM8403" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-PAM8403.html" }
datasheet: "https://www.diodes.com/part/view/PAM8403/"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The PAM8403 is a tiny stereo Class-D amp that accepts analog line-level input and drives two 3 W
speakers. It's not I²S — you need a DAC in front of it — but it's dirt cheap, efficient, and
mechanically tiny, so it's the default "real stereo speaker output" for ESP32 projects that go
through a PCM5102 or similar DAC.

## Key specs

| Spec | Value |
|---|---|
| Output | 2 × 3 W into 4 Ω at 5 V |
| THD | 10% at 3 W |
| Input | Analog line-in (~1 V p-p) |
| Efficiency | ~90% |
| Gain (typical) | 24 dB (fixed on most breakouts, pot-adjustable on others) |

## Interface & wiring

Three analog pins: L-in, R-in, GND (common). Two output pairs for L / R speakers. Power with
stable 5 V — the onboard bulk cap is usually inadequate for bass; add a 470–1000 µF near the
chip.

## Benefits

- Stereo in a $1 module.
- Very efficient (battery-friendly).
- Volume-adjust pot on most breakouts.

## Limitations / gotchas

- Needs analog line-in, so an ESP32 I²S DAC (PCM5102 etc.) is also required for clean audio.
- Pop at power-up if you don't sequence VCC + input correctly.
- Output is BTL — don't ground one speaker terminal.
- No shutdown input on the cheap board variants — it's always drawing idle current.

## Typical use cases

- Jukebox / MP3 player.
- Stereo effect pedals, sound toys.
- Desktop ambient-sound project.

## Pairs well with

- [`./pcm5102-pcm5122-dac.md`](./pcm5102-pcm5122-dac.md) for the ESP32 → analog DAC stage.
- [`./max98357a-amp.md`](./max98357a-amp.md) — the all-in-one I²S alternative.

## Where to buy

- Adafruit / SparkFun / Seeed — known-good breakouts.
- AliExpress for pennies.

## Software / libraries

- None required; it's analog.

## Notes for WeftOS

Speculative: in WeftOS terms this is the "analog audio sink" — symmetrical to the analog mic
case. The PCM5102 → PAM8403 chain is a good test case for chained surfaces where effects live
on the digital side and the analog path is just a terminal.
