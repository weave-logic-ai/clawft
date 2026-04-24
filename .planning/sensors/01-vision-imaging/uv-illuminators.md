---
title: "UV Illuminators (365 nm / 395 nm)"
slug: "uv-illuminators"
category: "01-vision-imaging"
part_numbers: ["UV LED 365nm", "UV LED 395nm", "UV LED strip", "UV torch"]
manufacturer: "various"
interface: ["GPIO", "PWM", "MOSFET"]
voltage: "3.3V – 12V"
logic_level: "3.3V logic, MOSFET switched"
current_ma: "100 mA – 2 A"
price_usd_approx: "$3 – $30"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [uv, illuminator, 365nm, 395nm, fluorescence, blacklight]
pairs_with:
  - "./guva-s12sd-uv.md"
  - "./uv-cameras.md"
  - "../07-control-actuators/mosfet-drivers.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=UV+LED" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=UV+LED" }
  - { vendor: DigiKey, url: "https://www.digikey.com/en/products/search?keywords=365nm%20UV%20LED" }
  - { vendor: Mouser, url: "https://www.mouser.com/c/?q=365nm%20UV%20LED" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-365nm-uv-led.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

UV illuminators come in two wavelengths that matter for hobby / ESP32 work: 395 nm (cheap, visible
violet, fine for casual "blacklight" effects and pet-urine detection) and 365 nm (more expensive,
much cleaner UV-A, the one you actually want for forensic and mineral fluorescence). Both pair
with a GUVA-S12SD for closed-loop safety.

## Key specs

| Wavelength | Cost | Visible light | Good for |
|---|---|---|---|
| 395 nm | Cheap (pennies per LED) | Noticeable violet bleed | Blacklight party effects, currency check, cheap fluorescence |
| 365 nm | 5–20× more | Almost none | Forensics, minerals, biology, serious fluorescence |

## Interface & wiring

Same pattern as IR illuminators: logic-level MOSFET on the low side, current-limit resistors per
LED, separate power rail. Use PWM for intensity control; keep PWM > 1 kHz to avoid visible
flicker in camera capture.

## Benefits

- Cheap and easy to drive.
- 395 nm is good enough for education / novelty builds.
- 365 nm gives real fluorescence imaging when combined with a decent camera.

## Limitations / gotchas

- **Eye / skin safety:** UV-A is still UV. Never stare into a 365 nm source; wear UV-blocking
  safety glasses for 1 W+ builds.
- 395 nm LEDs marketed as "UV" have huge visible-violet bleed that washes out fluorescence —
  add a UV-pass / visible-block filter in front of the camera for good images.
- UV degrades plastics and adhesives over time; don't point it at your own enclosure.

## Typical use cases

- Fluorescence macro photography.
- Counterfeit currency / ID inspection.
- Mineral / pet-stain detection.
- UV-cure resin build plates.

## Pairs well with

- [`./guva-s12sd-uv.md`](./guva-s12sd-uv.md) for intensity / on-off confirmation.
- [`./uv-cameras.md`](./uv-cameras.md) for UV-pass imaging.
- [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) — necessary switch.

## Where to buy

- Adafruit / SparkFun / DigiKey / Mouser for named LEDs.
- AliExpress for strips and pre-built torches — verify actual wavelength, not just "UV".

## Software / libraries

- ESP-IDF LEDC / Arduino `ledcWrite` — plain PWM.

## Notes for WeftOS

Speculative: a UV illuminator node should default to **disarmed** with an explicit "consent"
hook before turning on — eye-safety is the kind of invariant WeftOS's policy layer should own,
not each app.
