---
title: "DS1307 Real-Time Clock"
slug: "ds1307-rtc"
category: "10-storage-timing-power"
part_numbers: ["DS1307", "DS1307Z"]
manufacturer: "Analog Devices (Maxim)"
interface: ["I2C"]
voltage: "5 V (main); 2.0 – 3.5 V backup"
logic_level: "5V (use level shifter for ESP32)"
current_ma: "~1.5 mA active, ~500 nA backup"
price_usd_approx: "$1 – $3"
esp32_compat: ["ESP32 via level shifter"]
tags: [rtc, timing, i2c, ds1307, legacy]
pairs_with:
  - "./ds3231-rtc.md"
  - "./microsd.md"
  - "../08-communication/tca9548a-i2c-mux.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=ds1307" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=ds1307" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=ds1307" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-ds1307.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-ds1307.html" }
datasheet: "https://www.analog.com/en/products/ds1307.html"
libraries:
  - { name: "RTClib (Adafruit)",  url: "https://github.com/adafruit/RTClib" }
  - { name: "ESPHome ds1307",     url: "https://esphome.io/components/time/ds1307.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The DS1307 is the classic 1990s-era I²C RTC. It's cheap, simple, and absolutely
everywhere — but it uses an **external 32.768 kHz crystal** with no temperature
compensation, so its drift is poor (typ. ±20 ppm, easily > 1 minute/week in
uncontrolled environments). Included here for completeness and for when you
inherit legacy hardware; for any new design, prefer the
[`DS3231`](./ds3231-rtc.md).

## Key specs

| Spec | DS1307 |
|------|--------|
| Accuracy | ±20 ppm @ 25 °C, much worse outside |
| Drift | ~1–2 min/week typical |
| Main supply | 5 V (**not** 3.3 V) |
| Backup supply | 2.0 – 3.5 V (CR2032) |
| I²C address | 0x68 |
| Onboard SRAM | 56 bytes battery-backed |

## Interface & wiring

- I²C at 0x68 (collides with DS3231, MPU-6050, others — plan addresses).
- **Requires 5 V** on Vcc; the I²C pins are also 5 V push-pull. For a 3.3 V
  ESP32 you need a **level shifter** on SDA/SCL or just use a DS3231 instead.
- CR2032 on backup pin keeps time across power loss.
- Optional SQW pin for 1 Hz / 4 kHz / 8 kHz / 32 kHz square wave output.

## Benefits

- Dirt cheap (~$1 on AliExpress).
- Identical register map to DS3231 — same library code.
- 56 bytes of battery-backed SRAM for tiny persistent state.

## Limitations / gotchas

- **Bad accuracy.** Any temperature swing moves the crystal; typ. 1–2 min/week
  drift. Not suitable for timestamped logs beyond a few days without NTP.
- **5 V only** for the main supply — inconvenient on modern 3.3 V boards.
- Address-collides with DS3231 and MPU-6050 at 0x68.
- No alarm pin — the SQW output is fixed-frequency, not an alarm interrupt,
  so deep-sleep scheduling is clunkier than DS3231.
- Cheap boards often wire the crystal badly, making drift even worse.

## Typical use cases

- Legacy projects where "good enough clock for a day" is fine.
- Shields and kits that came with a DS1307 already fitted.
- Ultra-low-BOM-cost displays where ±1 min/week is acceptable.

## Pairs well with

- [`./microsd.md`](./microsd.md) — logging with low timestamp precision.
- [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md) — dodge the 0x68 address collision.
- [`./ds3231-rtc.md`](./ds3231-rtc.md) — the recommended successor.

## Where to buy

All hobby stores; DS1307 modules on AliExpress ~$1. For new designs buy
DS3231 instead at the same price.

## Software / libraries

- `RTClib` (Adafruit) — identical API as DS3231.
- ESPHome `time: platform: ds1307` — native support.

## Notes for WeftOS

Flag DS1307 in the sensor metadata as **low-accuracy** so the WeftOS planner
knows to schedule more-frequent NTP resyncs if networking is available. Battery
backup + drift spec should surface as a capability bound on timestamp
reliability.
