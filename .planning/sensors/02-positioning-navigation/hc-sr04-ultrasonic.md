---
title: "HC-SR04 Ultrasonic Rangefinder"
slug: "hc-sr04-ultrasonic"
category: "02-positioning-navigation"
part_numbers: ["HC-SR04", "HC-SR04+"]
manufacturer: "various"
interface: ["GPIO (trigger + echo)"]
voltage: "5V (HC-SR04) / 3–5.5V (HC-SR04+)"
logic_level: "5V echo (level shift for ESP32!) or 3.3V on HC-SR04+"
current_ma: "~15 mA"
price_usd_approx: "$1 – $3"
esp32_compat: ["ESP32 (via level shifter)", "ESP32-S3", "ESP32-C3"]
tags: [ultrasonic, range, distance, hc-sr04]
pairs_with:
  - "./jsn-sr04t-ultrasonic.md"
  - "./vl53l0x-vl53l1x-tof.md"
  - "./tfmini-tfluna-lidar.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=HC-SR04" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=HC-SR04" }
  - { vendor: DFRobot, url: "https://www.dfrobot.com/search-HC-SR04.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-HC-SR04.html" }
libraries:
  - { name: "ErickSimoes/Ultrasonic", url: "https://github.com/ErickSimoes/Ultrasonic" }
  - { name: "PaulStoffregen/NewPing", url: "https://github.com/PaulStoffregen/NewPing" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The HC-SR04 is the iconic $1 ultrasonic distance sensor: fire a 40 kHz ping on TRIG, measure the
echo pulse width on ECHO, divide by ~58 µs/cm. Good enough for most "don't hit the wall" robot
builds; not good enough for anything needing precision or multi-sensor concurrency.

## Key specs

| Spec | Value |
|---|---|
| Range | ~2 cm – 400 cm |
| Accuracy | ±3 mm (ideal conditions) |
| Beam angle | ~15° cone |
| Cycle time | ~60 ms per ping (to avoid self-echo) |
| Voltage | 5 V (original); 3–5.5 V on HC-SR04+ |

## Interface & wiring

Two GPIOs: TRIG out (10 µs pulse), ECHO in. On the original HC-SR04, **ECHO is 5 V**; use a
voltage divider (10 kΩ + 20 kΩ) or logic-level shifter before feeding ESP32. The HC-SR04+ variant
runs fully at 3.3 V and is worth the extra $0.50.

## Benefits

- Unbeatable price.
- Simple timing-based protocol — no library strictly required.
- Works on any material that reflects sound (unlike some ToF sensors that struggle with black
  surfaces).

## Limitations / gotchas

- Soft / angled surfaces scatter the echo and return bad readings.
- Cross-talk between multiple HC-SR04 units on the same robot is common — stagger pings.
- Temperature affects sound speed; calibrate if you need better than ~1 cm.
- Minimum range ~2 cm — it can't see things pressed against it.

## Typical use cases

- Robot obstacle avoidance.
- Parking sensors / garage sensors.
- Liquid level (with caveats; see JSN-SR04T for proper waterproof).

## Pairs well with

- [`./jsn-sr04t-ultrasonic.md`](./jsn-sr04t-ultrasonic.md) — waterproof sibling.
- [`./vl53l0x-vl53l1x-tof.md`](./vl53l0x-vl53l1x-tof.md) — laser ToF complement (different failure modes).
- [`./tfmini-tfluna-lidar.md`](./tfmini-tfluna-lidar.md) for longer range.

## Where to buy

- Adafruit / SparkFun / DFRobot for verified units.
- AliExpress by the 10-pack for pennies.

## Software / libraries

- `ErickSimoes/Ultrasonic` — minimal, works fine.
- `PaulStoffregen/NewPing` — handles multi-sensor and timeouts.

## Notes for WeftOS

Speculative: an HC-SR04 node fits a "scheduled ping → distance sample" pattern. WeftOS should
probably expose the ping as an explicit activation so contending sensors can be scheduled and
crosstalk avoided.
