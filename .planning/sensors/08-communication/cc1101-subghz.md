---
title: "CC1101 – Sub-GHz ISM Transceiver (315 / 433 / 868 / 915 MHz)"
slug: "cc1101-subghz"
category: "08-communication"
part_numbers: ["CC1101"]
manufacturer: "Texas Instruments"
interface: ["SPI"]
voltage: "1.8–3.6V"
logic_level: "3.3V (NOT 5V tolerant)"
current_ma: "~15 mA RX, ~30 mA TX @ +10 dBm, ~200 nA sleep"
price_usd_approx: "$3 – $8"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [sub-ghz, ism, rf, spi, cc1101, flipper, hacking]
pairs_with:
  - "../08-communication/sx1262-lora.md"
  - "../08-communication/nrf24l01.md"
  - "../07-control-actuators/relay-modules.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=CC1101" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=CC1101" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=CC1101" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-CC1101.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-CC1101.html" }
datasheet: "https://www.ti.com/product/CC1101"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The TI CC1101 is a highly configurable sub-GHz transceiver covering 315 /
433 / 868 / 915 MHz ISM bands with OOK, 2-FSK, 4-FSK, GFSK, and MSK
modulations. It's the radio behind Flipper Zero and the hacker-favorite
"listen to my neighbor's garage door" kits, but also a legitimate choice
for custom telemetry at ranges where WiFi won't reach and LoRa's data rate
is too slow.

Within this catalog, it sits between [nRF24L01+](nrf24l01.md) (2.4 GHz,
higher throughput, shorter range) and [SX1262 LoRa](sx1262-lora.md) (same
bands, much longer range, lower data rate).

## Key specs

| Spec | Value |
|------|-------|
| Bands | 300–348, 387–464, 779–928 MHz |
| Modulation | 2-FSK / 4-FSK / GFSK / MSK / OOK / ASK |
| Data rate | 0.6 – 600 kbps |
| TX power | up to +10 dBm (higher on modules with PA stage) |
| Sensitivity | −116 dBm at 0.6 kbps |
| Sleep | ~200 nA |

## Interface & wiring

Eight pins: `VCC / GND / SCK / MISO / MOSI / CSN / GDO0 / GDO2`. SPI plus
two general-purpose digital outputs (`GDO0`, `GDO2`) you can configure as
RX / TX activity / packet-received interrupts. 3.3 V only on VCC; inputs
tolerate 5 V on some board revisions but it's chip-dependent — assume not.

## Benefits

- Extremely flexible modulation / data-rate configuration.
- Very low sleep current; sub-µA battery nodes.
- Same bands as LoRa but with higher data rate for shorter distances.
- Huge amateur / hacker community ecosystem, including Flipper Zero apps.

## Limitations / gotchas

- **Regulatory / legal.** CC1101 can transmit on frequencies allocated to
  licensed services. Before you transmit anywhere near 315/433/868/915 MHz,
  know your region's allocation and duty-cycle rules. "Sub-GHz hacking"
  tutorials routinely break the law.
- Even more duty-cycle restrictive than LoRa in EU 868 — sub-bands vary.
- Small-pitch SMD module; PCB antenna matching varies wildly by
  manufacturer. A good antenna matters more than +3 dBm of power.
- Clones mislabel crystal frequency (26 vs 27 MHz); libraries get the
  offsets wrong and you transmit slightly off-band.
- 5 V on VCC instantly destroys the chip.

## Typical use cases

- Custom short-range telemetry where LoRa's ~37 kbps ceiling is too slow.
- Reverse-engineering OEM remote controls (garage door, weather stations).
- Private-band sensor networks at higher data rates than LoRa.

## Pairs well with

- [`sx1262-lora.md`](sx1262-lora.md) — LoRa for long-range, CC1101 for
  higher rate in the same band.
- [`nrf24l01.md`](nrf24l01.md) — 2.4 GHz counterpart.
- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) —
  sub-GHz remote control of relays.

## Where to buy

See `buy:`. Generic CC1101 modules with PCB antennas are widely cloned on
AliExpress; better RF performance comes from Anaren / TI reference designs.

## Software / libraries

- `RadioLib` (jgromes) — unified API for CC1101, LoRa, nRF24.
- `panstamp/panstamp` library — older but still referenced.
- For Flipper-style protocol exploration, `universal-radio-hacker` off-chip
  is the tool of choice.

## Notes for WeftOS

WeftOS must surface regulatory region (`EU868`, `US915`, etc.) as a
first-class node attribute and block transmit rules that exceed the region's
duty cycle or dwell-time limits. Don't ship CC1101-based nodes without
that guardrail.
