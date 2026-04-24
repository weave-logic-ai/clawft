---
title: "HMC5883L / QMC5883L Magnetometer (3-axis compass)"
slug: "hmc5883l-qmc5883l-compass"
category: "02-positioning-navigation"
part_numbers: ["HMC5883L (EOL)", "QMC5883L", "GY-271"]
manufacturer: "Honeywell (HMC, EOL) / QST (QMC clone)"
interface: ["I2C"]
voltage: "3.3V (module often 3–5V)"
logic_level: "3.3V"
current_ma: "~0.1 mA"
price_usd_approx: "$2 – $6"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [magnetometer, compass, hmc5883l, qmc5883l, gy-271]
pairs_with:
  - "./mpu6050-imu.md"
  - "./bno055-orientation.md"
  - "./neo-m8n-gnss.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=magnetometer" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=magnetometer" }
  - { vendor: DFRobot, url: "https://www.dfrobot.com/search-HMC5883L.html" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=QMC5883L" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-GY-271.html" }
libraries:
  - { name: "mprograms/QMC5883LCompass", url: "https://github.com/mprograms/QMC5883LCompass" }
  - { name: "adafruit/Adafruit_HMC5883_Unified", url: "https://github.com/adafruit/Adafruit_HMC5883_Unified" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The GY-271 "HMC5883L" compass breakout is an ESP32 classic — except ~95% of modules sold since
~2018 are actually QMC5883L clones with a different register map and I²C address. Both give you
a 3-axis digital magnetometer usable as a tilt-compensated compass when combined with an accel.

## Key specs

| Spec | HMC5883L | QMC5883L |
|---|---|---|
| Range | ±1–8 gauss | ±2 / ±8 gauss |
| Output | 12-bit | 16-bit |
| Update rate | up to 160 Hz | up to 200 Hz |
| I²C address | 0x1E | 0x0D |
| Availability | EOL | In production |

## Interface & wiring

Four-wire I²C. The catch: check which chip is actually on the board before picking a library —
if `Wire.requestFrom(0x1E,…)` returns zero but `0x0D` works, you have a QMC. `mprograms/QMC5883LCompass`
handles the common case.

## Benefits

- Cheap, small, well-understood physics.
- Adds a yaw reference that gyro-only IMUs lack.
- Straightforward I²C interface.

## Limitations / gotchas

- Chip identity confusion (HMC vs QMC) has bitten every Arduino tutorial.
- Hard-iron (offset) and soft-iron (matrix) calibration is mandatory per installation.
- Nearby motors, speakers, and steel bolts will wreck readings.
- Earth's field is tiny — any DC magnetic field from the same PCB dominates.

## Typical use cases

- Compass heading for ground vehicles.
- Yaw correction on 6-axis IMUs.
- Door-open / open-close sensing with a strong magnet.

## Pairs well with

- [`./mpu6050-imu.md`](./mpu6050-imu.md) to become a DIY 9-axis.
- [`./bno055-orientation.md`](./bno055-orientation.md) as an integrated alternative (skip the compass).
- [`./neo-m8n-gnss.md`](./neo-m8n-gnss.md) to seed heading when stopped.

## Where to buy

- Adafruit / SparkFun / DFRobot / Seeed for authentic breakouts.
- AliExpress / GY-271 for the QMC-as-HMC situation — be aware.

## Software / libraries

- `mprograms/QMC5883LCompass` — handles calibration.
- `adafruit/Adafruit_HMC5883_Unified` — genuine HMC only.

## Notes for WeftOS

Speculative: magnetometer data in WeftOS should expose a calibration-status signal on the side
channel — lots of bugs are "we never finished hard-iron calibration but used the data anyway."
