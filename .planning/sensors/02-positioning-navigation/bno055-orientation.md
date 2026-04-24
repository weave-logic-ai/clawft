---
title: "Bosch BNO055 Absolute Orientation Sensor"
slug: "bno055-orientation"
category: "02-positioning-navigation"
part_numbers: ["BNO055", "GY-BNO055"]
manufacturer: "Bosch Sensortec"
interface: ["I2C", "UART"]
voltage: "3.3V / 5V (breakout-dependent)"
logic_level: "3.3V"
current_ma: "~12.3 mA (NDOF mode)"
price_usd_approx: "$20 – $35"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [imu, bno055, fusion, absolute-orientation, quaternion]
pairs_with:
  - "./mpu6050-imu.md"
  - "./mpu9250-imu.md"
  - "./neo-m8n-gnss.md"
  - "./bmp180-280-388-baro.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=BNO055" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=BNO055" }
  - { vendor: DFRobot, url: "https://www.dfrobot.com/search-BNO055.html" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=BNO055" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-GY-BNO055.html" }
datasheet: "https://www.bosch-sensortec.com/products/smart-sensors/bno055/"
libraries:
  - { name: "adafruit/Adafruit_BNO055", url: "https://github.com/adafruit/Adafruit_BNO055" }
  - { name: "boschsensortec/BNO055_SensorAPI", url: "https://github.com/boschsensortec/BNO055_SensorAPI" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The BNO055 is a 9-axis IMU with an onboard Cortex-M0 running Bosch's sensor-fusion firmware. You
don't implement Madgwick or Mahony — the chip hands you Euler angles, quaternions, linear accel,
and gravity directly. That ease-of-use is its whole selling point.

## Key specs

| Spec | Value |
|---|---|
| Accel + Gyro + Mag | Combined BMA + BMG + BMM dies |
| Output rate (fusion) | 100 Hz |
| Modes | ACC, GYRO, MAG, IMU (6-axis fusion), NDOF (9-axis w/ fast calib) |
| Interface | I²C (400 kHz) or UART |
| Special outputs | Quaternion, Euler, linear accel, gravity vector |

## Interface & wiring

Four-wire I²C at address 0x28 or 0x29. The infamous "I²C clock-stretching" issue (the BNO055's
M0 can hold SCL long enough to break some masters) is well-handled on ESP32 via its I²C driver;
you can also fall back to UART at 115200 baud if you hit issues.

## Benefits

- Zero-effort absolute orientation — flash and read quaternions.
- On-chip sensor-fusion is robust and well-documented.
- Magnetometer auto-calibrates in the background (watch the calib-status register).

## Limitations / gotchas

- I²C clock stretching historically broke Arduino Uno I²C libs; ESP32 is fine but still use a
  reliable library that handles NACK properly.
- Fusion output is a black box — if you need raw sensor data for SLAM, use a different IMU.
- Requires magnetometer calibration dance (figure-8 motion) after each power-on until mag-calib
  reaches 3.

## Typical use cases

- Robot heading / balancing.
- AR headset pose.
- Pointing / aiming devices.
- Any project where you'd otherwise implement Madgwick yourself.

## Pairs well with

- [`./mpu6050-imu.md`](./mpu6050-imu.md) / [`./mpu9250-imu.md`](./mpu9250-imu.md) as raw-data alternatives.
- [`./neo-m8n-gnss.md`](./neo-m8n-gnss.md) for fused heading + position.
- [`./bmp180-280-388-baro.md`](./bmp180-280-388-baro.md) for altitude.

## Where to buy

- Adafruit / SparkFun / DFRobot / Seeed — reliable breakouts.
- AliExpress GY-BNO055 — cheap; quality ranges from OK to clone-of-clone.

## Software / libraries

- `adafruit/Adafruit_BNO055`.
- `boschsensortec/BNO055_SensorAPI` (official C driver).

## Notes for WeftOS

Speculative: BNO055 is a great first-class "orientation surface" provider — the fact that the
device already exposes quaternions means WeftOS doesn't have to pick a math convention; it can
pass through what the chip emits.
