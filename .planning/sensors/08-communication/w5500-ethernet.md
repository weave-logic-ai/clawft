---
title: "W5500 – SPI Ethernet with Hardware TCP/IP Stack"
slug: "w5500-ethernet"
category: "08-communication"
part_numbers: ["W5500"]
manufacturer: "WIZnet"
interface: ["SPI"]
voltage: "3.3V"
logic_level: "3.3V (5V tolerant I/O)"
current_ma: "~130 mA active"
price_usd_approx: "$5 – $15"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [ethernet, spi, tcp, ip, offload, wired]
pairs_with:
  - "../08-communication/enc28j60-ethernet.md"
  - "../07-control-actuators/relay-modules.md"
  - "../10-storage-timing-power/index.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=W5500" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=W5500" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=W5500" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-W5500.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-W5500-ethernet.html" }
datasheet: "https://www.wiznet.io/product-item/w5500/"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The WIZnet W5500 is a hardware Ethernet + TCP/IP chip: the full stack
(TCP / UDP / IPv4 / ARP / DHCP / PPPoE) runs on-chip, freeing the MCU to just
open sockets over SPI. It supports 10/100 Mbit Ethernet, up to 8 simultaneous
sockets, and draws ~130 mA. For WeftOS this is the go-to wired-IP option when
WiFi is unavailable, unreliable, or disallowed.

## Key specs

| Spec | Value |
|------|-------|
| PHY | 10/100 BASE-T, auto-negotiate |
| Stack | Hardware TCP/IP, IPv4 only |
| Sockets | 8 simultaneous |
| Interface | SPI, up to 80 MHz |
| Buffer | 32 KB on-chip TX/RX |
| Supply | 3.3 V, 5 V-tolerant I/O |

## Interface & wiring

Four-wire SPI (`SCS / SCLK / MOSI / MISO`) plus `INT`, `RESET`, and the
magjack's own power. Breakouts (HanRun HR911105A, WIZnet's own WIZ850io,
Olimex ENC-W5500) hide the magnetics. Tie `CS` to a dedicated GPIO — sharing
with SD cards is a well-known source of weird bugs.

## Benefits

- Offloaded stack → MCU only manages sockets, not IP fragments.
- IPv4 + DHCP + DNS client + 8 sockets means you can run MQTT + HTTP + NTP
  concurrently.
- Reliable link; Ethernet doesn't rot the way WiFi does near microwaves.
- Good drivers in Arduino (`Ethernet2`, `EthernetESP32`) and Rust
  (`embedded-nal` adapters).

## Limitations / gotchas

- **IPv4 only.** No IPv6. For modern networks, you may need the WiFi radio
  anyway for IPv6.
- **Do not share SPI CS** carelessly. W5500 doesn't tri-state MISO as
  cleanly as SD cards expect; use dedicated CS pins.
- No TLS on-chip. You still run mbedTLS / rustls on the ESP32 side, which
  burns RAM.
- Ethernet magjack position on small breakouts constrains enclosure design.

## Typical use cases

- Industrial / server-room WeftOS nodes where WiFi is banned.
- Always-on relays / switches where WiFi loss would cause rule misfires.
- MQTT brokers / gateways on a dedicated wired VLAN.
- PoE-powered sensors (pair with a PoE-splitter module).

## Pairs well with

- [`enc28j60-ethernet.md`](enc28j60-ethernet.md) — compare: W5500 is the
  modern choice.
- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) —
  Ethernet + relay is the industrial recipe.
- [`../10-storage-timing-power/index.md`](../10-storage-timing-power/index.md) —
  PoE splitters sit in this family.

## Where to buy

See `buy:`. WIZnet's WIZ850io is the canonical SMT module; the HanRun
HR911105A breakout is the standard DIY-friendly board.

## Software / libraries

- `Ethernet` (Arduino) / `Ethernet2` — first-class W5500 support.
- `EthernetESP32` — ESP32-targeted fork with DMA SPI.
- Rust: `w5500-hl` / `w5500` crates for bare-metal use.
- ESPHome supports W5500 as a first-class "ethernet" component.

## Notes for WeftOS

Always pin Ethernet-backed nodes with a static address or DHCP reservation;
WeftOS coordination gets painful when addresses rotate. Flag the node's
radio channel as `wired` so rule engines prefer it for high-reliability
signals.
