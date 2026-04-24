---
title: "Single RGB LEDs (common-anode / common-cathode, PWM)"
slug: "single-rgb-leds"
category: "04-light-display"
part_numbers: ["5 mm RGB LED", "PL9823 (self-addressable alt)"]
manufacturer: "various"
interface: ["GPIO", "PWM"]
voltage: "3.3V / 5V"
logic_level: "3.3V"
current_ma: "~20 mA per channel"
price_usd_approx: "$0.10 – $1"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["led", "rgb", "pwm", "status", "indicator"]
pairs_with:
  - "./ws2812b-neopixel.md"
  - "../07-control-actuators/mosfet-drivers.md"
  - "../11-wild-card/ky-series.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=rgb+led+5mm" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=rgb+led" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=rgb+led" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-rgb+led.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-5mm-rgb-led.html" }
libraries:
  - "esp-idf: ledc PWM driver"
  - "Rust: esp-hal LedcDriver"
  - "Arduino: analogWrite / ledcWrite"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A single 5 mm RGB LED gives three independent color channels driven over PWM. For WeftOS, these are the right answer when you want one well-placed status dot — the "power/state" indicator on a node enclosure, the "ready/error" light on a dev board, or the "mic open" bezel light on a voice terminal. They cost cents and draw under 60 mA.

## Key specs

- Common anode: pull each R/G/B cathode low (via PWM and a current-limit resistor) to illuminate.
- Common cathode: drive R/G/B high with PWM.
- Forward voltages differ per color: typical R ~2.0 V, G ~3.2 V, B ~3.2 V — size resistors per channel, not one shared.
- 20 mA per channel max; most ESP32 GPIOs can sink/source up to 40 mA but run at 10–15 mA in practice for lifetime.

## Interface & wiring

- Three PWM-capable GPIOs, one per color, each through its own current-limit resistor (R: ~150 Ω, G/B: ~100 Ω at 3.3 V; recalculate for your LED).
- Common-anode: tie shared pin to VCC, PWM pulls cathodes low (invert duty).
- Common-cathode: tie shared pin to GND, PWM drives anodes high (normal duty).
- Add a small (0.1 µF) cap near the LED if you see audible buzz from long PWM leads.

## Benefits

- Dirt cheap; no protocol, no timing, no driver IC.
- Easy to debug — if one channel is dead it's usually a wire.
- Each color on its own GPIO means you can hardware-gate one channel if needed.

## Limitations / gotchas

- Only one LED per three GPIOs — doesn't scale past a handful of indicators.
- Color mixing at low duty cycles looks stepped; use high PWM frequencies (>5 kHz) and gamma correction in software.
- Common-anode vs common-cathode is easy to get wrong and bricks nothing but is confusing to debug.
- Forward voltages drift with temperature; color balance shifts slightly over hours of run time.

## Typical use cases

- Single board-level status indicator.
- "Listening / thinking / speaking" light on a voice node.
- Pairing / provisioning feedback (blue = BLE advertising, green = connected, red = error).

## Pairs well with

- [`./ws2812b-neopixel.md`](./ws2812b-neopixel.md) — use NeoPixels when you need more than a few indicators.
- [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) — only if you're driving a high-power RGB floodlight rather than a 5 mm through-hole part.
- [`../11-wild-card/ky-series.md`](../11-wild-card/ky-series.md) — many KY modules bundle an RGB LED on a breakout with resistors already fitted.

## Where to buy

- Adafruit / SparkFun (quality binned parts, clear polarity markings).
- AliExpress bulk bags (100 for a few dollars; color balance varies).

## Software / libraries

- ESP-IDF `ledc` with 3 channels; 8-bit or 10-bit resolution is fine.
- Always apply a gamma LUT (e.g., `x^2.2`) before writing PWM duty, or the low end will look banded.

## Notes for WeftOS

- Prefer `ws2812b` for any indicator count > 3; single RGB LEDs are only justifiable when BOM / certifiable simplicity matters.
- Expose in the HAL as a 1-pixel "surface" with the same API as the NeoPixel driver — swap without app-code changes.
