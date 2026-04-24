---
title: "Smoke Detector Heads (Ionisation / Photoelectric)"
slug: "smoke-detector"
category: "05-environmental"
part_numbers: ["MQ-2 (smoke proxy)", "photoelectric smoke head", "CJMCU-5051"]
manufacturer: "generic"
interface: ["ADC", "GPIO (digital threshold)", "Dry contact (from certified detector)"]
voltage: "3.3V / 5V / mains (certified detector)"
logic_level: "3.3V"
current_ma: "~5 – 30 mA (hobby); negligible for dry-contact integrations"
price_usd_approx: "$3 – $30"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["smoke", "fire", "safety", "photoelectric", "ionisation"]
pairs_with:
  - "./flame-sensor.md"
  - "./mq-series.md"
  - "./mh-z19-co2.md"
  - "../07-control-actuators/relay-modules.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=smoke+sensor" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=smoke" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=smoke" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-smoke.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-smoke-sensor.html" }
libraries:
  - "MQ-2 / MQ-9 reuse the mq-series driver approach (ADC + curve)"
  - "Certified detectors: expose dry-contact as a digital input"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

"Smoke detector" in hobby land usually means one of three things:

1. **MQ-2-style metal-oxide** sold as a "smoke sensor" — really a general-combustible MOX with MQ-series caveats. See [mq-series.md](./mq-series.md).
2. **Photoelectric optical smoke head** (CJMCU-5051 and similar) — LED + photodiode in a smoke chamber; smoke scatters light into the photodiode. Conceptually similar to a UL 217 detector's chamber but without the certification.
3. **Certified consumer / commercial detectors** with a dry-contact or voltage output that WeftOS consumes as a digital input.

For any life-safety deployment, use a **certified detector** (Kidde, Nest Protect, FirstAlert, commercial addressable panel) and integrate via dry contact or manufacturer API. The hobby versions are useful for early-warning augmentation, not primary protection.

## Key specs

- MQ-2 smoke sensitivity: broad combustible / smoke, analog + digital.
- Photoelectric hobby chamber: 1 Hz LED pulse, photodiode integrates scattered light.
- Ionisation heads (Americium-241): not legally sold as bare parts — only in certified detectors. Better at fast-flaming fires than photoelectric.
- Certified UL 217 / EN 14604 detectors: self-test, low-battery chirp, and (often) an interconnect line you can read.

## Interface & wiring

- MQ-2: see [mq-series.md](./mq-series.md); 5 V heater, analog out.
- Photoelectric hobby heads: pulse the LED, sample the photodiode with tight timing, subtract baseline. Most hobbyists just wire them as MQ-2 substitutes, which loses the photoelectric advantage.
- **Certified detector integration:** use the detector's interconnect / relay output. Many residential detectors have a 9 V "interconnect" wire that pulses when the alarm sounds. WeftOS wires that through an opto-isolator into a GPIO input.
- Mains-powered detectors must stay on their own circuit; never merge low-voltage logic ground with mains ground.

## Benefits

- Photoelectric chambers catch smouldering fires earlier than ionisation or MOX.
- Certified detector integration gets WeftOS awareness without replacing a compliance-critical device.
- Cheap enough for redundant node placement (kitchen, garage, bedroom hallway).

## Limitations / gotchas

- **Hobby "smoke sensors" are not certified.** Do not depend on them for alarm; they miss smouldering fires and false-alarm on cooking steam.
- Photoelectric is less sensitive to fast flaming fires; ionisation is less sensitive to smouldering. A combined (photo + ion) certified detector covers both.
- Certified detectors are sealed and not legally modifiable — WeftOS must sit alongside them, not replace them.
- Some modern smart smoke alarms (Nest Protect, Ring) have official APIs; use them instead of hacking into the hardware.
- Americium-241 (ionisation source) is lightly radioactive; handling bare heads is regulated in some jurisdictions.

## Typical use cases

- Secondary early warning for garages, workshops, server rooms.
- Kitchen "smoke event" for vent-fan automation.
- 3D-printer / kiln enclosure monitoring (augmenting a dedicated heat detector).
- Integration point for bringing existing UL-listed detectors into a WeftOS dashboard.

## Pairs well with

- [`./flame-sensor.md`](./flame-sensor.md) — flame covers fast-ignition fires; smoke covers smouldering.
- [`./mq-series.md`](./mq-series.md) — MQ-2 as a third redundant indicator.
- [`./mh-z19-co2.md`](./mh-z19-co2.md) — rising CO₂ + falling O₂ proxy confirms combustion.
- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) — close the gas solenoid valve on alarm.

## Where to buy

- AliExpress / Amazon for MQ-2 and hobby photoelectric heads.
- Home-improvement retailers for UL 217 certified detectors (FirstAlert, Kidde, Nest).
- Commercial panels (Honeywell, System Sensor) for addressable-loop integrations.

## Software / libraries

- MQ-2: reuse the mq-series approach.
- Photoelectric hobby chamber: bespoke pulse-and-sample loop.
- Certified detectors: treat the dry contact as a standard digital input; debounce for ~5 s.

## Notes for WeftOS

- HAL distinguishes `smoke::UncertifiedIndicator` (MQ-2, hobby photoelectric) from `smoke::CertifiedAlarmInput` (dry contact from UL/CE detector). Only the latter should drive life-safety actions; the former is *early warning* only.
- UI must label uncertified detectors clearly to prevent users from assuming alarm-grade coverage.
- Best-practice deployment: one certified detector per room wired into the WeftOS dry-contact HAL, plus [MQ-7 (CO)](./mq-series.md) and [flame sensor](./flame-sensor.md) for complementary early warning.
- When an uncertified indicator fires, the HAL should *nudge* the user ("possible smoke, please check") but not trigger automation until a certified detector confirms.
