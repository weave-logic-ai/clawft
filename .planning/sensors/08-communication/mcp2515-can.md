---
title: "MCP2515 – Standalone CAN Bus Controller"
slug: "mcp2515-can"
category: "08-communication"
part_numbers: ["MCP2515", "TJA1050", "MCP2551"]
manufacturer: "Microchip Technology (MCP2515); NXP (TJA1050)"
interface: ["SPI", "CAN"]
voltage: "5V (typical module), 3.3V logic-level variants exist"
logic_level: "3.3V or 5V (module-dependent)"
current_ma: "~10 mA + TJA1050 transceiver"
price_usd_approx: "$3 – $8"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [can, can-bus, spi, automotive, industrial, fieldbus]
pairs_with:
  - "../08-communication/rs485-transceiver.md"
  - "../07-control-actuators/relay-modules.md"
  - "../10-storage-timing-power/index.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=MCP2515" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=MCP2515" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=MCP2515" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-CAN.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-MCP2515-CAN.html" }
datasheet: "https://www.microchip.com/en-us/product/MCP2515"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The MCP2515 is a standalone CAN 2.0B controller that talks SPI to the MCU and
speaks a differential CAN bus through a companion transceiver (TJA1050 is the
common choice; MCP2551 is older). ESP32 has a native CAN / TWAI controller,
so for ESP32 you often don't *need* an MCP2515 — but you do whenever you want
a second bus, 5 V-tolerant hardware, or you're porting Arduino CAN code.

## Key specs

| Spec | Value |
|------|-------|
| Standard | CAN 2.0A / 2.0B |
| Bitrate | up to 1 Mbps |
| Interface | SPI (to MCU), CAN-H / CAN-L differential |
| Buffers | 3 TX, 2 RX + filter/mask registers |
| Interrupt | `INT` pin signals RX / error / TX complete |
| Supply | 5 V (typical module); 3.3 V parts exist |

## Interface & wiring

Six-pin SPI (`SCK / MISO / MOSI / CS / INT / RESET`) plus `CAN-H` / `CAN-L`
on a two-pin screw terminal. Put a **120 Ω termination resistor** across
CAN-H / CAN-L at each end of the bus (most breakouts have a solder jumper
for the onboard 120 Ω). Common CAN ground for multi-node setups.

## Benefits

- Drops onto any MCU with SPI; the ESP32 gains a second isolated CAN bus.
- CAN's differential signaling + CRC handles noisy automotive / industrial
  environments far better than RS232.
- Plenty of libraries, and ESPHome has a `canbus` component.

## Limitations / gotchas

- **Termination matters.** Missing 120 Ω = reflection errors, missing frames,
  a hundred forum threads of confusion.
- Most cheap MCP2515 modules are **5 V logic only**. For a 3.3 V ESP32,
  either buy a 3.3 V module or wire a level shifter on `SI / SO / SCK / CS /
  INT`.
- Clock speed is *module-dependent* — 8 MHz vs 16 MHz crystals are both
  common; bitrate tables differ. Match your library config.
- Lots of fake / suspect "CAN modules" on AliExpress omit the isolation
  between CAN and SPI grounds; ground loops through a vehicle chassis can
  pop SPI pins.

## Typical use cases

- OBD-II reader / logger for automotive diagnostics.
- Industrial / agricultural CAN networks (ISOBUS, NMEA 2000).
- Second CAN channel on an ESP32 that already uses its native TWAI.

## Pairs well with

- [`rs485-transceiver.md`](rs485-transceiver.md) — different fieldbus with
  similar deployment patterns.
- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) —
  CAN-triggered relay drops (automotive accessory control).
- [`../10-storage-timing-power/index.md`](../10-storage-timing-power/index.md) —
  current monitoring on CAN-linked loads.

## Where to buy

See `buy:`. The SeeedStudio CAN-BUS shield and SparkFun Qwiic CAN modules are
well-documented; AliExpress MCP2515 modules are cheap but watch the logic
level.

## Software / libraries

- `MCP_CAN_lib` / `mcp_can` — canonical Arduino library.
- `ACAN2515` — modern alternative, good error handling.
- ESPHome `canbus` platform.

## Notes for WeftOS

WeftOS can treat CAN as a structured event bus: decode DBC frames and surface
typed sensor values (e.g. engine RPM) into the sensor graph. Firewall
cloud-bound CAN data — vehicle CAN traces are surprisingly identifying.
