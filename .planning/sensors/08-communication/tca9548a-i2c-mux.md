---
title: "TCA9548A – 8-Channel I²C Multiplexer"
slug: "tca9548a-i2c-mux"
category: "08-communication"
part_numbers: ["TCA9548A", "PCA9548A"]
manufacturer: "Texas Instruments (TCA9548A) / NXP (PCA9548A)"
interface: ["I2C"]
voltage: "1.65–5.5V"
logic_level: "3.3V or 5V"
current_ma: "< 1 mA"
price_usd_approx: "$2 – $7"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [i2c, mux, multiplexer, address-collision, bus-expansion]
pairs_with:
  - "../07-control-actuators/bss138-level-shifter.md"
  - "../07-control-actuators/pca9685-servo-driver.md"
  - "../06-biometric-presence/max30102-max30105.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=TCA9548A" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=TCA9548A" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=TCA9548A" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-TCA9548A.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-TCA9548A.html" }
datasheet: "https://www.ti.com/product/TCA9548A"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The TCA9548A is a **1-to-8 I²C multiplexer**: one upstream SCL/SDA pair from
the ESP32 fans out to eight independent downstream branches, of which you
enable one (or multiple) at a time by writing a single byte to the mux's
address register. The classic use case is four identical I²C sensors whose
addresses collide — mount one per branch, select a branch before talking,
done.

## Key specs

| Spec | Value |
|------|-------|
| Upstream | Single I²C master |
| Downstream | 8 branches (SD0..SD7 / SC0..SC7) |
| Control | Single 8-bit register, bit N = branch N enabled |
| Addresses | 0x70–0x77 (3 hardware address pins) |
| Supply | 1.65 – 5.5 V |
| Speed | Passes I²C through up to 400 kHz fast-mode |

## Interface & wiring

Upstream `SCL/SDA` from the ESP32 to the mux's `SCL/SDA`. Each downstream
branch has its own `SDn/SCn` pair with its own pull-ups. Up to eight
TCA9548As on one bus via `A0/A1/A2` address pins. `RESET` is active-LOW —
tie high or wire a GPIO for recovery.

Flow: write `0x04` to the mux to enable only branch 2, talk to a device on
branch 2, write `0x00` to deselect.

## Benefits

- Breaks I²C address collisions without changing sensor firmware.
- Lets you run multiple otherwise-identical sensors (e.g. eight MAX30102s).
- Isolates downstream branches electrically so one faulty pull-up doesn't
  kill the whole bus.
- Daisy-chain up to 8 muxes = 64 isolated channels on one upstream.

## Limitations / gotchas

- You **must** select the branch before every transaction (or keep
  transactions to one branch at a time). Forgetting this is the #1 "my mux
  is broken" report.
- The mux adds a small propagation delay; very-fast I²C (> 400 kHz) on a
  long cable fan-out gets marginal.
- If an I²C device on a selected branch locks up the SDA line, the mux
  cannot rescue the bus — you must toggle its `RESET`.
- Power the mux from the **lowest** voltage in the system (usually 3.3 V);
  mixing 3.3 V master + 5 V downstream needs a level shifter on each branch.

## Typical use cases

- Sensor arrays ("a grid of eight MAX30102s").
- Multiple I²C OLEDs on fixed 0x3C / 0x3D addresses.
- Daisy-chaining a 5 V legacy sensor behind a [BSS138 level shifter](../07-control-actuators/bss138-level-shifter.md)
  while keeping the rest of the bus at 3.3 V.
- Escape hatch when two libraries assume the same sensor address.

## Pairs well with

- [`../07-control-actuators/bss138-level-shifter.md`](../07-control-actuators/bss138-level-shifter.md) —
  per-branch voltage translation.
- [`../07-control-actuators/pca9685-servo-driver.md`](../07-control-actuators/pca9685-servo-driver.md) —
  when you run out of PCA9685 addresses.
- [`../06-biometric-presence/max30102-max30105.md`](../06-biometric-presence/max30102-max30105.md) —
  the canonical "I want 8 of them" sensor.

## Where to buy

See `buy:`. Adafruit and SparkFun sell Qwiic-compatible breakouts; the
generic purple CJMCU board from AliExpress is the cheap clone.

## Software / libraries

- Adafruit's `Adafruit_BusIO` / manual `Wire.beginTransmission(0x70)` +
  single-byte write for branch select.
- ESPHome's `i2c` hub with channel-switching via custom lambdas.

## Notes for WeftOS

WeftOS's I²C abstraction should treat `(bus, mux_channel, address)` as the
addressing tuple. Surface mux health (per-channel error counter) so the rule
engine can demote flaky channels automatically.
