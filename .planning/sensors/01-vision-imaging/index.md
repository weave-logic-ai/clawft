---
title: "Vision & Imaging Modules"
slug: "01-vision-imaging"
category: "01-vision-imaging"
tags: [vision, camera, ir, thermal, uv, imaging, hyperspectral]
source: "../../weftos_sensors.md"
status: "draft"
---

# Category 01 — Vision & Imaging

ESP32 (and especially ESP32-S3 with its PSRAM and AI instructions) is a surprisingly capable
vision platform. This category covers visible-light cameras, modified NoIR night-vision builds,
thermal arrays, UV/hyperspectral experiments, and the paired active lighting needed to make each
of them useful.

## Modules

| Module | One-liner |
|---|---|
| [ESP32-CAM (OV2640)](esp32-cam-ov2640.md) | The ubiquitous AI-Thinker board — 2 MP OV2640 + SD slot + Wi-Fi for ~$8. |
| [OV3660](ov3660-cam.md) | Drop-in 3 MP upgrade for ESP32-CAM-compatible sockets. |
| [M5Stack Camera](m5stack-camera.md) | Enclosed M5 ecosystem camera (Unit-A I2C, Unit-B USB) with Grove connector. |
| [TTGO T-Camera](ttgo-t-camera.md) | LilyGo board bundling OV2640 + OLED + optional PIR in one PCB. |
| [Freenove WROVER CAM](freenove-wrover-cam.md) | PSRAM-heavy kit board aimed at hobby AI / image classification. |
| [ESP-EYE](esp-eye.md) | Espressif's official reference board for face / voice AI. |
| [Arducam NoIR](arducam-noir.md) | IR-cut filter removed for night vision — pair with 850/940 nm illuminators. |
| [MLX90640 thermal](mlx90640-thermal.md) | 32×24 IR thermal array, 55° or 110° FOV. |
| [AMG8833 Grid-EYE](amg8833-thermal.md) | Panasonic 8×8 thermal, low-res but very cheap. |
| [ESP32-S3 IR Thermal](esp32-s3-ir-thermal.md) | 80×62 thermal module with integrated S3 + USB-C. |
| [GUVA-S12SD UV](guva-s12sd-uv.md) | Analog UV-index photodiode, ~240–370 nm. |
| [UV / hyperspectral cameras](uv-cameras.md) | Niche — modified CMOS and emerging hyperspectral options. |
| [IR illuminators](ir-illuminators.md) | 850 nm / 940 nm LED arrays for NoIR night vision. |
| [UV illuminators](uv-illuminators.md) | 365/395 nm for fluorescence, forensics, leak-detection. |
| [Structured light sources](structured-light-sources.md) | WS2812B patterns + laser diodes for depth experiments. |

## Creative "Lego" prompts

- **Wireless night-vision cam** — ESP32-CAM + 850 nm IR illuminator + microSD recording.
- **Heat + air combo** — ESP32-S3 IR thermal + BME680 for heat-map overlaid with VOC / humidity.
- **Fluorescence macro rig** — OV3660 + 365 nm UV illuminator + MAX9814 clap trigger for timed shots.
- **Structured-light depth toy** — OV2640 + laser-line diode scanner on a pan-tilt servo.

## Cross-category pairings

- Active lighting tightly couples this category to [`../04-light-display/`](../04-light-display/index.md)
  (WS2812B, MOSFET-driven high-current LEDs).
- Continuous video or frame burst storage → `../10-storage-timing-power/microsd.md`.
- Pan/tilt rigs and focus actuators → `../07-control-actuators/` (servos, steppers, MOSFETs).
- Bus contention when running camera + I²C sensors → `../08-communication/tca9548a-i2c-mux.md`.
