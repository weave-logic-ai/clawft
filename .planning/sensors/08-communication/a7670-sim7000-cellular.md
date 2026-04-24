---
title: "A7670 / SIM7000 – LTE-Cat-M / NB-IoT / GSM Cellular Modems"
slug: "a7670-sim7000-cellular"
category: "08-communication"
part_numbers: ["A7670E", "A7670SA", "SIM7000G", "SIM7000E", "SIM7070"]
manufacturer: "SIMCom"
interface: ["UART", "USB (some variants)"]
voltage: "3.8–4.2V (PMIC-driven), typ. LiPo-friendly"
logic_level: "3.3V UART (1.8V on some variants — check)"
current_ma: "~50 mA idle, ~500 mA TX peak, up to 2 A inrush"
price_usd_approx: "$15 – $40"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [cellular, lte, cat-m, nb-iot, gsm, modem, uart]
pairs_with:
  - "../08-communication/sx1262-lora.md"
  - "../07-control-actuators/mosfet-drivers.md"
  - "../10-storage-timing-power/index.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=SIM7000" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=SIM7000" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=SIM7000" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-SIM7000.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-A7670-SIM7000.html" }
datasheet: "https://www.simcom.com/"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

SIMCom's A7670 and SIM7000 families are low-power cellular modems for IoT:
LTE-Cat-M1 + NB-IoT + 2G-GSM fallback. They talk classic AT commands over
UART, optionally USB. Typical form factor is a postage-stamp module soldered
to a carrier PCB that also hosts a SIM slot, antenna connectors (main +
GPS), and a PMIC-friendly battery input. LilyGo's T-SIM boards integrate
them directly with an ESP32.

These are **the** fallback when your WeftOS node deploys somewhere WiFi
doesn't exist.

## Key specs

| Spec | Value |
|------|-------|
| Bands | LTE-Cat-M1 + NB-IoT (variant-dependent), GSM 850/900/1800/1900 |
| Voltage | 3.8–4.2 V (via PMIC; LiPo-friendly) |
| Current | ~50 mA idle registered, ~500 mA TX, up to 2 A inrush peaks |
| Interface | UART 115200 8N1 AT commands; USB on some |
| GNSS | Integrated on SIM7000G / A7670 (varies) |
| SIM | Nano-SIM, 1.8/3 V |

## Interface & wiring

Four UART pins (`TX / RX / PWRKEY / RESET`) plus power (3.8–4.2 V into `VBAT`).
**PSRAM warning: cellular modems draw 2 A inrush and 500 mA TX spikes.** A
3.3 V regulator shared with an ESP32 + PSRAM will brown out, the ESP32
resets, the modem misses your AT sequence, and you spend a day debugging.
Power the modem from its own LiPo-grade rail with a bulk cap (1000 µF) right
at the module.

Pulse `PWRKEY` LOW for ~1 s to boot, LOW for ~2.5 s to shut down.

## Benefits

- LTE-M and NB-IoT are genuinely low-power (seconds of TX per day for a
  sensor), unlike classic 4G.
- GSM fallback covers older towers where LTE-M isn't deployed.
- GNSS on-module saves a whole GPS module.
- AT command API is boring — meaning: stable, documented, debuggable.

## Limitations / gotchas

- **Inrush is vicious.** Budget for a 2 A-capable rail (LiPo + 1000 µF) or
  watch the ESP32 brown-out on every TX.
- Carrier certification is a pain. For commercial deployment in the US,
  Verizon / AT&T require end-product certification on top of module
  certification.
- LTE-Cat-M coverage varies wildly by country; check before deploying.
- Many cheap AliExpress carrier boards omit the bulk cap, then the inrush
  browns out the modem itself. Add the cap.
- 2G is being shut down region by region — don't design around GSM-only
  fallback for long-lived deployments.

## Typical use cases

- Remote weather / livestock / agricultural sensors.
- Asset trackers with GNSS + cellular + LoRa hybrids.
- Failover link when WiFi / Ethernet is unavailable.
- Emergency / fire-ground telemetry.

## Pairs well with

- [`sx1262-lora.md`](sx1262-lora.md) — LoRa for local, cellular for backhaul.
- [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) —
  power-gate the modem between transmissions.
- [`../10-storage-timing-power/index.md`](../10-storage-timing-power/index.md) —
  LiPo charger + fuel gauge for off-grid cellular nodes.

## Where to buy

See `buy:`. LilyGo T-SIM boards and Waveshare SIM7000/A7670 breakouts are the
canonical integrated options; SIMCom sells bare modules through distributors.

## Software / libraries

- `TinyGSM` — portable AT-command wrapper, supports both A7670 and SIM7000.
- `ArduinoJson` + MQTT client over `TinyGsmClient` is the standard recipe.
- ESPHome has a `sim800l`-family component that works for simple SMS/HTTP.

## Notes for WeftOS

Put the cellular modem behind a MOSFET power-gate and model data-byte cost
as a first-class scheduling metric. WeftOS rules should prefer LoRa / WiFi
when available; cellular is a fallback, not a default.
