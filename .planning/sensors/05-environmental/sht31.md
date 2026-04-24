---
title: "Sensirion SHT31 Temperature & Humidity"
slug: "sht31"
category: "05-environmental"
part_numbers: ["SHT31-DIS", "SHT30", "SHT35"]
manufacturer: "Sensirion"
interface: ["I2C"]
voltage: "3.3V (2.4 – 5.5 V on die)"
logic_level: "3.3V"
current_ma: "~1.5 mA measuring; ~0.2 µA sleep"
price_usd_approx: "$10 – $18"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["temperature", "humidity", "i2c", "precision", "industrial"]
pairs_with:
  - "./bme280.md"
  - "./aht10-aht20-aht30.md"
  - "./ds18b20.md"
  - "../08-communication/tca9548a-i2c-mux.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=sht31" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=sht31" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=sht31" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-sht31.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-sht31.html" }
datasheet: "https://sensirion.com/products/catalog/SHT31-DIS-B/"
libraries:
  - "Rust: shtcx, sht3x crates"
  - "Arduino: Adafruit_SHT31, SHT31 (Sensirion)"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The SHT31 from Sensirion is the "do it right" T/RH sensor — ± 0.2 °C, ± 2 % RH, factory-calibrated, I²C, on-chip heater for condensation recovery. For WeftOS it's the choice when you actually need to trust a humidity reading (HVAC control, greenhouse, lab logging) rather than just display one.

## Key specs

- Temperature: −40 to +125 °C, ± 0.2 °C (best-case).
- Humidity: 0–100 % RH, ± 2 %.
- Interface: I²C, address 0x44 (ADDR low) or 0x45 (ADDR high).
- On-chip heater (~3–33 mW) to dry off the sensor after condensation.
- Full NIST-traceable calibration from Sensirion.

## Interface & wiring

- I²C: `SDA`, `SCL`, `VCC`, `GND`, optional `ALERT` and `RESET` pins.
- Clock-stretching supported — some ESP32 I²C drivers need clock-stretching enabled in config.
- Heater is software-enabled via the command register; leave **off** for normal measurements (heater shifts RH readings while running).
- Keep sensor physically far from any source of warmth — a 1 °C bias halves RH accuracy.

## Benefits

- Best-in-class accuracy at this price point.
- Factory-calibrated and traceable — suitable for regulatory / lab work.
- Heater feature eliminates "sensor stuck at 100 % RH" after condensation.
- Real industrial pedigree (Sensirion) with long datasheet lifecycle.

## Limitations / gotchas

- Pricier than AHT20 / BME280 — overkill for casual use.
- Address 0x44/0x45 collides with AS7341 and some TMP117 variants; watch your bus.
- No pressure channel — if you need P too, use [BME280](./bme280.md) and accept lower RH accuracy, or run both.
- I²C clock stretching requires proper driver config on ESP32; misconfig shows up as random NAKs.
- Still drifts ~0.25 % RH / yr if exposed to aerosols (cooking, solvents).

## Typical use cases

- HVAC / cleanroom monitoring.
- Calibrated reference sensor that all other T/RH in a WeftOS deployment are cross-checked against.
- Outdoor weather station with condensation (heater clears morning dew).
- Incubators, fermenters, lab instruments.

## Pairs well with

- [`./bme280.md`](./bme280.md) — BME280 for pressure, SHT31 for accurate T/RH.
- [`./aht10-aht20-aht30.md`](./aht10-aht20-aht30.md) — cheaper T/RH on peripheral nodes; SHT31 as the reference.
- [`./ds18b20.md`](./ds18b20.md) — waterproof T-only complement.
- [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md) — if 0x44/0x45 clashes.

## Where to buy

- Adafruit, SparkFun, Sensirion direct, DigiKey, Mouser.
- AliExpress — clones exist but are rarer; prefer named vendors for SHT31.

## Software / libraries

- `shtcx` / `sht3x` Rust crates.
- `Adafruit_SHT31` for Arduino.
- Sensirion publishes a reference C driver.

## Notes for WeftOS

- Make SHT31 the *gold-standard* T/RH source in the HAL; other T/RH sensors get a "peripheral" flag and are offset-calibrated against it at setup time.
- Expose the heater as a discrete actuator; the HAL should auto-enable after "RH pinned at 100 %" for >N minutes and then re-measure after cool-down.
- Store Sensirion serial number (readable over I²C) in the device record for audit trail.
