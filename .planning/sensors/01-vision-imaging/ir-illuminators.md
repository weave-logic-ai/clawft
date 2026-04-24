---
title: "IR Illuminators (850 nm / 940 nm)"
slug: "ir-illuminators"
category: "01-vision-imaging"
part_numbers: ["850nm IR LED", "940nm IR LED", "IR LED array board"]
manufacturer: "various"
interface: ["GPIO", "PWM", "MOSFET"]
voltage: "3.3V – 12V (LED-dependent)"
logic_level: "3.3V logic, high current via MOSFET"
current_ma: "50 mA – 1 A+ per array"
price_usd_approx: "$2 – $15"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [ir, illuminator, led, 850nm, 940nm, night-vision]
pairs_with:
  - "./arducam-noir.md"
  - "./esp32-cam-ov2640.md"
  - "../07-control-actuators/mosfet-drivers.md"
  - "../04-light-display/ws2812b-neopixel.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=IR+LED" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=IR+LED" }
  - { vendor: DigiKey, url: "https://www.digikey.com/en/products/search?keywords=850nm%20IR%20LED" }
  - { vendor: Mouser, url: "https://www.mouser.com/c/?q=850nm%20IR%20LED" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-850nm-ir-led-board.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

IR illuminators are arrays of 850 nm or 940 nm LEDs that flood a scene with IR so an NoIR camera
can see in the dark. 850 nm is brighter-effective (visible as a faint red glow) and 940 nm is
truly covert (invisible to humans, but typical CMOS sensitivity is roughly halved).

## Key specs

| Wavelength | Visible? | Camera sensitivity (OV2640) | Typical module |
|---|---|---|---|
| 850 nm | Faint red glow | ~70% of peak | 3–12 LED arrays, ~100–500 mA |
| 940 nm | Invisible | ~30–40% of peak | Same arrays or IR-cam security-grade boards |

## Interface & wiring

You do *not* drive an IR LED array directly from a GPIO. Use:

- A logic-level N-channel MOSFET (e.g. AO3400, IRLZ44N) on the low side.
- A current-limit resistor or constant-current driver — data sheet Vf is 1.2–1.5 V per LED.
- Separate power rail for the array; don't fight a camera on the same 3.3 V.
- Optional PWM for dimming / sync-with-frame; keep PWM > 200 Hz to avoid rolling-shutter banding.

## Benefits

- Completely transforms NoIR cameras' usable range in the dark.
- Cheap LEDs and drivers are everywhere.
- 940 nm variants give you covert surveillance without visible glow.

## Limitations / gotchas

- IR LEDs dissipate real heat — long runtime needs a heatsink.
- 940 nm arrays need ~2× the drive current of 850 nm for equivalent image brightness.
- Reflections from the case / lens housing cause washed-out images — keep the illuminator off-axis or shrouded.
- Eye-safety: high-current IR is not felt as heat or light — don't stare into a 1 A array up close.

## Typical use cases

- Night-vision security cam (NoIR + 850 nm).
- Covert doorway watcher (940 nm).
- Pulsed IR flash for per-frame "with / without" differential imaging.

## Pairs well with

- [`./arducam-noir.md`](./arducam-noir.md) — the primary sensor.
- [`./esp32-cam-ov2640.md`](./esp32-cam-ov2640.md) host board.
- [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) — must-have switch.
- [`../04-light-display/ws2812b-neopixel.md`](../04-light-display/ws2812b-neopixel.md) — visible status indicator.

## Where to buy

- Adafruit / SparkFun / DigiKey / Mouser for named LEDs.
- AliExpress for pre-built 12-LED security-cam-style boards.

## Software / libraries

- ESP-IDF LEDC (PWM) or Arduino `ledcWrite` — that's the whole driver.

## Notes for WeftOS

Speculative: an IR illuminator is an "active-lighting effect" tied to a camera surface — WeftOS
could model it as a co-scheduled actuator that asserts/deasserts relative to frame capture
(global-shutter cameras) or simply stays on (rolling-shutter).
