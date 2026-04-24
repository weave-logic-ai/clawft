---
title: "DS1302 Real-Time Clock (3-wire, legacy)"
slug: "ds1302-rtc"
category: "10-storage-timing-power"
part_numbers: ["DS1302"]
manufacturer: "Analog Devices (Maxim)"
interface: ["3-wire (bit-banged)"]
voltage: "2.0 – 5.5 V"
logic_level: "matches supply"
current_ma: "< 300 nA backup, ~1 mA active"
price_usd_approx: "$1 – $2"
esp32_compat: ["ESP32 (bit-banged GPIO)"]
tags: [rtc, timing, ds1302, legacy, bit-bang]
pairs_with:
  - "./ds3231-rtc.md"
  - "./ds1307-rtc.md"
  - "./pcf8563-rtc.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=ds1302" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=ds1302" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=ds1302" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-ds1302.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-ds1302.html" }
datasheet: "https://www.analog.com/en/products/ds1302.html"
libraries:
  - { name: "Rtc (Makuna)",       url: "https://github.com/Makuna/Rtc" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The DS1302 is an older 3-wire (RST / IO / SCLK) RTC with 31 bytes of
battery-backed SRAM. It's included here because it's bundled into a lot of
cheap "Arduino starter kits" and Chinese education modules — if you inherit
one, here is how it fits. For any new project, prefer
[`DS3231`](./ds3231-rtc.md) or [`PCF8563`](./pcf8563-rtc.md).

## Key specs

| Spec | DS1302 |
|------|--------|
| Accuracy | ±20 ppm (crystal-dependent) |
| Backup current | < 300 nA |
| Interface | 3-wire (not I²C, not SPI) |
| Onboard SRAM | 31 bytes battery-backed |
| Voltage | 2.0 – 5.5 V |

## Interface & wiring

- 3-wire protocol: RST (CE), I/O (bidirectional data), SCLK. Plus Vcc + GND +
  Vbat for backup battery.
- **Not I²C, not SPI** — it's a Dallas-proprietary bit-banged protocol, usually
  implemented on any three GPIO pins.
- Breakouts include a 32.768 kHz crystal and a CR2032 socket.
- Charging on the Vbat pin is user-controlled via a software register. Make
  sure the register is zeroed at boot if you use a non-rechargeable CR2032
  (otherwise you are slowly ruining the cell).

## Benefits

- Ultra-low idle current (sub-µA backup).
- Pin-efficient for MCUs without I²C — though that's not ESP32's problem.
- Cheap and still widely stocked.

## Limitations / gotchas

- **Legacy 3-wire protocol** — no standard bus, every library uses bit-banging.
  Avoid if you can.
- Same ±20 ppm drift as DS1307 — not a TCXO.
- Single alarm missing — no hardware alarm at all. For scheduled wake you
  generally pair it with a separate watchdog or just time-count in the MCU.
- Vbat charging register is easy to misconfigure and destroy a CR2032.

## Typical use cases

- Rescuing an existing "Arduino learning kit" project.
- Low-MCU-pin projects where three GPIOs are available but no I²C/SPI.

## Pairs well with

- [`./ds3231-rtc.md`](./ds3231-rtc.md) — the modern replacement.
- [`./pcf8563-rtc.md`](./pcf8563-rtc.md) — when you specifically need low idle on I²C.

## Where to buy

Any hobby store and AliExpress ~$1 per module.

## Software / libraries

- `Rtc` by Makuna — covers DS1302, DS1307, DS3231 uniformly.
- Avoid rolling your own bit-bang unless debugging.

## Notes for WeftOS

Avoid for new WeftOS nodes. If encountered, flag the sensor capability as
**low-accuracy + no-alarm**, forcing the planner to schedule wakes in MCU
software instead of via the RTC.
