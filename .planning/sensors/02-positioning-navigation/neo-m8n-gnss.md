---
title: "u-blox NEO-M8N GNSS"
slug: "neo-m8n-gnss"
category: "02-positioning-navigation"
part_numbers: ["NEO-M8N", "NEO-M8N-0-10"]
manufacturer: "u-blox"
interface: ["UART", "I2C (DDC)", "SPI"]
voltage: "3.3V"
logic_level: "3.3V"
current_ma: "~25 mA tracking"
price_usd_approx: "$18 – $45"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [gps, gnss, u-blox, neo-m8n, navigation, multi-constellation]
pairs_with:
  - "./neo-6m-gps.md"
  - "./bno055-orientation.md"
  - "./dwm3000-uwb.md"
  - "./bmp180-280-388-baro.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=NEO-M8N" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=NEO-M8N" }
  - { vendor: DFRobot, url: "https://www.dfrobot.com/search-NEO-M8N.html" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=NEO-M8N" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-NEO-M8N.html" }
datasheet: "https://www.u-blox.com/en/product/neo-m8-series"
libraries:
  - { name: "mikalhart/TinyGPSPlus", url: "https://github.com/mikalhart/TinyGPSPlus" }
  - { name: "sparkfun/SparkFun_Ublox_Arduino_Library", url: "https://github.com/sparkfun/SparkFun_Ublox_Arduino_Library" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The NEO-M8N is the grown-up NEO-6M: same form factor, same easy NMEA interface, but with
concurrent reception of up to 3 GNSS constellations (GPS + Galileo + GLONASS or BeiDou), 10 Hz
updates, and much better urban-canyon performance. It's the default drone / mobile-robot GNSS.

## Key specs

| Spec | Value |
|---|---|
| Constellations | GPS + Galileo + GLONASS + BeiDou (up to 3 concurrent) |
| Max fix rate | 10 Hz |
| Sensitivity | -167 dBm tracking |
| Cold start | ~26 s |
| Hot start | ~1 s |
| Interfaces | UART, I²C (DDC), SPI, USB (on some boards) |
| Accuracy | ~2.0 m CEP |

## Interface & wiring

UART is the easy path; I²C (u-blox calls it DDC) is useful on crowded designs. Most breakouts
ship with an active antenna — you want this. Power noise kills GNSS sensitivity: dedicated LDO,
solid ground plane, and keep it away from DC-DC switchers.

## Benefits

- Far better fix quality than NEO-6M under trees, between buildings, or in motion.
- 10 Hz rate is good for drones and fast robots.
- UBX binary protocol exposes rich status (SVs, DOP, velocity) beyond NMEA.

## Limitations / gotchas

- Counterfeit/clone NEO-M8N modules are common on AliExpress; some are relabelled M8 variants
  without the -N extended-capability firmware.
- No onboard IMU — pair with BNO055 or MPU-6050 for tilt/heading when stopped.
- Power-up inrush can brown out shared 3.3 V rails; give it its own LDO if you can.

## Typical use cases

- Drone / UAV navigation.
- Fast vehicle telemetry.
- Precision timing / PPS reference.

## Pairs well with

- [`./neo-6m-gps.md`](./neo-6m-gps.md) — the cheaper alternative.
- [`./bno055-orientation.md`](./bno055-orientation.md) for heading.
- [`./dwm3000-uwb.md`](./dwm3000-uwb.md) for indoor/outdoor switching.
- [`./bmp180-280-388-baro.md`](./bmp180-280-388-baro.md) for altitude fusion.

## Where to buy

- Adafruit / SparkFun / DFRobot / Seeed (trusted stock).
- AliExpress for cheaper — verify with `u-center` that it reports M8N firmware.

## Software / libraries

- `mikalhart/TinyGPSPlus` for NMEA.
- `sparkfun/SparkFun_Ublox_Arduino_Library` for UBX-protocol access.
- u-blox's own `u-center` desktop app is essential for configuration and debugging.

## Notes for WeftOS

Speculative: WeftOS should probably expose GNSS as a "location surface" with constellation mask
and per-SV SNR accessible as side-channel metadata — effects like mapping need to know when the
fix is bad, not just what the last reported position was.
