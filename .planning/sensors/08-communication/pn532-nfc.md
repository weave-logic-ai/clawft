---
title: "PN532 – NFC Reader / Writer / Emulator"
slug: "pn532-nfc"
category: "08-communication"
part_numbers: ["PN532"]
manufacturer: "NXP Semiconductors"
interface: ["I2C", "SPI", "UART"]
voltage: "2.7–5.4V"
logic_level: "3.3V (5V tolerant on most breakouts)"
current_ma: "~50–100 mA active"
price_usd_approx: "$10 – $25"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [nfc, rfid, 13.56mhz, iso14443, iso15693, emulation, p2p]
pairs_with:
  - "../08-communication/mfrc522-rfid.md"
  - "../07-control-actuators/solenoids.md"
  - "../10-storage-timing-power/index.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=PN532" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=PN532" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=PN532" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-PN532.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-PN532.html" }
datasheet: "https://www.nxp.com/products/PN532"
libraries:
  - { name: "Adafruit-PN532", url: "https://github.com/adafruit/Adafruit-PN532" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The PN532 is NXP's flagship 13.56 MHz NFC front-end and sits a generation
above the [MFRC522](mfrc522-rfid.md): it reads and writes the same Mifare
family **plus** Type 1/2/3/4 NFC tags, FeliCa, and NFC-DEP P2P, and — crucially
— can **emulate a card** to an external reader (tap-to-pay-style demos, phone
interaction). Breakouts ship with DIP switches to pick between I²C, SPI, and
UART; PCB or round-coil antennas are both common.

## Key specs

| Spec | Value |
|------|-------|
| Frequency | 13.56 MHz |
| Supported tags | Mifare Classic/Plus/DESFire, NTAG, FeliCa, ISO14443A/B |
| Modes | Reader, writer, card emulation, NFC peer-to-peer |
| Interfaces | I²C / SPI / HSU (UART) — switch-selectable |
| Range | ~3–5 cm PCB antenna, larger with external coil |
| Supply | 2.7–5.4 V |

## Interface & wiring

Flick DIP switches on the breakout to pick I²C (default), SPI, or UART.
Adafruit's breakout adds a 3.3 V LDO and level translators, so you can
power it from 5 V and still run 3.3 V logic. `IRQ` lets the chip wake the
MCU when a tag is in the field. Longer-range designs use an external
antenna coil — tuning matters; badly-tuned antennas cut range in half.

## Benefits

- Full NFC stack, including card emulation (unusual at this price).
- Android phones pair as NFC peers → "tap to transfer config".
- Works at 5 V if the module has a translator (Adafruit does, cheap clones
  don't).
- I²C / SPI / UART flexibility; fits any pin budget.

## Limitations / gotchas

- Card emulation is limited vs. a real smartphone; you can emulate Type 4A
  tags and simple NDEF but not full EMV payment.
- Antenna tuning and return-loss matter far more than with MFRC522 — bad
  coil placement silently halves range.
- The switch-to-UART mode uses 115200 8N1 but is less battle-tested than
  the I²C / SPI paths; some clones don't honor the switch.
- NFC is a family of standards: "my tag doesn't read" often means "my tag
  is NFC-V / ISO15693" — the PN532 doesn't cover 15693. Use a PN5180 if you
  need it.

## Typical use cases

- Smart locks that can also read NFC-enabled phones.
- Tap-to-pair kiosks / configuration stations (write WiFi creds to NFC tag,
  phone reads).
- Inventory with mixed Mifare / NTAG / FeliCa fleet.
- NDEF-driven UI — "tap this tag to switch scene".

## Pairs well with

- [`mfrc522-rfid.md`](mfrc522-rfid.md) — cheaper alternative when you only
  need Mifare reads.
- [`../07-control-actuators/solenoids.md`](../07-control-actuators/solenoids.md) —
  canonical lock actuator.
- [`../10-storage-timing-power/index.md`](../10-storage-timing-power/index.md) —
  pair with a LiPo fuel gauge for battery-powered NFC readers.

## Where to buy

See `buy:`. Adafruit's PN532 Shield / breakout is the reference; the Elechouse
NFC V3 board is the canonical cheap variant with PCB antenna.

## Software / libraries

- `Adafruit-PN532` — works over I²C, SPI, HSU.
- `Seeed_Arduino_NFC` — NDEF helpers.
- ESPHome's `pn532` platform wraps reads cleanly for Home Assistant.

## Notes for WeftOS

Card-emulation rules must be feature-gated — emulating a card inside a
WeftOS device is a security-sensitive action and should require explicit
user opt-in. Never log raw UIDs to cloud; hash with a per-device salt.
