---
title: "Rotary Encoders (KY-040 / EC11)"
slug: "rotary-encoders"
category: "09-input-hmi"
part_numbers: ["KY-040", "EC11", "EC12"]
manufacturer: "ALPS / generic"
interface: ["GPIO", "PCNT"]
voltage: "3.3V / 5V"
logic_level: "3.3V (KY-040 onboard pull-ups are to VCC)"
current_ma: "< 1 mA"
price_usd_approx: "$1 – $3"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [input, encoder, quadrature, knob, ky-040, ec11, pcnt]
pairs_with:
  - "./momentary-buttons.md"
  - "../04-light-display/ssd1306-oled.md"
  - "../07-control-actuators/hobby-servos.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=rotary+encoder" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=rotary+encoder" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=rotary+encoder" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-rotary%20encoder.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-ky-040.html" }
datasheet: ""
libraries:
  - { name: "ESP32Encoder",     url: "https://github.com/madhephaestus/ESP32Encoder" }
  - { name: "AiEsp32RotaryEncoder", url: "https://github.com/igorantolic/ai-esp32-rotary-encoder" }
  - { name: "ESPHome rotary_encoder", url: "https://esphome.io/components/sensor/rotary_encoder.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

An incremental quadrature rotary encoder produces two 90°-phased pulse trains
(A and B) as the shaft rotates — their lead/lag tells you direction, their edges
count position. Most hobby encoders (KY-040, EC11) also bundle a push-button on
the shaft. They are **not** absolute encoders; power-cycle forgets position.

## Key specs

| Spec | KY-040 / EC11 typical |
|------|----------------------|
| Detents per revolution | 20 or 30 |
| Pulses per detent | 1, 2, or 4 (varies) |
| Shaft push-button | Yes, common |
| Bounce | Worse than a tact switch |
| Life | ~30 k – 100 k cycles |

## Interface & wiring

- 5 pins: GND, `+` (Vcc), SW (push), DT (B), CLK (A).
- KY-040 has 10 kΩ pull-ups to Vcc on the signals — tie `+` to 3.3 V, not 5 V,
  so the ESP32 sees 3.3 V logic.
- Use ESP32's **PCNT** (pulse counter) peripheral via `ESP32Encoder` for
  hardware-debounced, interrupt-free decoding. Much better than polling.
- If PCNT isn't available, attach interrupts on both A and B and decode with a
  4-state lookup table. Add a 10 nF cap from A→GND and B→GND to tame contact
  bounce.

## Benefits

- Infinite rotation (no end stops).
- Tactile detent per count — great for menu scroll.
- Push-button gives "select" for free.
- PCNT peripheral makes them essentially free CPU-wise.

## Limitations / gotchas

- Bouncy contacts — without hardware debounce (RC) or PCNT, you will count
  double/ghost steps.
- Some cheap KY-040 boards ship with bad resistor values — if every other click
  is missed, replace pull-ups with 4.7 kΩ.
- The push-button is a plain tact switch and needs its own debounce.
- Not absolute — store position in NVS if you need it across reboots.

## Typical use cases

- Menu scroll + click on OLED/TFT UIs.
- Tuning knob for a DSP / synth.
- Gimbal / camera pan control.
- Volume control for I²S audio DAC.

## Pairs well with

- [`./momentary-buttons.md`](./momentary-buttons.md) — add more discrete buttons.
- [`../04-light-display/ssd1306-oled.md`](../04-light-display/ssd1306-oled.md) — rotate-to-scroll menus.
- [`../07-control-actuators/hobby-servos.md`](../07-control-actuators/hobby-servos.md) — jog-wheel for servos.

## Where to buy

KY-040 clones are ubiquitous on AliExpress at ~$0.50 each. Genuine ALPS EC11
from Mouser/Digi-Key for long-life panel-mount builds.

## Software / libraries

- `ESP32Encoder` — wraps PCNT, zero interrupts in user code.
- `AiEsp32RotaryEncoder` — higher-level, handles long-press, acceleration.
- ESPHome `sensor: platform: rotary_encoder` — drop-in for Home Assistant.

## Notes for WeftOS

Model as a **CounterSource** (signed int) with optional derivative (rad/s) and
a **DiscreteEventSource** for the push button. PCNT backend means the substrate
can sample at 100 Hz without jitter.
