---
title: "u-blox NEO-6M GPS"
slug: "neo-6m-gps"
category: "02-positioning-navigation"
part_numbers: ["NEO-6M", "NEO-6M-0-001"]
manufacturer: "u-blox"
interface: ["UART"]
voltage: "3.3V / 5V (breakout dependent)"
logic_level: "3.3V"
current_ma: "~45 mA active, ~11 mA tracking"
price_usd_approx: "$8 – $18"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [gps, gnss, u-blox, neo-6m, navigation]
pairs_with:
  - "./neo-m8n-gnss.md"
  - "./mpu6050-imu.md"
  - "./bmp180-280-388-baro.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=NEO-6M" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=NEO-6M" }
  - { vendor: DFRobot, url: "https://www.dfrobot.com/search-NEO-6M.html" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=NEO-6M" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-NEO-6M.html" }
datasheet: "https://www.u-blox.com/en/product/neo-6-series"
libraries:
  - { name: "mikalhart/TinyGPSPlus", url: "https://github.com/mikalhart/TinyGPSPlus" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The NEO-6M is the canonical "hello world" GPS for ESP32 — cheap, GPS-only, 1 Hz fix, and every
tutorial on the internet uses it. It's not the best module any more (NEO-M8N and up are better)
but it's still the easiest first GPS to get working.

## Key specs

| Spec | Value |
|---|---|
| Constellations | GPS only |
| Max fix rate | 5 Hz (1 Hz default) |
| Sensitivity | -161 dBm tracking |
| Cold start | ~27 s typical |
| Hot start | ~1 s |
| Interface | UART (NMEA), default 9600 baud |
| Accuracy | ~2.5 m CEP |

## Interface & wiring

Four wires: VCC, GND, TX, RX. Breakouts usually include the 3 V LDO and a backup battery for
ephemeris/almanac retention (keeps hot-start fast between power cycles). Use any free UART on
ESP32 (HardwareSerial). Keep the antenna with a clear sky view — indoor tests almost always fail.

## Benefits

- Cheap, plentiful, well-documented.
- Onboard backup battery speeds repeat starts.
- Works with any NMEA parser (TinyGPS / TinyGPSPlus).

## Limitations / gotchas

- GPS-only: no Galileo/GLONASS/BeiDou, so fix quality is worse than newer modules.
- 1 Hz default is fine for walking, bad for drones.
- Antennas shipped on cheap boards are mediocre; expect cold-start >60 s indoors next to a window.
- Many "NEO-6M" boards on AliExpress are actually older NEO-6 variants or outright clones.

## Typical use cases

- Teaching GPS basics.
- Slow-moving data loggers.
- Time-reference source (pulse-per-second on some breakouts).

## Pairs well with

- [`./neo-m8n-gnss.md`](./neo-m8n-gnss.md) — the better modern alternative.
- [`./mpu6050-imu.md`](./mpu6050-imu.md) for dead-reckoning between fixes.
- [`./bmp180-280-388-baro.md`](./bmp180-280-388-baro.md) for altitude corroboration.

## Where to buy

- Adafruit / SparkFun / DFRobot / Seeed for verified modules.
- AliExpress for cheap clones (quality varies).

## Software / libraries

- `mikalhart/TinyGPSPlus` — standard NMEA parser.
- `u-blox/ubxlib` for UBX-protocol features if you want binary.

## Notes for WeftOS

Speculative: a GPS surface in WeftOS should expose fix state + HDOP as first-class, not just
lat/lon — effects that assume a valid fix need a clean "no-fix" signal, which is where most
naive projects go wrong.
