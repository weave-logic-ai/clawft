---
title: "Benewake TFmini-S / TF-Luna Single-Beam LiDAR"
slug: "tfmini-tfluna-lidar"
category: "02-positioning-navigation"
part_numbers: ["TFmini-S", "TF-Luna", "TFmini Plus"]
manufacturer: "Benewake"
interface: ["UART", "I2C (TF-Luna, TFmini Plus)"]
voltage: "5V (TFmini-S), 3.7–5.2V (TF-Luna)"
logic_level: "3.3V compatible"
current_ma: "~140 mA (TFmini-S), ~70 mA (TF-Luna)"
price_usd_approx: "$25 – $50"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [lidar, single-beam, tfmini, tf-luna, benewake]
pairs_with:
  - "./ld06-ld19-lidar.md"
  - "./vl53l0x-vl53l1x-tof.md"
  - "./hc-sr04-ultrasonic.md"
buy:
  - { vendor: DFRobot, url: "https://www.dfrobot.com/search-TFmini.html" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=tfmini" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=TF-Luna" }
  - { vendor: Mouser, url: "https://www.mouser.com/c/?q=TF-Luna" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-TF-Luna.html" }
datasheet: "https://en.benewake.com/"
libraries:
  - { name: "budryerson/TFMini_Plus", url: "https://github.com/budryerson/TFMini-Plus" }
  - { name: "budryerson/TFLuna-I2C", url: "https://github.com/budryerson/TFLuna-I2C" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The Benewake TFmini-S (12 m) and TF-Luna (8 m) are single-beam time-of-flight LiDAR modules with
integer-centimeter output at up to 250 Hz. They split the difference between VL53L1X (short
range, cheap) and spinning LiDAR (2D maps, expensive). Great for drone altimeters, medium-range
obstacle sensors, or sweep-mount DIY LiDARs.

## Key specs

| Spec | TFmini-S | TF-Luna |
|---|---|---|
| Range | 0.1 – 12 m | 0.2 – 8 m |
| Accuracy | ±6 cm @ ≤6 m, ±1% @ >6 m | ±6 cm @ ≤3 m, ±2% @ >3 m |
| Rate | 1–1000 Hz (default 100) | 1–250 Hz |
| Beam divergence | ~2° | ~2° |
| Interface | UART (CAN/I²C via firmware swap) | UART + I²C |

## Interface & wiring

UART at 115200 baud is the default; frames are 9 bytes (header 0x59 0x59, distance low/high,
signal strength, temp, checksum). TF-Luna additionally supports I²C at address 0x10. Power wants
a solid 5 V; the TFmini-S briefly spikes to ~800 mA at boot — don't share a weak 3.3 V rail.

## Benefits

- Real LiDAR-grade range at a mid-tier price.
- High sample rate (100–250 Hz) suitable for flight-control altitude hold.
- Compact and rigid housing, easier to mount than DIY IR arrays.

## Limitations / gotchas

- Current spikes on boot can brown out marginal regulators.
- Direct sunlight reduces range; dust on window degrades performance.
- Different firmware revisions change the command set — always check the version byte.
- Not eye-safe IEC-60825 Class 1 on *all* SKUs; check the label before pointing it at faces.

## Typical use cases

- Drone altitude hold.
- Large-room occupancy / people counting.
- Rotating-mount DIY 2-D scanner.
- Outdoor presence detection (car in driveway, etc.).

## Pairs well with

- [`./ld06-ld19-lidar.md`](./ld06-ld19-lidar.md) for full 360° alternative.
- [`./vl53l0x-vl53l1x-tof.md`](./vl53l0x-vl53l1x-tof.md) for close-range complement.
- [`./hc-sr04-ultrasonic.md`](./hc-sr04-ultrasonic.md) as redundant short-range backup.

## Where to buy

- DFRobot / Seeed / SparkFun — most convenient in US/EU.
- Mouser for TF-Luna in volume.
- AliExpress for TFmini-S at the lowest price.

## Software / libraries

- `budryerson/TFMini-Plus` (UART).
- `budryerson/TFLuna-I2C` (I²C).

## Notes for WeftOS

Speculative: TFmini/TF-Luna are a good fit for a "1-D ranging surface with signal quality" type
in WeftOS — the signal-strength byte is crucial for downstream filtering and should not be
collapsed away.
