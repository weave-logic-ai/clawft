---
title: "UV LED Arrays (365 nm / 395 nm)"
slug: "uv-led-arrays"
category: "04-light-display"
part_numbers: ["365nm UV LED", "395nm UV LED", "UV LED strip 5050"]
manufacturer: "various (Nichia, Seoul Viosys, LG Innotek, generic)"
interface: ["GPIO", "PWM"]
voltage: "5V / 12V (array dependent)"
logic_level: "3.3V (switched via MOSFET)"
current_ma: "20 mA per LED (single); 500 mA – 2+ A for arrays"
price_usd_approx: "$2 (single LED) – $40 (strip / panel)"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["uv", "365nm", "395nm", "illuminator", "fluorescence", "cure"]
pairs_with:
  - "../01-vision-imaging/guva-s12sd-uv.md"
  - "../07-control-actuators/mosfet-drivers.md"
  - "../07-control-actuators/relay-modules.md"
  - "../01-vision-imaging/"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=uv+led" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=uv+led" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=uv+led" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-uv+led.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-365nm-uv-led.html" }
libraries:
  - "none — GPIO + MOSFET + PWM; pair with GUVA-S12SD for closed loop"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

UV LED arrays in the 365 nm (UV-A "blacklight") and 395 nm ("near-UV") bands are cheap illuminators for fluorescence imaging, counterfeit detection, resin curing, minor disinfection (UV-A at 395 nm is **not** sterilizing, despite marketing), and plant growth / fluorescence monitoring. In WeftOS, they pair with the [GUVA-S12SD UV sensor](../01-vision-imaging/guva-s12sd-uv.md) for closed-loop intensity control and with [UV-capable cameras](../01-vision-imaging/) for fluorescence capture.

## Key specs

- Wavelength: 365 nm (true UV-A, more fluorescence pop, more expensive, more dangerous) vs 395 nm (near-UV, cheap, visibly purple).
- Per-die power: 20 mA forward current, ~3.4 V forward voltage typical.
- Array form factors: single 5 mm, 5050 SMD strips, 12 V modules, panel lamps.
- UV-A is partially filtered by glass and most plastics — use a UV-transparent window if illuminating through a barrier.
- 365 nm output from cheap "365 nm" LEDs is often actually 385–395 nm; measure with a spectrometer or reputable sensor if it matters.

## Interface & wiring

- **Do not** drive arrays from a GPIO directly. Use a low-side [MOSFET driver](../07-control-actuators/mosfet-drivers.md) (e.g., IRLZ44N / AO3400) with GPIO at gate and PWM for brightness.
- 5 V arrays: drive the FET from a GPIO with a pull-down; power LEDs from a separate regulated rail.
- 12 V strip arrays: ensure your PSU can handle inrush; 30 px/m of UV 5050 draws ~0.5 A per meter.
- Add a TVS diode across the LED supply if the module has long leads.

## Benefits

- Cheap way to surface fluorescent responses (plants, fungus, inks, resins).
- PWM brightness control gives continuous intensity control for closed-loop work with a UV sensor.
- Small form factor arrays fit into cameras as ring lights.

## Limitations / gotchas

- **Eye and skin hazard.** 365 nm UV-A exposure causes cataract and skin damage at even moderate doses. WeftOS product builds must include an interlock (proximity sensor + door switch) before enabling UV beyond low duty cycles.
- 395 nm LEDs are often sold as "365 nm" — if your application actually needs 365 nm (fluorescence spectra, passport IR/UV inks), buy from named vendors (Nichia, Seoul Viosys) and verify.
- UV-A is **not** a germicide — UV-C (254 nm) is, and UV-C hardware is a whole different category with real safety implications.
- UV degrades plastics and some sensors over months of exposure; keep UV off your own PCB silkscreen and lens coatings.
- Thermal management: high-power UV arrays need a heatsink; lifetime drops fast above ~85 °C junction.

## Typical use cases

- Plant fluorescence / chlorophyll imaging paired with a UV-sensitive [Pi camera](../01-vision-imaging/).
- Resin cure station (with interlocks).
- Counterfeit-detection kiosk (banknotes, ID inks).
- Aesthetic "mood" blacklight reacting to events.
- Hydroponic growth experiments with short UV pulses.

## Pairs well with

- [`../01-vision-imaging/guva-s12sd-uv.md`](../01-vision-imaging/guva-s12sd-uv.md) — closed-loop dose measurement.
- [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) — the mandatory switching element.
- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) — hard-off mains-level isolation for big panels.
- [`../01-vision-imaging/`](../01-vision-imaging/) — UV-capable or NoIR camera for fluorescence capture.

## Where to buy

- Adafruit (named-vendor 365 nm and 395 nm LEDs).
- DigiKey / Mouser for Nichia NCSU275T and Seoul Viosys.
- AliExpress for 5050 UV strips (almost always 395 nm regardless of label).

## Software / libraries

- No dedicated library — treat as a PWM-controlled load.
- Build a `UvIlluminator` HAL abstraction with dose integration (intensity × time) and a safety cap.

## Notes for WeftOS

- Always require an interlock event (door closed, user absent per [`../06-biometric-presence/`](../06-biometric-presence/)) before the HAL will enable UV.
- Log every UV-on event with duration and integrated dose; treat exceeding a policy-defined dose as a reportable safety event.
- Expose the UV illuminator as a *privileged actuator* that only signed apps can drive at > 5 % duty cycle.
