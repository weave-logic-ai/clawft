---
title: "Momentary Buttons / Tact Switches"
slug: "momentary-buttons"
category: "09-input-hmi"
part_numbers: ["6x6mm tact", "12x12mm tact", "button board (generic)"]
manufacturer: "Generic / Omron / C&K"
interface: ["GPIO"]
voltage: "3.3V"
logic_level: "3.3V"
current_ma: "<1 mA (pull-up leakage)"
price_usd_approx: "$0.05 – $1 each"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [input, button, switch, gpio, debounce]
pairs_with:
  - "./rotary-encoders.md"
  - "./4x4-matrix-keypad.md"
  - "../04-light-display/ssd1306-oled.md"
  - "../08-communication/tca9548a-i2c-mux.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=tactile+button" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=tactile+switch" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=button" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-button.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-tact-switch-6x6.html" }
datasheet: ""
libraries:
  - { name: "Bounce2",            url: "https://github.com/thomasfredericks/Bounce2" }
  - { name: "OneButton",          url: "https://github.com/mathertel/OneButton" }
  - { name: "ESPHome binary_sensor (gpio)", url: "https://esphome.io/components/binary_sensor/gpio.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The workhorse momentary switch: press closes the contact, release opens it. Comes
as a bare 6×6 mm or 12×12 mm tact switch, or pre-soldered on a "button module"
board with a pull-up/down resistor and sometimes an LED. Perfect for wake pins,
menu navigation, emergency stops, and any other discrete "did a human push it?"
signal.

## Key specs

| Spec | Typical |
|------|---------|
| Actuation force | 1–3 N |
| Contact resistance | < 100 mΩ new |
| Bounce time | 1–20 ms |
| Life | 100 k – 10 M cycles |
| Contact rating | 50 mA @ 12 VDC (logic use only) |

## Interface & wiring

- Wire one side to GPIO, the other to GND; enable ESP32 internal pull-up
  (`INPUT_PULLUP`). Button closed = logic LOW.
- If you need a "press = HIGH" signal, use an external pull-down and wire the
  other side to 3.3 V.
- For long cable runs, add a 10 kΩ series resistor + 10 nF cap to ground (RC
  low-pass) to suppress EMI-induced false triggers.
- Avoid ESP32 strapping pins (GPIO 0, 2, 12, 15) for boot-critical behavior.

## Benefits

- Cheapest input available; single GPIO per button.
- Zero driver software — it's a wire.
- Supports wake-from-deep-sleep via `esp_sleep_enable_ext0_wakeup`.

## Limitations / gotchas

- **Always debounce.** 5–20 ms of bounce is normal. Software debounce with
  `Bounce2` or similar; hardware debounce with an RC + Schmitt if the MCU is
  too busy.
- Pre-built "button modules" sometimes have a pull-up **and** pull-down, creating
  a voltage divider — read the schematic before trusting the logic level.
- Long-press, double-tap, and release-edge need explicit state-machine logic —
  use `OneButton` to avoid rolling your own.

## Typical use cases

- Boot-mode / reset / user button on dev boards.
- Menu up/down/enter on an OLED UI.
- Manual override for automated actuators (garage door, light).
- Single-switch scanning input for accessibility.

## Pairs well with

- [`./rotary-encoders.md`](./rotary-encoders.md) — press + twist combos.
- [`../04-light-display/ssd1306-oled.md`](../04-light-display/ssd1306-oled.md) — simple menu UI.
- [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md) — via PCF8574 expander for many buttons.

## Where to buy

See `buy:` list above — every hobby electronics store carries them; buy tact
switches in 100-packs on AliExpress for pennies each.

## Software / libraries

- `Bounce2` — minimal, fast, timer-based debouncer.
- `OneButton` — click / double-click / long-press events from a single GPIO.
- ESPHome `binary_sensor: platform: gpio` with `filters: - delayed_on: 50ms`.

## Notes for WeftOS

In the WeftOS sensor graph, a momentary button is a **DiscreteEventSource** with
optional debounce filter and long-press virtualizer. Emits `press`, `release`,
`click`, `long_press` events. Good candidate for the "hello world" of the event
pipeline.
