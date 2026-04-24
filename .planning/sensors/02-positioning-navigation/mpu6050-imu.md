---
title: "Invensense MPU-6050 (6-axis IMU)"
slug: "mpu6050-imu"
category: "02-positioning-navigation"
part_numbers: ["MPU-6050", "GY-521"]
manufacturer: "InvenSense (TDK)"
interface: ["I2C"]
voltage: "3.3V (module usually tolerates 5V VCC via LDO)"
logic_level: "3.3V"
current_ma: "~3.9 mA"
price_usd_approx: "$2 – $6"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [imu, accel, gyro, mpu6050, 6-axis, gy-521]
pairs_with:
  - "./mpu9250-imu.md"
  - "./hmc5883l-qmc5883l-compass.md"
  - "./bno055-orientation.md"
  - "./neo-6m-gps.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=MPU-6050" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=MPU-6050" }
  - { vendor: DFRobot, url: "https://www.dfrobot.com/search-MPU6050.html" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=MPU6050" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-GY-521.html" }
datasheet: "https://invensense.tdk.com/products/motion-tracking/6-axis/mpu-6050/"
libraries:
  - { name: "adafruit/Adafruit_MPU6050", url: "https://github.com/adafruit/Adafruit_MPU6050" }
  - { name: "jrowberg/i2cdevlib", url: "https://github.com/jrowberg/i2cdevlib" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The MPU-6050 is the default cheap 6-axis IMU — 3-axis accelerometer + 3-axis gyro + temperature
on an I²C bus. It's how most people first learn about IMUs; the "GY-521" breakout is ubiquitous
on AliExpress for a couple dollars.

## Key specs

| Spec | Value |
|---|---|
| Accelerometer | ±2 / 4 / 8 / 16 g |
| Gyroscope | ±250 / 500 / 1000 / 2000 °/s |
| Sample rate | up to 1 kHz (gyro), 1 kHz (accel) |
| Interface | I²C, 400 kHz |
| DMP | Onboard Digital Motion Processor (undocumented but usable via libraries) |
| Temp sensor | Yes (not calibrated for room-temp accuracy) |

## Interface & wiring

Four wires: VCC, GND, SDA, SCL. Address 0x68 (0x69 if AD0 pulled high). The INT pin is useful for
data-ready triggers. Many GY-521 modules have 5 V input + onboard LDO; true 3.3 V is safer for
ESP32 with level-matched pull-ups.

## Benefits

- Dirt cheap.
- Huge library / tutorial support.
- Onboard DMP can do orientation fusion without CPU overhead — but it's quirky.

## Limitations / gotchas

- No magnetometer → yaw drifts over time.
- Gyro bias is temperature-dependent; recalibrate after warm-up.
- "MPU-6050" on AliExpress often ships as silicon-compatible clones (MPU-6500-like) with
  subtle behavior differences.
- InvenSense has deprecated this part in new designs; long-term availability is uncertain.

## Typical use cases

- Tilt/angle detection.
- Step counting and basic pedestrian dead-reckoning.
- Quadcopter / drone stabilization (budget tier).

## Alternatives

See [`./mpu9250-imu.md`](./mpu9250-imu.md) (adds magnetometer) and
[`./lsm6dsx-imu.md`](./lsm6dsx-imu.md) (ST Micro family, better specs / availability).

## Pairs well with

- [`./hmc5883l-qmc5883l-compass.md`](./hmc5883l-qmc5883l-compass.md) to add heading.
- [`./bno055-orientation.md`](./bno055-orientation.md) if you want fused orientation without DIY filters.
- [`./neo-6m-gps.md`](./neo-6m-gps.md) for outdoor position + IMU fusion.

## Where to buy

- Adafruit / SparkFun / DFRobot / Seeed — authentic parts.
- AliExpress — cheap GY-521 clones.

## Software / libraries

- `adafruit/Adafruit_MPU6050`.
- `jrowberg/i2cdevlib` (classic DMP access).

## Notes for WeftOS

Speculative: expose the MPU-6050 as a raw 6-axis surface plus an optional fused "attitude"
surface; the fusion choice (Madgwick, Mahony, DMP) should be a pluggable effect, not hardwired.
