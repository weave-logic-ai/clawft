---
title: "WS2812B / NeoPixel Addressable RGB (strips, rings, matrices)"
slug: "ws2812b-neopixel"
category: "04-light-display"
part_numbers: ["WS2812B", "WS2813", "WS2815", "SK6812", "SK6812-RGBW"]
manufacturer: "WorldSemi / Adafruit (NeoPixel) / Opsco"
interface: ["GPIO"]
voltage: "5V (3.3V tolerant data with level shift)"
logic_level: "5V (level shifter recommended from 3.3V MCUs)"
current_ma: "~60 mA per pixel at full white (20 mA per channel)"
price_usd_approx: "$4 – $40 (strip/ring); $20 – $90 (matrices)"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["led", "rgb", "addressable", "neopixel", "status", "ambient"]
pairs_with:
  - "../07-control-actuators/mosfet-drivers.md"
  - "../10-storage-timing-power/ads1115-ads1015.md"
  - "./single-rgb-leds.md"
  - "./eink-epaper.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=neopixel" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=ws2812" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=ws2812b" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-ws2812.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-ws2812b.html" }
libraries:
  - "esp-idf: esp_idf_led_strip (RMT/SPI driver)"
  - "Rust: smart-leds + ws2812-esp32-rmt-driver"
  - "Arduino: FastLED, Adafruit_NeoPixel"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

WS2812B ("NeoPixel") is the workhorse addressable RGB pixel: the LED and driver IC are integrated in one 5050-sized package, daisy-chained over a single one-wire data line. They come as flexible strips (30 / 60 / 144 px/m), rings (12 / 16 / 24 / 32 / 60 px), and matrices in 8×8, 16×16, and 32×32. For WeftOS, they are the default channel for ambient status, presence awareness, and any "where is the user looking" feedback.

## Key specs

- 24-bit color (8 bits each R/G/B); WRGB variants (SK6812-RGBW) add a dedicated white die with better CRI.
- ~800 kbit/s one-wire protocol, strict timing (NRZ, ~1.25 µs bit time).
- 5 V logic and power; ground must be bonded to the MCU.
- Per-pixel current: ~20 mA per channel, so full white ≈ 60 mA.
- WS2813 / WS2815: add a backup data line so a single failed pixel doesn't kill the rest of the strip. WS2815 runs on 12 V (lower voltage drop on long strips).

## Interface & wiring

- Single GPIO to `DIN` of the first pixel; `DOUT` of each pixel feeds the next.
- Put a 300–500 Ω series resistor at the data input and a 1000 µF cap across V+/GND at the strip head.
- On ESP32, use the RMT peripheral (or I2S/SPI driver) to get jitter-free timing — bit-banging from `FreeRTOS` tasks rarely works reliably.
- For >1 m, inject 5 V every ~1 m of 60 px/m strip to avoid end-of-strip voltage droop (reddish whites).
- 3.3 V MCU → 5 V data: use a 74HCT125/74AHCT1G125 level shifter on `DIN`, or power pixel 1 from ~3.7 V so its logic threshold matches.

## Benefits

- Massive ecosystem, cheap, one data wire regardless of pixel count.
- Arbitrary per-pixel color and brightness at tens-of-Hz refresh rates.
- Flexible physical form factor — strips, rings, matrices, PCBs, wearables.

## Limitations / gotchas

- Current is enormous at full white (a 16×16 matrix = 256 px × 60 mA ≈ 15 A). Always set a global brightness cap in firmware.
- Strict timing means long ISRs or Wi-Fi TX bursts on ESP32 can glitch pixels; use RMT and avoid blocking.
- Quality varies wildly by batch; cheap AliExpress reels often have dead pixels or wrong color order (GRB vs RGB).
- WS2812 vs WS2813 vs SK6812: different redundancy / timing / whites. Don't mix on the same data line unless specs match.
- Not rated for continuous full-white — heat will shorten life on dense matrices.

## Typical use cases

- Ambient and presence status rings around camera / mic arrays.
- VU meters and activity bars for audio / LLM token streaming.
- Low-res art displays and accent lighting.
- Matrix "mood wall" or "notification ticker" driven by WeftOS events.

## Pairs well with

- [`./eink-epaper.md`](./eink-epaper.md) — always-on data + NeoPixel for live signal.
- [`./single-rgb-leds.md`](./single-rgb-leds.md) — single accent LED where a whole strip is overkill.
- [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) — gate the 5 V rail when the strip is idle to cut standby current.
- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) — monitor rail voltage sag on dense matrices.

## Where to buy

- Adafruit (NeoPixel-branded, well QC'd), SparkFun, Seeed Studio.
- AliExpress for bulk strips — expect 1–2 % dead pixels.
- Pimoroni for matrices and Unicorn HATs (Pi-focused).

## Software / libraries

- `esp_idf_led_strip` (ESP-IDF, RMT-backed) — stable for production.
- `ws2812-esp32-rmt-driver` + `smart-leds` (Rust) for the WeftOS native stack.
- `FastLED` is the reference implementation on Arduino; port any novel effects from there.

## Notes for WeftOS

- Treat LEDs as a *surface*: expose them in WeftOS as a 1-D or 2-D framebuffer with a `present()` method that internally batches via RMT. This keeps the surface-host abstraction clean.
- Global gamma + brightness cap belongs in the HAL, not in each animation.
- On battery-powered nodes, tie the LED rail to a [MOSFET driver](../07-control-actuators/mosfet-drivers.md) and enable only when a user is present — presence sensors in [`../06-biometric-presence/`](../06-biometric-presence/) (PIR, mmWave) can drive this.
