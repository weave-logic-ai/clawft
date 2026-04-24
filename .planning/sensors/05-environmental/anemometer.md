---
title: "Anemometer (cup) + Wind Vane Pair"
slug: "anemometer"
category: "05-environmental"
part_numbers: ["Davis 6410 (prosumer)", "Misol WH-SP-WS01", "generic 3-cup anemometer"]
manufacturer: "Davis Instruments, Misol, generic"
interface: ["GPIO (reed-switch pulse)", "ADC (wind vane voltage divider)"]
voltage: "3.3V / 5V"
logic_level: "3.3V"
current_ma: "~1 mA idle (reed switch)"
price_usd_approx: "$25 – $150"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["wind", "anemometer", "wind-vane", "weather"]
pairs_with:
  - "./bme280.md"
  - "./rain-water-level.md"
  - "./leaf-wetness.md"
  - "../10-storage-timing-power/ads1115-ads1015.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=anemometer" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=anemometer" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=anemometer" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-anemometer.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-anemometer.html" }
libraries:
  - "no dedicated driver — count reed-switch pulses + sample vane ADC"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A classic hobby / prosumer weather-station wind package: a 3-cup **anemometer** that pulses a reed switch once per rotation, and a **wind vane** that rotates a magnet over a ring of resistors, producing one of 8 or 16 discrete analog voltages per direction. The Misol WH-SP-WS01 combo is sold on Amazon / AliExpress for ~$25; Davis parts are much more accurate and rugged. For WeftOS, these turn any outdoor node into a real weather station.

## Key specs

- Anemometer: reed-switch pulse per rotation; calibration factor (Misol standard) = 2.4 km/h per Hz.
- Wind vane: 8 or 16 positions, each a unique resistance value in a divider network; reading the divider voltage gives direction.
- Supply: 3.3 V or 5 V (reed switch is passive; vane divider needs reference voltage).
- Interfaces: GPIO interrupt (anemometer), ADC (vane).

## Interface & wiring

- **Anemometer:** reed switch across two wires. Connect as a pull-up switch: one wire to a GPIO with internal pull-up, other to GND. Count falling edges via interrupt; convert Hz → km/h with the calibration factor.
- **Wind vane:** two wires into a voltage divider with a known pull-up resistor (~10 kΩ). Feed the midpoint to an ADC (or [ADS1115](../10-storage-timing-power/ads1115-ads1015.md)). Match the measured voltage to the nearest of the 8/16 "compass-position" reference values.
- Debounce the anemometer in software (~5 ms) — reed switches bounce at low speeds.
- Keep ADC reference stable; vane direction detection is sensitive to rail droop.

## Benefits

- Passive reed / resistor design — survives outdoor weather for years.
- No protocol — pure GPIO + ADC; works with any MCU.
- Cheap enough to deploy multiple per property.
- Can be mounted on a drone or mast with minimal weight.

## Limitations / gotchas

- Misol wind-vane resistor values have ~5 % tolerance; table from the datasheet is a *starting point* — verify with the vane pointed at each cardinal direction.
- Starting threshold of cheap anemometers is ~1 km/h — slower winds read zero.
- Ice and snow can freeze the cups solid; cold-climate stations need heaters or acceptance of winter gaps.
- Reed switch can fail closed (dirty contacts); a reading of 0 km/h that persists across a known-windy day is a red flag.
- Very strong gusts > 150 km/h can bend the cups on cheap units.

## Typical use cases

- Backyard weather station.
- Agriculture / vineyard wind logging for frost alerts.
- Solar / wind microgrid monitoring.
- Sailing / kitesurfing station.
- Drone pre-flight wind gate.

## Pairs well with

- [`./bme280.md`](./bme280.md) — canonical weather-station T/RH/P companion.
- [`./rain-water-level.md`](./rain-water-level.md) — tipping-bucket rain gauge for the third weather metric.
- [`./leaf-wetness.md`](./leaf-wetness.md) — agri-use pairing.
- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) — stable ADC for wind-vane direction.

## Where to buy

- Davis Instruments direct (prosumer — highly accurate, expensive).
- Amazon / AliExpress for Misol WH-SP-WS01 combo sets.
- SparkFun weather meter kit.

## Software / libraries

- No driver — interrupt counting for the anemometer, ADC lookup table for the vane.
- ESPHome has a weather-station component that handles both.

## Notes for WeftOS

- HAL emits `Wind { speed_kmh, direction_deg, gust_kmh }` with a sliding-window 10-minute gust calculator.
- Calibration factor per-unit stored in config; wizard in Admin app to confirm direction mapping.
- Subscribe to wind events for actions like "auto-retract awning", "close greenhouse vents", "abort drone takeoff".
