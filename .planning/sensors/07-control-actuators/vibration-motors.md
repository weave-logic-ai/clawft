---
title: "Vibration Motors – Coin / ERM & LRA Haptics"
slug: "vibration-motors"
category: "07-control-actuators"
part_numbers: ["10mm coin ERM", "LRA 1020", "DRV2605L"]
manufacturer: "Precision Microdrives / Texas Instruments / various"
interface: ["PWM", "GPIO"]
voltage: "3–5V typ"
logic_level: "MOSFET-driven or DRV2605L over I²C"
current_ma: "30–100 mA"
price_usd_approx: "$1 – $10"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [haptic, vibration, erm, lra, wearable, ui]
pairs_with:
  - "../07-control-actuators/mosfet-drivers.md"
  - "../06-biometric-presence/max30102-max30105.md"
  - "../04-light-display/ws2812b-neopixel.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=vibration+motor" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=vibration+motor" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=vibration+motor" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-vibration+motor.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-vibration-motor-coin.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

Hobby vibration motors come in two flavors: **ERM** (eccentric rotating mass —
the classic pager-motor coin) and **LRA** (linear resonant actuator — a
spring-mounted mass driven at a specific resonant frequency). ERMs give you a
cheap "buzz on / buzz off" channel; LRAs give you crisp, configurable haptic
clicks that feel like a smartphone, but need an AC drive signal (TI's
DRV2605L is the go-to driver IC).

## Key specs

| Variant | Drive | Voltage | Current | Feel |
|---------|-------|---------|---------|------|
| ERM 10 mm coin | DC + PWM | 3 V typ | 60–100 mA | Dull buzz, slow ramp |
| ERM cylinder | DC + PWM | 3–5 V | 30–80 mA | Same but punchier |
| LRA 1020 | AC at ~170 Hz (via DRV2605L) | 2 Vrms | ~50 mA | Crisp click |

## Interface & wiring

**ERM:** logic-level N-channel MOSFET between motor and ground, flyback diode
across the motor, PWM from an ESP32 LEDC pin. That's it.

**LRA:** you cannot drive an LRA directly from a GPIO or a plain MOSFET — it
needs an alternating signal at its resonant frequency. Use TI's `DRV2605L`
haptic driver (I²C control, integrated library of effects). The LRA solders
to the DRV2605L's output pins.

Always use a flyback diode on ERMs; they're as inductive as any small DC
motor.

## Benefits

- Tiny, cheap, work fine from a LiPo / 3 V button cell.
- DRV2605L has a built-in effect library (clicks, buzzes, ramps, textures).
- LRAs feel far more "premium" than ERMs for the same energy.

## Limitations / gotchas

- **ERM ramp time** is tens of milliseconds — you can't "click" with an ERM.
- **LRA needs its driver.** Hooking an LRA directly to a MOSFET is a
  common-and-wrong mistake; it barely moves and wastes power.
- Mount surface matters. A coin motor loose in an enclosure is quiet; bolted
  to a rigid panel, it's loud.
- Battery drain is real for always-on haptics — gate power between pulses.

## Typical use cases

- Wearable notifications (HR alert, navigation).
- Accessibility UI feedback on an IoT dashboard.
- Alarm clocks and silent pagers.

## Pairs well with

- [`mosfet-drivers.md`](mosfet-drivers.md) — the driver for ERMs.
- [`../06-biometric-presence/max30102-max30105.md`](../06-biometric-presence/max30102-max30105.md) —
  haptic HR / breathing feedback.
- [`../04-light-display/ws2812b-neopixel.md`](../04-light-display/ws2812b-neopixel.md) —
  combine haptic and visual feedback.

## Where to buy

See `buy:`. Precision Microdrives (via Digikey) sells the high-end LRAs;
Adafruit stocks a DRV2605L breakout + LRA combo; AliExpress is fine for ERM
coin motors.

## Software / libraries

- `Adafruit_DRV2605` — I²C driver with effect library selection.
- No driver for bare ERMs beyond PWM.

## Notes for WeftOS

Haptic channels should be rate-limited in rules — nobody wants a notification
cascade to buzz their wrist 50 times. Treat the haptic subsystem as a
one-signal-per-event abstraction.
