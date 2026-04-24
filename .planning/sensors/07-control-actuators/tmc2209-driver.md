---
title: "TMC2209 – Silent Stepper Driver (StealthChop / CoolStep)"
slug: "tmc2209-driver"
category: "07-control-actuators"
part_numbers: ["TMC2209"]
manufacturer: "Trinamic (now Analog Devices)"
interface: ["GPIO", "STEP/DIR", "UART"]
voltage: "4.75–29V motor, 3.3–5V logic"
logic_level: "3.3V compatible"
current_ma: "up to 2 A/phase RMS, 2.8 A peak"
price_usd_approx: "$4 – $10"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [stepper, driver, stealthchop, coolstep, uart, silent]
pairs_with:
  - "../07-control-actuators/a4988-driver.md"
  - "../02-positioning-navigation/index.md"
  - "../10-storage-timing-power/index.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=TMC2209" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=TMC2209" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=TMC2209" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-TMC2209.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-TMC2209.html" }
datasheet: "https://www.analog.com/en/products/tmc2209.html"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The TMC2209 is Trinamic's (now Analog Devices') mainstream silent stepper
driver. Pin-compatible with A4988 / DRV8825 on the 3D-printer footprint, but
with two killer features: **StealthChop2** (PWM-mode chopping that's literally
inaudible under ~50 mm/s) and **UART** configuration (current, microstepping,
stealth/spread thresholds, stallGuard — all in software).

For any WeftOS project that moves a stepper near people, this is the default.

## Key specs

| Spec | Value |
|------|-------|
| Motor voltage | 4.75 – 29 V |
| Current | up to 2.0 A RMS, 2.8 A peak |
| Microsteps | 1/8, 1/16, 1/32, 1/64, 1/128, 1/256 (interpolated) |
| Modes | StealthChop2, SpreadCycle, CoolStep, StallGuard4 |
| UART | single-wire, up to 500 kbps |
| Protection | thermal, over-current, under-voltage |

## Interface & wiring

Same STEP / DIR / EN / VMOT / VDD pin-out as A4988. The extra pin you care
about is `PDN_UART`: tie a single GPIO through a 1 kΩ resistor to this pin
and you have a full-duplex-over-half-duplex UART into the driver's register
map. With UART you can set current in software (no trimpot), pick microstep
count, enable SpreadCycle above a configurable speed, and read stallGuard
back.

## Benefits

- **Silent** — StealthChop2 is genuinely inaudible at low speeds.
- UART configuration survives firmware rebuilds; no per-board trimpot trimming.
- StallGuard enables sensorless homing — skip the endstop switch.
- Pin-compatible with A4988, so you can upgrade without PCB changes.

## Limitations / gotchas

- **UART mode is worth the wiring.** In pin-config mode you get a modern
  driver but miss half its value.
- StealthChop loses torque above a crossover speed; budget a `TPWMTHRS`
  transition into SpreadCycle for high-speed moves.
- StallGuard sensitivity is mechanical — it needs calibration per axis / load.
- Clones (notably BIGTREETECH, FYSETC) are fine, but very cheap AliExpress
  "TMC2209" listings sometimes ship old TMC2208 silicon. Check the chip marking.

## Typical use cases

- Quiet in-room 3D printers, CNCs, and camera sliders.
- Bedroom / classroom robots where A4988 buzz would be a dealbreaker.
- Sensorless-homing designs that save an endstop per axis.

## Pairs well with

- [`a4988-driver.md`](a4988-driver.md) — same footprint, noisy baseline.
- [`../02-positioning-navigation/index.md`](../02-positioning-navigation/index.md) —
  pair with encoders when you need absolute accuracy beyond stallGuard.
- [`../10-storage-timing-power/index.md`](../10-storage-timing-power/index.md) —
  current monitoring for thermal headroom.

## Where to buy

See `buy:`. BIGTREETECH / FYSETC modules from 3D-printer vendors are the
cleanest source; Trinamic (Analog Devices) sells the bare chip for custom
boards.

## Software / libraries

- `TMCStepper` (teemuatlut) is the canonical Arduino / ESP32 library for UART
  register access.
- `AccelStepper` still handles STEP/DIR motion planning on top.

## Notes for WeftOS

Treat TMC2209 UART as a first-class control surface: WeftOS rules should be
able to dial current per-axis (quiet at night, strong during the day) and
listen for stallGuard events as a fault signal.
