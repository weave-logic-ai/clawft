---
title: "DHT11 / DHT22 (AM2302) Temperature & Humidity"
slug: "dht11-dht22"
category: "05-environmental"
part_numbers: ["DHT11", "DHT22", "AM2302"]
manufacturer: "Aosong"
interface: ["GPIO (proprietary 1-wire)"]
voltage: "3.3V / 5V"
logic_level: "3.3V"
current_ma: "~1 mA measurement; ~50 µA standby"
price_usd_approx: "$2 (DHT11) – $5 (DHT22)"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["temperature", "humidity", "legacy", "cheap"]
pairs_with:
  - "./bme280.md"
  - "./aht10-aht20-aht30.md"
  - "./sht31.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=dht22" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=dht22" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=dht22" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-dht22.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-dht22.html" }
libraries:
  - "Rust: dht-sensor"
  - "Arduino: DHT sensor library (Adafruit), DHTesp"
  - "ESP-IDF: esp-idf-lib DHT component"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

DHT11 and DHT22 (AM2302 is the wired-lead version of DHT22) are the classic low-cost combined temperature/humidity modules from Aosong. They are the hobby default but show their age next to modern I²C parts like [SHT31](./sht31.md) and [BME280](./bme280.md). For WeftOS, they're fine for "is it roughly room-temp?" but should not be trusted for HVAC control or compliance logging.

## Key specs

| | DHT11 | DHT22 / AM2302 |
|-|-------|----------------|
| T range | 0 – 50 °C | −40 – 80 °C |
| T accuracy | ± 2 °C | ± 0.5 °C |
| RH range | 20 – 90 % | 0 – 100 % |
| RH accuracy | ± 5 % | ± 2 – 5 % |
| Sample rate | 1 Hz max | 0.5 Hz max |

- Interface: proprietary one-wire "pulse-length" protocol — **not** Maxim 1-Wire, **not** I²C.
- Supply: 3.3 V or 5 V; many breakouts include a 10 kΩ pull-up already on the data line.

## Interface & wiring

- `VCC`, `GND`, `DATA` (with ~10 kΩ pull-up to `VCC`).
- Protocol is a timing-sensitive sequence of pulses; most MCU drivers bit-bang it with interrupts disabled for the ~25 ms exchange.
- On ESP32, use a driver that uses RMT (like `esp-idf-lib`) — task preemption will otherwise corrupt reads.

## Benefits

- Cheap, widely stocked, zero board-level complexity.
- Tolerates 5 V, so works with Arduino-era modules.
- Many tutorials and libraries; good first-sensor for learners.

## Limitations / gotchas

- **Slow** — one sample per second (DHT11) or per two seconds (DHT22). Useless for any fast transient.
- Timing-sensitive protocol; unreliable on multi-tasking MCUs unless the driver handles interrupts right.
- RH accuracy drifts with age and condensation exposure; not suitable for HVAC control.
- Often one CRC/parity fail per ~20 reads — **always** retry and median-filter.
- Clones are common; some "DHT22" modules are actually DHT11 with a relabeled case.

## Typical use cases

- Toy / demo / first-build weather station.
- Non-critical "is this room hot?" status.
- Legacy node replacement where an upgrade isn't worth the effort.

## Pairs well with

- [`./bme280.md`](./bme280.md) — cleaner upgrade path (adds pressure, I²C, faster).
- [`./aht10-aht20-aht30.md`](./aht10-aht20-aht30.md) — same price band, better accuracy, I²C.
- [`./sht31.md`](./sht31.md) — the "do it right" choice for T/RH.

## Where to buy

- Adafruit / SparkFun / Seeed (genuine Aosong parts).
- AliExpress bulk (expect clones; accept 10 % failure rate).

## Software / libraries

- `dht-sensor` (Rust, bit-banged via `embedded-hal`).
- `esp-idf-lib` DHT component for RMT-driven reads on ESP32.
- Adafruit `DHT sensor library` is the reference on Arduino.

## Notes for WeftOS

- Gate new deployments to modern I²C parts; prefer [BME280](./bme280.md) or [AHT20](./aht10-aht20-aht30.md) for any new node.
- The HAL should surface `TempHumiditySensor` as a trait; DHT22 is one of several implementations.
- Never publish DHT11/22 readings as "accurate" telemetry without error bars (±2 °C / ±5 % RH).
