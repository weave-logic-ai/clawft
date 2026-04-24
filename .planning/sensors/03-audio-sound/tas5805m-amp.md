---
title: "TI TAS5805M Hi-Fi Stereo I²S Amp"
slug: "tas5805m-amp"
category: "03-audio-sound"
part_numbers: ["TAS5805M"]
manufacturer: "Texas Instruments"
interface: ["I2S (audio)", "I2C (control)"]
voltage: "4.5–26V (PVDD), 3.3V (DVDD)"
logic_level: "3.3V"
current_ma: "~20 mA idle, several A peak"
price_usd_approx: "$12 – $35 (breakout)"
esp32_compat: ["ESP32", "ESP32-S3"]
tags: [amp, hi-fi, i2s, dsp, tas5805m, ti, class-d]
pairs_with:
  - "./max98357a-amp.md"
  - "./pcm5102-pcm5122-dac.md"
  - "./pam8403-amp.md"
buy:
  - { vendor: TI, url: "https://www.ti.com/product/TAS5805M" }
  - { vendor: DigiKey, url: "https://www.digikey.com/en/products/search?keywords=TAS5805M" }
  - { vendor: Mouser, url: "https://www.mouser.com/c/?q=TAS5805M" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-TAS5805M.html" }
datasheet: "https://www.ti.com/product/TAS5805M"
libraries:
  - { name: "sonocotta/esp-tas5805m", url: "https://github.com/sonocotta/esp-tas5805m" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The TAS5805M is TI's Class-D stereo audio amplifier with on-chip DSP — EQ, dynamic range
control, and automatic level control are configurable via I²C. You feed it I²S audio at up to
96 kHz/32-bit and speaker wires come out the other side at up to ~20 W per channel. It is the
"serious audio" upgrade from MAX98357A.

## Key specs

| Spec | Value |
|---|---|
| Output | 2 × 23 W into 4 Ω (at high PVDD) |
| THD+N | 0.03% at 1 W |
| DSP | EQ, DRC, AGL, mixers, pre-gain — all I²C-programmable |
| Input | I²S up to 96 kHz / 32-bit |
| Control | I²C address 0x2C / 0x2D |

## Interface & wiring

I²S (BCLK, LRCLK, DIN, optional MCLK) + I²C control + two speaker outputs. PVDD wants a good
12–24 V supply for real headroom; lower PVDD works but limits output power. The chip has
register-set "tuning" that typically requires TI's PPC3 tool to generate, then the ESP32 writes
the register blob at boot.

## Benefits

- Hi-fi quality in ESP32 projects without external DAC + analog amp chain.
- On-chip EQ / DRC means you can tune a speaker enclosure in software.
- I²C control makes mute, volume, EQ presets all software-addressable.

## Limitations / gotchas

- PVDD needs a meaningful bench/battery supply; 5 V USB won't get you hi-fi power.
- Register init is a hundreds-of-bytes blob from TI's PPC3 tool; not hand-writable.
- Package is QFN — breakouts are necessary; soldering it is a challenge.
- I²C address and I²S mode registers are easy to misconfigure and silence-the-output.

## Typical use cases

- DIY wireless speakers / smart speakers with real bass.
- Home-theater-style ESP32 audio endpoints.
- Anywhere MAX98357A runs out of headroom.

## Pairs well with

- [`./max98357a-amp.md`](./max98357a-amp.md) — the simpler alternative.
- [`./pcm5102-pcm5122-dac.md`](./pcm5102-pcm5122-dac.md) if you prefer DAC + analog amp separation.
- [`./pam8403-amp.md`](./pam8403-amp.md) as the opposite extreme (cheap, noisy).

## Where to buy

- TI direct (datasheet + samples).
- DigiKey / Mouser for parts.
- AliExpress for pre-built TAS5805M breakout boards — quality varies but most work.

## Software / libraries

- `sonocotta/esp-tas5805m` — ESP-IDF init + control driver.
- TI PPC3 desktop software for register-blob generation.

## Notes for WeftOS

Speculative: the TAS5805M is where a WeftOS "audio sink with DSP" surface would be useful — EQ
parameters are effect knobs, not hardware details; volume / mute belong in a consistent audio
policy layer.
