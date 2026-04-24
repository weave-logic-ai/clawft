---
title: "RCWL-0516 – Microwave Doppler Motion Sensor"
slug: "rcwl-0516-microwave"
category: "06-biometric-presence"
part_numbers: ["RCWL-0516"]
manufacturer: "RCWL / various clones"
interface: ["GPIO"]
voltage: "4V – 28V"
logic_level: "3.3V digital output"
current_ma: "~2–3 mA active"
price_usd_approx: "$1 – $3"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [presence, motion, microwave, doppler, radar, gpio]
pairs_with:
  - "../06-biometric-presence/pir-hc-sr501.md"
  - "../07-control-actuators/relay-modules.md"
  - "../01-vision-imaging/esp32-cam-ov2640.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=RCWL-0516" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=microwave+motion" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=RCWL-0516" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-RCWL.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-RCWL-0516.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The RCWL-0516 is a ~3 GHz microwave Doppler motion board. A small onboard
oscillator emits microwaves; anything moving in the beam path reflects them back
with a frequency shift. The board integrates detection and raises a digital pin
HIGH while motion is present. Unlike PIR it is **not** limited to warm bodies,
and the beam penetrates drywall, glass, and plastic — which is both its best
feature and its worst.

## Key specs

| Spec | Value |
|------|-------|
| Frequency | ~3.2 GHz (informal; not FCC-certified for sale as a radar) |
| Range | ~5–7 m typical |
| Supply | 4–28 V |
| Output | 3.3 V digital, HIGH while motion present |
| Hold time | ~2 s fixed, can be tuned with C-TM cap |
| Sensitivity | Tunable with R-GN resistor |

## Interface & wiring

Three pins you care about: `VIN / GND / OUT`. The board also exposes `CDS` (tie
to ground to disable in daylight with a photoresistor) and `3V3` (regulator
output you can use to power the ESP32 on small battery builds). Treat `OUT`
exactly like a PIR output.

## Benefits

- Sees through thin walls, plastic enclosures, and glass — useful for hidden
  installs.
- Works on non-human moving objects (doors, fans), useful as a "something
  changed" trigger.
- Extremely cheap.

## Limitations / gotchas

- **Not regulatory-certified** in most regions. Don't design it into a product
  you plan to sell; for hobby use it's fine.
- Notoriously triggered by HVAC airflow, fluorescent ballasts, nearby WiFi
  radios, and power-supply noise. Lay it out with a clean linear supply and
  keep it away from switch-mode regulators.
- No adjustable hold time / sensitivity via GPIO — you're tuning with a
  soldering iron.
- **Still motion-based.** Seated humans are invisible after the hold expires,
  same failure mode as PIR.

## Typical use cases

- "Someone walked past the hallway" trigger where PIR is blocked by a door.
- Hidden-inside-a-box motion detection for art installations.
- Second-opinion sensor to ANDed with PIR to reduce false positives (different
  physics → uncorrelated noise).

## Pairs well with

- [`pir-hc-sr501.md`](pir-hc-sr501.md) — AND the two outputs for a much lower
  false-positive rate.
- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) —
  drive lighting / fans on motion.
- [`../01-vision-imaging/esp32-cam-ov2640.md`](../01-vision-imaging/esp32-cam-ov2640.md) —
  cheap wake-on-motion for a through-the-wall camera.

## Where to buy

See `buy:`. AliExpress is effectively the canonical source; North-American
distributors resell the same board.

## Software / libraries

None needed. Treat as a GPIO interrupt. ESPHome's `binary_sensor: gpio` works.

## Notes for WeftOS

Tag RCWL-0516 events as `motion/microwave`, distinct from `motion/pir`, so the
rules engine can use them in AND / OR combinations. Always rate-limit at the
edge; their false-positive rate will spam any naive cloud stream.
