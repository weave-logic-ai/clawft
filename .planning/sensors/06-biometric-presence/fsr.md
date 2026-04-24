---
title: "FSR 402 / 406 – Force-Sensitive Resistor"
slug: "fsr"
category: "06-biometric-presence"
part_numbers: ["FSR 402", "FSR 406", "FSR 400"]
manufacturer: "Interlink Electronics"
interface: ["ADC"]
voltage: "any (passive resistor)"
logic_level: "ADC-dependent"
current_ma: "< 1 mA (divider current)"
price_usd_approx: "$5 – $15"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [force, pressure, analog, adc, hmi, presence]
pairs_with:
  - "../10-storage-timing-power/ads1115-ads1015.md"
  - "../09-input-hmi/index.md"
  - "../11-wild-card/hx711-load-cell.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=FSR" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=FSR" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=force+sensitive+resistor" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-force+sensitive.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-FSR-force-sensitive.html" }
datasheet: "https://www.interlinkelectronics.com/fsr-400-series"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

FSRs from Interlink Electronics are thin polymer pads whose resistance drops as
you press harder. The FSR 402 is a ~12 mm circular pad; the FSR 406 is a ~38 mm
square; the FSR 400 is a tiny 5 mm disc. They aren't linear load cells — they
give a noisy but monotonically decreasing resistance vs force — which is exactly
what you want for "is someone sitting here", "is a finger on the button", or
"how hard did they tap".

In a presence catalog they belong in the seat / floor / mat detector family.

## Key specs

| Spec | Value |
|------|-------|
| Force range | ~0.1 N – 10 N usable, saturates beyond |
| Resistance | > 10 MΩ unpressed, ~250 Ω at full press |
| Response time | < 3 ms |
| Supply | any (it's a passive resistor) |
| Lifetime | > 10 M actuations |

## Interface & wiring

Wire the FSR in a voltage divider with a fixed resistor (typically 10 kΩ) to
3.3 V, measure the midpoint with an ADC pin. The ESP32's built-in ADC is good
enough for "button / no button" and "seat occupied"; for nuanced force scaling,
use an [ADS1115](../10-storage-timing-power/ads1115-ads1015.md).

## Benefits

- Passive, cheap, thin enough to hide under a cushion, mat, or keycap.
- No inrush current, no precise calibration needed for on/off use.
- Millisecond response, quieter than mechanical switches.

## Limitations / gotchas

- **Not a scale.** Force-to-resistance is noisy and varies by temperature,
  mechanical backing, and where on the pad you press. For actual weight, use a
  [HX711 + strain-gauge load cell](../11-wild-card/hx711-load-cell.md).
- Hysteresis ~10 %. Don't chase tight thresholds.
- Cheap clones from AliExpress are much less consistent than genuine Interlink
  parts — pay the premium for real FSRs if you care.
- Under a rigid puck the edges produce hot spots; always put a compliant foam
  layer between FSR and load.

## Typical use cases

- "Is someone in the chair?" seat-occupancy sensor.
- Drumpad / velocity-sensitive MIDI controller.
- Sleep / infant mat that reports movement and presence.
- Tamper / jar-lid pad.

## Pairs well with

- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) —
  16-bit ADC upgrade for smoother force curves.
- [`../11-wild-card/hx711-load-cell.md`](../11-wild-card/hx711-load-cell.md) —
  pair both when you want presence + actual weight.
- [`../09-input-hmi/index.md`](../09-input-hmi/index.md) — FSRs are a common
  HMI element in DIY controllers.

## Where to buy

See `buy:`. Interlink Electronics sells direct and through all the usual hobby
distributors. Pimoroni and Adafruit both stock the 402 and 406.

## Software / libraries

No driver. It's a resistor. Calibrate per mechanical mount and store the
per-install thresholds.

## Notes for WeftOS

Seat-occupancy FSRs are a **presence** input (boolean) that doesn't leak
biometric data — good default for shared spaces where mmWave would be too
invasive. Stream only the boolean, never the raw ADC counts, off-device.
