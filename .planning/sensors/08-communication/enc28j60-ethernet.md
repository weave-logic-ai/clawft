---
title: "ENC28J60 – SPI 10 Mbit Ethernet MAC (Legacy)"
slug: "enc28j60-ethernet"
category: "08-communication"
part_numbers: ["ENC28J60"]
manufacturer: "Microchip Technology"
interface: ["SPI"]
voltage: "3.3V"
logic_level: "3.3V (NOT 5V tolerant)"
current_ma: "~180 mA active"
price_usd_approx: "$4 – $8"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [ethernet, spi, mac-only, legacy, wired]
pairs_with:
  - "../08-communication/w5500-ethernet.md"
  - "../07-control-actuators/relay-modules.md"
  - "../10-storage-timing-power/index.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=ENC28J60" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=ENC28J60" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=ENC28J60" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-ENC28J60.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-ENC28J60.html" }
datasheet: "https://www.microchip.com/en-us/product/ENC28J60"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The ENC28J60 is Microchip's 10 Mbit SPI Ethernet **MAC + PHY** without a
built-in TCP/IP stack — unlike the [W5500](w5500-ethernet.md), here the MCU
has to run the full IP stack in firmware. Flashy 10-years-ago for Arduino
Ethernet shields, today it's mostly there because it's cheap and you already
own one. We document it so WeftOS users know why to **pick W5500 instead**.

## Key specs

| Spec | Value |
|------|-------|
| PHY | 10 BASE-T only (no 100 Mbit) |
| Stack | MAC only; TCP/IP runs on the MCU |
| Sockets | as many as your MCU's RAM allows |
| Interface | SPI, up to 20 MHz |
| Buffer | 8 KB on-chip TX/RX |
| Supply | 3.3 V |

## Interface & wiring

SPI (`SCK / MOSI / MISO / CS`) plus `INT` and `RESET`. 3.3 V only — no 5 V
tolerance. The common HR911105A-style breakout integrates the magjack and
its magnetics; just provide SPI and power.

## Benefits

- Cheap; sometimes $2 on AliExpress.
- Works anywhere there's an `UIPEthernet` / `EthernetENC` library.
- Some legacy code bases are written against it.

## Limitations / gotchas

- **MCU runs the TCP stack.** `UIPEthernet` and `EthernetENC` do the job but
  use significant RAM/CPU, hurt concurrent performance, and handle multiple
  sockets poorly. On ESP32 this is fine; on smaller MCUs it kills throughput.
- **10 Mbit only.** You'll still hit 1 Mbps of usable throughput, but many
  switches log link-speed mismatches.
- Chip runs hot — "hot enough to be uncomfortable to touch" warm.
- Erratic behavior on noisy power rails; add bulk decoupling.
- Silicon errata around packet filtering that bit half the community — read
  the errata.

## Typical use cases

- Retrofitting an old Arduino Ethernet shield onto an ESP32.
- Ultra-cheap wired IoT where $5 matters more than RAM.
- Classroom projects that reference existing ENC28J60 tutorials.

## Pairs well with

- [`w5500-ethernet.md`](w5500-ethernet.md) — strongly preferred for new work.
- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) —
  wired-Ethernet relay control.
- [`../10-storage-timing-power/index.md`](../10-storage-timing-power/index.md) —
  PoE splitters for power-over-ethernet.

## Where to buy

See `buy:`. The HR911105A and HanRun breakouts dominate AliExpress;
Microchip sells the chip for custom PCBs.

## Software / libraries

- `EthernetENC` / `UIPEthernet` — Arduino-compatible.
- `ethernet` generic adapter in ESP-IDF (via `esp_eth`).
- Rust: no actively maintained crate; prefer W5500 for Rust work.

## Notes for WeftOS

Mark ENC28J60 as `legacy=true, prefer_alternative=w5500` in the catalog.
Only use it when retrofitting an existing install.
