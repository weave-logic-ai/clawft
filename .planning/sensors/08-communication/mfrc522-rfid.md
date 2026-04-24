---
title: "MFRC522 – 13.56 MHz RFID Reader"
slug: "mfrc522-rfid"
category: "08-communication"
part_numbers: ["MFRC522", "RC522"]
manufacturer: "NXP Semiconductors"
interface: ["SPI", "I2C", "UART"]
voltage: "2.5–3.3V"
logic_level: "3.3V (NOT 5V tolerant)"
current_ma: "~15–25 mA active"
price_usd_approx: "$1 – $4"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [rfid, nfc, 13.56mhz, mifare, spi]
pairs_with:
  - "../08-communication/pn532-nfc.md"
  - "../08-communication/rdm6300-lf-rfid.md"
  - "../06-biometric-presence/as608-fingerprint.md"
  - "../07-control-actuators/solenoids.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=MFRC522" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=RC522" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=MFRC522" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-RC522.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-MFRC522-RC522.html" }
datasheet: "https://www.nxp.com/products/MFRC522"
libraries:
  - { name: "MFRC522 (miguelbalboa)", url: "https://github.com/miguelbalboa/rfid" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The MFRC522 (sold as "RC522" on every $2 hobby board) is NXP's 13.56 MHz RFID
reader IC. It reads and writes Mifare Classic 1K / 4K / Ultralight cards and
NTAG stickers, which is approximately all of the blue / white hotel-key-card
hobby fobs in existence. The red hobby board pairs the chip with the PCB
antenna and exposes an SPI header (I²C and UART are also possible with
resistor-strap reconfiguration). Cheap, tiny, and the default for any DIY
door-lock / kiosk / inventory project.

## Key specs

| Spec | Value |
|------|-------|
| Frequency | 13.56 MHz |
| Protocols | ISO/IEC 14443A — Mifare Classic, Mifare Ultralight, NTAG |
| Interface | SPI (default); I²C or UART via strap |
| Range | ~3 cm (PCB antenna) |
| Supply | 2.5–3.3 V |
| Current | ~15–25 mA |

## Interface & wiring

Eight pins on the hobby board: `SDA (SS) / SCK / MOSI / MISO / IRQ / GND /
RST / 3.3V`. Wire to an ESP32's default SPI bus (HSPI or VSPI). `IRQ` is
optional — the library polls if you don't connect it. The module is **3.3 V
only**; do not power it from 5 V.

## Benefits

- Costs a dollar.
- Huge library ecosystem — reading UIDs is 10 lines of code.
- Mifare Classic auth / read / write supported.
- Small PCB antenna fits in a keyring-sized enclosure.

## Limitations / gotchas

- **Mifare Classic is broken.** The CRYPTO1 cipher has been exploited for
  over a decade; keys can be extracted with standard tools. Use Mifare Plus
  / DESFire / Ultralight C if you care about security.
- **RFID cloning is trivial.** A blank Mifare "magic card" + MFRC522 + any
  tutorial clones a fob in seconds. Assume every UID can be spoofed.
- ~3 cm range on the PCB antenna; cards must literally touch the module.
- 5 V on VCC will destroy the chip instantly.
- I²C / UART mode requires desoldering straps on most boards — not plug-and-play.

## Typical use cases

- DIY door lock (MFRC522 + [solenoid](../07-control-actuators/solenoids.md)).
- Inventory / laundry bin tracking.
- Kiosk "tap to start" UX.
- Teaching the basics of contactless cards.

## Pairs well with

- [`pn532-nfc.md`](pn532-nfc.md) — upgrade when you need real NFC (card
  emulation, P2P, Type 4 tags).
- [`rdm6300-lf-rfid.md`](rdm6300-lf-rfid.md) — complementary for 125 kHz fobs.
- [`../06-biometric-presence/as608-fingerprint.md`](../06-biometric-presence/as608-fingerprint.md) —
  card + fingerprint = passable two-factor.
- [`../07-control-actuators/solenoids.md`](../07-control-actuators/solenoids.md) —
  the canonical "card → unlock" recipe.

## Where to buy

See `buy:`. The red RC522 board from AliExpress is functionally identical to
Adafruit / SparkFun variants, just less documented.

## Software / libraries

- `miguelbalboa/rfid` is the canonical Arduino / ESP32 library.
- ESPHome exposes the chip via its `rc522_spi` / `rc522_i2c` platforms.

## Notes for WeftOS

RFID UIDs should never be used as the sole auth factor in WeftOS access
rules. Layer with fingerprint, PIN, or time-of-day policy. Store only hashed
UIDs (per-device salt) off the MCU — raw UIDs in cloud logs are re-identifying.
