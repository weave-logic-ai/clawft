---
title: "IR Flame Sensor (470–1100 nm)"
slug: "flame-sensor"
category: "05-environmental"
part_numbers: ["KY-026", "flame sensor module"]
manufacturer: "generic"
interface: ["ADC", "GPIO (digital threshold)"]
voltage: "3.3V / 5V"
logic_level: "3.3V"
current_ma: "~3 mA"
price_usd_approx: "$1 – $4"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["flame", "fire", "ir", "safety", "ky-026"]
pairs_with:
  - "./smoke-detector.md"
  - "./mq-series.md"
  - "./mh-z19-co2.md"
  - "../07-control-actuators/relay-modules.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=flame+sensor" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=flame+sensor" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=flame+sensor" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-flame.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-ky-026-flame.html" }
libraries:
  - "no dedicated driver — ADC / GPIO threshold"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The KY-026-class flame sensor is a phototransistor with a 470–1100 nm filter that responds to the IR signature of open flames. Output is typically both analog (distance-dependent intensity) and a digital threshold (via on-board comparator / trimpot). For WeftOS, it's a cheap "open flame in view" detector — *not* a replacement for a UL/CE-certified smoke or flame alarm in life-safety applications.

## Key specs

- Spectral response: ~470–1100 nm (infrared).
- Detection range: 20 cm (typical) up to ~1 m for a large flame.
- FOV: ~60° (wide) to directional (narrow) depending on board.
- Output: analog + digital threshold (trimpot sets comparator).
- Voltage: 3.3 V / 5 V; module has integrated comparator IC (LM393 typical).

## Interface & wiring

- 4 pins: `VCC`, `GND`, `DO` (digital threshold), `AO` (analog).
- Digital pin → any GPIO with interrupt; trimpot on-board sets the threshold.
- Analog pin → ADC for distance / magnitude.
- Aim the sensor at the area you want to guard; IR is line-of-sight.

## Benefits

- Sub-$5 "is there a flame?" detector.
- Fast response (milliseconds).
- Works well near expected-flame appliances (gas stoves, boilers) where visible flame is normal.
- Both digital + analog output gives flexibility.

## Limitations / gotchas

- **Sunlight and incandescent bulbs** emit in the same IR band and cause false positives. Use a narrower IR filter (940 nm band) or shield from direct IR sources.
- Very small / distant flames may not trigger; very large fires may saturate the detector and still trigger — but it's not a rate-of-rise detector.
- Does not sense smoke or CO — pair with a [smoke detector](./smoke-detector.md) and/or [MQ-7/MQ-9 CO sensor](./mq-series.md) for a complete fire-safety picture.
- Not certified for use as a primary life-safety flame detector. Do not build an AHJ-facing fire system around a KY-026.
- Phototransistor ages and drifts sensitivity downward over years.

## Typical use cases

- Gas-stove "flame out" detection (flame should be there, but isn't).
- Candle / incense hobby safety timer.
- Kiln / forge / 3D-printer enclosure fire-check (secondary to a real detector).
- Robot "don't run into the fire" sensor.

## Pairs well with

- [`./smoke-detector.md`](./smoke-detector.md) — smoke + flame covers complementary failure modes.
- [`./mq-series.md`](./mq-series.md) — MQ-7 (CO) and MQ-2 (combustibles) add gas detection.
- [`./mh-z19-co2.md`](./mh-z19-co2.md) — CO₂ rise during combustion confirms fire.
- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) — cut power / close gas valve on detection.

## Where to buy

- AliExpress / Amazon KY-026 for a few dollars.
- SparkFun / DFRobot branded modules with slightly better filters.

## Software / libraries

- None; read digital for trigger, analog for intensity + distance estimate.

## Notes for WeftOS

- Tag any alert from this sensor as "unverified flame event — confirm with smoke / CO sensor". Do not auto-dial emergency services from a KY-026 alone.
- Implement sunlight-rejection: if ambient lux ([BH1750](./bh1750-lux.md)) is high **and** the flame sensor fires, require a secondary confirmation before alerting.
- For product-grade fire safety, WeftOS nodes should integrate with certified detectors through dry contacts to the HAL, treating those contacts as authoritative.
