---
title: "OV3660 Camera (3 MP swap)"
slug: "ov3660-cam"
category: "01-vision-imaging"
part_numbers: ["OV3660"]
manufacturer: "OmniVision"
interface: ["SPI", "I2C"]
voltage: "3.3V"
logic_level: "3.3V"
current_ma: "~140 mA streaming"
price_usd_approx: "$8 – $18"
esp32_compat: ["ESP32", "ESP32-S3"]
tags: [camera, rgb, ov3660, 3mp]
pairs_with:
  - "./esp32-cam-ov2640.md"
  - "./ir-illuminators.md"
  - "../10-storage-timing-power/microsd.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=OV3660" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=OV3660" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=OV3660" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-OV3660.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-OV3660.html" }
libraries:
  - { name: "espressif/esp32-camera", url: "https://github.com/espressif/esp32-camera" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The OV3660 is a 3 MP OmniVision sensor that drops into most ESP32-CAM sockets in place of an
OV2640 for better dynamic range and a bit more detail. It's the cheapest meaningful resolution
bump without switching boards.

## Key specs

| Spec | Value |
|---|---|
| Max resolution | 2048 × 1536 (QXGA) |
| Output | JPEG / YUV / RGB565 |
| Lens | M12 (same as OV2640 modules) |
| Interface | DVP 8-bit + SCCB (I²C) |
| Best frame rate | ~15 fps @ XGA, ~3–5 fps @ QXGA |

## Interface & wiring

Same DVP camera header as OV2640 — on ESP32-CAM boards it's a straight swap; on custom boards
verify XCLK, VSYNC, HREF, PCLK and D0–D7 routing. Power 3.3 V analog + digital; put a 10 µF +
100 nF local decoupling pair on AVDD.

## Benefits

- Higher resolution than OV2640 without changing boards.
- Better low-light and color response than OV2640.
- Same `esp32-camera` driver handles it.

## Limitations / gotchas

- Bandwidth goes up — JPEG is fine, RGB888 will starve the bus.
- ESP32 (original) struggles to keep frame rate up at full 3 MP; ESP32-S3 does better.
- Module quality varies wildly on AliExpress; lens tuning is hit-or-miss.

## Typical use cases

- QR / document capture where OV2640 resolution is marginal.
- Still-image security snapshot uploads.
- Nature / trail cam with SD store.

## Pairs well with

- [`./esp32-cam-ov2640.md`](./esp32-cam-ov2640.md) for the host board it drops into.
- [`./ir-illuminators.md`](./ir-illuminators.md) + NoIR mod for better night detail.
- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md) for QXGA stills.

## Where to buy

- Adafruit / SparkFun / Seeed / DFRobot search links (stock intermittent).
- AliExpress is the main volume supplier for M12-lens OV3660 modules.

## Software / libraries

- `espressif/esp32-camera` autodetects OV3660 over SCCB.

## Notes for WeftOS

Speculative: treat the OV3660 as a "higher-detail variant" of the same canonical camera stream as
OV2640 — same surface type, different resolution/FPS profile descriptor, so effects can negotiate.
