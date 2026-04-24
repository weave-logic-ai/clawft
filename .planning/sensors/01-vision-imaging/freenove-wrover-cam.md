---
title: "Freenove ESP32-WROVER CAM"
slug: "freenove-wrover-cam"
category: "01-vision-imaging"
part_numbers: ["Freenove ESP32-WROVER CAM Board", "FNK0060"]
manufacturer: "Freenove"
interface: ["SPI", "I2C", "UART", "USB"]
voltage: "5V"
logic_level: "3.3V"
current_ma: "~130–260 mA streaming"
price_usd_approx: "$18 – $30"
esp32_compat: ["ESP32"]
tags: [camera, freenove, wrover, psram, learning-kit]
pairs_with:
  - "./esp32-cam-ov2640.md"
  - "./esp-eye.md"
  - "../10-storage-timing-power/microsd.md"
buy:
  - { vendor: Amazon,   url: "https://www.amazon.com/s?k=Freenove+ESP32-WROVER+CAM" }
  - { vendor: Freenove, url: "https://freenove.com/search?q=wrover+cam" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-freenove-wrover-cam.html" }
libraries:
  - { name: "espressif/esp32-camera", url: "https://github.com/espressif/esp32-camera" }
  - { name: "Freenove/Freenove_ESP32_WROVER_Board", url: "https://github.com/Freenove" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

Freenove's ESP32-WROVER CAM board is a learning-kit-friendly ESP32 camera: WROVER module with
PSRAM, onboard USB-UART, reset/boot buttons, microSD, and an OV2640 (or OV3660 depending on
batch). It's the board most commonly shipped in "robot car with camera" kits.

## Key specs

| Spec | Value |
|---|---|
| Module | ESP32-WROVER (with PSRAM) |
| Sensor | OV2640 standard (OV3660 on some SKUs) |
| USB | Onboard USB-UART, auto-reset |
| Storage | microSD |
| Extras | Buttons, LEDs pre-wired for kit examples |

## Interface & wiring

Biggest quality-of-life difference vs bare ESP32-CAM: onboard USB bridge and auto-reset. Otherwise
the camera pinout matches the typical AI-Thinker layout, and the PSRAM is addressable for full
UXGA JPEG.

## Benefits

- Flashable over plain USB — no IO0 jumper dance.
- PSRAM + SD make it a complete video-capture node.
- Well-documented kit tutorials (Freenove's own repos are thorough).

## Limitations / gotchas

- Slightly bigger footprint than bare ESP32-CAM.
- SKU variance — check sensor, presence of SD, and USB chip before flashing.
- Example code is sometimes kit-centric (robot car pins), not generic.

## Typical use cases

- Teaching / classroom camera projects.
- Rover / robot-car FPV streaming.
- Quick prototypes where soldering a USB-UART is annoying.

## Pairs well with

- [`./esp32-cam-ov2640.md`](./esp32-cam-ov2640.md) for the bare-board comparison.
- [`./esp-eye.md`](./esp-eye.md) as a step up for AI workloads.
- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md).

## Where to buy

- Amazon (Freenove storefront).
- Freenove site directly.
- AliExpress for the same hardware cheaper, slower shipping.

## Software / libraries

- `espressif/esp32-camera`.
- Freenove's GitHub organization — `Freenove/Freenove_ESP32_WROVER_Board`.

## Notes for WeftOS

Speculative: because USB + auto-reset "just works", this is a good candidate for a WeftOS
reference dev node in documentation — the same canonical camera surface as bare ESP32-CAM but
with much lower first-run friction.
