---
title: "LD3320 Offline Voice Recognition (legacy)"
slug: "ld3320-voice"
category: "03-audio-sound"
part_numbers: ["LD3320"]
manufacturer: "ICRoute (discontinued / limited availability)"
interface: ["SPI", "I2S", "UART (module dependent)"]
voltage: "3.3V / 5V"
logic_level: "3.3V"
current_ma: "~40 mA"
price_usd_approx: "$8 – $20 (availability caveat)"
esp32_compat: ["ESP32", "ESP32-S3"]
tags: [voice, recognition, offline, ld3320, legacy]
pairs_with:
  - "./dual-mic-arrays.md"
  - "./inmp441-i2s-mic.md"
buy:
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-LD3320.html" }
  - { vendor: eBay, url: "https://www.ebay.com/sch/i.html?_nkw=LD3320" }
  - { vendor: Amazon, url: "https://www.amazon.com/s?k=LD3320" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The LD3320 is an older single-chip "offline voice recognition" IC — you pre-program ~50 keyword
slots and it fires an event when it hears one. It's the ancestor of the modern ESP32-S3 /
esp-sr pipeline, and while it still works, it's legacy: stock is spotty, docs are mostly
Chinese, and ESP32-S3 WakeNet dwarfs it.

## Key specs

| Spec | Value |
|---|---|
| Keyword slots | Up to ~50 |
| Interface | SPI or parallel |
| Sample rate | 8–48 kHz |
| Audio input | Electret mic with onboard preamp |
| Language | Mandarin / any phonetic key-word (English works) |

## Interface & wiring

Module boards typically expose SPI (CS, SCK, MOSI, MISO, RST, IRQ) and an analog mic input. You
write keyword strings into the chip's memory and it asserts IRQ when it matches. Power tolerance
ranges from 3.3 V to 5 V depending on module.

## Benefits

- Truly offline voice triggering on tiny MCUs — no cloud, no Wi-Fi needed.
- Simple event model (IRQ + keyword index).
- Lower total cost than building an ESP32-S3-based AI voice stack for one wake word.

## Limitations / gotchas

- **Availability:** the chip is effectively EOL; clones are hit-or-miss.
- Docs are thin outside Chinese forums.
- Accuracy and robustness nowhere near ESP32-S3 WakeNet / esp-sr.
- Programming-keywords flow is quirky; expect to read chip datasheet PDFs carefully.

## Typical use cases

- Legacy "shout a command, it fires a relay" appliances.
- Extremely low-cost voice-triggered toys where ESP32-S3 is overkill.
- Educational reference for how dedicated voice-recognition ASICs work.

## Pairs well with

- [`./dual-mic-arrays.md`](./dual-mic-arrays.md) as the modern ESP32-S3 alternative.
- [`./inmp441-i2s-mic.md`](./inmp441-i2s-mic.md) if you move off LD3320 to do your own wake-word.

## Where to buy

- AliExpress / eBay / Amazon — only marginally trustworthy stock.
- If you need reliability for a product, pick a modern alternative (S3 + esp-sr) instead.

## Software / libraries

- Vendor's LDChip driver (usually shipped as a Chinese PDF + C file); ports floating around GitHub.

## Notes for WeftOS

Speculative: LD3320 is mostly of historical interest. Don't plan a first-class WeftOS driver for
it; keep the interface the same as modern wake-word engines ("keyword matched" event) and let a
community port plug in if anyone wants it.
