---
title: "RS485 Transceivers – MAX485 / SN65HVD72"
slug: "rs485-transceiver"
category: "08-communication"
part_numbers: ["MAX485", "SN65HVD72", "SP3485"]
manufacturer: "Analog Devices (Maxim) / Texas Instruments / various"
interface: ["UART"]
voltage: "5V (MAX485), 3.3V (SN65HVD72 / SP3485)"
logic_level: "5V or 3.3V (chip-dependent)"
current_ma: "~1–5 mA idle, higher during TX"
price_usd_approx: "$1 – $5 per breakout"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [rs485, modbus, industrial, differential, uart, fieldbus]
pairs_with:
  - "../08-communication/mcp2515-can.md"
  - "../07-control-actuators/relay-modules.md"
  - "../10-storage-timing-power/index.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=RS485" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=RS485" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=RS485" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-RS485.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-RS485-MAX485.html" }
datasheet: "https://www.ti.com/product/SN65HVD72"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

RS485 is a differential, half-duplex, multi-drop serial standard — the
backbone of Modbus RTU, DMX512, BACnet MS/TP, and a thousand industrial
sensors. A tiny transceiver chip (MAX485 for 5 V, SN65HVD72 / SP3485 for
3.3 V) converts UART TX/RX to the A/B differential pair that travels a
kilometer of twisted-pair cable without losing bits.

## Key specs

| Chip | Supply | Speed | Notes |
|------|--------|-------|-------|
| MAX485 | 5 V | up to 2.5 Mbps | Classic, 5 V logic |
| SN65HVD72 | 3.3 V | up to 20 Mbps | TI, 3.3 V-native — pick for ESP32 |
| SP3485 | 3.3 V | up to 10 Mbps | SiPex/Exar, common on cheap modules |

- Signaling: differential A / B, 120 Ω terminated at both ends
- Topology: daisy-chain, up to 32 nodes (256 with modern transceivers)
- Cable: CAT5/6 UTP works fine for short runs

## Interface & wiring

ESP32 UART TX → transceiver's `DI`, UART RX ← `RO`. A single GPIO drives
both `DE` and `/RE` (tied together) for direction control — HIGH to
transmit, LOW to receive. Provide 120 Ω termination at the two physical
ends of the bus. Add bias resistors (680 Ω to VCC on A, 680 Ω to GND on B)
somewhere on the bus to prevent floating when idle.

For ESP32, ESPHome's `uart` component and the `ModbusMaster` library handle
the DE/RE flip internally if you tell them which pin to use.

## Benefits

- Kilometer-scale noise-immune serial.
- Drop-dead simple at the hardware level; chip costs pennies.
- Industrial sensor / PLC universe speaks RS485 as a native language.
- Multi-drop means 30+ nodes on one pair of wires.

## Limitations / gotchas

- **Half-duplex.** One talker at a time, you must arbitrate. Modbus's
  master-slave pattern solves it; DMX512's single-master does too.
- **Direction pin timing.** Flip `DE/RE` too late after a byte and you cut
  off your own transmission; too early and the next request sees garbage.
- Cheap MAX485 modules use 5 V logic — driving their `RO` output into a
  3.3 V ESP32 RX pin is overvoltage. Use SN65HVD72 or SP3485 for 3.3 V
  projects.
- Terminating the bus wrong is the #1 "it works on the bench, fails in the
  field" failure mode.

## Typical use cases

- Modbus RTU to PLCs, VFDs, energy meters, temperature controllers.
- DMX512 stage-lighting bridges.
- Long-haul sensor networks (greenhouses, barns).
- Daisy-chain between distant WeftOS nodes without WiFi.

## Pairs well with

- [`mcp2515-can.md`](mcp2515-can.md) — industrial fieldbus cousin.
- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) —
  Modbus-addressable relay boards exist at every industrial supplier.
- [`../10-storage-timing-power/index.md`](../10-storage-timing-power/index.md) —
  energy meter / current monitor Modbus devices.

## Where to buy

See `buy:`. Any "MAX485 TTL to RS485" module on AliExpress works for 5 V
builds; for 3.3 V-native, SparkFun and Adafruit sell SN65HVD72 breakouts.

## Software / libraries

- `ModbusMaster` / `ArduinoModbus` — Modbus RTU client.
- `ESP32-Modbus` — IDF-native server/client.
- ESPHome's `modbus_controller` — wraps a whole field of sensors per device.

## Notes for WeftOS

Model each Modbus address as a WeftOS sensor node with its own polling
interval. Rate-limit writes — slamming a VFD register at WeftOS loop rate
will trip its internal watchdog.
