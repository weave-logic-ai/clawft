---
title: "PCF8591 8-bit ADC + DAC (I²C)"
slug: "pcf8591-adc-dac"
category: "10-storage-timing-power"
part_numbers: ["PCF8591", "PCF8591T"]
manufacturer: "NXP"
interface: ["I2C"]
voltage: "2.5 – 6.0 V (5 V on most breakouts)"
logic_level: "5V (prefer a level shifter for ESP32 I²C)"
current_ma: "~1 mA active"
price_usd_approx: "$1 – $3"
esp32_compat: ["ESP32 via 5V level shifter"]
tags: [adc, dac, i2c, pcf8591, legacy, 5v]
pairs_with:
  - "./ads1115-ads1015.md"
  - "../09-input-hmi/potentiometers.md"
  - "../03-audio-sound/max98357a-amp.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=pcf8591" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=pcf8591" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=pcf8591" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-pcf8591.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-pcf8591.html" }
datasheet: "https://www.nxp.com/products/analog-and-mixed-signal/analog-to-digital-converters-adcs/low-power-8-bit-cmos-data-acquisition-device-with-i2c-bus:PCF8591"
libraries:
  - { name: "PCF8591_Simple", url: "https://github.com/xreef/PCF8591_library" }
  - { name: "ESPHome pcf8591", url: "https://esphome.io/components/sensor/pcf8591.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The NXP PCF8591 is a legacy I²C ADC + DAC: 4 single-ended 8-bit inputs plus 1
single-ended 8-bit output. It's on every Chinese education kit because it's
cheap and simple, but for anything precision-critical the
[`ADS1115`](./ads1115-ads1015.md) is a much better choice. Included here for
completeness and because you'll meet it in the wild.

## Key specs

| Spec | PCF8591 |
|------|---------|
| ADC resolution | 8 bits (~20 mV/LSB at 5 V) |
| ADC channels | 4 single-ended (or 2 diff / mixed) |
| DAC channels | 1 single-ended, 8 bits |
| Sample rate | ~10 ksps (limited by I²C) |
| Supply | 2.5 – 6 V (5 V typical on breakouts) |
| I²C address | 0x48 (collides with ADS1115!) |

## Interface & wiring

- **Almost all breakouts are 5 V**: Vcc **and** the I²C logic level. For a
  3.3 V ESP32, add a bidirectional I²C level shifter on SDA/SCL, and keep the
  PCF8591 on its own 5 V rail for AGND referencing.
- I²C address 0x48 — **collides with ADS1115 / ADS1015**. If you want both on
  the same bus, pull the ADS addr pin.
- `AIN0`–`AIN3` are the analog inputs; `AOUT` is the DAC.
- Onboard breakouts often have a thermistor, LDR, and pot wired to AIN0/1/3 —
  read the silkscreen before assuming a clean 4-channel ADC.

## Benefits

- DAC + ADC in one chip — rare combination in hobbyland.
- Dirt cheap.
- Well-supported libraries.

## Limitations / gotchas

- **Only 8 bits.** That's ~20 mV/LSB at 5 V reference — coarse. Any real
  measurement wants the [`ADS1115`](./ads1115-ads1015.md).
- **5 V part.** Direct wiring to an ESP32 I²C bus will either fail to read or
  damage the MCU over time — always level-shift.
- Address collides with ADS1115.
- DAC output impedance is high (~few kΩ); buffer with an op-amp for real loads.
- No internal reference — accuracy depends on Vcc stability.

## Typical use cases

- Rescuing an "Arduino learner kit" that includes a PCF8591.
- Ultra-low-cost signal generator for test rigs (DAC + Arduino = arbitrary
  waveform at low speed).
- Slow analog I/O on an already-crowded I²C bus when 8 bits is enough.

## Pairs well with

- [`./ads1115-ads1015.md`](./ads1115-ads1015.md) — the modern replacement for the ADC side.
- [`../09-input-hmi/potentiometers.md`](../09-input-hmi/potentiometers.md) — as the downstream ADC.
- [`../03-audio-sound/max98357a-amp.md`](../03-audio-sound/max98357a-amp.md) — for the DAC's audio-ish use case.

## Where to buy

AliExpress PCF8591 modules ~$1.50. Adafruit discontinued theirs; SparkFun has
Qwiic ADS1115 instead.

## Software / libraries

- `PCF8591_Simple` / `PCF8591_library`.
- ESPHome `sensor: platform: pcf8591`.

## Notes for WeftOS

Flag PCF8591 in sensor metadata with `resolution: 8-bit`, `level: 5V`, and the
**I²C address collision** with ADS1115. The WeftOS bus-address allocator should
treat these two parts as mutually exclusive at 0x48 without muxing.
