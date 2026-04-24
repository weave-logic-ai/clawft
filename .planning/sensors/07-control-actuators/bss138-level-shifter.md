---
title: "BSS138 Bidirectional Logic-Level Shifter"
slug: "bss138-level-shifter"
category: "07-control-actuators"
part_numbers: ["BSS138"]
manufacturer: "onsemi / Nexperia / various"
interface: ["I2C", "GPIO"]
voltage: "1.8V – 5V on either side"
logic_level: "3.3V ↔ 5V (most common use)"
current_ma: "passive; limited by FET on-resistance"
price_usd_approx: "$1 – $4 per 4-channel board"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [level-shifter, i2c, bidirectional, mosfet, interface]
pairs_with:
  - "../08-communication/tca9548a-i2c-mux.md"
  - "../06-biometric-presence/max30102-max30105.md"
  - "../07-control-actuators/relay-modules.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=BSS138+level+shifter" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=level+shifter" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=level+shifter" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-level+shifter.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-BSS138-level-shifter.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

BSS138 level-shifter boards are the canonical four-channel bidirectional
logic-level translator. Each channel is a single BSS138 N-channel MOSFET with
pull-ups on both sides, implementing the "Philips AN97055" trick that lets
I²C and slow GPIO pass cleanly between 3.3 V (ESP32) and 5 V (Arduino-era
peripherals) — in either direction, without a dedicated translator IC.

## Key specs

| Spec | Value |
|------|-------|
| Channels | 4 per board (stackable) |
| Voltage range | 1.8 V – 5 V on either side |
| Speed | Suitable for I²C fast-mode (400 kHz), some UART up to ~1 Mbps |
| Direction | Bidirectional per channel |
| Pull-ups | 10 kΩ on each side, usually populated |

## Interface & wiring

Two rails on the board: `LV` (low-voltage side, tie to 3.3 V) and `HV` (high
side, tie to 5 V). Four pairs of channels sit between them. Wire ESP32 SDA /
SCL to the LV side, 5 V peripheral SDA / SCL to the HV side, common both
grounds.

## Benefits

- Simple, cheap, works for both I²C and GPIO direction-changing signals.
- Four channels on one board is enough for an I²C bus plus two utility lines.
- No configuration, no logic direction pin.

## Limitations / gotchas

- **Not for high-speed SPI.** The RC response from pull-ups + FET capacitance
  limits reliable speed to ~1 MHz; SPI peripherals will corrupt.
- Onboard pull-ups plus the ones inside the master plus any other device on
  the bus can add up. On shared I²C busses you may need to remove the
  shifter's pull-ups.
- Not a buffer. It doesn't drive capacitive loads on long cables; keep runs
  < 30 cm.

## Typical use cases

- Wiring a 5 V hobby sensor (old BMPs, SD-card breakouts, MAX7219) to an
  ESP32.
- Isolating ESP32 from the 5 V `SDA`/`SCL` on sensor shields.
- Talking to 5 V-logic UART peripherals (GPS modules, some camera control).

## Pairs well with

- [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md) —
  combine with the shifter when some downstream branches are 5 V.
- [`../06-biometric-presence/max30102-max30105.md`](../06-biometric-presence/max30102-max30105.md) —
  specific example of a 5 V-module-on-3.3 V-ESP32 case.
- [`relay-modules.md`](relay-modules.md) — many 5 V relay boards fire reliably
  from 3.3 V directly, but the shifter fixes the odd case that doesn't.

## Where to buy

See `buy:`. Adafruit's 4-channel board is the reference design; Sparkfun,
Seeed, and AliExpress clones are all mechanically the same.

## Software / libraries

None. It's transparent to firmware.

## Notes for WeftOS

WeftOS sensor-graph nodes don't model the level shifter — it's a hardware
concern. But treat "bus voltage domain" as a per-device property so the graph
can flag mis-wired configurations during bring-up.
