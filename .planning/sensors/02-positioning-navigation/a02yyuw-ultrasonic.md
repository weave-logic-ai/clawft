---
title: "A02YYUW Waterproof UART Ultrasonic"
slug: "a02yyuw-ultrasonic"
category: "02-positioning-navigation"
part_numbers: ["A02YYUW", "DFRobot SEN0311"]
manufacturer: "DFRobot / various"
interface: ["UART"]
voltage: "3.3V / 5V"
logic_level: "3.3V compatible"
current_ma: "~8 mA typical"
price_usd_approx: "$10 – $20"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [ultrasonic, waterproof, uart, a02yyuw, dfrobot]
pairs_with:
  - "./jsn-sr04t-ultrasonic.md"
  - "./hc-sr04-ultrasonic.md"
buy:
  - { vendor: DFRobot, url: "https://www.dfrobot.com/product-1935.html" }
  - { vendor: Seeed, url: "https://www.seeedstudio.com/catalogsearch/result/?q=A02YYUW" }
  - { vendor: Amazon, url: "https://www.amazon.com/s?k=A02YYUW" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-A02YYUW.html" }
libraries:
  - { name: "DFRobot/DFRobot_RaspberryPi_A02YYUW", url: "https://github.com/DFRobot/DFRobot_RaspberryPi_A02YYUW" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

DFRobot's A02YYUW is a cleaner take on the JSN-SR04T: waterproof transducer, UART-only output
(no trigger-pulse timing), and 3.3 V-friendly logic. Distance arrives as a short framed packet
every ~100 ms — ideal for ESP32 where you don't want to burn a hardware timer on pulse-width
measurement.

## Key specs

| Spec | Value |
|---|---|
| Range | ~3 cm – 450 cm |
| Resolution | 1 mm |
| Accuracy | ±1% of reading |
| Output | 9600-baud UART frames (4 bytes: header 0xFF + high + low + checksum) |
| Voltage | 3.3 V or 5 V |
| Transducer | Waterproof, cable-mounted |

## Interface & wiring

VCC, GND, RX, TX to an ESP32 UART. Leave the UART at 9600 baud and parse the 4-byte frames. A
500 ms timeout + checksum verify is plenty of error handling.

## Benefits

- No timing-critical code — plain UART consumer.
- 3.3 V native.
- Better minimum range than JSN-SR04T (~3 cm vs ~25 cm).

## Limitations / gotchas

- More expensive than JSN-SR04T.
- Cable length is fixed (~1 m typical); longer runs need clean power.
- Only one sample every ~100 ms — not for fast control loops.

## Typical use cases

- Water-tank level with wet-safe transducer.
- Outdoor proximity (mailbox, garage, gate).
- Industrial bin-fill monitoring.

## Pairs well with

- [`./jsn-sr04t-ultrasonic.md`](./jsn-sr04t-ultrasonic.md) — cheaper trigger/echo sibling.
- [`./hc-sr04-ultrasonic.md`](./hc-sr04-ultrasonic.md) — dry indoor alternative.

## Where to buy

- DFRobot (original SEN0311).
- Seeed / Amazon / AliExpress for resales and clones.

## Software / libraries

- `DFRobot/DFRobot_RaspberryPi_A02YYUW` — small and readable; easy to port.

## Notes for WeftOS

Speculative: good baseline example of the "framed UART sensor" contract — WeftOS's sensor
driver layer should ship a generic 4-byte-frame reader so modules like this need ~20 lines
of integration, not 200.
