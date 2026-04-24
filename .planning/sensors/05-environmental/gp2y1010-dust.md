---
title: "Sharp GP2Y1010AU0F Optical Dust Sensor"
slug: "gp2y1010-dust"
category: "05-environmental"
part_numbers: ["GP2Y1010AU0F"]
manufacturer: "Sharp"
interface: ["ADC (pulsed IR LED + analog output)"]
voltage: "5V"
logic_level: "3.3V"
current_ma: "~20 mA peak (pulsed LED)"
price_usd_approx: "$10 – $15"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["dust", "particulate", "pm2.5", "optical", "legacy"]
pairs_with:
  - "./pms5003.md"
  - "../10-storage-timing-power/ads1115-ads1015.md"
  - "./bme280.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=gp2y1010" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=gp2y1010" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=gp2y1010" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-gp2y1010.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-gp2y1010.html" }
datasheet: "https://www.sharpsde.com/products/optoelectronics/dust-sensor/"
libraries:
  - "no dedicated driver — pulse LED 320 µs on, read ADC at 280 µs, wait 9.68 ms"
  - "Arduino: GP2Y1010AU0F-style demo sketches"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The Sharp GP2Y1010AU0F is a pre-PMS5003 optical dust sensor: an IR LED and a photodiode at right angles, pulsed briefly to detect light scattered by dust passing through a small chamber. It gives a relative analog output roughly proportional to dust density. For WeftOS it's mostly a legacy / budget option — the [PMS5003](./pms5003.md) is strictly better for the same price class.

## Key specs

- Output: analog voltage, roughly proportional to dust density in mg/m³.
- LED pulse timing (important): 320 µs on, sample at ~280 µs, total cycle 10 ms.
- Operating voltage: 5 V (LED), 5 V analog supply.
- Particle size: sensitive to > 1 µm; **not** PM2.5-specific — lumps everything > 1 µm together.
- No fan; relies on convection for airflow (readings drift badly with enclosure orientation).

## Interface & wiring

- LED pulse pin: GPIO drives the internal IR LED via a 150 Ω resistor and 220 µF cap (per datasheet; most breakouts already have these).
- Analog out: feed through voltage divider to ESP32 ADC, or preferably into an [ADS1115](../10-storage-timing-power/ads1115-ads1015.md) for 16-bit reads.
- Timing is picky: you must pulse, wait ~280 µs, sample, then release. Polling too fast burns the LED duty cycle beyond spec.

## Benefits

- Cheap and still stocked, often the only dust sensor in budget starter kits.
- Simple analog interface — works on any ADC.
- No fan = silent and low-power compared to PMS5003.

## Limitations / gotchas

- **No real PM2.5 / PM10 separation.** Output is a blur of "dust above ~1 µm". Calling it PM2.5 is a stretch.
- No fan means airflow depends on convection and orientation; readings are inconsistent node-to-node.
- Very sensitive to dust buildup **inside** the sensor — drops into "always high" over months without cleaning.
- LED ages and drifts over ~1 year.
- Needs its own 5 V supply; shared rails with noisy loads (Wi-Fi modules) inject directly into the analog output.
- Humidity affects readings in the same way PMS5003 is affected, but without a proper mass conversion to correct.

## Typical use cases

- Budget dust trigger when [PMS5003](./pms5003.md) isn't available.
- Educational projects where understanding the optical principle matters.
- Legacy replacements for older nodes.

## Pairs well with

- [`./pms5003.md`](./pms5003.md) — upgrade path for real PM measurement.
- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) — clean ADC reads.
- [`./bme280.md`](./bme280.md) — pair with T/RH for partial humidity compensation.

## Where to buy

- Sharp / DigiKey / Mouser for genuine modules.
- Adafruit / SparkFun / AliExpress for breakouts with the external resistor / cap already fitted.

## Software / libraries

- No dedicated driver; copy the 10 ms duty-cycle pulse loop from the datasheet.
- Arduino demo sketches abound and are trivial.

## Notes for WeftOS

- HAL should tag this sensor as "relative dust density, uncalibrated". Do not display µg/m³ values without a co-calibration against a PMS5003.
- Recommend retirement: in the WeftOS device registry, prefer PMS5003 and mark GP2Y1010 deployments as "legacy, migrate".
- Schedule a periodic self-clean reminder (blow compressed air into the chamber every ~6 months).
