---
title: "M5Stack Camera (Unit A / Unit B)"
slug: "m5stack-camera"
category: "01-vision-imaging"
part_numbers: ["M5Camera-A", "M5Camera-B", "Timer Camera", "ESP32-CAM-F"]
manufacturer: "M5Stack"
interface: ["Grove (I2C)", "UART", "USB"]
voltage: "5V"
logic_level: "3.3V"
current_ma: "~150–250 mA streaming"
price_usd_approx: "$16 – $35"
esp32_compat: ["ESP32"]
tags: [camera, m5, ecosystem, enclosed, grove]
pairs_with:
  - "./esp32-cam-ov2640.md"
  - "../09-input-hmi/index.md"
  - "../10-storage-timing-power/microsd.md"
buy:
  - { vendor: M5Stack,  url: "https://shop.m5stack.com/search?q=camera" }
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=M5Stack+camera" }
  - { vendor: DigiKey,  url: "https://www.digikey.com/en/products/search?keywords=M5Stack%20camera" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-m5stack-camera.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

M5Stack's camera line takes the OV2640 + ESP32 combo and wraps it in a Lego-style enclosure with
a Grove connector, sensible USB programming, and predictable power. Unit A uses a standard lens,
Unit B swaps in a wider lens, and the Timer Camera variant adds a battery + RTC for intermittent
wake-and-shoot builds.

## Key specs

| Spec | Value |
|---|---|
| Sensor | OV2640 (2 MP) |
| Lens | Unit-A standard; Unit-B ~120° wide |
| Programming | USB-C (CP2104/CH9102) |
| Connector | Grove (I²C / UART) |
| Extras (Timer Camera) | BM8563 RTC, 140 mAh LiPo, deep-sleep wake |

## Interface & wiring

Programming is plug-and-play over USB-C — a huge ergonomic win over the bare AI-Thinker board.
The Grove port exposes I²C for peripherals (sensors, displays) while the camera bus stays
internal. On the Timer Camera, the RTC wakes the ESP32 on schedule and the onboard LDO handles
the battery.

## Benefits

- No flashing dance — USB-C just works.
- Clean mechanical mount (Lego-friendly case).
- Timer Camera nails the "wake, shoot, upload, sleep" pattern out of the box.

## Limitations / gotchas

- More expensive than bare ESP32-CAM.
- Closed enclosure means no trivial sensor swap.
- On some units Grove + camera can contend for CPU time during streaming.

## Typical use cases

- Wildlife / garden timelapse with battery.
- Classroom kits where flashing bare ESP32-CAMs is a time sink.
- Integrating a camera into a larger M5 stack of HAT modules.

## Pairs well with

- [`./esp32-cam-ov2640.md`](./esp32-cam-ov2640.md) as the bare-board alternative.
- [`../09-input-hmi/index.md`](../09-input-hmi/index.md) for M5 display units showing the stream.
- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md) for buffering.

## Where to buy

- M5Stack official store (best stock).
- Adafruit / DigiKey for US-domestic shipping.
- AliExpress for clones (caveat emptor).

## Software / libraries

- `m5stack/M5Unified` / `M5Stack_Camera` examples.
- `espressif/esp32-camera` under the hood.

## Notes for WeftOS

Speculative: Timer Camera's "wake → frame → sleep" pattern is a useful WeftOS scheduling primitive
— a cyclic surface-producer with explicit energy budget rather than a continuous stream.
