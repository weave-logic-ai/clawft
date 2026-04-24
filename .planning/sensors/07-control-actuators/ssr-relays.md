---
title: "Solid-State Relays (SSR)"
slug: "ssr-relays"
category: "07-control-actuators"
part_numbers: ["G3MB-202P", "SSR-25DA", "SSR-40DA"]
manufacturer: "Omron / Fotek / various"
interface: ["GPIO"]
voltage: "3–32 VDC control, switching 24–480 VAC"
logic_level: "3.3V-compatible on most hobby boards"
current_ma: "~10–20 mA control current"
price_usd_approx: "$5 – $25"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [ssr, relay, ac, zero-cross, actuator]
pairs_with:
  - "../07-control-actuators/relay-modules.md"
  - "../07-control-actuators/triac-ac-dimmer.md"
  - "../05-environmental/mq-series.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=solid+state+relay" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=ssr" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=solid+state+relay" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-solid+state+relay.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-SSR-solid-state-relay.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A solid-state relay uses an optocoupler + triac (for AC) or MOSFET (for DC) to
switch a load silently and with no mechanical wear. For WeftOS projects they're
the right answer whenever you need **silent switching, long cycle life, or
relatively fast duty-cycle control** of an AC load (heaters, kettles, fermenters,
fans). Zero-cross variants (`-ZA` suffix) only switch at the mains zero-crossing,
reducing EMI.

## Key specs

| Spec | Value |
|------|-------|
| Control input | 3–32 VDC, typ. 10–20 mA |
| Load rating | 24–480 VAC, 25 A / 40 A common Fotek-style |
| Zero-cross | Some variants; others are random-fire |
| Isolation | Optocoupler, ~2.5 kV |
| Switching speed | ~10 ms (bounded by zero-cross) |

## Interface & wiring

Hobby SSR **bricks** (Fotek-style) have screw terminals on both sides: `+/-`
for the DC control input, `L1/L2` for the AC load. Wire an ESP32 GPIO through
a small current-limiting resistor (typically 330 Ω–1 kΩ) to the `+` input.
**Bolt the brick to a heatsink** if you're above ~5 A continuous — the internal
triac dissipates about 1 W per amp.

## Benefits

- Silent — no click, no coil buzz.
- Mechanical lifetime essentially unlimited; ideal for PWM-style duty-cycling.
- Zero-cross variants emit far less EMI than mechanical relays or triac
  dimmers.

## Limitations / gotchas

- **Heat.** A 40 A Fotek brick without a heatsink cooks itself at 10 A. Budget
  thermal design.
- AC only for most hobby bricks; DC-SSRs exist but are less common and often
  mis-labelled on AliExpress.
- Zero-cross units **cannot** do phase-cut dimming — use a [triac dimmer](triac-ac-dimmer.md)
  for that.
- A fair number of Fotek-branded units on AliExpress are **counterfeit** and
  rated far below the label. For anything behind a wall, buy name-brand.

## Typical use cases

- Heater / kettle PID controllers (slow PWM, 1–10 s period).
- Server / appliance remote power switch.
- Fermentation / brewing temperature control.

## Pairs well with

- [`relay-modules.md`](relay-modules.md) — mechanical relays for hard-off
  safety interlock upstream of the SSR.
- [`triac-ac-dimmer.md`](triac-ac-dimmer.md) — when you need dimming instead
  of on/off.
- [`../05-environmental/mq-series.md`](../05-environmental/mq-series.md) —
  SSR-driven heater pair with a gas sensor for hot-loop experiments.

## Where to buy

See `buy:`. For anything safety-critical, source from Omron / Crouzet /
Carlo Gavazzi rather than AliExpress.

## Software / libraries

GPIO. Use a slow software PWM (1–10 s period) for thermal loads.

## Notes for WeftOS

WeftOS should enforce a **max duty cycle** policy on any SSR-driven heater and
wire it to a temperature-sensor interlock: if the sensor disappears, the SSR
must fail off.
