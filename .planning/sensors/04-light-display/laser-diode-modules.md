---
title: "Laser Diode Modules (650 nm red dot, 5 mW)"
slug: "laser-diode-modules"
category: "04-light-display"
part_numbers: ["KY-008", "generic 650nm 5mW laser module"]
manufacturer: "various"
interface: ["GPIO"]
voltage: "3.3V / 5V"
logic_level: "3.3V"
current_ma: "~30 mA"
price_usd_approx: "$1 – $5"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["laser", "650nm", "pointer", "alignment", "class-IIIa"]
pairs_with:
  - "../02-positioning-navigation/"
  - "../01-vision-imaging/"
  - "../07-control-actuators/mosfet-drivers.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=laser+diode" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=laser" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=laser+module" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-laser.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-650nm-laser-module.html" }
libraries:
  - "none required — treat as a GPIO-switched LED"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A 650 nm red laser dot module (often sold as KY-008 in the Arduino ecosystem) is a tiny laser diode plus its current-limit resistor on a board. For WeftOS it's useful as a cheap pointer, alignment aid, structured-light source, or crude range finder when paired with a camera that can triangulate the return spot.

## Key specs

- Wavelength: 650 nm (visible red). Some modules are 532 nm (green, brighter to the eye).
- Power: ≤ 5 mW (Class IIIa / 3R). Never buy "high-power" unbranded modules — eye hazard and legality issues.
- Drive: on-board resistor; the whole module is essentially a switchable LED from the MCU's perspective.
- Operating voltage: 3.3 V or 5 V depending on board; check silk screen before hooking up.

## Interface & wiring

- `+`/`-` pins direct to 3.3 V or 5 V rail and ground; `S` (signal) to a GPIO.
- Treat it as a GPIO-switched load: HIGH = laser on.
- For modulation (e.g., pulsed ranging), drive via a MOSFET — some modules draw above the GPIO's recommended sink current.
- Keep leads short; unshielded laser modules radiate RFI that can upset nearby I²C lines.

## Benefits

- Pennies, zero protocol, zero driver IC.
- Visible red spot makes mechanical alignment (cameras, LiDAR, heliostats) trivial.
- Can be pulsed to tens of kHz for simple modulation / lock-in sensing.

## Limitations / gotchas

- **Class IIIa laser.** Bright enough to cause retinal damage on prolonged direct exposure. WeftOS product builds must include a physical interlock if the laser points anywhere a user could look into it.
- Beam pattern from cheap modules is frequently an oval blob, not a crisp dot; factor into any CV triangulation budget.
- Optical power drifts with temperature — don't use as a photometric reference.
- Red 650 nm is poorly visible on red backgrounds and in sunlight; consider green if outdoors.
- Lifetime is shortened dramatically by reverse voltage or >2× rated current.

## Typical use cases

- Alignment reticle for cameras, telescopes, solar trackers.
- Poor-man's rangefinder paired with a camera (triangulate dot position in frame).
- Structured-light stripe when fanned through a cylindrical lens.
- Visible "aim point" on a non-visual sensor (e.g., an IR thermometer).

## Pairs well with

- [`../02-positioning-navigation/`](../02-positioning-navigation/) — cross-check laser triangulation against ToF readings.
- [`../01-vision-imaging/`](../01-vision-imaging/) — a camera is what turns the dot into a distance.
- [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) — clean switching for pulsed / modulated operation.

## Where to buy

- Adafruit / SparkFun / Seeed for quality-binned, power-rated parts.
- AliExpress for bulk bags — be aware of over-spec'd "5 mW" modules that actually emit 20+ mW (illegal to sell in many jurisdictions).

## Software / libraries

- None. A single GPIO `set_high()` / `set_low()` or `ledc` PWM for modulation is all that's needed.

## Notes for WeftOS

- Expose as a boolean actuator in the HAL with a max-on-time watchdog (forced off after N seconds without re-arm).
- Any app that enables the laser should log an audit event; treat it like a "laser on" system privilege.
- Consider a physical key-switch or tamper interlock on enclosure panels for any product-grade deployment.
