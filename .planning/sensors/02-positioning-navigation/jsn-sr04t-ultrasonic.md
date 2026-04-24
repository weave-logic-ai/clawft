---
title: "JSN-SR04T Waterproof Ultrasonic"
slug: "jsn-sr04t-ultrasonic"
category: "02-positioning-navigation"
part_numbers: ["JSN-SR04T", "AJ-SR04M"]
manufacturer: "various"
interface: ["GPIO (trigger + echo)", "UART (mode select)"]
voltage: "5V"
logic_level: "5V (level-shift ECHO for ESP32)"
current_ma: "~8 mA idle, ~30 mA active"
price_usd_approx: "$4 – $9"
esp32_compat: ["ESP32 (level shift)", "ESP32-S3", "ESP32-C3"]
tags: [ultrasonic, waterproof, distance, liquid-level]
pairs_with:
  - "./hc-sr04-ultrasonic.md"
  - "./a02yyuw-ultrasonic.md"
buy:
  - { vendor: DFRobot, url: "https://www.dfrobot.com/search-JSN-SR04T.html" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=JSN-SR04T" }
  - { vendor: Amazon, url: "https://www.amazon.com/s?k=JSN-SR04T" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-JSN-SR04T.html" }
libraries:
  - { name: "PaulStoffregen/NewPing", url: "https://github.com/PaulStoffregen/NewPing" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The JSN-SR04T splits the HC-SR04 in two: a waterproof transducer at the end of a cable, and a
tiny driver PCB inside your enclosure. This makes it the default cheap sensor for liquid level,
outdoor mount, or any "keep the electronics dry" use.

## Key specs

| Spec | Value |
|---|---|
| Range | ~25 cm – 450 cm |
| Minimum distance | ~20–25 cm (worse than HC-SR04) |
| Accuracy | ±1 cm |
| Transducer | Sealed, IP67-class |
| Modes | TRIG/ECHO (mode 1, like HC-SR04), UART (mode 2), serial auto-output (mode 4) |

## Interface & wiring

Default mode matches HC-SR04 — 10 µs TRIG pulse, measure ECHO width. 5 V logic, so use a
voltage divider or level-shifter on ECHO. Some boards expose mode-select pads that let you
switch to UART (much cleaner for industrial use). Transducer cable can be extended; longer
cables add noise but usually work at ≤10 m.

## Benefits

- Waterproof transducer → outdoor / tank / rain-tolerant deployments.
- Drop-in replacement for HC-SR04 timing code.
- Mode 2 (UART) gives clean digital distance bytes — no timing pulse games.

## Limitations / gotchas

- Larger minimum range than HC-SR04; useless for close-in robots.
- Narrower beam means you have to aim more carefully.
- Clone variants ("AJ-SR04M") behave subtly differently in mode selection.
- Cable noise: keep it away from motor drivers and SMPS.

## Typical use cases

- Tank / sump / cistern level.
- Outdoor garage door sensor.
- Exterior approach detector.

## Pairs well with

- [`./hc-sr04-ultrasonic.md`](./hc-sr04-ultrasonic.md) for indoor / short-range.
- [`./a02yyuw-ultrasonic.md`](./a02yyuw-ultrasonic.md) — cleaner UART waterproof sibling.

## Where to buy

- DFRobot / Seeed / Amazon for verified modules.
- AliExpress for cheapest, including the AJ-SR04M clones.

## Software / libraries

- `PaulStoffregen/NewPing` (mode 1).
- Plain UART reads in mode 2.

## Notes for WeftOS

Speculative: the UART mode is a good template for a "serial distance surface" in WeftOS —
periodic framed samples with a known schema, far less fragile than GPIO pulse-width reads.
