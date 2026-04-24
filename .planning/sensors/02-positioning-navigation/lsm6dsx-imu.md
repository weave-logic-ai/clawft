---
title: "STMicro LSM6DS3 / LSM9DS1 IMU family"
slug: "lsm6dsx-imu"
category: "02-positioning-navigation"
part_numbers: ["LSM6DS3", "LSM6DSO", "LSM6DSOX", "LSM9DS1"]
manufacturer: "STMicroelectronics"
interface: ["I2C", "SPI"]
voltage: "1.71–3.6V (module typ. 3.3V)"
logic_level: "3.3V"
current_ma: "~0.65 mA (LSM6DS3), ~4 mA (LSM9DS1 9-axis)"
price_usd_approx: "$5 – $15"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [imu, lsm6ds3, lsm9ds1, st, 6-axis, 9-axis]
pairs_with:
  - "./mpu6050-imu.md"
  - "./mpu9250-imu.md"
  - "./bno055-orientation.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=LSM6DS3" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=LSM9DS1" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=LSM6DS3" }
  - { vendor: DigiKey, url: "https://www.digikey.com/en/products/search?keywords=LSM6DS3" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-LSM6DS3.html" }
datasheet: "https://www.st.com/en/mems-and-sensors/inemo-inertial-modules.html"
libraries:
  - { name: "adafruit/Adafruit_LSM6DS", url: "https://github.com/adafruit/Adafruit_LSM6DS" }
  - { name: "arduino-libraries/Arduino_LSM9DS1", url: "https://github.com/arduino-libraries/Arduino_LSM9DS1" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

ST's LSM6DS3, LSM6DSO(X), and LSM9DS1 fill the same niche as the Invensense family but with
better long-term availability and cleaner datasheets. LSM6DS3/DSO is 6-axis; LSM9DS1 adds an
LIS3MDL-compatible magnetometer for 9-axis in a single package.

## Key specs (LSM6DSOX representative)

| Spec | Value |
|---|---|
| Accel | ±2 / 4 / 8 / 16 g |
| Gyro | ±125 / 250 / 500 / 1000 / 2000 °/s |
| Mag (LSM9DS1 only) | ±4 / 8 / 12 / 16 gauss |
| Output rate | up to 6.66 kHz |
| Extras (DSOX) | Machine-learning core on-chip (32-decision-tree classifier) |
| Interface | I²C up to 400 kHz, SPI up to 10 MHz |

## Interface & wiring

Four-wire I²C or classic SPI. LSM9DS1 has separate accel/gyro and mag dies with different I²C
addresses (0x6A and 0x1C on Adafruit/SparkFun modules). All parts run at 3.3 V native.

## Benefits

- Excellent data sheet and long-term STMicro support (good for products).
- DSOX variant includes an on-chip ML core — wake-on-activity without CPU.
- 9-axis option with a canonical STMicro magnetometer (well-behaved calibration).

## Limitations / gotchas

- Slightly more expensive than MPU-6050 clones.
- LSM9DS1 is also nearing EOL; LSM6DSOX + LIS3MDL is the newer modular combo.
- Community tutorials lag behind MPU series — more time reading datasheets.

## Typical use cases

- Commercial-quality wearables.
- Low-power wake-on-motion (DSOX ML core).
- Robotics when you want a non-EOL IMU.

## Pairs well with

- [`./mpu6050-imu.md`](./mpu6050-imu.md) as the cheap alternative.
- [`./mpu9250-imu.md`](./mpu9250-imu.md) as the older 9-axis comparison.
- [`./bno055-orientation.md`](./bno055-orientation.md) for fused orientation instead of raw.

## Where to buy

- Adafruit / SparkFun / Seeed / DigiKey for known-good breakouts.
- AliExpress for bare modules (less common than MPU clones).

## Software / libraries

- `adafruit/Adafruit_LSM6DS`.
- `arduino-libraries/Arduino_LSM9DS1` (bundled on Nano 33 boards).

## Notes for WeftOS

Speculative: the LSM6DSOX ML core is interesting for WeftOS — it could emit *pre-classified*
events (e.g. "walking", "shake") upstream without waking the main MCU, a nice fit for the
"effect runs near the sensor" model.
