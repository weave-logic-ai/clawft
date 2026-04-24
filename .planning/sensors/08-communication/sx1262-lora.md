---
title: "SX1262 – LoRa Transceiver (LilyGo T-LoRa, Heltec V3)"
slug: "sx1262-lora"
category: "08-communication"
part_numbers: ["SX1262", "SX1261", "SX1268"]
manufacturer: "Semtech"
interface: ["SPI"]
voltage: "1.8–3.7V"
logic_level: "3.3V"
current_ma: "~4.6 mA RX, ~120 mA TX @ +22 dBm, ~160 nA sleep"
price_usd_approx: "$6 – $20 (bare), $15 – $30 (integrated LilyGo board)"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [lora, rf, sub-ghz, low-power, spi, semtech, lilygo]
pairs_with:
  - "../08-communication/sx1276-rfm95-lora.md"
  - "../08-communication/heltec-rak-lora.md"
  - "../10-storage-timing-power/microsd.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=SX1262" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=SX1262" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=SX1262" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-LoRa.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-SX1262-LoRa.html" }
datasheet: "https://www.semtech.com/products/wireless-rf/lora-connect/sx1262"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The SX1262 is Semtech's second-generation LoRa transceiver and the default
for all new designs. Compared to [SX1276](sx1276-rfm95-lora.md) it cuts RX
current by roughly half (~4.6 mA), bumps max TX power to +22 dBm, and
simplifies the register map. LilyGo's T-LoRa line and Heltec V3 boards pair
it with an ESP32-S3 on a single PCB; SparkFun and Adafruit sell bare
breakouts.

## Key specs

| Spec | Value |
|------|-------|
| Bands | 150–960 MHz (SX1262 = 860–930 MHz; SX1261 = sub-GHz low; SX1268 = China) |
| TX power | up to +22 dBm |
| RX current | ~4.6 mA boosted RX |
| Sensitivity | down to −148 dBm |
| Sleep | ~160 nA |
| Interface | SPI + DIO1 interrupt + BUSY flag |

## Interface & wiring

Five SPI pins plus `BUSY`, `DIO1`, and `RESET`. SX126x handshaking requires
polling `BUSY` between SPI commands — the RadioLib / Sandeep-LoRa libraries
handle this. 3.3 V only. Pair with a band-appropriate antenna.

## Benefits

- Half the RX current of SX1276 → doubles battery life on listening nodes.
- +22 dBm option useful for uplink-only remote sensors.
- Same LoRa + FSK modes, better RX and TX numbers.
- Clean integration on LilyGo / Heltec boards means you can skip wiring.

## Limitations / gotchas

- **Not drop-in compatible with SX1276** at the software level — register
  map differs. RadioLib or Sandeep Mistry's `RadioLib` branch handles both,
  but naive copy-paste from old SX1276 code fails.
- `BUSY` polling must be implemented correctly or transfers silently fail.
- Same LoRa duty-cycle rules apply (EU 1 %, US dwell-time / FHSS).
- Early LilyGo T-LoRa batches had wrong SPI pin labels in silkscreen — verify
  against board revision before wiring.

## Typical use cases

- Battery-powered sensor nodes where RX current matters (always-on listener).
- Meshtastic / off-grid messaging builds.
- LoRaWAN class-C devices that listen continuously.

## Pairs well with

- [`sx1276-rfm95-lora.md`](sx1276-rfm95-lora.md) — legacy counterpart; both
  can interoperate on the same LoRa PHY with compatible settings.
- [`heltec-rak-lora.md`](heltec-rak-lora.md) — integrated dev-board form.
- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md) —
  buffer outgoing payloads while duty-cycle budget recovers.

## Where to buy

See `buy:`. LilyGo sells direct; AliExpress distributes the same boards with
lead times. SparkFun's breakout is the cleanest bare-chip option.

## Software / libraries

- `RadioLib` (jgromes) — unified API across SX126x, SX127x, CC1101, nRF24.
- `MCCI LMIC` fork supports SX1262 for LoRaWAN.
- Meshtastic firmware already supports LilyGo T-LoRa and Heltec V3 targets.

## Notes for WeftOS

Prefer SX1262 in new WeftOS node designs. Keep a per-node airtime budget in
persistent memory — rule engine must not burn through the hour's 1 % in a
burst.
