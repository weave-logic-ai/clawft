---
title: "SX1276 / RFM95 – LoRa Transceiver (868 / 915 MHz)"
slug: "sx1276-rfm95-lora"
category: "08-communication"
part_numbers: ["SX1276", "SX1278", "RFM95", "RFM96", "RFM98"]
manufacturer: "Semtech (SX1276); HopeRF (RFM95 module)"
interface: ["SPI"]
voltage: "1.8–3.7V"
logic_level: "3.3V (NOT 5V tolerant)"
current_ma: "~10 mA RX, ~120 mA TX @ +20 dBm, ~200 nA sleep"
price_usd_approx: "$5 – $15"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [lora, rf, sub-ghz, long-range, spi, semtech, hoperf]
pairs_with:
  - "../08-communication/sx1262-lora.md"
  - "../07-control-actuators/relay-modules.md"
  - "../10-storage-timing-power/microsd.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=RFM95" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=RFM95" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=RFM95" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-LoRa.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-SX1276-RFM95.html" }
datasheet: "https://www.semtech.com/products/wireless-rf/lora-connect/sx1276"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The Semtech SX1276 and its HopeRF module sibling, the RFM95, are the classic
"my WiFi doesn't reach that far" radio. They implement Semtech's LoRa
chirp-spread-spectrum modulation at 868 MHz (EU) or 915 MHz (US), with 6+ km
line-of-sight range at sub-$15 cost. Two more SX127x variants cover other
bands (SX1278 for 433 MHz, SX1272 for 860–1020 MHz). From the ESP32's side
it's just an SPI slave with a couple of interrupt pins.

## Key specs

| Spec | Value |
|------|-------|
| Bands | 137–1020 MHz (part-dependent; 868 / 915 most common) |
| Modulation | LoRa + FSK/OOK |
| TX power | up to +20 dBm |
| RX sensitivity | down to −148 dBm (SF12, narrow BW) |
| Data rate | 0.018 – 37.5 kbps (LoRa) |
| Current | ~10 mA RX, ~120 mA TX, ~200 nA sleep |

## Interface & wiring

SPI (`MISO / MOSI / SCK / NSS`) plus `RESET` and one to three `DIOn`
interrupt pins. 3.3 V only — do not put 5 V anywhere near it. Add a λ/4 wire
or SMA antenna appropriate for your band; no antenna → TX damages the PA.

## Benefits

- Kilometer-scale range at a few milliwatts.
- Genuine low-power sleep for battery sensor nodes.
- Huge open-source ecosystem (LMIC, RadioHead, LoRaMac-node).

## Limitations / gotchas

- **Duty-cycle regulation.** EU 868 MHz is limited to 1 % airtime per
  sub-band per hour by ERC 70-03; US 915 has dwell-time rules and FHSS
  expectations. Firmware must rate-limit or you'll either break the law or
  break the network.
- Low data rate. LoRa is for tiny packets (tens of bytes), not file transfer.
- LoRa and LoRaWAN are different things. SX1276 is the physical layer;
  LoRaWAN needs a gateway + network server (TTN, Helium, self-hosted).
- Supply rails must be clean — switch-mode regulators nearby will wreck
  sensitivity.

## Typical use cases

- Remote sensor nodes (weather, soil, livestock).
- Asset-tracking tags paired with a gateway.
- Private point-to-point telemetry in rural / industrial sites.

## Pairs well with

- [`sx1262-lora.md`](sx1262-lora.md) — newer replacement with lower RX
  current.
- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) —
  remote actuation at the edge of WiFi range.
- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md) —
  store-and-forward payloads on gateways.

## Where to buy

See `buy:`. Adafruit and Sparkfun sell breakouts with matched SMA connectors;
Heltec and LilyGo integrate these on ESP32 boards.

## Software / libraries

- `RadioHead` — simple point-to-point LoRa.
- `LMIC` / `MCCI-LMIC` — LoRaWAN stack.
- `arduino-LoRa` (Sandeep Mistry) — small, readable, great for learning.

## Notes for WeftOS

Tag LoRa links in WeftOS with `airtime_budget_ms_per_hour`; the rules engine
must enforce budget before transmitting. Treat LoRa bytes as expensive —
batch rather than per-event publish.
