---
title: "Relay Modules (1 / 2 / 4 / 8 / 16-Channel, 3.3V & 5V)"
slug: "relay-modules"
category: "07-control-actuators"
part_numbers: ["SRD-05VDC-SL-C", "SRD-03VDC-SL-C", "JQC-3FF"]
manufacturer: "Songle / Omron / various module vendors"
interface: ["GPIO"]
voltage: "3.3V or 5V coil rails"
logic_level: "3.3V or 5V (board-specific; watch the jumper)"
current_ma: "~70 mA per energized relay coil"
price_usd_approx: "$2 (1-ch) – $15 (16-ch)"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [relay, actuator, ac, dc, optocoupler, isolation]
pairs_with:
  - "../06-biometric-presence/pir-hc-sr501.md"
  - "../07-control-actuators/ssr-relays.md"
  - "../10-storage-timing-power/index.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=relay+module" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=relay" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=relay" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-relay.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-relay-module-esp32.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

Hobby relay modules are boards that carry N mechanical relays (typically Songle
SRD-series), each with an optocoupler on the input and a flyback diode across
the coil. They come in 1 / 2 / 4 / 8 / 16-channel flavors, with **5 V coil**
(the common case) or **3.3 V coil** (ESP32-friendly but less common). Many 5 V
boards also route optocoupler power through a `JD-VCC` jumper so you can keep
the coil rail fully isolated from the MCU rail.

## Key specs

| Spec | Value |
|------|-------|
| Channels | 1, 2, 4, 8, 16 |
| Contact rating | typ. 10 A @ 250 VAC / 10 A @ 30 VDC |
| Coil voltage | 3.3 V or 5 V |
| Coil current | ~70 mA per energized relay |
| Isolation | Optocoupler (typ. PC817) between input and coil |
| Switching | Mechanical, ~10 ms, audible click |

## Interface & wiring

One GPIO per channel. **Active-low** on most boards — pulling the input LOW
energizes the coil. Remove the `VCC↔JD-VCC` jumper and feed `JD-VCC` from a
separate 5 V supply if you want galvanic isolation between MCU and coil rail;
otherwise leave it jumpered and supply 5 V to the board.

## Benefits

- Galvanic isolation: mains side and MCU side don't share a ground path.
- High current / voltage handling in a sub-$5 board.
- Works for both AC and DC loads (read the contact rating).
- Visible LED per channel simplifies debugging.

## Limitations / gotchas

- **5 V coil + 3.3 V GPIO:** many "5 V" boards still switch reliably from
  ESP32's 3.3 V via the optocoupler, but not all. Confirm with a multimeter
  on the input or buy an explicitly 3.3 V board.
- Mechanical relays click audibly and wear out at ~100 k cycles — use an SSR
  if you're PWM-dimming or cycling many times per minute.
- Inrush on AC loads can weld contacts — pick a relay rated above your
  steady-state current, not below.
- Never route mains across a breadboard. Use a proper enclosure, strain relief,
  and certified wiring.

## Typical use cases

- Smart lamp / fan controller (one channel per device).
- Garage-door / garden-pump controller.
- Power-cycle bank for a rack of ESP32 devices that occasionally need a kick.

## Pairs well with

- [`ssr-relays.md`](ssr-relays.md) — quieter / faster AC switching alternative.
- [`../06-biometric-presence/pir-hc-sr501.md`](../06-biometric-presence/pir-hc-sr501.md) —
  motion → light automations.
- [`../10-storage-timing-power/index.md`](../10-storage-timing-power/index.md) —
  pair with an INA219/INA226 to measure the switched load.

## Where to buy

See `buy:`. SainSmart-style 4-channel and 8-channel boards are the canonical
pick; pay a bit more for name-brand relays if you're switching real mains.

## Software / libraries

No driver — it's a GPIO. ESPHome's `switch: gpio` / `relay` platform wires
straight to Home Assistant.

## Notes for WeftOS

Relays are the primary actuator abstraction in WeftOS rules (`turn_on(load)`).
Keep per-channel switch-cycle counters in local memory to warn before contacts
wear out. Never expose raw channel GPIO to cloud rules — always wrap in a
named logical switch with interlock / rate-limit policy.
