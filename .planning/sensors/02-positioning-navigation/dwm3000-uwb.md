---
title: "Qorvo DWM3000 UWB Module"
slug: "dwm3000-uwb"
category: "02-positioning-navigation"
part_numbers: ["DWM3000", "DWM3000EVB", "MakerFabs ESP32-UWB DW3000"]
manufacturer: "Qorvo (formerly Decawave)"
interface: ["SPI"]
voltage: "3.3V"
logic_level: "3.3V"
current_ma: "~130 mA peak TX, ~70 mA RX"
price_usd_approx: "$20 – $45 (module); $45+ (ESP32-UWB boards)"
esp32_compat: ["ESP32", "ESP32-S3"]
tags: [uwb, rtls, positioning, dwm3000, apple-u1, qorvo]
pairs_with:
  - "./neo-m8n-gnss.md"
  - "./mpu6050-imu.md"
  - "./bno055-orientation.md"
  - "../08-communication/index.md"
buy:
  - { vendor: Qorvo, url: "https://www.qorvo.com/products/p/DWM3000" }
  - { vendor: Makerfabs, url: "https://www.makerfabs.com/search?q=UWB" }
  - { vendor: Mouser, url: "https://www.mouser.com/c/?q=DWM3000" }
  - { vendor: DigiKey, url: "https://www.digikey.com/en/products/search?keywords=DWM3000" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-DWM3000.html" }
datasheet: "https://www.qorvo.com/products/p/DWM3000"
libraries:
  - { name: "Makerfabs/Makerfabs-ESP32-UWB-DW3000", url: "https://github.com/Makerfabs/Makerfabs-ESP32-UWB-DW3000" }
  - { name: "foldedtoad/dwm3000", url: "https://github.com/foldedtoad/dwm3000" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The DWM3000 is Qorvo's IEEE 802.15.4z ultra-wideband radio — the same technology in Apple U1 and
AirTags. With a set of fixed "anchor" nodes and a moving "tag," it delivers 10–30 cm real-time
location in environments where GPS doesn't reach. The MakerFabs ESP32-UWB DW3000 boards make it
approachable.

## Key specs

| Spec | Value |
|---|---|
| Standard | IEEE 802.15.4-2020 (4z) HRP UWB |
| Channels | 5 (6.5 GHz) and 9 (8 GHz) |
| Ranging accuracy | ±10 cm typical, ±30 cm worst |
| Typical range | 30–50 m LOS, 10–20 m NLOS |
| Data rate | 110 kb/s – 6.8 Mb/s |
| Interface | SPI up to 38 MHz + IRQ |
| Power | ~130 mA peak |

## Interface & wiring

SPI + IRQ + RST. Don't skip the IRQ line — polling the DW3000 registers is painful. Clock the SPI
at 8 MHz to start; push up to 20 MHz once it's stable. Keep analog / RF sections away from
digital noise; an ESP32 and DW3000 on the same board need careful ground planes.

## Benefits

- cm-class indoor positioning, GPS-denied environments.
- Works through thin walls (2.4 GHz-like penetration but with ranging).
- Secure Apple Nearby Interaction (U1) interop is possible on 4z modules like this one.

## Limitations / gotchas

- You need at least 3 anchors for 2-D and 4 for 3-D. Installation is real work.
- NLOS ranging multipath errors can fool you; fuse with IMU for robustness.
- Regulatory: UWB is unlicensed in US/EU but power limits apply; stick to reference firmware.
- DW3000 driver code is a beast — use the Qorvo API verbatim, don't try to rewrite it.

## Typical use cases

- Indoor RTLS for warehouses / retail / art installations.
- Robot-follow-me / drone-follow-me without GPS.
- Secure digital key / proximity unlocking.
- Sport / gaming position tracking.

## Pairs well with

- [`./neo-m8n-gnss.md`](./neo-m8n-gnss.md) for seamless indoor/outdoor handoff.
- [`./mpu6050-imu.md`](./mpu6050-imu.md) / [`./bno055-orientation.md`](./bno055-orientation.md) for between-UWB-update dead-reckoning.
- [`../08-communication/index.md`](../08-communication/index.md) for backhaul of positions to a server.

## Where to buy

- Qorvo direct (DWM3000EVB).
- MakerFabs for ESP32-UWB combo boards.
- Mouser / DigiKey for DWM3000 modules.
- AliExpress for MakerFabs clones.

## Software / libraries

- `Makerfabs/Makerfabs-ESP32-UWB-DW3000` — ready-to-run tag/anchor firmware.
- `foldedtoad/dwm3000` — leaner port of Qorvo's API to Arduino/ESP-IDF.

## Notes for WeftOS

Speculative: WeftOS can model UWB as a "position surface with covariance," which plays well with
the GNSS surface contract — effects that do fusion shouldn't care whether the position came from
satellites or radios, only about the covariance and timestamp.
