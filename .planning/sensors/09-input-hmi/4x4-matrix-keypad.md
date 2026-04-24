---
title: "4×4 Matrix Keypad"
slug: "4x4-matrix-keypad"
category: "09-input-hmi"
part_numbers: ["4x4 membrane keypad", "3x4 membrane keypad"]
manufacturer: "Generic"
interface: ["GPIO", "I2C (via PCF8574 expander)"]
voltage: "3.3V"
logic_level: "3.3V"
current_ma: "< 1 mA"
price_usd_approx: "$1 – $5"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [input, keypad, matrix, membrane, gpio-expander]
pairs_with:
  - "./momentary-buttons.md"
  - "../08-communication/tca9548a-i2c-mux.md"
  - "../04-light-display/ssd1306-oled.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=membrane+keypad" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=keypad" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=keypad" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-keypad.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-4x4-membrane-keypad.html" }
datasheet: ""
libraries:
  - { name: "Keypad (Arduino)",            url: "https://github.com/Chris--A/Keypad" }
  - { name: "Keypad_I2C (PCF8574)",        url: "https://github.com/joeyoung/arduino_keypads" }
  - { name: "ESPHome matrix_keypad",       url: "https://esphome.io/components/matrix_keypad.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A 4×4 membrane keypad is 16 momentary switches wired as a grid: 4 row lines and
4 column lines. Firmware drives one line at a time and reads the others to find
the pressed key. Ubiquitous on door-lock projects, DIY calculators, safes, and
DTMF-style numeric entry. 3×4 (phone layout) and 4×4 (hex) variants exist.

## Key specs

| Spec | Typical |
|------|---------|
| Keys | 16 (4×4) or 12 (3×4) |
| Pins needed | 8 (rows + cols) on bare matrix |
| Key force | ~2 N |
| Contact bounce | 5–20 ms |
| Life | 1M actuations (membrane) |

## Interface & wiring

- 8 pins for a bare 4×4: R0..R3, C0..C3.
- Firmware loop: set one column LOW, set rows as `INPUT_PULLUP`, read rows; the
  LOW row identifies the pressed key. Rotate column; repeat.
- Ghosting (two keys pressed appearing as a third) is real for 3+
  simultaneous presses — add a diode per key if you need true N-key rollover.
- **GPIO budget saver:** offload the 8 lines to a PCF8574 (I²C, 8 pins) or
  MCP23017 (I²C, 16 pins). The `Keypad_I2C` library handles the bit-banging.
  See [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md)
  for bus multiplexing when you have multiple I²C devices competing for addresses.

## Benefits

- Cheap and familiar user input.
- Well-supported by every Arduino / ESPHome stack.
- With a PCF8574, only 2 wires (SDA + SCL) — huge GPIO win.

## Limitations / gotchas

- Eats 8 GPIOs unless you use an expander.
- No tactile feedback on cheap membrane keypads (mushy).
- Debounce still required — the `Keypad` library does it for you.
- Cable length > 30 cm can pick up noise; shield or add RC filters per row.
- Pin order on generic ribbon connectors is inconsistent — probe with a
  multimeter before trusting the silkscreen.

## Typical use cases

- PIN entry for door lock or safe.
- DTMF dialer / telephone prop.
- Menu + numeric config on a headless ESP32 node.
- Macro pad for streaming / productivity.

## Pairs well with

- [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md) — I²C expansion via PCF8574.
- [`../04-light-display/ssd1306-oled.md`](../04-light-display/ssd1306-oled.md) — text feedback for PIN entry.
- [`./momentary-buttons.md`](./momentary-buttons.md) — extra modifier keys.

## Where to buy

AliExpress 4×4 membrane keypads ~$1. Adafruit sells nicer rigid-PCB versions.

## Software / libraries

- `Keypad` (Arduino) — de-facto standard, scan + debounce.
- `Keypad_I2C` — wraps `Keypad` to scan through a PCF8574.
- ESPHome `matrix_keypad` component — drop-in.

## Notes for WeftOS

Model as a **DiscreteEventSource** with a symbol alphabet of 16 (or 12) keys.
If behind a PCF8574, the sensor's transport metadata should record the expander
address so the event pipeline can detect bus contention.
