---
title: "LD06 / LD19 360° Rotating LiDAR"
slug: "ld06-ld19-lidar"
category: "02-positioning-navigation"
part_numbers: ["LD06", "LD19", "LDS-02 (variant)"]
manufacturer: "LDRobot (LDROBOT)"
interface: ["UART (230400 baud)"]
voltage: "5V"
logic_level: "3.3V compatible"
current_ma: "~180 mA (motor + laser)"
price_usd_approx: "$65 – $95"
esp32_compat: ["ESP32", "ESP32-S3"]
tags: [lidar, 360, slam, ld06, ld19]
pairs_with:
  - "./tfmini-tfluna-lidar.md"
  - "./mpu6050-imu.md"
  - "../10-storage-timing-power/index.md"
buy:
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=LD19" }
  - { vendor: Inno-Maker, url: "https://www.inno-maker.com/search?q=ld19" }
  - { vendor: Amazon, url: "https://www.amazon.com/s?k=LD19+lidar" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-LD19-lidar.html" }
libraries:
  - { name: "linorobot/ldlidar_stl_ros", url: "https://github.com/linorobot/ldlidar_stl_ros" }
  - { name: "ldrobotSensorTeam/ldlidar_sdk", url: "https://github.com/ldrobotSensorTeam/ldlidar_sdk" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

LDRobot's LD06 and LD19 are ~$70 360° 2D rotating LiDAR units that ship with robotic vacuums
(LD19 is the sensor from the Xiaomi STL-19 P2). They put real SLAM-grade scan data into the
hobby tier, streaming a full ring of ~450 points at ~10 Hz over plain UART.

## Key specs

| Spec | LD06 | LD19 |
|---|---|---|
| Range | 0.02 – 12 m | 0.02 – 12 m |
| Scan rate | 5–13 Hz (default 10 Hz) | 5–13 Hz |
| Points per scan | ~450 | ~450 |
| Angular resolution | ~0.72° | ~0.72° |
| Interface | UART 230400 baud | UART 230400 baud |
| Motor | Internal, speed-controlled via PWM or UART |

## Interface & wiring

Connector typically gives: 5 V (motor + logic), GND, TX (data), and motor PWM (not always
broken out). Data frames carry 12 points each with angle and distance; ~2.5 kB/s sustained. ESP32
can keep up fine on a hardware UART; don't use SoftwareSerial.

## Benefits

- SLAM-quality 2-D scans at vacuum-robot prices.
- Open documentation (LDRobot publishes the frame format).
- Works with ROS 2 via existing drivers out of the box.

## Limitations / gotchas

- Mechanical spinning part → moving wear item, dust-sensitive.
- Needs stable 5 V; the motor kicks the rail around.
- Not eye-safe rated for all SKUs; treat as Class-1 at most and don't disassemble.
- Mounting orientation matters (some units expect "flat down" for self-leveling calibrations).

## Typical use cases

- Hobby SLAM / autonomous floor robot.
- 2-D room mapper.
- Furniture-scanner for making CAD floorplans.

## Pairs well with

- [`./tfmini-tfluna-lidar.md`](./tfmini-tfluna-lidar.md) as longer single-beam complement.
- [`./mpu6050-imu.md`](./mpu6050-imu.md) — odometry fusion needs an IMU.
- [`../10-storage-timing-power/index.md`](../10-storage-timing-power/index.md) — plan power for motor inrush.

## Where to buy

- Seeed / Inno-Maker (often bundled with cables).
- Amazon / AliExpress for bare modules.

## Software / libraries

- `linorobot/ldlidar_stl_ros` (ROS 2 nodes).
- `ldrobotSensorTeam/ldlidar_sdk` (C++ reference).

## Notes for WeftOS

Speculative: a 360° LiDAR fits a "polar scan surface" type — angle-indexed distance + intensity
at a fixed period. Downstream SLAM should be a WeftOS effect, not hardwired to one backend.
