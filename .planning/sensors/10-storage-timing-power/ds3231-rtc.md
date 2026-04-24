---
title: "DS3231 TCXO Real-Time Clock"
slug: "ds3231-rtc"
category: "10-storage-timing-power"
part_numbers: ["DS3231", "DS3231SN", "DS3231M"]
manufacturer: "Analog Devices (Maxim)"
interface: ["I2C"]
voltage: "2.3 – 5.5 V"
logic_level: "3.3V / 5V tolerant"
current_ma: "~170 µA active, ~2 µA on backup battery"
price_usd_approx: "$2 – $6"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [rtc, timing, i2c, tcxo, ds3231, battery-backed]
pairs_with:
  - "./microsd.md"
  - "./ds1307-rtc.md"
  - "./pcf8563-rtc.md"
  - "../08-communication/sx1276-rfm95-lora.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=ds3231" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=ds3231" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=ds3231" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-ds3231.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-ds3231.html" }
datasheet: "https://www.analog.com/en/products/ds3231.html"
libraries:
  - { name: "RTClib (Adafruit)",  url: "https://github.com/adafruit/RTClib" }
  - { name: "DS3231 (Rinky-Dink)", url: "https://github.com/rodan/ds3231" }
  - { name: "ESPHome ds1307",     url: "https://esphome.io/components/time/ds1307.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The DS3231 is the gold-standard hobby RTC: it integrates a **temperature-compensated
crystal oscillator (TCXO)**, so it's factory-trimmed to **±2 ppm** over the
industrial temperature range — roughly 1 minute of drift per year, no tuning
needed. It also exposes the temperature sensor readout over I²C and has two
programmable alarm registers. On breakout boards it ships with a CR2032 socket
or LIR2032 rechargeable holder for multi-year battery backup.

## Key specs

| Spec | DS3231 |
|------|--------|
| Accuracy | ±2 ppm (0–40 °C), ±3.5 ppm (–40 – +85 °C) |
| Drift | < 1 min/year typical |
| Backup current | 2 µA typical |
| I²C address | 0x68 |
| Temperature sensor | ±3 °C, 10-bit |
| Alarms | 2 programmable alarms, SQW/INT pin |

## Interface & wiring

- I²C at 0x68. Pull-ups usually on the breakout.
- Power: 3.3 V for ESP32; module itself accepts up to 5.5 V.
- SQW / INT pin outputs a programmable square wave (1 Hz, 1.024 kHz, 4 kHz,
  8 kHz) or alarm interrupt — great for waking ESP32 from deep sleep on a cron
  schedule.
- 32 kHz output pin available for clocking other devices.
- Keep the CR2032 on the board; the RTC keeps time across ESP32 reboots.

## Benefits

- TCXO accuracy — no NTP required for "close enough" wall time.
- Onboard temperature sensor (used internally for compensation).
- Alarm interrupt output is a **perfect deep-sleep wake source** for battery
  nodes (wake every 15 min, sample, sleep).
- Drop-in replacement for DS1307 — same I²C address, more accurate, same
  libraries.

## Limitations / gotchas

- Many AliExpress boards ship with a **non-rechargeable CR2032 wired through a
  charging resistor** — this violates CR2032 specs and can leak/vent. Either
  remove the charging resistor, or use an LIR2032 (rechargeable lithium).
- I²C address collides with MPU-6050 on 0x68 — use a different address on one
  device, or put them on different I²C buses via
  [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md).
- Temperature readout is noisy (±3 °C) — for actual temperature measurement use
  a dedicated sensor.
- Date/time registers are **BCD-encoded** — the library handles it, but raw
  register reads need conversion.

## Typical use cases

- Datalogger timestamp source (paired with microSD).
- Scheduled wake-from-deep-sleep for battery-powered nodes.
- Offline clock/display projects.
- Forensic event timestamps in mesh networks without NTP.

## Pairs well with

- [`./microsd.md`](./microsd.md) — timestamp every log line.
- [`./max17048-fuel-gauge.md`](./max17048-fuel-gauge.md) — "time-of-day + state-of-charge" dashboard.
- [`../08-communication/sx1276-rfm95-lora.md`](../08-communication/sx1276-rfm95-lora.md) — timeslotted LoRa TDMA.

## Where to buy

Adafruit 3013, SparkFun DS3231 breakout for quality; generic ZS-042 on
AliExpress for ~$2 (watch the battery-charging-resistor gotcha).

## Software / libraries

- `RTClib` (Adafruit) — covers DS3231, DS1307, PCF8523 uniformly.
- ESPHome `time: platform: ds1307` — works with DS3231 at the same address.
- ESP-IDF `rtc.h` — optional native bindings.

## Notes for WeftOS

The DS3231 should be WeftOS's **default wall-clock source** for any battery /
offline deployment. Its alarm pin pairs with ESP32 `ext0`/`ext1` wakeup to
implement the canonical "scheduled sampling" pattern with negligible idle draw.
