---
title: "ASAIR AHT10 / AHT20 / AHT30 Temperature & Humidity"
slug: "aht10-aht20-aht30"
category: "05-environmental"
part_numbers: ["AHT10", "AHT20", "AHT21", "AHT30"]
manufacturer: "ASAIR (Aosong)"
interface: ["I2C"]
voltage: "3.3V (2.2 – 5.5 V on die)"
logic_level: "3.3V"
current_ma: "~0.25 mA measuring; ~0.25 µA standby"
price_usd_approx: "$2 – $5"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["temperature", "humidity", "i2c", "cheap", "upgrade-from-dht"]
pairs_with:
  - "./dht11-dht22.md"
  - "./bme280.md"
  - "./sht31.md"
  - "../08-communication/tca9548a-i2c-mux.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=aht20" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=aht20" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=aht20" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-aht20.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-aht20.html" }
libraries:
  - "Rust: aht20-driver, aht10 crate"
  - "Arduino: Adafruit_AHTX0, AHT20"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

ASAIR's AHT series is the modern, I²C-based replacement for the DHT11/DHT22. AHT10 is the original; AHT20 fixes accuracy and calibration issues; AHT21/AHT30 are pin-compatible improvements. For WeftOS it's the right "cheap T/RH" part — more accurate than DHT22, faster, and on a real bus.

## Key specs

- Temperature: −40 to +85 °C.
- Accuracy (AHT20): ± 0.3 °C T, ± 2 % RH.
- Accuracy (AHT10): ± 0.3 °C T, ± 3 % RH (and was famously unstable at RH extremes).
- Interface: I²C, fixed address 0x38.
- Sample rate: up to ~10 Hz (with >100 ms measurement time).

## Interface & wiring

- I²C: `SDA`, `SCL`, `VCC`, `GND`. Address 0x38 is **fixed** — can't change without a mux.
- 3.3 V recommended; 5 V-tolerant on many breakouts.
- Soft-reset command `0xBA` at boot clears stale state.
- Two reads of status register needed after triggering a measurement; library handles this.

## Benefits

- Cheaper than BME280 and almost as accurate on T/RH.
- Real I²C — works on multitasking MCUs without timing grief.
- Very low power in standby.
- Compact (3×3 mm or smaller DFN).

## Limitations / gotchas

- **Fixed address 0x38** — two AHT20s on one bus need a [TCA9548A mux](../08-communication/tca9548a-i2c-mux.md).
- Also conflicts with FT6206 capacitive touch controllers on 0x38.
- AHT10 is known for drift and condensation-induced errors; prefer AHT20 or newer.
- No pressure channel; use [BME280](./bme280.md) if you need T/RH/P in one chip.
- Pinout silk on cheap breakouts is sometimes wrong — verify with a multimeter before powering.

## Typical use cases

- Drop-in DHT22 replacement.
- Grow tents, terrariums, baby monitors (non-safety-critical).
- Small I²C environmental nodes on battery.

## Pairs well with

- [`./dht11-dht22.md`](./dht11-dht22.md) — the predecessor you're probably replacing.
- [`./bme280.md`](./bme280.md) — adds pressure.
- [`./sht31.md`](./sht31.md) — higher accuracy if you need ± 0.2 °C / ± 1.5 % RH.
- [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md) — mandatory for multi-AHT20 setups.

## Where to buy

- Adafruit (AHT20 Qwiic breakout), SparkFun, Pimoroni, Seeed.
- AliExpress for bulk — quality is consistent enough for indoor use.

## Software / libraries

- `aht20-driver` Rust crate (embedded-hal).
- Adafruit_AHTX0 (Arduino) is the reference.
- Bare I²C is also straightforward — the datasheet protocol fits in ~50 lines.

## Notes for WeftOS

- Register as the default T/RH implementation when no BME280 / SHT31 is present.
- Emit a warning if a second 0x38 device is detected (address collision).
- Budget 100 ms measurement latency when duty-cycling — don't poll faster than 5 Hz.
