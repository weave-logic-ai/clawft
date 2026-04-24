---
title: "MOSFET Driver Modules – IRF520 & Dual-MOSFET PWM Boards"
slug: "mosfet-drivers"
category: "07-control-actuators"
part_numbers: ["IRF520", "IRLZ44N", "AO3400", "IRF3205"]
manufacturer: "various module vendors"
interface: ["PWM", "GPIO"]
voltage: "5–36V load, 3.3V PWM input (board-specific)"
logic_level: "3.3V or 5V"
current_ma: "up to ~5 A (IRF520 module); 15+ A dual-MOSFET boards"
price_usd_approx: "$2 – $8"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [mosfet, pwm, power-gate, actuator, dimmer]
pairs_with:
  - "../04-light-display/ws2812b-neopixel.md"
  - "../01-vision-imaging/esp32-cam-ov2640.md"
  - "../07-control-actuators/solenoids.md"
  - "../07-control-actuators/vibration-motors.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=MOSFET" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=MOSFET" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=MOSFET" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-MOSFET.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-IRF520-mosfet-module.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

MOSFET driver modules are low-side N-channel switches with screw terminals, a
gate resistor, and sometimes a pull-down. The classic IRF520 board handles up
to ~5 A at 0–24 V from a PWM input; "dual MOSFET PWM" boards (usually built
around IRF3205 or IRLZ44N + a gate driver IC) push 15 A+ at 5–36 V. They are
the go-to for **power-gating** peripherals that the ESP32 can't natively switch
— cameras, LED strips, heaters, solenoids, small DC motors.

## Key specs

| Variant | Part | Load V | Load I | PWM |
|---------|------|--------|--------|-----|
| IRF520 hobby board | IRF520 | 0–24 V | ~5 A (heatsink dependent) | 3.3–5 V input |
| "Dual MOSFET PWM" | IRLZ44N / IRF3205 + gate driver | 5–36 V | 15 A+ | 3.3–5 V input, better drive |
| AO3400 SOT-23 | AO3400 | 0–20 V | ~3 A | true logic-level |

## Interface & wiring

Screw terminals for `VIN` / `GND` and `V+` / `V-` for the load. PWM input on
the signal header goes straight to an ESP32 `ledc` PWM pin. Add your own
flyback diode for inductive loads (motors, solenoids, relay coils) — the
bare IRF520 board does not include one.

## Benefits

- Cheap power-gate: cut current to a camera / radar / strip to save hundreds
  of mA in idle.
- PWM-capable, so you get brightness / speed / heat control for free.
- High-current boards handle loads an ESP32 GPIO couldn't dream of.

## Limitations / gotchas

- **IRF520 is not a true logic-level MOSFET.** Its `VGS(th)` is ~4 V, so at a
  3.3 V gate it's partially on and dissipates heat. Prefer IRLZ44N, AO3400, or
  any board with a gate driver IC for ESP32-direct drive.
- No flyback diode on plain IRF520 boards — add one across inductive loads.
- Share the ESP32 ground with the load ground, or your PWM reference floats.
- Ground bounce matters: keep the load ground return short, thick, and
  separate from signal ground.

## Typical use cases

- Power-cycling an ESP32-CAM between shots (huge current savings).
- Dimming 12 V LED strips, incandescents, or fans.
- Driving 12 V solenoids, vibration motors, pumps.
- Heated-bed / hot-end control (with thermal interlock).

## Pairs well with

- [`../01-vision-imaging/esp32-cam-ov2640.md`](../01-vision-imaging/esp32-cam-ov2640.md) —
  power-gate the camera between shots.
- [`../04-light-display/ws2812b-neopixel.md`](../04-light-display/ws2812b-neopixel.md) —
  power-gate the strip's 5 V rail so quiescent draw is zero.
- [`solenoids.md`](solenoids.md) / [`vibration-motors.md`](vibration-motors.md) —
  canonical MOSFET-driven loads.

## Where to buy

See `buy:`. AliExpress carries both the IRF520 and dual-MOSFET boards for a
few dollars; Adafruit stocks logic-level "power MOSFET breakouts" if you want
documentation.

## Software / libraries

Use the ESP32 `ledc` driver for PWM. No library specific to the board.

## Notes for WeftOS

MOSFET power-gates are a core WeftOS pattern: every hungry peripheral
(cameras, mmWave radar, cellular modems) sits behind one, and the scheduler
decides when to turn it on. Track on-time in local memory to estimate
per-peripheral energy cost.
