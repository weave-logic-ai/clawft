---
title: "Heltec / RAK – Integrated LoRa + ESP32 Boards"
slug: "heltec-rak-lora"
category: "08-communication"
part_numbers: ["Heltec WiFi LoRa 32 V3", "LilyGo T-Beam", "RAK4631", "RAK3172"]
manufacturer: "Heltec / LilyGo / RAKwireless"
interface: ["SPI (onboard)"]
voltage: "USB / LiPo"
logic_level: "3.3V"
current_ma: "depends on ESP32 + radio duty"
price_usd_approx: "$20 – $50"
esp32_compat: ["ESP32", "ESP32-S3"]
tags: [lora, esp32, dev-board, integrated, heltec, rak, lilygo, board]
pairs_with:
  - "../08-communication/sx1262-lora.md"
  - "../08-communication/sx1276-rfm95-lora.md"
  - "../10-storage-timing-power/microsd.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=Heltec+LoRa" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=LoRa+board" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=Heltec+LoRa" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-heltec.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-Heltec-LoRa-32.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

This entry is a **pointer**, not a standalone module. Several vendors
integrate an ESP32 (or nRF52) with a Semtech SX1262 / SX1276, a LiPo charger,
and often an OLED / GPS on one PCB — the result is a ready-to-flash LoRa
node. Notable names:

- **Heltec WiFi LoRa 32 V3** — ESP32-S3 + SX1262 + 0.96" OLED.
- **LilyGo T-Beam** — ESP32 + SX1276/SX1262 + NEO-6M/M8 GPS + 18650 holder.
- **LilyGo T-LoRa** — minimal ESP32 + SX1262 board.
- **RAK3172 / RAK4631** — STM32 + SX1262 modules, not ESP32, but used in
  many ESP32 projects via UART AT-command bridge.

If you just need "an ESP32 that speaks LoRa", grab one of these instead of
wiring a bare [RFM95](sx1276-rfm95-lora.md) to a dev board.

## Key specs

- Integrated ESP32 (or ESP32-S3) dev board
- SX1262 or SX1276 radio, 868 / 915 MHz variants
- LiPo charger + battery connector
- OLED display, reset / user buttons
- U.FL or SMA antenna connector

## Interface & wiring

None — it's a board. Each vendor publishes a pin map that tells you which
ESP32 pins connect to the radio's `NSS / SCK / MISO / MOSI / RST / DIO1 /
BUSY`. Plug USB-C, flash firmware.

## Benefits

- Zero wiring mistakes.
- Proper antenna matching and PCB layout — better RF than hand-wired.
- Come with reference firmware (Meshtastic, ESPHome, vendor examples).

## Limitations / gotchas

- **Pin maps vary between board revisions** — copy the exact `#define` set
  for your revision or the radio won't talk.
- Some revisions tie the radio's `BUSY` or `RESET` to GPIOs needed for SD
  cards or the OLED. Expect conflicts and read the schematic.
- LilyGo "T-Beam" GPS quality varies — early batches shipped mismatched
  antennas.
- "Heltec" branded clones exist on AliExpress that have subtly different
  pinouts.

## Typical use cases

- Meshtastic off-grid messaging.
- LoRaWAN sensor nodes (TTN / ChirpStack).
- Field deployable telemetry without hand-soldering.

## Pairs well with

- [`sx1262-lora.md`](sx1262-lora.md) — the radio inside modern revisions.
- [`sx1276-rfm95-lora.md`](sx1276-rfm95-lora.md) — the radio inside older
  revisions.
- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md) —
  many boards expose an SD slot for store-and-forward.

## Where to buy

See `buy:`. Heltec and LilyGo sell direct via their own stores; RAK via
`store.rakwireless.com`. Amazon and AliExpress resell.

## Software / libraries

- Meshtastic has first-class support for most variants.
- `RadioLib` picks up Heltec / LilyGo via community pin-map defines.
- ESPHome supports Heltec V3 as a built-in target.

## Notes for WeftOS

Integrated boards simplify WeftOS node bring-up but hard-code peripheral
pins. Parameterize the board in your config so the same WeftOS firmware
image can target multiple revisions.
