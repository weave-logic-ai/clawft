---
title: "Dual / Multi-Mic Arrays (ESP32-S3 AI voice)"
slug: "dual-mic-arrays"
category: "03-audio-sound"
part_numbers: ["ESP32-S3-BOX", "ESP32-S3-Korvo-2", "Seeed ReSpeaker Lite", "dual INMP441"]
manufacturer: "Espressif / Seeed / DIY"
interface: ["I2S (2+ mics)", "TDM"]
voltage: "3.3V"
logic_level: "3.3V"
current_ma: "~3–10 mA per mic"
price_usd_approx: "$10 (DIY pair) – $60 (dev kit)"
esp32_compat: ["ESP32-S3"]
tags: [mic, array, beamforming, aec, esp32-s3, wake-word]
pairs_with:
  - "./inmp441-i2s-mic.md"
  - "./sph0645lm4h-mic.md"
  - "./max98357a-amp.md"
buy:
  - { vendor: Espressif, url: "https://www.espressif.com/en/news/esp32-s3-box" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=ReSpeaker" }
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=dual+microphone" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-esp32-mic-array.html" }
libraries:
  - { name: "espressif/esp-skainet", url: "https://github.com/espressif/esp-skainet" }
  - { name: "espressif/esp-sr", url: "https://github.com/espressif/esp-sr" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

Once you pair mics to do AI voice on ESP32-S3, you start wanting array techniques — two mics for
stereo / minimal beamforming, or 3–6 mics for acoustic echo cancellation (AEC) and steered
beamforming. Espressif ships `esp-skainet` and `esp-sr` libraries that expect a 2-mic input on
S3; dev boards like ESP32-S3-BOX and ESP32-S3-Korvo-2 provide the hardware.

## Key specs

| Setup | Mics | Good for |
|---|---|---|
| DIY dual INMP441 | 2 | Stereo capture, basic beamforming |
| ESP32-S3-BOX | 2 | Wake-word + voice assistant demos |
| ESP32-S3-Korvo-2 | 3 mics + ref loopback | AEC + beamforming, the Espressif reference |
| ReSpeaker Lite / 2-mic HAT | 2 | Home-assistant style voice UI |

## Interface & wiring

Two INMP441s share SCK and WS; each gets its own SD line, OR they share SD with opposite L/R
selects so one word pair carries both channels. TDM variants pack 4+ mics on a single data line.
For AEC you additionally feed the speaker output back as a reference channel.

## Benefits

- Real wake-word + AEC on-device (the whole point of ESP32-S3).
- Beamforming improves SNR in noisy rooms without changing the mic hardware.
- Espressif reference code "just works" on the matched dev boards.

## Limitations / gotchas

- Board layout for arrays matters — mic spacing defines beam geometry; copy the reference
  layout before inventing.
- AEC needs the exact speaker signal as a reference — can't cheat with "we'll pipe it over Wi-Fi."
- `esp-sr` models are sizable — you need the ESP32-S3 variant with ≥8 MB PSRAM.
- Not all ESP32-S3 modules have the right I²S / TDM pin muxing — check the variant.

## Typical use cases

- Home assistants / smart speakers.
- Always-on wake-word triggers.
- Voice-activated lighting / appliances.
- Directional sound pickup for classroom mics.

## Pairs well with

- [`./inmp441-i2s-mic.md`](./inmp441-i2s-mic.md) — the building block for DIY arrays.
- [`./sph0645lm4h-mic.md`](./sph0645lm4h-mic.md) as a mix-in alternative.
- [`./max98357a-amp.md`](./max98357a-amp.md) for the speaker half of an AEC loop.

## Where to buy

- Espressif for ESP32-S3-BOX / Korvo-2.
- Seeed for ReSpeaker Lite.
- Adafruit for individual MEMS mics.
- AliExpress for clones of Korvo / BOX (verify PSRAM spec).

## Software / libraries

- `espressif/esp-skainet` — wake-word + VUI framework.
- `espressif/esp-sr` — AEC, NS, AGC, VAD, WakeNet.

## Notes for WeftOS

Speculative: multi-mic arrays are the "multi-channel audio surface" case — WeftOS should carry
per-channel metadata (geometry, reference-channel flag) so effects like AEC or beamforming know
which channel is which. This is where audio differs structurally from a camera frame.
