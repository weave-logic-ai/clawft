---
title: "Solenoids – Small 12V Push/Pull Actuators"
slug: "solenoids"
category: "07-control-actuators"
part_numbers: ["JF-0530B", "ZHO-0530S", "ROB-11015"]
manufacturer: "various"
interface: ["MOSFET-driven"]
voltage: "5–24V (12V most common)"
logic_level: "driven via MOSFET / relay"
current_ma: "300 mA – 2 A hold / inrush"
price_usd_approx: "$3 – $15"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [solenoid, actuator, inductive, flyback, push-pull]
pairs_with:
  - "../07-control-actuators/mosfet-drivers.md"
  - "../07-control-actuators/relay-modules.md"
  - "../06-biometric-presence/as608-fingerprint.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=solenoid" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=solenoid" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=solenoid" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-solenoid.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-solenoid-12V.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

Solenoids convert a DC pulse into a linear push or pull. A coil surrounds a
spring-loaded iron plunger; energize the coil, the plunger slams in or out.
Hobby solenoids are typically 5–24 V (12 V is the sweet spot), rated for a
specified duty cycle, and consume 300 mA to a couple of amps while energized.
They are the go-to for DIY door locks, dispensers, pinball-style kickers, and
valve actuation.

## Key specs

| Spec | Typical |
|------|---------|
| Voltage | 5, 12, 24 V DC |
| Coil current | 300 mA – 2 A |
| Stroke | 5–15 mm |
| Force | 1–10 N (varies wildly) |
| Duty cycle | **Not 100 %** on most hobby parts — read the datasheet |
| Response | 10–50 ms |

## Interface & wiring

Never wire a solenoid directly to an ESP32 GPIO. Put an N-channel MOSFET
(logic-level, e.g. IRLZ44N / AO3400) between coil and ground, PWM or on/off
from the ESP32. **Always** place a flyback diode (1N400x or Schottky) across
the coil, cathode to V+. Supply the coil rail from its own 12 V brick with
enough headroom for inrush (2–3× steady current).

## Benefits

- Clean digital "thump" — instant actuation where a servo would be too slow.
- Hold force is repeatable.
- Easy to stack several on a MOSFET bank for multi-actuator machines.

## Limitations / gotchas

- **Duty cycle.** Many hobby solenoids are rated for 25 % or 50 % duty;
  continuous energization cooks the coil. PWM-hold at 30–50 % duty once
  engaged to save power and heat.
- Back-EMF without a flyback diode will destroy your driver MOSFET on the
  first release. No exceptions.
- Magnetic fringe fields can corrupt nearby hall sensors and SD-card reads;
  mind layout.
- Force drops off fast with stroke — pick a solenoid with more stroke than
  you think you need.

## Typical use cases

- DIY electric door locks (fingerprint + solenoid).
- Pill dispensers, vending mechanisms.
- Marble-run kickers, kinetic art.
- Valve actuation in pneumatic / fluidic rigs.

## Pairs well with

- [`mosfet-drivers.md`](mosfet-drivers.md) — canonical driver.
- [`relay-modules.md`](relay-modules.md) — alternative driver if PWM isn't
  needed.
- [`../06-biometric-presence/as608-fingerprint.md`](../06-biometric-presence/as608-fingerprint.md) —
  fingerprint → solenoid is the classic DIY-lock recipe.

## Where to buy

See `buy:`. Adafruit stocks small Adafruit-branded push/pull solenoids with
good datasheets; AliExpress has the widest selection at the cost of documentation.

## Software / libraries

None needed — it's a MOSFET GPIO. Worth building a "pulse-then-hold" helper
so you always drop to low-duty hold after the initial slam.

## Notes for WeftOS

Wrap solenoids in a WeftOS actuator primitive with a **max duty cycle**
clamp and a **minimum off time** — otherwise rule chaining will burn coils.
