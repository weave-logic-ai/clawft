---
title: "LED Bar Graphs (10-segment bar arrays)"
slug: "led-bar-graphs"
category: "04-light-display"
part_numbers: ["HDSP-4836", "5620AS / 5620BS", "generic 10-seg bar"]
manufacturer: "various (Kingbright, Lite-On)"
interface: ["GPIO", "PWM"]
voltage: "3.3V / 5V"
logic_level: "3.3V"
current_ma: "~10 – 20 mA per segment"
price_usd_approx: "$0.50 – $3 (bare) / $3 – $8 (on driver board)"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["led", "bar-graph", "vu-meter", "analog-indicator"]
pairs_with:
  - "./ws2812b-neopixel.md"
  - "./tm1637-7seg.md"
  - "../07-control-actuators/mosfet-drivers.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=bar+graph" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=bar+graph" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=bargraph" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-bar+graph.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-10-segment-led-bar.html" }
libraries:
  - "shift register: 74HC595 + any shift-register crate"
  - "TLC5940 / HT16K33 drivers for PWM per-segment"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A 10-segment LED bar graph is a discrete array of 10 rectangular LEDs in a DIP-20 package, often found in green / yellow / red progressions. They are the archetypal "analog" indicator — VU meters, fuel gauges, mic level, signal strength. WeftOS uses them where a single scalar needs to be visible from far away without any framework overhead.

## Key specs

- 10 independent LEDs in a single DIP-20 (5 anode + 5 cathode, or 10+10 common).
- Forward voltage 2.0–2.1 V (red/yellow/green), 3.0+ V (blue).
- Per-segment current: 10–20 mA; resistor per segment required.
- Often bundled on a breakout with an LM3914 (linear) or LM3915 (log / dB) driver for a pure-analog input.

## Interface & wiring

- Bare package: 10 resistors + either 10 GPIOs, a shift register (74HC595) on SPI, or a PWM LED driver (TLC5940, HT16K33).
- With LM3914/LM3915: analog voltage in, bar/dot mode selectable via pin, sets brightness with one resistor.
- From an ESP32 the shift-register route is clean: 3 pins for any number of bars.

## Benefits

- Cheapest possible "analog-looking" meter.
- Readable from many feet away — great for loud environments.
- No framework needed: a shift register and a scalar-to-level function.

## Limitations / gotchas

- Only 10 steps of resolution unless you PWM each segment.
- Common-anode vs common-cathode vs split packages is easy to get wrong — check datasheet by part number.
- LM3914/3915 builds react in ~1 ms; don't drive them from a noisy analog line without filtering.
- Non-addressable — no animations beyond solid / blink / fade.

## Typical use cases

- Microphone / audio level meter.
- CPU / memory / network utilization on a dev node.
- Battery remaining indicator on a portable WeftOS device.
- "Too loud" warning in a room.

## Pairs well with

- [`./ws2812b-neopixel.md`](./ws2812b-neopixel.md) — RGB alternative when you want color-coded zones.
- [`./tm1637-7seg.md`](./tm1637-7seg.md) — pair the bar with a numeric readout.
- [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) — only needed for high-brightness outdoor bars.

## Where to buy

- SparkFun / Adafruit for named Kingbright parts with full datasheets.
- AliExpress for cheap 10-seg bars and LM3915 kits.

## Software / libraries

- No dedicated driver needed — any shift-register or PWM library works.
- HT16K33 backpacks (Adafruit) give I²C-addressable brightness per segment.

## Notes for WeftOS

- Model as a `scalar indicator` in the HAL: input 0.0–1.0 float; the HAL maps to segments, applies gamma, and clamps to a brightness policy.
- Log-scale (LM3915-style) is usually more useful than linear for audio or signal strength — expose both modes.
