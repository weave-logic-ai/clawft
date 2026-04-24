---
title: "08 – Communication & Wireless Expansion"
slug: "08-communication"
category: "08-communication"
description: "Expand ESP32 connectivity beyond its built-in WiFi/BT: LoRa, RFID/NFC, Ethernet, CAN, RS485, cellular, and sub-GHz radios."
tags: [communication, lora, rfid, nfc, ethernet, can, rs485, cellular, esp32, index]
source: "../../weftos_sensors.md"
status: "draft"
---

# 08 – Communication & Wireless Expansion

The ESP32 ships with WiFi and Bluetooth — good for a local network, useless if
there isn't one. This category covers every **other** link WeftOS projects
reach for: long-range low-bandwidth (LoRa, sub-GHz), contact-free identity
(RFID, NFC), wired industrial (Ethernet, CAN, RS485), and mobile-network
fallback (cellular).

## Modules

| Module | Band / Link | Interface | One-liner |
|--------|-------------|-----------|-----------|
| [SX1276 / RFM95](sx1276-rfm95-lora.md) | 868 / 915 MHz LoRa | SPI | Classic LoRa transceiver. |
| [SX1262](sx1262-lora.md) | 868 / 915 MHz LoRa | SPI | Newer, lower-power LoRa; LilyGo T-LoRa boards. |
| [Heltec / RAK LoRa boards](heltec-rak-lora.md) | LoRa + ESP32 integrated | — | Board-level dev kits. |
| [MFRC522](mfrc522-rfid.md) | 13.56 MHz RFID | SPI | Mifare-compatible reader/writer. |
| [PN532](pn532-nfc.md) | 13.56 MHz NFC | I²C / SPI / UART | Full NFC (reader + card emulator + P2P). |
| [RDM6300](rdm6300-lf-rfid.md) | 125 kHz LF RFID | UART | EM4100 keyfob reader. |
| [W5500](w5500-ethernet.md) | 10/100 Ethernet | SPI | Offloaded TCP/IP stack in silicon. |
| [ENC28J60](enc28j60-ethernet.md) | 10 Mbit Ethernet | SPI | MAC-only; older, heavier MCU load. |
| [MCP2515](mcp2515-can.md) | CAN 2.0B | SPI | Standalone CAN controller. |
| [RS485 (MAX485 / SN65HVD72)](rs485-transceiver.md) | Differential serial | UART | Industrial half-duplex. |
| [TCA9548A](tca9548a-i2c-mux.md) | I²C mux | I²C | 8-channel bus multiplexer. |
| [A7670 / SIM7000](a7670-sim7000-cellular.md) | LTE-Cat-M / NB-IoT / GSM | UART | Cellular modems. |
| [nRF24L01(+)](nrf24l01.md) | 2.4 GHz ShockBurst | SPI | Cheap proprietary pairs. |
| [CC1101](cc1101-subghz.md) | sub-GHz (315–915 MHz) | SPI | Hacker / ISM radio. |

## Protocol quick-picker

| Need | Pick | Why |
|------|------|-----|
| Kilometers, tiny packets, battery | [LoRa SX1262](sx1262-lora.md) | long range, µA sleep |
| Across-the-room cheap mesh | [nRF24L01+](nrf24l01.md) | bi-directional, trivial to use |
| Reverse-engineering alarms / remotes | [CC1101](cc1101-subghz.md) | tunable sub-GHz |
| Bank-grade auth token | [PN532](pn532-nfc.md) | full NFC stack |
| Cheap door fob | [MFRC522](mfrc522-rfid.md) | Mifare clones everywhere |
| Legacy 125 kHz card | [RDM6300](rdm6300-lf-rfid.md) | it's the only one |
| Car / industrial fieldbus | [MCP2515 + TJA1050](mcp2515-can.md) | CAN 2.0B |
| Modbus / warehouse daisy-chain | [RS485](rs485-transceiver.md) | long cable, noise-immune |
| "No WiFi here" fallback | [A7670 / SIM7000](a7670-sim7000-cellular.md) | cellular |
| Reliable wired IoT | [W5500](w5500-ethernet.md) | offloaded TCP/IP |
| Save the I²C bus | [TCA9548A](tca9548a-i2c-mux.md) | 8 isolated branches |

## Regulatory notes (read before broadcasting)

- **LoRa duty cycle:** EU 868 MHz is capped at 1 % duty cycle per sub-band by
  ERC 70-03. US 915 MHz has no duty cycle cap but has dwell-time and
  frequency-hopping requirements. Either way — budget for it in firmware.
- **Sub-GHz CC1101:** you can technically hit unlicensed pagers, car fobs,
  garage openers. Don't.
- **Cellular:** A7670 / SIM7000 modules are certified modems but integrating
  them into a product you sell means carrier certification (AT&T / Verizon
  approval). Hobby use is fine.
- **RFID cloning:** MFRC522 + common blank cards will happily clone most
  "secure" 13.56 MHz fobs. Assume 125 kHz EM4100 is effectively unprotected.

## Creative Lego ideas (from source)

- LoRa + RFID → asset tracking across a warehouse without WiFi.
- Ethernet + relays → industrial control panel with zero radio.
- TCA9548A + four identical I²C sensors → array measurement (one address,
  eight instances).

## Cross-links

- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) —
  the actuator side of "receive command → switch load".
- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md) —
  log LoRa / RS485 traffic locally.
- [`../06-biometric-presence/as608-fingerprint.md`](../06-biometric-presence/as608-fingerprint.md) —
  pair with RFID for two-factor access.
