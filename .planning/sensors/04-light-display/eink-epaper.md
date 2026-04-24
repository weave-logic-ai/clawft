---
title: "E-Ink / E-Paper Displays (1.54\" – 7.5\"+, Waveshare & CrowPanel ESP32-S3)"
slug: "eink-epaper"
category: "04-light-display"
part_numbers: ["Waveshare 1.54\"/2.13\"/2.9\"/4.2\"/5.79\"/7.5\" e-Paper", "CrowPanel ESP32-S3 E-Ink"]
manufacturer: "Waveshare, GoodDisplay, Pervasive Displays, Elecrow"
interface: ["SPI"]
voltage: "3.3V"
logic_level: "3.3V"
current_ma: "~0 mA idle; ~20 mA during refresh pulses"
price_usd_approx: "$15 – $90"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["display", "e-paper", "eink", "low-power", "bistable", "signage"]
pairs_with:
  - "./ws2812b-neopixel.md"
  - "./ssd1306-oled.md"
  - "../10-storage-timing-power/microsd.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=eink" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=e-paper" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=e-paper" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-e-ink.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-waveshare-e-paper.html" }
libraries:
  - "Rust (embedded-graphics): epd-waveshare"
  - "Arduino: GxEPD2"
  - "ESP-IDF: LVGL + epdiy (for large panels)"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

E-paper panels are bistable displays — once a pixel is updated, it holds its state with no power. Waveshare and GoodDisplay cover sizes from 1.54" badge tags to 7.5"+ signage, in black/white and black/white/red (or b/w/yellow) variants. CrowPanel ESP32-S3 E-Ink is a compact all-in-one dev board: ESP32-S3 + battery + e-paper in one enclosure. For WeftOS, e-paper is the right answer whenever the content changes slowly and needs to be visible with zero power.

## Key specs

- Sizes (Waveshare naming): 1.54", 2.13", 2.9", 4.2", 5.79", 7.5", 10.3", 13.3".
- Colors: most panels are 1-bit B/W; "tricolor" variants add red or yellow.
- Refresh: **full refresh** 1–15 s with flicker; **partial refresh** ~300 ms on supported panels. Partial is glitchy over time, so vendors recommend a full refresh every ~5–10 partials.
- Interface: SPI + a few control lines (`DC`, `RES`, `BUSY`).
- Power: essentially 0 mW idle; refresh pulses draw tens of mA.
- Lifetime: 1–10 million full refreshes, or "indefinite" if mostly displayed (not constantly refreshing).

## Interface & wiring

- SPI: `SCLK`, `MOSI`, `CS`, `DC`, `RES`, `BUSY` (input from panel, high while refresh is in progress).
- Must wait on `BUSY` between commands; polling a GPIO is simpler than an interrupt for most apps.
- 3.3 V logic. Large panels (5.79"+) often need the vendor's HAT driver board to generate the higher refresh voltages.

## Benefits

- True zero-power display: disconnect the battery and the last image stays.
- Readable in direct sunlight; no backlight, no glare.
- Wide viewing angle, paper-like contrast.
- Cheap per square inch vs comparable TFT.

## Limitations / gotchas

- **Slow.** Full refresh takes seconds. Not for animation; plan UX around "state changed" events, not video.
- Partial refresh accumulates ghosting; you must force a full refresh periodically or the display degrades.
- **Burn-in** is real if you show the same pixels for years without refresh; some vendors recommend a "conditioning" full refresh every N updates.
- Temperature sensitive: cold (<0 °C) slows refresh dramatically and shifts color; hot (>40 °C) can over-stress the ink.
- Tricolor panels refresh 2–3× slower than B/W because red/yellow needs extra frames.
- Not all vendor-supplied demo code is redistributable; check licensing.

## Typical use cases

- Meeting-room / door name plates.
- "Last known good" telemetry: temperature, CO₂, Wi-Fi status — visible through a power outage.
- E-ink dashboards, to-do lists, calendar badges.
- Always-on dashboards on remote battery nodes ([solar + LiPo](../10-storage-timing-power/)).
- WeftOS canon / status plaques that survive reboots.

## Pairs well with

- [`./ws2812b-neopixel.md`](./ws2812b-neopixel.md) — hybrid signage: e-paper for persistent numbers, LEDs for live state.
- [`./ssd1306-oled.md`](./ssd1306-oled.md) — small OLED for debug HUD while keeping e-paper for the user-facing surface.
- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md) — cache rendered bitmaps so wakeups don't recompute full frames.

## Where to buy

- Waveshare direct (waveshare.com) or via Amazon — canonical reference panels and HAT drivers.
- Adafruit for stocked small panels with tutorials.
- Elecrow for CrowPanel ESP32-S3 E-Ink all-in-one dev kits.

## Software / libraries

- `epd-waveshare` crate (Rust) covers most Waveshare panels on `embedded-graphics`.
- `GxEPD2` (Arduino) is the most-supported C++ library.
- `epdiy` for large (>6") DIY panels.
- LVGL has experimental e-paper driver glue for richer UIs.

## Notes for WeftOS

- Surface host should schedule full refreshes explicitly (user-visible event) and rate-limit partial refreshes.
- Expose a `draft` vs `commit` drawing API so apps batch edits into one full-frame pulse.
- Track refresh count in NVS and warn the operator when approaching ~5M full refreshes.
- Use [`../05-environmental/bh1750-lux.md`](../05-environmental/bh1750-lux.md) to decide when ambient light is high enough for e-paper vs needing to wake a TFT.
