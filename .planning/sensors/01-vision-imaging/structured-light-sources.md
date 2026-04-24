---
title: "Structured Light Sources (WS2812B patterns + laser diodes)"
slug: "structured-light-sources"
category: "01-vision-imaging"
part_numbers: ["650nm laser diode module", "532nm laser", "WS2812B strip", "line laser module"]
manufacturer: "various"
interface: ["PWM", "GPIO", "WS2812 (1-wire)"]
voltage: "3.3V – 5V (WS2812), 3V–5V (laser modules)"
logic_level: "3.3V with level shifter for 5V WS2812"
current_ma: "30–80 mA per laser, 60 mA per WS2812 at full white"
price_usd_approx: "$2 – $15"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [structured-light, laser, ws2812, depth, 3d]
pairs_with:
  - "../04-light-display/ws2812b-neopixel.md"
  - "./esp32-cam-ov2640.md"
  - "./arducam-noir.md"
  - "../07-control-actuators/mosfet-drivers.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=line+laser" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=laser" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=laser+module" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-line-laser-module.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

Structured light = projecting a known pattern (line, grid, dots, stripes) onto a scene so a
camera can triangulate depth from how the pattern deforms. The hobbyist combo is cheap: a
line-laser module swept by a servo, or a WS2812B strip driving a moving stripe pattern. This page
is a pattern guide, not a single-module spec sheet.

## Key specs (by approach)

| Pattern source | Resolution of depth | Difficulty |
|---|---|---|
| Single line laser + servo sweep | Depends on sweep steps (~1 mm @ 300 mm) | Easy |
| Cross-hair / grid laser module | Medium (static) | Easy |
| DMD / mini-projector | High (many patterns) | Hard — not ESP32 friendly |
| WS2812 fringe on a diffuser | Low, but visible-safe | Medium |
| IR laser + NoIR camera | Covert depth (like Kinect v1) | Medium |

## Interface & wiring

- **Laser diode modules (5 mW typical):** 3.3–5 V, a current-limit resistor or onboard regulator;
  switch via MOSFET or PWM for intensity. Don't PWM below ~200 Hz if the camera is rolling
  shutter — you'll get stripes.
- **WS2812B patterns:** one-wire RMT/SPI drive. See [`../04-light-display/ws2812b-neopixel.md`](../04-light-display/ws2812b-neopixel.md).
- **Servo sweep for line laser:** see [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) and a standard servo PWM channel.

## Benefits

- Real depth data from a $5 laser and an ESP32-CAM.
- Can be done in IR (NoIR camera + 780 nm line laser) for invisible depth.
- Educational — great way to understand epipolar geometry.

## Limitations / gotchas

- **Laser safety:** class 3R lasers can damage eyes instantly. Even 5 mW needs care. Don't aim at
  people or pets, and build in an e-stop that defaults to OFF.
- Calibration (baseline, angle, lens intrinsics) is fiddly.
- Ambient sunlight drowns visible lasers; IR variants are more robust.
- ESP32 is slow for full stereo matching — do the depth math on a host, or pre-bin on ESP32-S3.

## Typical use cases

- Hobby 3D scanner (turntable + line laser + camera).
- Kinect-ish depth experiments (IR laser + NoIR camera).
- Robot obstacle-avoidance sweeps.

## Pairs well with

- [`../04-light-display/ws2812b-neopixel.md`](../04-light-display/ws2812b-neopixel.md) for visible pattern sources.
- [`./esp32-cam-ov2640.md`](./esp32-cam-ov2640.md) / [`./arducam-noir.md`](./arducam-noir.md) for the imaging half.
- [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) for safe laser enable.

## Where to buy

- Adafruit / SparkFun / Seeed for named line-laser modules.
- AliExpress for cheap 650 nm and 780 nm laser diodes.

## Software / libraries

- Host-side: OpenCV, `structuredlight` Python packages, Open3D.
- ESP32 side: `FastLED` / `NeoPixelBus` for patterns; your own laser-enable task.

## Notes for WeftOS

Speculative: structured light is where WeftOS's "co-scheduled actuator + camera" story gets
interesting — the emitted pattern is an effect output *and* a precondition for a depth surface.
A clean policy model for "this surface is valid only while that actuator is on" would be useful.
