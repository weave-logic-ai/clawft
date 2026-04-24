---
title: "Geiger Counter (SBM-20 + MightyOhm / RadiationD)"
slug: "geiger-counter"
category: "11-wild-card"
part_numbers: ["SBM-20", "J305", "MightyOhm Geiger kit", "RadiationD v1.1"]
manufacturer: "MightyOhm / various"
interface: ["Pulse on GPIO", "UART"]
voltage: "5 V (board); 400 V HV for tube"
logic_level: "3.3V / 5V"
current_ma: "~30 – 50 mA"
price_usd_approx: "$40 – $100"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [radiation, geiger, sbm-20, mightyohm, cpm, niche]
pairs_with:
  - "../03-audio-sound/pcm5102-pcm5122-dac.md"
  - "../04-light-display/ssd1306-oled.md"
  - "../08-communication/sx1276-rfm95-lora.md"
  - "../10-storage-timing-power/microsd.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=geiger" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=geiger" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=geiger" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-geiger.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-radiation-d.html" }
datasheet: "https://mightyohm.com/blog/products/geiger-counter/"
libraries:
  - { name: "RadiationWatch library", url: "https://github.com/MonsieurV/ArduinoPocketGeiger" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A Geiger-Müller counter detects ionising radiation (alpha, beta, gamma — depending
on tube). A Soviet-surplus **SBM-20** tube is the most common hobby choice; the
**MightyOhm Geiger kit** and the Chinese **RadiationD v1.1** board provide the
~400 V step-up supply, pulse-shaper, and click audio. Output is a **TTL pulse
per ionisation event** that the ESP32 counts to derive CPM (counts per minute)
and convert to µSv/h.

## Key specs

| Spec | SBM-20 + MightyOhm / RadiationD |
|------|---------------------------------|
| Tube | SBM-20 (beta/gamma) or J305 |
| Supply | 5 V (board generates 400 V internally) |
| Output | TTL pulse (~100 µs) per event |
| Conversion | ~1 CPM ≈ 0.0057 µSv/h for SBM-20 |
| Background rate | ~15–30 CPM natural background |

## Interface & wiring

- Board needs **5 V + GND**; on-board flyback generates 380–500 V for the tube.
- Pulse output pin goes to any ESP32 GPIO; use **PCNT** to count without
  interrupts, or `attachInterrupt(RISING)` with a 100 µs debounce.
- Audio "click" output (speaker / piezo) is optional but iconic — splice to an
  I²S DAC for a more serious-sounding alarm.
- **Warning: the tube carries 400 V** — keep fingers off the tube end caps and
  expose it through a grounded mesh in any enclosure.

## Benefits

- Detects beta + gamma radiation (SBM-20); real-world utility for anyone
  living near radioactive minerals, legacy radium dials, or medical isotopes.
- Pulse output is trivially integrated into any MCU.
- The "click-click-click" audio signature is an incredibly effective UX.

## Limitations / gotchas

- **Not a spectrometer** — can't tell what isotope, just "ionisation is
  happening".
- **Not sensitive to alpha** unless the tube has a mica window (SBM-20 doesn't).
- **Dead-time** of a GM tube (~100 µs) means CPM saturates above ~10⁵ CPM —
  useless for very strong sources.
- HV supply is noisy — keep the Geiger board 10+ cm from sensitive analog /
  radio on the same PCB, and shield ground returns.
- Sv/h conversion factor depends on tube + source energy; don't claim
  calibrated dose without a reference source.
- Buying SBM-20 tubes — verify authenticity; Chinese counterfeits have shown up.

## Typical use cases

- Distributed radiation-mapping mesh (post-Fukushima-style community sensors).
- Uranium-ore detection for amateur mineralogy.
- Science-museum interactive exhibits.
- Lab safety / survey-meter replacement (non-calibrated).

## Pairs well with

- [`../03-audio-sound/pcm5102-pcm5122-dac.md`](../03-audio-sound/pcm5102-pcm5122-dac.md) — real-sounding click via DAC + speaker.
- [`../04-light-display/ssd1306-oled.md`](../04-light-display/ssd1306-oled.md) — CPM / µSv/h display.
- [`../08-communication/sx1276-rfm95-lora.md`](../08-communication/sx1276-rfm95-lora.md) — offsite telemetry.
- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md) — event-stamped log.

## Where to buy

MightyOhm kit direct from [mightyohm.com](https://mightyohm.com/blog/products/geiger-counter/).
RadiationD v1.1 on AliExpress ~$40.

## Software / libraries

- Pulse counting via ESP32 PCNT peripheral (no library needed).
- `ArduinoPocketGeiger` (Radiation Watch) — similar API if you use that pocket
  sensor variant.

## Notes for WeftOS

Model CPM as a **CounterSource** with derived `µSv/h` via a tube-specific
calibration factor in metadata. WeftOS should treat radiation data as
time-series with mandatory rate-limit protection (never flood a LoRa uplink at
~1 event/sec for months).
