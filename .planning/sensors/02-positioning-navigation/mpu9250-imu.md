---
title: "Invensense MPU-9250 (9-axis IMU)"
slug: "mpu9250-imu"
category: "02-positioning-navigation"
part_numbers: ["MPU-9250", "GY-9250"]
manufacturer: "InvenSense (TDK) — EOL"
interface: ["I2C", "SPI"]
voltage: "3.3V"
logic_level: "3.3V"
current_ma: "~3.7 mA"
price_usd_approx: "$6 – $15"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [imu, 9-axis, mpu9250, magnetometer, ak8963]
pairs_with:
  - "./mpu6050-imu.md"
  - "./bno055-orientation.md"
  - "./lsm6dsx-imu.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=MPU-9250" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=MPU-9250" }
  - { vendor: DFRobot, url: "https://www.dfrobot.com/search-MPU9250.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-MPU-9250.html" }
datasheet: "https://invensense.tdk.com/products/motion-tracking/9-axis/mpu-9250/"
libraries:
  - { name: "bolderflight/invensense-imu", url: "https://github.com/bolderflight/invensense-imu" }
  - { name: "hideakitai/MPU9250", url: "https://github.com/hideakitai/MPU9250" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The MPU-9250 stacks an AK8963 magnetometer onto the MPU-6500 accel+gyro die, giving a 9-axis IMU
in a single chip. Officially end-of-life from TDK/InvenSense, but still widely stocked from
old-stock and clones. It's the go-to when you need an affordable 9-axis sensor without the
premium price of a BNO055.

## Key specs

| Spec | Value |
|---|---|
| Accel | ±2 / 4 / 8 / 16 g |
| Gyro | ±250 / 500 / 1000 / 2000 °/s |
| Magnetometer | AK8963, ±4912 µT |
| Sample rate | up to 8 kHz gyro, 1 kHz accel, 100 Hz mag |
| Interface | I²C 400 kHz or SPI up to 1 MHz |

## Interface & wiring

Two I²C devices internally — MPU at 0x68/0x69 and AK8963 at 0x0C (addressed via internal
pass-through). Some libraries handle both transparently. SPI mode is preferred if you already
run SPI and want higher sample rates.

## Benefits

- 9-axis in one low-cost part.
- High gyro sample rate (useful for vibration analysis, drones).
- SPI option gives headroom over I²C.

## Limitations / gotchas

- **EOL** — supply risk. Don't design for volume.
- AK8963 magnetometer needs calibration (hard/soft iron) per installation.
- Many "MPU-9250" AliExpress modules are actually ICM-20948 or MPU-9255; driver may need patching.
- Documentation is fragmented.

## Typical use cases

- Drone/quad stabilization with heading.
- Wearables / head trackers.
- AR headset prototypes.

## Pairs well with

- [`./mpu6050-imu.md`](./mpu6050-imu.md) — cheaper 6-axis alternative.
- [`./bno055-orientation.md`](./bno055-orientation.md) if you want fusion done on-chip.
- [`./lsm6dsx-imu.md`](./lsm6dsx-imu.md) as a modern STMicro replacement for new designs.

## Where to buy

- Adafruit / SparkFun — while stock lasts.
- AliExpress — cheap, but verify part ID after powering up.

## Software / libraries

- `bolderflight/invensense-imu`.
- `hideakitai/MPU9250`.

## Notes for WeftOS

Speculative: until a genuine modern 9-axis (e.g., ICM-20948 / LSM9DS1) baseline is picked, treat
MPU-9250 as the bridge — same 9-axis surface contract, different provider. The surface shouldn't
care which chip is under the hood.
