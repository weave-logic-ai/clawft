---
title: "Triac AC Dimmer – RobotDyn-Style Phase-Cut Module"
slug: "triac-ac-dimmer"
category: "07-control-actuators"
part_numbers: ["RobotDyn AC Light Dimmer"]
manufacturer: "RobotDyn / various"
interface: ["GPIO"]
voltage: "110 / 230 VAC mains, 3.3–5V logic"
logic_level: "3.3V compatible"
current_ma: "2 A or 8 A load (module variant)"
price_usd_approx: "$5 – $15"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [triac, ac-dimmer, phase-cut, zero-cross, mains, actuator]
pairs_with:
  - "../07-control-actuators/ssr-relays.md"
  - "../07-control-actuators/relay-modules.md"
  - "../06-biometric-presence/ld2410-mmwave.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=dimmer" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=dimmer" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=dimmer" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-dimmer.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-robotdyn-ac-dimmer.html" }
datasheet: "https://robotdyn.com/"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

RobotDyn-style AC dimmer modules pair an **opto-isolated zero-cross detector**
with an **opto-triac output stage** to let a 3.3 V MCU phase-cut mains AC.
They come in 2 A and 8 A variants, for 110 V or 230 V regions. Unlike an SSR
(which is zero-cross only — on or off), a triac dimmer gives true brightness
control on incandescent bulbs, heating elements, and dimmable LEDs.

## Key specs

| Spec | Value |
|------|-------|
| Load voltage | 110 or 230 VAC |
| Load current | 2 A or 8 A (module variant) |
| Zero-cross output | Isolated digital pulse each half-cycle |
| Control input | 3.3 V or 5 V GPIO |
| Logic isolation | Opto-isolators both directions |

## Interface & wiring

Four logic pins: `VCC / GND / Z-C / PWM`. Hook `Z-C` (zero-cross) to an ESP32
GPIO with rising-edge interrupt and `PWM` (gate trigger) to another GPIO. On
each zero-crossing, start a timer; after a delay proportional to desired
brightness, pulse `PWM` HIGH briefly to fire the triac. The load stays on for
the rest of that half-cycle. The module's mains terminals are screw-down;
follow every mains-wiring safety rule you know.

## Benefits

- Real dimming (not PWM on/off) of dimmable AC loads.
- Opto-isolation keeps mains off your MCU board.
- Works for heaters, fans (universal motors), and filament bulbs.

## Limitations / gotchas

- **Not all AC loads dim.** Cheap LED bulbs, switch-mode supplies, and most
  fluorescent fixtures will flicker, buzz, or outright refuse.
- Phase-cut dimming emits EMI. Fit a ferrite bead and, ideally, a snubber.
- Timing jitter in the ESP32's GPIO ISR shows up as visible brightness
  flicker. Use the ESP32's hardware timer (`esp_timer` or `gptimer`) — don't
  try to use `delayMicroseconds()` in the ISR.
- Mains wiring is life-safety: enclosure, strain relief, fuse upstream, GFCI
  if it'll ever be near water.
- Universal motors (hair dryers, some fans) draw lots of inrush and create
  ugly commutation noise.

## Typical use cases

- Smart incandescent / halogen dimmers.
- PID-tuned heater elements (more granular than SSR zero-cross).
- Stage-light style bulb dimming for art installations.

## Pairs well with

- [`ssr-relays.md`](ssr-relays.md) — SSR for on/off safety interlock above
  the dimmer.
- [`relay-modules.md`](relay-modules.md) — backup hard-off.
- [`../06-biometric-presence/ld2410-mmwave.md`](../06-biometric-presence/ld2410-mmwave.md) —
  presence-driven soft-on / soft-off lighting.

## Where to buy

See `buy:`. RobotDyn is the canonical OEM; AliExpress has clones; some
European shops carry CE-marked variants.

## Software / libraries

- `RBDdimmer` (RobotDyn's Arduino library) handles the zero-cross + timer
  dance. ESP32 forks exist.
- For higher-level control, ESPHome `light: monochromatic` driven by a custom
  `output` component.

## Notes for WeftOS

Treat dimmer channels as floating-point 0.0–1.0 levels in rules; clamp ramp
rate so inrush on resistive loads doesn't trip breakers. Require the user to
explicitly opt a load type (`incandescent` / `dimmable_led` / `heater`)
because dimming behavior differs.
