---
title: "microSD Card Breakout (SPI / SDMMC)"
slug: "microsd"
category: "10-storage-timing-power"
part_numbers: ["generic microSD breakout", "Catalex SD", "Adafruit 254"]
manufacturer: "Generic / Adafruit / SparkFun"
interface: ["SPI", "SDMMC (ESP32 host)"]
voltage: "3.3V (card) / 5V logic-levelled boards"
logic_level: "3.3V"
current_ma: "idle ~0.5 mA, read ~20 mA, write 50–150 mA peaks"
price_usd_approx: "$1 – $8"
esp32_compat: ["ESP32", "ESP32-S3"]
tags: [storage, sd, microsd, fat32, datalogging, spi, sdmmc]
pairs_with:
  - "./ds3231-rtc.md"
  - "./tp4056-lipo-charger.md"
  - "./ina219-ina260.md"
  - "../05-environmental/bme280.md"
  - "../01-vision-imaging/esp32-cam-ov2640.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=microsd+breakout" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=microsd" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=microsd" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-microsd.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-micro-sd-card-module.html" }
datasheet: ""
libraries:
  - { name: "Arduino SD",        url: "https://www.arduino.cc/reference/en/libraries/sd/" }
  - { name: "SdFat",             url: "https://github.com/greiman/SdFat" }
  - { name: "ESP-IDF sdmmc host",url: "https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/storage/sdmmc.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A microSD breakout lets the ESP32 read/write FAT32-formatted SD cards — the
cheapest way to get gigabytes of persistent, removable storage. Used for
datalogging, camera image buffers, audio playback, OTA firmware cache, and
"pull the card and read it on a laptop" workflows.

## Key specs

| Spec | Typical |
|------|---------|
| Card capacity | up to 32 GB FAT32 (or exFAT with SdFat) |
| Supply | 3.3 V (some breakouts include a level shifter for 5 V MCUs) |
| SPI clock | up to ~25 MHz on ESP32 |
| SDMMC clock | up to 80 MHz, 4-bit mode ~4–8× faster than SPI |
| Current (write peak) | 50–150 mA |

## Interface & wiring

Two host options on the ESP32:

- **SPI host** — 4 signals (CS, SCK, MOSI, MISO) + Vcc + GND. Works on any free
  SPI bus. Slowest but most flexible.
- **SDMMC host** — ESP32 has a dedicated SDMMC controller supporting 1-bit or
  4-bit modes. Much faster (up to ~20 MB/s), required for high-throughput use
  like ESP32-CAM video capture. Pins are **fixed** on classic ESP32 (GPIO
  2/4/12/13/14/15), which clashes with bootstrap pins — know the strapping-pin
  rules before wiring.
- The ESP32-CAM's onboard SD uses SDMMC 1-bit mode by default.
- Add a 10 µF bulk cap near the card — write peaks can brown out a weakly
  regulated 3.3 V rail.

## Benefits

- Cheap, removable, gigabytes of storage.
- Works with standard FAT32 — read on any laptop.
- SDMMC mode gives real-time camera / audio throughput.

## Limitations / gotchas

- **Brown-out is the #1 failure mode.** Write peaks of 100+ mA can crash a
  cheap LDO; the card half-finishes a block write and corrupts the FAT. Add a
  bulk cap, use a proper 3.3 V buck, and always call `file.sync()` / `flush()`.
- **Card-in-detect switch** (CD pin on good breakouts) lets firmware notice
  removal and unmount gracefully — not all breakouts break it out.
- **Card lifetime is finite.** Consumer SD cards wear out after ~1000–10,000
  write cycles per block. Use **industrial / high-endurance cards** for 24/7
  logging. Wear-level by rolling log files.
- **SPI vs SDMMC** — you must choose at compile time. SPI is universal but slow;
  SDMMC is fast but hogs specific pins and needs the ESP32 SDMMC driver.
- Some cards refuse to enter SPI mode — buy a known-good brand (SanDisk
  Industrial, Samsung EVO).
- No mount detection in SPI mode unless your breakout has a CD switch.

## Typical use cases

- Datalogging weather / power / sensor streams (BME280, INA219, GPS).
- ESP32-CAM image and video capture.
- WAV file playback through an I²S DAC.
- OTA firmware staging.

## Pairs well with

- [`./ds3231-rtc.md`](./ds3231-rtc.md) — timestamp each log line.
- [`./ina219-ina260.md`](./ina219-ina260.md) — power-consumption logs.
- [`../01-vision-imaging/esp32-cam-ov2640.md`](../01-vision-imaging/esp32-cam-ov2640.md) — camera frame buffer.
- [`../05-environmental/bme280.md`](../05-environmental/bme280.md) — classic T/RH/P logger payload.

## Where to buy

Adafruit 254 and SparkFun "microSD Shield" are known-good. Generic Catalex
boards on AliExpress work but may lack card-detect or level shifter.

## Software / libraries

- Arduino `SD` / `SD_MMC` — simple FAT32 read/write.
- `SdFat` — faster, supports exFAT, long filenames, ring buffers.
- ESP-IDF `sdmmc_host` — lowest-level, required for 4-bit mode performance.

## Notes for WeftOS

Storage should be modeled as a **persistence sink** attached to the event
pipeline, with async flushing and explicit brown-out guards. The substrate
should expose `sd_mounted`, `card_present`, and `write_bytes` as first-class
telemetry so WeftOS can surface "logging is silently failing" conditions.
