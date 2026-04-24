---
title: "PCF8563 Real-Time Clock"
slug: "pcf8563-rtc"
category: "10-storage-timing-power"
part_numbers: ["PCF8563", "PCF8563T"]
manufacturer: "NXP"
interface: ["I2C"]
voltage: "1.0 – 5.5 V"
logic_level: "3.3V / 5V tolerant"
current_ma: "~250 nA backup, ~550 µA active"
price_usd_approx: "$1 – $4"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [rtc, timing, i2c, pcf8563, low-power, nxp]
pairs_with:
  - "./ds3231-rtc.md"
  - "./microsd.md"
  - "./max17048-fuel-gauge.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=pcf8563" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=pcf8563" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=pcf8563" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-pcf8563.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-pcf8563.html" }
datasheet: "https://www.nxp.com/products/analog-and-mixed-signal/real-time-clocks/real-time-clocks-watch-ics/real-time-clock-calendar:PCF8563"
libraries:
  - { name: "Rtc_Pcf8563",        url: "https://github.com/elpaso/Rtc_Pcf8563" }
  - { name: "ESPHome pcf8563",    url: "https://esphome.io/components/time/pcf8563.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The NXP PCF8563 is a low-power I²C / calendar RTC. It's less accurate than the
DS3231 but much more power-sipping (sub-microamp backup draw) and operates down
to 1.0 V, making it a better choice for ultra-low-power coin-cell-backed
projects — think e-paper weather stations, LoRaWAN nodes on solar + supercap.
Used on LilyGo T-Watch and several Pine64 / RAK modules.

## Key specs

| Spec | PCF8563 |
|------|---------|
| Accuracy | ±20 ppm crystal-dependent (~1 min/month typical) |
| Supply | 1.0 – 5.5 V |
| Backup current | ~250 nA |
| I²C address | 0x51 |
| Alarm | 1 programmable alarm + timer |
| INT pin | open-drain, shareable |

## Interface & wiring

- I²C at 0x51 — **no collision with DS3231/DS1307 at 0x68**, nice for dual-RTC
  experiments.
- Open-drain `INT` pin — active low on alarm; wake ESP32 from deep-sleep via
  `ext0`.
- Also has a 32.768 kHz CLKOUT pin useful for clocking an SX127x LoRa module's
  crystal-less ultra-low-duty cycles.
- External 32.768 kHz crystal is onboard on every breakout you'll buy.

## Benefits

- Lowest idle current in this category — ideal for solar + supercap nodes.
- Non-conflicting I²C address.
- NXP part, broadly stocked, reliable.

## Limitations / gotchas

- No TCXO — drift matches DS1307 (~±20 ppm). If wall-clock accuracy matters,
  use DS3231.
- Only 1 alarm (vs DS3231's 2) — scheduling flexibility slightly reduced.
- Watch for CR2032-charging mistake on cheap breakouts (same story as DS3231).

## Typical use cases

- LoRaWAN sensor nodes running months on a coin cell.
- Solar / supercap e-paper badges / dashboards.
- Secondary RTC when an I²C address collision pushes DS3231 off-bus.

## Pairs well with

- [`./max17048-fuel-gauge.md`](./max17048-fuel-gauge.md) — long-life battery telemetry.
- [`../08-communication/sx1276-rfm95-lora.md`](../08-communication/sx1276-rfm95-lora.md) — low-duty-cycle radio scheduling.
- [`./microsd.md`](./microsd.md) — datalogging.

## Where to buy

Common as an add-on to LilyGo / Heltec boards; standalone breakouts on
AliExpress for ~$2.

## Software / libraries

- `Rtc_Pcf8563` — Arduino driver.
- `RTClib` also has PCF8563 support in newer versions.
- ESPHome `time: platform: pcf8563`.

## Notes for WeftOS

Model it as a low-accuracy but low-power clock source; record the drift
estimate in sensor metadata so the scheduler knows to resync over NTP / LoRa
time-beacons when the radio is up.
