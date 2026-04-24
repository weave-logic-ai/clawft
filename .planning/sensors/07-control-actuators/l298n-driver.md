---
title: "L298N – Dual H-Bridge DC Motor Driver (Legacy)"
slug: "l298n-driver"
category: "07-control-actuators"
part_numbers: ["L298N"]
manufacturer: "STMicroelectronics (original IC); generic module vendors"
interface: ["PWM", "GPIO"]
voltage: "5V logic, 5–35V motor rail"
logic_level: "5V input (ESP32 3.3V marginally works)"
current_ma: "2 A per channel continuous (with heatsink)"
price_usd_approx: "$3 – $8"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [h-bridge, motor-driver, dc-motor, legacy, pwm]
pairs_with:
  - "../07-control-actuators/tb6612-l9110s.md"
  - "../02-positioning-navigation/index.md"
  - "../10-storage-timing-power/index.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=L298N" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=L298N" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=L298N" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-L298N.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-L298N-motor-driver.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The L298N is a BJT-based dual H-bridge motor driver from the 1990s. Hobby
modules pair it with a 7805 linear regulator, screw terminals, and two
3-pin headers (`ENA/IN1/IN2` and `ENB/IN3/IN4`). It's everywhere in the
tutorial ecosystem — every "Arduino robot car" kit ships one — but by modern
standards it's inefficient, hot, and eats ~2 V per bridge leg before the
motor ever sees current.

We document it mostly so WeftOS projects know when **not** to use it.

## Key specs

| Spec | Value |
|------|-------|
| Bridges | 2 |
| Continuous current | 2 A per channel (with heatsink) |
| Motor rail | 5–35 V |
| Logic supply | 5 V (can piggyback 7805 from motor rail < 12 V) |
| Drop | ~1.5–2 V per bridge leg |
| PWM | up to ~20 kHz |

## Interface & wiring

Per channel: `IN1` / `IN2` set direction, `ENA` takes the PWM speed signal.
Tie `ENA` / `ENB` jumpers high for always-on bridges if you don't care about
PWM. Supply the motor rail (e.g. 9 V) to `VIN`, and if motor rail ≤ 12 V leave
the 5 V regulator jumper in place to power the logic side; otherwise jumper
off and feed the logic side separately.

## Benefits

- Cheap, rugged, handles 35 V motor rails.
- Documentation and tutorials are everywhere.
- Tolerant of student-grade wiring mistakes.

## Limitations / gotchas

- **Inefficient.** The ~2 V bridge drop becomes ~4 W wasted at 2 A — that's
  why every module has a heatsink.
- Can't drive small 3.3–5 V motors well (too much drop).
- Logic inputs want 5 V. ESP32's 3.3 V works in practice but isn't spec.
- No current sense, no thermal shutdown, no sleep mode.
- 3.3 V brushed hobby motors stall at L298N's output voltage.

## Typical use cases

- Classroom / kit robotics with large 12 V motors where efficiency isn't the
  point.
- Rebuilding legacy projects where the drop-in replacement isn't worth the
  effort.

## Pairs well with

- [`tb6612-l9110s.md`](tb6612-l9110s.md) — prefer these for any new design.
- [`../02-positioning-navigation/index.md`](../02-positioning-navigation/index.md) —
  pair with encoders for closed-loop robot-car builds.
- [`../10-storage-timing-power/index.md`](../10-storage-timing-power/index.md) —
  INA219 to measure what you're wasting across the bridge.

## Where to buy

See `buy:`. Any "L298N motor driver module" result is functionally identical
— they're all clones of the same reference schematic.

## Software / libraries

Generic PWM + two GPIOs. ESPHome's `output: ledc` + `fan: speed` abstraction
models it fine.

## Notes for WeftOS

Flag L298N in the sensor graph as `legacy=true, efficiency=poor`. New WeftOS
projects should default to TB6612 or DRV8833.
