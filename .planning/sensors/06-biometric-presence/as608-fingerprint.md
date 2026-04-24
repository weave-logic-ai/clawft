---
title: "AS608 / R307 – Optical Fingerprint Module"
slug: "as608-fingerprint"
category: "06-biometric-presence"
part_numbers: ["AS608", "R307", "R503"]
manufacturer: "Synochip / Hilink / various"
interface: ["UART"]
voltage: "3.3V – 6V"
logic_level: "3.3V (most modules); some 5V TTL variants"
current_ma: "~50 mA active, few mA idle"
price_usd_approx: "$8 – $25"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [biometric, fingerprint, uart, access-control]
pairs_with:
  - "../07-control-actuators/relay-modules.md"
  - "../04-light-display/ws2812b-neopixel.md"
  - "../08-communication/mfrc522-rfid.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=fingerprint" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=fingerprint" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=fingerprint" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-fingerprint.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-AS608-fingerprint.html" }
libraries:
  - { name: "Adafruit-Fingerprint-Sensor-Library", url: "https://github.com/adafruit/Adafruit-Fingerprint-Sensor-Library" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The AS608 and R307 are self-contained **optical fingerprint modules**. The module
handles the imaging sensor, template extraction, on-module template storage, and
1:N matching. The ESP32 just talks a simple UART protocol: enroll, search, match,
delete. Templates live on the module's internal flash (typically 100–1000 slots),
so the host never has to handle raw fingerprint images.

The R503 is a newer capacitive variant with a round glass surface and an integrated
ring LED — pin-compatible in spirit and driven by the same Adafruit library.

## Key specs

| Spec | Value |
|------|-------|
| Interface | UART, typically 57600 8N1 |
| Templates | 100 – 1000 on-module slots (varies) |
| Enroll time | 1–2 s |
| Match time | < 1 s (1:N across all stored templates) |
| Supply | 3.3 V – 6 V |
| Current | ~50 mA active |

## Interface & wiring

Four wires: `VCC / GND / TX / RX`. Use a dedicated hardware UART on the ESP32 if
possible (UART1 or UART2); SoftwareSerial works at 57600 but is less reliable.
Some modules expose a separate `WAKEUP` pin so the MCU only powers the optics when
a finger is present.

## Benefits

- Matching is done **on-module** — no fingerprint image or template ever has to
  touch WeftOS host memory.
- Dead-simple protocol; enrollment + matching in a few dozen lines.
- Cheap enough to use as a "physical password" anywhere.

## Limitations / gotchas

- Optical sensors are spoofable with high-resolution prints + gelatin; capacitive
  R503 is better but not immune.
- Dry, cold, or worn fingertips can fail to enroll/match — budget for fallback.
- The on-module template DB must be **backed up** (via `DownloadCharacter` / vendor
  tool) or you lose enrolments on module failure.
- Biometric data is legally regulated in many jurisdictions (GDPR, BIPA in Illinois,
  etc.) — even on-module storage may count.

## Typical use cases

- DIY smart lock / workbench access control (pair with a relay).
- Per-user mode selection on a shared appliance (kiosk mode).
- Tamper-evident drawer / box.

## Pairs well with

- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) —
  fingerprint match → relay energizes lock.
- [`../04-light-display/ws2812b-neopixel.md`](../04-light-display/ws2812b-neopixel.md) —
  visual feedback ring around the reader.
- [`../08-communication/mfrc522-rfid.md`](../08-communication/mfrc522-rfid.md) —
  two-factor (card + finger) access control.

## Where to buy

See the `buy:` block. These modules are sold as bare fingerprint readers with
ribbon connectors; pick one with a plastic bezel if it will be user-facing.

## Software / libraries

- Adafruit's `Adafruit-Fingerprint-Sensor-Library` handles AS608 / R307 / R503.
- Vendors often ship a Windows SDK for bulk template management; worth using
  for enrolment farms.

## Notes for WeftOS

WeftOS should **never** serialize raw fingerprint templates into shared memory
namespaces. The module's match ID (an integer slot number) is the only thing
that should cross the MCU↔cloud boundary, and even then it should be hashed
with a per-device salt.
