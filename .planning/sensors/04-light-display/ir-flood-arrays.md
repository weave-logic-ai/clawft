---
title: "IR Flood Arrays (850 nm / 940 nm)"
slug: "ir-flood-arrays"
category: "04-light-display"
part_numbers: ["850nm IR LED", "940nm IR LED", "IR illuminator ring"]
manufacturer: "various (Osram, Vishay, generic)"
interface: ["GPIO", "PWM"]
voltage: "5V / 12V (array dependent)"
logic_level: "3.3V (switched via MOSFET)"
current_ma: "100 mA – 1 A+ per array"
price_usd_approx: "$3 – $25"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["infrared", "850nm", "940nm", "night-vision", "illuminator", "camera"]
pairs_with:
  - "../01-vision-imaging/"
  - "../07-control-actuators/mosfet-drivers.md"
  - "../06-biometric-presence/pir-hc-sr501.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=ir+led" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=ir+led" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=ir+illuminator" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-ir+led.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-850nm-ir-illuminator.html" }
libraries:
  - "none — PWM via MOSFET driver"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

IR flood arrays are near-infrared LEDs bundled into a ring, bar, or panel, used to light a scene for a camera that is sensitive in the NIR (e.g., a Raspberry Pi NoIR camera). The 850 nm variant is brighter and cheaper but visibly glows dim red when on; 940 nm is dimmer but essentially invisible. For WeftOS, an IR flood is the standard pairing for any "see in the dark" camera node, gaze tracker, or after-hours security view.

## Key specs

- Wavelengths: 850 nm (faint red glow, ~2× brighter per watt) vs 940 nm ("covert", no visible glow, lower quantum efficiency).
- Forward voltage: 1.2–1.6 V per LED; arrays are series-parallel strings.
- Drive current: 100 mA – 1 A+ depending on module.
- Panels often have an integrated LDR / photoresistor that auto-enables at night.
- Camera sensor must have the NIR filter removed ("NoIR" variant) or it will see nothing at 850/940 nm.

## Interface & wiring

- Switch via a [low-side MOSFET](../07-control-actuators/mosfet-drivers.md); never direct GPIO for anything > 20 mA.
- For 12 V panels, use a dedicated supply and share ground with the MCU.
- PWM the flood for intensity control; high-PWM (>2 kHz) can beat against rolling-shutter cameras and cause banding — pick a frequency that's either much slower than a frame or synchronized to exposure.
- If pairing with [PIR / mmWave presence sensors](../06-biometric-presence/) for dusk activation, add a couple of seconds of hysteresis so headlights don't flip the flood on and off.

## Benefits

- Turns a cheap NoIR camera into a workable night-vision sensor.
- Covert (940 nm) surveillance without visible emitters.
- Cheap per lumen compared to visible LED floods for this use case.
- PWM-dimmable, so no visible flicker to a person even at 850 nm.

## Limitations / gotchas

- **The camera must be NoIR.** Stock Pi / phone cameras have a hot-mirror filter that blocks 850/940 nm and you'll see a black frame.
- 850 nm has a visible "red eye" glow if you look straight at the array — not covert.
- 940 nm LEDs are ~50 % less bright than 850 nm for the same current; scenes look grainier.
- IR is invisible to humans but **is still radiation** — avoid staring into high-power arrays (thermal risk at close range).
- IR interference: other IR sources in the room (remote controls, sunlight, some TOF sensors) can wash out or blind the camera.
- LED die wavelength tolerance is ± 20 nm; mixing batches in one array gives uneven spectrum.

## Typical use cases

- Night-vision security camera / front-porch doorbell.
- Gaze and pupil tracking (paired with an IR filter on a monochrome camera).
- Invisible illumination for a reading / posture desk cam.
- Fill light for a face-recognition kiosk that must work at night.

## Pairs well with

- [`../01-vision-imaging/`](../01-vision-imaging/) — NoIR cameras and monochrome global-shutter modules.
- [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) — required for safe switching.
- [`../06-biometric-presence/pir-hc-sr501.md`](../06-biometric-presence/pir-hc-sr501.md) — PIR trigger that only activates the IR flood when someone's there.

## Where to buy

- Adafruit, SparkFun, Seeed for small arrays (850 nm dominant).
- AliExpress / Amazon for 48-LED / 72-LED IR ring illuminators (common on CCTV domes).
- DigiKey / Mouser for Osram and Vishay high-efficiency dies when you need known specs.

## Software / libraries

- No dedicated library; treat as a PWM-controlled load behind a MOSFET.
- On Pi / Linux camera pipelines, disable auto-white-balance when under IR flood; it makes AWB drift to red/purple.

## Notes for WeftOS

- Represent IR floods and visible LEDs as different *illumination policies* in the HAL; camera nodes should request "illumination for scene capture" and the HAL picks the invisible-first option.
- Duty-cycle caps and inrush limits belong in the driver, not per-app.
- Privacy policy: on-prem cameras that can see in total darkness should expose a hardware indicator (even a dim visible LED) that is **not** software-maskable — the HAL should refuse to enable the IR flood without the indicator.
