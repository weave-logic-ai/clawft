---
title: "LD2410 / LD2410C / LD2410S – 24 GHz mmWave Presence Sensor"
slug: "ld2410-mmwave"
category: "06-biometric-presence"
part_numbers: ["LD2410", "LD2410B", "LD2410C", "LD2410S"]
manufacturer: "Hi-Link"
interface: ["UART", "GPIO"]
voltage: "5V (module), 3.3V logic"
logic_level: "3.3V"
current_ma: "~70–80 mA active"
price_usd_approx: "$5 – $12"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [presence, mmwave, radar, 24ghz, micro-motion, uart]
pairs_with:
  - "../07-control-actuators/relay-modules.md"
  - "../04-light-display/ws2812b-neopixel.md"
  - "../06-biometric-presence/pir-hc-sr501.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=LD2410" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=LD2410" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=LD2410" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-LD2410.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-LD2410.html" }
datasheet: "https://www.hlktech.net/"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The Hi-Link LD2410 family is a **24 GHz FMCW radar presence sensor** that finally
solves the "seated human" problem that PIR and microwave Doppler can't. It
detects both macro motion (walking) and micro-motion (breathing, typing, small
fidgets) and reports distance / energy per range gate over UART. Several variants
exist — bare LD2410, LD2410B (Bluetooth config), LD2410C (compact), and LD2410S
(newer, better micro-motion) — but they share a protocol.

## Key specs

| Spec | Value |
|------|-------|
| Frequency | 24 GHz ISM |
| Detection range | ~6 m (moving), ~4 m (stationary/breathing) |
| Gate resolution | 0.75 m per gate, 9 gates |
| Output | UART (256000 8N1) + one GPIO "occupancy" pin |
| Supply | 5 V, logic 3.3 V |
| Current | ~70–80 mA |
| FoV | ~120° H, ~60° V |

## Interface & wiring

Five pins: `5V / GND / TX / RX / OUT`. `OUT` is a fast presence/no-presence
boolean you can wire straight to a GPIO for instant-reaction logic. The UART
stream exposes per-gate energies and lets you reconfigure sensitivity, gate
count, and presence-hold time. A vendor Windows tool or the ESPHome integration
is the easiest way to get started.

## Benefits

- Detects **stationary** occupants via respiration / micro-motion — a huge step
  up from PIR.
- Per-gate data lets you define zones ("detect only beyond 2 m so I don't see
  myself typing").
- Configurable hold time avoids the PIR flap-cycle problem.
- Cheap enough to put in every room.

## Limitations / gotchas

- **Privacy:** 24 GHz radar cannot produce images, but it exposes breathing rate,
  sleep patterns, and occupancy schedules. Treat as biometric-adjacent.
- 24 GHz emissions are FCC/CE Part-15-ish but many Hi-Link modules ship without
  formal certification for end-product integration. Fine for hobby; dicey for
  a commercial enclosure.
- Moving fans, curtains, and pets all register as motion; needs tuning per room.
- Metal objects in the near field cause multi-path ghosts.

## Typical use cases

- Smart lighting that stays on while someone is seated reading.
- HVAC zoning based on real occupancy, not just motion.
- Bed-occupancy / sleep detection (pair with the [C4001](c4001-mmwave.md) for
  breathing rate).

## Pairs well with

- [`pir-hc-sr501.md`](pir-hc-sr501.md) — AND a PIR to reject microwave ghosts.
- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) —
  drive lights / HVAC on presence.
- [`../04-light-display/ws2812b-neopixel.md`](../04-light-display/ws2812b-neopixel.md) —
  visualize per-gate occupancy on an LED strip for debugging.

## Where to buy

See `buy:`. Hi-Link is the OEM; every hobby reseller rebadges the same board.

## Software / libraries

- ESPHome's `ld2410` component exposes every register and per-gate energy as
  Home Assistant entities — the most complete integration.
- Arduino driver libs exist but the protocol is simple enough to implement by
  hand (~150 lines).

## Notes for WeftOS

Promote LD2410 to **primary presence** in any WeftOS scene that wants to beat
the PIR "seated human" failure mode. Rate-limit per-gate energy streams before
they leave the device; they are leaky enough to infer private routines.
