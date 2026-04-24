---
title: "Arducam NoIR (OV2640 no IR-cut)"
slug: "arducam-noir"
category: "01-vision-imaging"
part_numbers: ["Arducam NoIR OV2640", "B0041"]
manufacturer: "Arducam / UCTRONICS"
interface: ["DVP (camera parallel) + SCCB I²C"]
voltage: "3.3V"
logic_level: "3.3V"
current_ma: "~140 mA streaming"
price_usd_approx: "$8 – $18"
esp32_compat: ["ESP32", "ESP32-S3"]
tags: [camera, noir, night-vision, ir-sensitive, ov2640]
pairs_with:
  - "./esp32-cam-ov2640.md"
  - "./ir-illuminators.md"
  - "./structured-light-sources.md"
buy:
  - { vendor: Arducam, url: "https://www.arducam.com/?s=noir" }
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=NoIR+camera" }
  - { vendor: Amazon, url: "https://www.amazon.com/s?k=Arducam+NoIR+OV2640" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-noir-ov2640.html" }
libraries:
  - { name: "espressif/esp32-camera", url: "https://github.com/espressif/esp32-camera" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A NoIR ("No Infrared cut-filter") OV2640 is a stock 2 MP OmniVision sensor with the tiny IR-cut
filter removed so it stays sensitive to 700–950 nm IR. Paired with an 850 nm illuminator it gives
you real night vision on an ESP32-CAM socket. Arducam sells it pre-modified; you can also do the
mod yourself with a hobby knife and patience.

## Key specs

| Spec | Value |
|---|---|
| Sensor | OV2640 2 MP, IR-cut filter removed |
| Spectral response | Visible + ~700–1000 nm IR |
| Daylight color | Tinted pink/red unless white-balance is corrected |
| Pinout | Identical to stock OV2640 module |

## Interface & wiring

Drop-in replacement for OV2640 on ESP32-CAM-compatible sockets. Wiring, SCCB, and DVP bus are
unchanged — the only difference is the glass on the sensor die.

## Benefits

- Real night vision when paired with an IR illuminator.
- No software changes required.
- Cheap compared to a dedicated IR sensor or thermal camera.

## Limitations / gotchas

- Daylight images look pink / washed-out without custom white balance.
- DIY filter removal can scratch the die if you're careless.
- 940 nm illuminators work but deliver roughly half the effective range because OV2640 quantum
  efficiency drops sharply past ~900 nm; 850 nm is the sweet spot.

## Typical use cases

- Night security / wildlife cam.
- Covert indoor monitoring (940 nm LEDs invisible to human eye).
- Vein / surface-feature imaging experiments.

## Pairs well with

- [`./esp32-cam-ov2640.md`](./esp32-cam-ov2640.md) as the host board.
- [`./ir-illuminators.md`](./ir-illuminators.md) — you almost always need one.
- [`./structured-light-sources.md`](./structured-light-sources.md) for IR-laser depth experiments.

## Where to buy

- Arducam direct (pre-modified).
- Adafruit / Amazon — occasional stock.
- AliExpress for cheap bare NoIR modules.

## Software / libraries

- `espressif/esp32-camera` — set AWB/AWB-gain manually for best daylight color.

## Notes for WeftOS

Speculative: a NoIR node could expose two logical surfaces — a visible-ish stream and an
IR-intensity stream — by toggling between daylight and IR-illuminator-on frames, so effects can
subscribe to whichever spectrum they need.
