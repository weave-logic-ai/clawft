---
title: "RDM6300 – 125 kHz LF RFID Reader"
slug: "rdm6300-lf-rfid"
category: "08-communication"
part_numbers: ["RDM6300", "EM4100"]
manufacturer: "various"
interface: ["UART"]
voltage: "5V"
logic_level: "TTL (5V, but 3.3V ESP32 usually reads it fine)"
current_ma: "~50 mA"
price_usd_approx: "$3 – $8"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [rfid, 125khz, em4100, uart, legacy]
pairs_with:
  - "../08-communication/mfrc522-rfid.md"
  - "../07-control-actuators/relay-modules.md"
  - "../07-control-actuators/solenoids.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=125kHz+RFID" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=125kHz+RFID" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=RDM6300" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-RDM6300.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-RDM6300.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The RDM6300 is a cheap 125 kHz low-frequency RFID reader for EM4100-family
cards and keyfobs — the thick round tokens and clamshell badges that predate
modern Mifare. It ships as a PCB with an attached coil; on a card-present
event it streams a 14-byte ASCII packet over UART containing the card's
decimal UID. If your building, gym, or hotel still uses 125 kHz fobs, this
is the reader.

## Key specs

| Spec | Value |
|------|-------|
| Frequency | 125 kHz |
| Tag family | EM4100 / EM4102 / EM4305 |
| Interface | UART, 9600 8N1 |
| Range | ~5 cm with the stock coil |
| Supply | 5 V |
| Output | 14-byte ASCII on detect |

## Interface & wiring

Four pins that matter: `5V / GND / TX / RX`. Wire the module's `TX` into an
ESP32 RX GPIO through a level-check (the 5 V TTL output is typically OK on
ESP32's 3.3 V input in practice, but a divider is safer). On card present
the module spits the UID; there's no command set beyond "power on and
listen".

## Benefits

- Trivially simple — read UART, parse ASCII, done.
- 125 kHz penetrates thicker enclosures and wallets than 13.56 MHz.
- Cheap and reliable for the one job it does.

## Limitations / gotchas

- **EM4100 is unprotected.** The UID is the whole token; anyone with an
  RFID cloner (sub-$20 on AliExpress) can duplicate a fob in seconds. For
  anything more than "which staff member is here", pair with a second
  factor.
- 125 kHz cards and 13.56 MHz cards are **not interchangeable**. An MFRC522
  cannot read an EM4100 and vice versa — pick one ecosystem or install both
  readers.
- Range drops sharply with metal behind the coil.
- Long UART cables pick up noise; keep the reader close to the MCU or use
  RS485.

## Typical use cases

- Legacy-building access upgrades where replacing all fobs isn't practical.
- Cheap member-identification at maker spaces.
- Hobby card-swipe kiosks.

## Pairs well with

- [`mfrc522-rfid.md`](mfrc522-rfid.md) — dual-frequency reader stations.
- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) —
  swipe-to-unlock.
- [`../07-control-actuators/solenoids.md`](../07-control-actuators/solenoids.md) —
  bolt-level locks.

## Where to buy

See `buy:`. Functionally interchangeable clones from AliExpress are the
norm; Seeed and DFRobot sell better-documented variants.

## Software / libraries

No driver needed — it's ASCII UART. Simple parse-and-verify is 20 lines.
ESPHome has community components for RDM6300.

## Notes for WeftOS

EM4100 UIDs must never be used as the sole auth factor in a WeftOS access
rule. The rules engine should flag any rule that treats a 125 kHz read as
identity-proof without a second factor.
