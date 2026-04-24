---
title: "28BYJ-48 + ULN2003 – Geared Unipolar Stepper"
slug: "28byj-48-stepper"
category: "07-control-actuators"
part_numbers: ["28BYJ-48", "ULN2003"]
manufacturer: "Kiatronics / various"
interface: ["GPIO"]
voltage: "5V (often rewired for 12V)"
logic_level: "3.3V / 5V GPIO"
current_ma: "~240 mA per coil"
price_usd_approx: "$2 – $4 (motor + driver board)"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [stepper, unipolar, geared, uln2003, cheap]
pairs_with:
  - "../07-control-actuators/a4988-driver.md"
  - "../07-control-actuators/mosfet-drivers.md"
  - "../10-storage-timing-power/index.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=28BYJ-48" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=28BYJ" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=28BYJ-48" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-stepper.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-28BYJ-48-stepper.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The 28BYJ-48 is a tiny 5 V **unipolar, geared** stepper that ships glued to a
ULN2003 Darlington-array driver board. The gearbox gives it a practical step
resolution of about **4096 steps per output revolution** — slow but very fine.
Torque is modest but not trivial (~300 gf·cm). It's the cheapest way to get a
positional actuator running on an ESP32, and it's everywhere in tutorial land.

## Key specs

| Spec | Value |
|------|-------|
| Type | Unipolar, 5-wire |
| Steps/rev (motor) | 64 full steps |
| Gearbox | ~1:64 (nominal; actual ≈ 1:63.68) |
| Steps/rev (output) | ~2048 full, ~4096 half |
| Supply | 5 V (most common); 12 V variant exists |
| Current | ~240 mA peak per coil |

## Interface & wiring

The motor plugs into the ULN2003 board's 5-pin socket. The board exposes
`IN1–IN4` plus `VCC` / `GND`. Drive the four inputs from any four ESP32 GPIOs
— logic levels are fine at 3.3 V because the ULN2003 just needs > 2 V at its
inputs. Supply the motor's V+ from a **separate** 5 V rail; don't pull it
from the ESP32's onboard regulator — inrush will brown the MCU.

## Benefits

- The cheapest geared positional actuator there is.
- 4096 steps/rev without microstepping means smooth, precise moves at low
  effort.
- Works from a 3.3 V GPIO directly — no driver chip configuration.

## Limitations / gotchas

- **Slow.** Top speed is a few RPM at the output shaft — fine for clocks,
  valves, blinds; not for wheels.
- Plastic gearbox; don't back-drive hard loads.
- ULN2003 dissipates heat at high duty — it's Darlington, not MOSFET.
- The motor continues to **hold current** when idle; energize only during
  motion or it cooks itself and your battery.
- Not interchangeable with bipolar steppers — it is 5-wire unipolar. A4988 /
  TMC2209 **will not** drive it directly.

## Typical use cases

- Blinds / curtain openers.
- Analog clock hands, mechanical indicators.
- Valve actuators for plant watering.
- Teaching demos — one motor + driver for under $4.

## Pairs well with

- [`a4988-driver.md`](a4988-driver.md) — for contrast; note 28BYJ-48 is
  unipolar and **cannot** use A4988.
- [`mosfet-drivers.md`](mosfet-drivers.md) — power-gate the motor rail
  between moves; the ULN2003 alone can't do that cleanly.
- [`../10-storage-timing-power/index.md`](../10-storage-timing-power/index.md) —
  monitor hold current so you remember to turn the coils off.

## Where to buy

See `buy:`. Sold in pairs (motor + driver) for under $3 on AliExpress, $5–8
from Adafruit / SparkFun.

## Software / libraries

- Arduino `Stepper` library works out of the box.
- `AccelStepper` supports 4-wire mode for smoother acceleration profiles.

## Notes for WeftOS

Always gate motor power in WeftOS rules: `move(steps) -> energize -> sleep`.
Persist the step count so absolute position (e.g. blind angle) survives
reboots.
