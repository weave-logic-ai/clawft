---
title: "ST VL53L0X / VL53L1X ToF Laser Rangefinder"
slug: "vl53l0x-vl53l1x-tof"
category: "02-positioning-navigation"
part_numbers: ["VL53L0X", "VL53L1X", "GY-530"]
manufacturer: "STMicroelectronics"
interface: ["I2C"]
voltage: "2.6–3.5V (module boards often 2.8–5V with LDO)"
logic_level: "3.3V (use level shifter from 5V hosts)"
current_ma: "~19 mA (L0X), ~18 mA (L1X) active"
price_usd_approx: "$4 – $12"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [tof, laser, vl53l0x, vl53l1x, ranging]
pairs_with:
  - "./tof050c-tof10120.md"
  - "./tfmini-tfluna-lidar.md"
  - "../08-communication/tca9548a-i2c-mux.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=VL53L1X" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=VL53L1X" }
  - { vendor: Pololu, url: "https://www.pololu.com/search/compare?query=VL53L" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=VL53L0X" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-GY-530.html" }
datasheet: "https://www.st.com/en/imaging-and-photonics-solutions/proximity-sensors.html"
libraries:
  - { name: "pololu/vl53l0x-arduino", url: "https://github.com/pololu/vl53l0x-arduino" }
  - { name: "pololu/vl53l1x-arduino", url: "https://github.com/pololu/vl53l1x-arduino" }
  - { name: "stm32duino/VL53L1X", url: "https://github.com/stm32duino/VL53L1X" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The VL53L0X (30 mm–1.2 m) and VL53L1X (30 mm–4 m) are STMicro's laser time-of-flight
rangefinders. A single IR laser + SPAD array gives mm-precision distance that doesn't care about
object color the way ultrasonic does. They're small, I²C-only, and arguably the best cheap
rangefinder type per dollar.

## Key specs

| Spec | VL53L0X | VL53L1X |
|---|---|---|
| Range | 30 mm – 1200 mm | 40 mm – 4000 mm |
| Accuracy | ±3% | ±1% (short mode) |
| Rate | up to 50 Hz | up to 50 Hz |
| Field of view | ~25° | 15°–27° configurable |
| Bus | I²C 400 kHz | I²C 400 kHz |

## Interface & wiring

I²C at default 0x29. All VL53 chips power up with the same address, so for multi-sensor builds
either (a) use the XSHUT pin to reset and re-address them one at a time, or (b) put them behind a
TCA9548A I²C mux (see `../08-communication/tca9548a-i2c-mux.md`).

## Benefits

- Mm-class resolution in a tiny package.
- Doesn't care about surface color / soft material (unlike ultrasonic).
- L1X's configurable ROI lets you do crude multi-zone ranging from one sensor.

## Limitations / gotchas

- Direct sunlight can saturate the SPAD detector — outdoor daylight range drops sharply.
- Black matte surfaces still reflect enough IR to read, but range is reduced.
- Default I²C address collisions — plan XSHUT or mux from the start.
- VL53L0X is going EOL; design new work around VL53L1X / VL53L4CD.

## Typical use cases

- Robot obstacle sensor.
- Gesture / proximity triggers.
- Level sensing of non-reflective liquids via a target float.
- Multi-sensor "virtual bumper" rings.

## Pairs well with

- [`./tof050c-tof10120.md`](./tof050c-tof10120.md) as cheaper short-range siblings.
- [`./tfmini-tfluna-lidar.md`](./tfmini-tfluna-lidar.md) for longer-range outdoor LiDAR.
- [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md) for multi-sensor arrays.

## Where to buy

- Adafruit / SparkFun / Pololu — best-quality breakouts with XSHUT broken out.
- Seeed / AliExpress for bare GY-530 modules.

## Software / libraries

- `pololu/vl53l0x-arduino`, `pololu/vl53l1x-arduino` — clean, small.
- `stm32duino/VL53L1X` — uses ST's official API.

## Notes for WeftOS

Speculative: ToF sensors are a natural candidate for "virtual array surface" synthesis — WeftOS
could hide 8 VL53L1X behind a single "proximity ring" surface that exposes one logical
multi-direction distance field.
