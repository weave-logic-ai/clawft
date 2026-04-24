---
title: "ESP32-S3 IR Thermal Module (80×62)"
slug: "esp32-s3-ir-thermal"
category: "01-vision-imaging"
part_numbers: ["ESP32-S3 IR Thermal 80x62"]
manufacturer: "various (clones of Seeed / LilyGo / generic)"
interface: ["USB-C", "SPI (internal)"]
voltage: "5V (USB)"
logic_level: "3.3V"
current_ma: "~180 mA"
price_usd_approx: "$55 – $140"
esp32_compat: ["ESP32-S3"]
tags: [thermal, ir, esp32-s3, 80x62, usb-c]
pairs_with:
  - "./mlx90640-thermal.md"
  - "../05-environmental/index.md"
  - "../10-storage-timing-power/microsd.md"
buy:
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=esp32+thermal" }
  - { vendor: LilyGo, url: "https://www.lilygo.cc/search?type=product&q=thermal" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-esp32-s3-thermal-camera.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A newer generation of hobby thermal cameras pairs an ESP32-S3 with a 80×62 (4960 pixel) IR
sensor on a USB-C board. Resolution is ~6× the MLX90640, and the S3's PSRAM + vector
instructions make on-device bilinear and false-color rendering smooth.

## Key specs

| Spec | Value |
|---|---|
| Resolution | 80 × 62 pixels (~4960) |
| FOV variants | 45° or 90° (wide) |
| Temp range | vendor-dependent, commonly -20 °C to +400 °C |
| Host MCU | ESP32-S3 with PSRAM |
| Connectivity | USB-C, Wi-Fi, BT |
| Display | some variants include a 1.54"–2.0" LCD |

## Interface & wiring

The thermal sensor is internally wired to the ESP32-S3 over SPI / parallel (board-specific). From
the user's side it's a USB-C plug and Wi-Fi: flash custom firmware, or use the vendor app.

## Benefits

- Meaningful thermal resolution for the price (still below FLIR Lepton).
- ESP32-S3 host can run on-device colormap / person-blob detection.
- USB-C PD means you can power and stream over one cable.

## Limitations / gotchas

- Sensor vendor varies by batch — driver support is uneven.
- Radiometric accuracy isn't FLIR-grade; treat absolute temps with skepticism.
- Many boards ship with locked firmware; flipping to open firmware may be non-trivial.

## Typical use cases

- Portable thermal inspection (electronics, HVAC, leaks).
- Upgrade path from MLX90640 when 32×24 isn't enough.
- Battery-backed thermography handheld.

## Pairs well with

- [`./mlx90640-thermal.md`](./mlx90640-thermal.md) as the lower-res comparison point.
- [`../05-environmental/index.md`](../05-environmental/index.md) for BME680/BMP388 context.
- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md) for radiometric clip storage.

## Where to buy

- Seeed / LilyGo listings (official-ish).
- AliExpress for the bulk of the market.

## Software / libraries

- Vendor SDKs — check GitHub for the specific SKU; open-source support is still emerging.

## Notes for WeftOS

Speculative: this is a nice reference for a "high-res thermal surface" in WeftOS — an ESP32-S3
that runs effects locally (colormap, blob, track) and only publishes derived overlays upstream.
