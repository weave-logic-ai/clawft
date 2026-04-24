---
title: "Hobby Servos – SG90 / MG90S / MG996R"
slug: "hobby-servos"
category: "07-control-actuators"
part_numbers: ["SG90", "MG90S", "MG996R"]
manufacturer: "TowerPro / generic"
interface: ["PWM"]
voltage: "4.8–6V"
logic_level: "5V PWM (3.3V marginally works)"
current_ma: "100–650 mA idle/stall depending on model"
price_usd_approx: "$3 – $15"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [servo, actuator, pwm, rotation, hobby]
pairs_with:
  - "../07-control-actuators/pca9685-servo-driver.md"
  - "../07-control-actuators/mosfet-drivers.md"
  - "../10-storage-timing-power/index.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=servo" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=servo" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=servo" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-servo.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-SG90-MG996R-servo.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

Three-wire hobby servos are the default "rotate to an angle" actuator: closed-loop
position controllers in a 20 mm package. The SG90 is the classic 9 g plastic-gear
micro servo; MG90S is the same size with metal gears and more torque; MG996R is a
55 g standard-size servo with ~11 kg·cm of torque and roughly 650 mA stall current.
All three speak the same 50 Hz PWM protocol where pulse widths of 1.0–2.0 ms map
to roughly ±90° of rotation.

## Key specs

| Model | Weight | Torque (6 V) | Idle / stall current | Gears |
|-------|--------|--------------|----------------------|-------|
| SG90 | 9 g | ~1.8 kg·cm | ~10 / ~250 mA | Plastic |
| MG90S | 14 g | ~2.0 kg·cm | ~10 / ~400 mA | Metal |
| MG996R | 55 g | ~11 kg·cm | ~10 / ~650 mA | Metal |

- PWM: 50 Hz, 1.0–2.0 ms pulse; center at 1.5 ms.
- Supply: 4.8–6 V. Do **not** power from the 3.3 V rail.

## Interface & wiring

Three wires: orange/white = PWM signal, red = V+, brown/black = GND. The PWM
signal line tolerates 3.3 V from ESP32 reasonably well, though 5 V is in-spec.
Share the ground with the MCU; supply servo V+ from a dedicated 5 V, 2–5 A rail
(a buck regulator or BEC) rather than the ESP32's onboard regulator.

## Benefits

- Trivial PWM protocol, huge ecosystem, cheap.
- Closed-loop internal controller — "go to angle X" is literally one PWM write.
- Metal-geared variants survive respectable loads for a few dollars.

## Limitations / gotchas

- **Stall current is huge.** MG996R stall can crash a 1 A supply. Budget a
  dedicated servo rail with > 2× the aggregate stall current.
- Analog hobby servos don't report position or load — you command an angle and
  hope. Under load they hunt, buzz, and heat up.
- Glitching on boot: servos often jerk to random positions while the MCU's
  PWM stabilizes. Wire through a transistor or a PCA9685 + `OE` pin to
  suppress.
- SG90 plastic gears strip easily; MG90S or MG996R for anything load-bearing.

## Typical use cases

- Pan-tilt camera heads, robotic grippers, door latches, feeders.
- Mechanical indicator dials ("servo clock").
- Cheap single-axis actuators for art installations.

## Pairs well with

- [`pca9685-servo-driver.md`](pca9685-servo-driver.md) — offload PWM for 6+
  servos.
- [`mosfet-drivers.md`](mosfet-drivers.md) — power-gate the servo rail to
  silence idle buzz and save power.
- [`../10-storage-timing-power/index.md`](../10-storage-timing-power/index.md) —
  INA219 to catch stall events via current spikes.

## Where to buy

See `buy:`. Genuine TowerPro parts cost a bit more and fail less often than
unbranded AliExpress clones.

## Software / libraries

`ESP32Servo` is the canonical Arduino library on ESP32; it uses the LEDC PWM
channels underneath. ESPHome's `servo` component wraps it for rule-driven use.

## Notes for WeftOS

Model each servo with a clamp (`min_us`, `max_us`, `max_rate_deg_per_s`) so
WeftOS rules can't accidentally slam the gearbox. Power-gate the servo rail
when no rule is commanding movement.
