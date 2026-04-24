---
title: "Niche / Creative Wild-Card Modules"
slug: "11-wild-card"
category: "11-wild-card"
description: "Load cells, color, Geiger counters, breathalyzers, vibration switches, KY-series Arduino-kit modules, and expansion shields — the fun stuff."
tags: [wildcard, niche, creative, ky-series, breakout, expansion]
source: "../../weftos_sensors.md"
status: "draft"
---

# 11 — Wild Card: Niche & Creative Modules

This is where the fun stuff lives. The other ten categories cover "what every
project needs"; this one collects everything that makes a project weird,
memorable, or uniquely useful — plus the catch-all KY-0xx sensor kit that ships
on every Arduino beginner's doorstep, and the expansion shields / connector
systems that turn an ESP32 into a Lego platform.

## Modules

| Module | File | Interface | Notes |
|--------|------|-----------|-------|
| HX711 + load cell | [`hx711-load-cell.md`](hx711-load-cell.md) | 2-wire serial | Strain-gauge scale |
| TCS3200 / TCS34725 color | [`tcs3200-color.md`](tcs3200-color.md) | Digital freq / I²C | RGB + clear channel |
| Geiger counter (SBM-20) | [`geiger-counter.md`](geiger-counter.md) | Pulse / UART | Ionising radiation |
| MQ-3 breathalyzer | [`mq3-breathalyzer.md`](mq3-breathalyzer.md) | ADC | Alcohol detection |
| SW-420 vibration / shock | [`sw-420-vibration.md`](sw-420-vibration.md) | GPIO | Ball-switch |
| KY-0xx series (master index) | [`ky-series.md`](ky-series.md) | various | Arduino sensor kit |
| ESP32 breakout / expansion shields | [`esp32-breakout-shields.md`](esp32-breakout-shields.md) | passive / connector | Qwiic, Grove, Gravity, Feather |

## Why "wild card"?

These modules don't fit cleanly into sensing, control, comm, or power
categories — they're domain-specific (radiation, breath alcohol, weight),
bundle-specific (the KY-0xx kit), or ecosystem-specific (Qwiic / Stemma QT /
Grove / Gravity connector systems). They become the **hook** that makes a
project interesting rather than the **plumbing** that makes it work.

## Creative Lego ideas

- **Smart scale** — HX711 + load cell + OLED + LoRa for a livestock / beehive
  weighing station.
- **Color sorter** — TCS34725 + servo sorter arm for candy / recycling demos.
- **Geiger-counter audio dashboard** — pulse input → I²S click to speaker,
  lifetime dose to SD card, LoRa uplink for a radiation-mapping mesh.
- **Breathalyzer ignition interlock** — MQ-3 + relay, strictly for hobby only.
- **Shock-sensor tripwire** — SW-420 + ESP-NOW notification.
- **"Sensor kit" teaching rig** — KY-0xx bundle + Qwiic hub, each sensor
  hot-pluggable into a WeftOS graph.

## Pairs with

- [`../03-audio-sound/pcm5102-pcm5122-dac.md`](../03-audio-sound/pcm5102-pcm5122-dac.md) — Geiger click / scale beep.
- [`../04-light-display/ssd1306-oled.md`](../04-light-display/ssd1306-oled.md) — weight / color / CPM dashboard.
- [`../08-communication/sx1276-rfm95-lora.md`](../08-communication/sx1276-rfm95-lora.md) — offsite radiation / shock telemetry.
- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) — clean ADC for MQ / color analog.
