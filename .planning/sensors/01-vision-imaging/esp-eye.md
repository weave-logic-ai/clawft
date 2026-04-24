---
title: "Espressif ESP-EYE"
slug: "esp-eye"
category: "01-vision-imaging"
part_numbers: ["ESP-EYE"]
manufacturer: "Espressif"
interface: ["USB", "I2C", "I2S"]
voltage: "5V (USB)"
logic_level: "3.3V"
current_ma: "~170 mA streaming"
price_usd_approx: "$22 – $40"
esp32_compat: ["ESP32"]
tags: [camera, esp-eye, ai, face-recognition, voice, official]
pairs_with:
  - "./esp32-cam-ov2640.md"
  - "../03-audio-sound/inmp441-i2s-mic.md"
  - "../10-storage-timing-power/microsd.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=ESP-EYE" }
  - { vendor: DigiKey,  url: "https://www.digikey.com/en/products/search?keywords=ESP-EYE" }
  - { vendor: Mouser,   url: "https://www.mouser.com/c/?q=ESP-EYE" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=ESP-EYE" }
datasheet: "https://www.espressif.com/en/products/devkits/esp-eye/overview"
libraries:
  - { name: "espressif/esp-who", url: "https://github.com/espressif/esp-who" }
  - { name: "espressif/esp32-camera", url: "https://github.com/espressif/esp32-camera" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

ESP-EYE is Espressif's own reference board for AI vision + voice on ESP32: WROVER module, 2 MP
OV2640, PDM microphone, USB-C programming, and a form factor tuned for face-recognition and
wake-word demos. It's the board the `esp-who` and `esp-skainet` samples target first.

## Key specs

| Spec | Value |
|---|---|
| MCU module | ESP32-WROVER (8 MB flash, 8 MB PSRAM on current rev) |
| Sensor | OV2640, 2 MP |
| Mic | Digital PDM mic onboard |
| USB | USB-C, built-in bridge |
| Buttons | BOOT, RESET, function |

## Interface & wiring

Single USB-C cable. Mic is exposed via PDM; camera via the standard DVP + SCCB. No SD slot, so
plan on streaming / remote storage unless you add one via SPI.

## Benefits

- First-class support in Espressif AI reference code.
- Decent mic on the same board — good for wake-word + visual combos.
- Reliable and predictable — the "known-good" ESP32 vision board.

## Limitations / gotchas

- Pricier than AI-Thinker ESP32-CAM for similar imaging hardware.
- No SD slot by default.
- Original ESP32 (not S3), so modern AI model performance is limited vs ESP32-S3 boards.

## Typical use cases

- Face recognition demos (`esp-who`).
- Wake-word + camera combined triggers.
- Reference hardware when reproducing Espressif tutorials verbatim.

## Pairs well with

- [`./esp32-cam-ov2640.md`](./esp32-cam-ov2640.md) for the cheap alternative.
- [`../03-audio-sound/inmp441-i2s-mic.md`](../03-audio-sound/inmp441-i2s-mic.md) for a higher-quality external mic.
- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md) for add-on storage.

## Where to buy

- Adafruit / DigiKey / Mouser / Seeed — official distributor chain.

## Software / libraries

- `espressif/esp-who` — face detection/recognition framework.
- `espressif/esp32-camera`.

## Notes for WeftOS

Speculative: ESP-EYE is the right board to certify "WeftOS AI camera surface" against first —
vendor-supported, stable, and the same hardware the Espressif DL docs assume.
