---
title: "A4988 – Stepper Motor Driver"
slug: "a4988-driver"
category: "07-control-actuators"
part_numbers: ["A4988"]
manufacturer: "Allegro Microsystems"
interface: ["GPIO", "STEP/DIR"]
voltage: "8–35V motor, 3.3–5V logic"
logic_level: "3.3V compatible"
current_ma: "up to 2 A/phase with heatsink + cooling"
price_usd_approx: "$2 – $5"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [stepper, driver, step-dir, microstepping, a4988]
pairs_with:
  - "../07-control-actuators/tmc2209-driver.md"
  - "../07-control-actuators/28byj-48-stepper.md"
  - "../02-positioning-navigation/index.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=A4988" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=A4988" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=A4988" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-A4988.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-A4988-stepper.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The A4988 is the 3D-printer-ecosystem stepper driver: a small DIP-like module
that takes STEP + DIR pins, a `VREF` trimpot for current limit, and three
microstep-select pins (`MS1 / MS2 / MS3`). It drives a bipolar stepper at up
to 2 A/phase and supports full, half, 1/4, 1/8, and 1/16 microstepping. Cheap,
loud, and everywhere — though for new WeftOS designs the [TMC2209](tmc2209-driver.md)
is quieter and smarter.

## Key specs

| Spec | Value |
|------|-------|
| Motor voltage | 8–35 V |
| Current | up to 2 A/phase (needs heatsink + fan for > 1 A) |
| Microsteps | 1, 1/2, 1/4, 1/8, 1/16 |
| Logic | 3.3 V / 5 V compatible |
| Enable | active-LOW |
| Sleep | pulled via `SLEEP` pin |

## Interface & wiring

ESP32 drives `STEP` (rising edges = one microstep) and `DIR`. `MS1 / MS2 /
MS3` set microstep mode. `VMOT` takes motor rail with a 100 µF cap across it.
`VDD` = 3.3 V or 5 V logic. Set the `VREF` trimpot **before** connecting the
motor: `Imax = VREF × 2.5` for the standard Pololu-style board (check your
board — clones vary). Exceed the rating and the chip thermal-shutdowns.

## Benefits

- Cheap, ubiquitous, drop-in pin-compatible with TMC2209 / DRV8825.
- Simple STEP/DIR interface.
- Microstepping smooths motion at no MCU cost.

## Limitations / gotchas

- **Loud.** Audibly buzzy in the 1–20 kHz step-rate range. If silence matters,
  use a TMC2209.
- Needs a heatsink above ~1 A/phase. Many modules ship a sticker heatsink; it
  is not optional.
- No UART / SPI — current and microstep are set by trimpot + pins, not software.
- Cheap clones may have inverted `VREF` formulas or weak traces; budget for
  one or two DOA modules per batch.

## Typical use cases

- 3D-printer / CNC axis drivers (4–5 modules per machine).
- DIY camera sliders, pan-tilt rigs.
- Pump / dosing systems.

## Pairs well with

- [`tmc2209-driver.md`](tmc2209-driver.md) — pin-compatible quieter upgrade.
- [`28byj-48-stepper.md`](28byj-48-stepper.md) — note that 28BYJ-48 is
  unipolar and uses ULN2003, **not** A4988.
- [`../02-positioning-navigation/index.md`](../02-positioning-navigation/index.md) —
  pair with endstops / encoders for real positioning.

## Where to buy

See `buy:`. Pololu carries the reference design; every 3D-printer-parts vendor
sells clones.

## Software / libraries

`AccelStepper` is the canonical Arduino library for trapezoidal-velocity
profiles. ESP32 supports `MCPWM` and `RMT` for hardware-timed STEP pulses —
worth using for jitter-sensitive motion.

## Notes for WeftOS

A4988 is the "cheap default" stepper driver. WeftOS should track step counts
per motor in persistent memory so absolute position survives reboots (no
encoder = no ground truth; the step count is your best estimate).
