---
title: "HX711 + Strain-Gauge Load Cell"
slug: "hx711-load-cell"
category: "11-wild-card"
part_numbers: ["HX711", "1kg / 5kg / 20kg / 50kg / 200kg load cell"]
manufacturer: "Avia Semiconductor (HX711)"
interface: ["2-wire serial (DOUT / PD_SCK)"]
voltage: "2.6 – 5.5 V"
logic_level: "3.3V / 5V"
current_ma: "~1.5 mA active, < 1 µA power-down"
price_usd_approx: "$2 – $15 (HX711 + cell)"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [scale, load-cell, strain-gauge, hx711, weight, 24-bit-adc]
pairs_with:
  - "../10-storage-timing-power/ads1115-ads1015.md"
  - "../10-storage-timing-power/microsd.md"
  - "../04-light-display/ssd1306-oled.md"
  - "../08-communication/sx1276-rfm95-lora.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=load+cell" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=hx711" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=hx711" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-hx711.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-hx711-load-cell.html" }
datasheet: "https://www.avia-semi.com/en/product_detail/4.html"
libraries:
  - { name: "HX711 (bogde)",  url: "https://github.com/bogde/HX711" }
  - { name: "HX711 (Rob Tillaart)", url: "https://github.com/RobTillaart/HX711" }
  - { name: "ESPHome hx711",  url: "https://esphome.io/components/sensor/hx711.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The HX711 is a 24-bit sigma-delta ADC with an integrated low-noise PGA,
designed specifically for **bridge sensors** — i.e., Wheatstone-bridge strain
gauges glued to metal bars to make a load cell. Pair it with a commercial load
cell (1 kg, 5 kg, 20 kg, 50 kg, 200 kg bar-style; or 4× 50 kg half-bridge cells
for a full-body scale) and you have an ESP32-compatible scale with 0.1 g
resolution.

## Key specs

| Spec | HX711 |
|------|-------|
| Resolution | 24-bit (effective ~19–22 bits after noise) |
| PGA gain | 32 / 64 / 128 |
| Sample rate | 10 or 80 sps (selectable) |
| Input range | ±20 mV (gain 128) |
| Supply | 2.6 – 5.5 V |
| Interface | 2-wire (DOUT + PD_SCK), no SPI |

## Interface & wiring

- 2-wire bit-banged serial — any two free GPIOs.
- Load cell has 4 wires: E+ (red), E– (black), A+ (white), A– (green). Colors
  vary — always verify with a multimeter.
- E± is the bridge excitation (supply from HX711). A± is the differential
  signal. HX711's PGA handles the tiny mV-level signal; no op-amp needed.
- Mount the load cell correctly — bar-style cells have an arrow showing load
  direction; ignoring it halves the signal and doubles the nonlinearity.
- Keep the cell wiring **short** (< 30 cm) and away from motor drivers. For
  longer runs use shielded cable with the shield at the HX711 ground.

## Benefits

- 24-bit ADC dedicated to millivolt bridge signals — trivial to get 0.1 g
  resolution on a 1 kg cell.
- Purpose-built; no general-purpose ADC (not even ADS1115) is as convenient for
  strain gauges.
- Cheap and universally available.

## Limitations / gotchas

- **Noise** — cheap HX711 breakouts bounce 5–15 counts. Average 10–16 samples;
  reject the first read after power-on.
- Only 10 or 80 sps — not a high-speed ADC. Fine for weighing, not for
  vibration analysis.
- Load cells are **temperature-sensitive** — a 2 g/°C drift on a 1 kg cell is
  normal. Apply a temperature-compensation table or recalibrate often.
- Cheap 4-wire load cells from AliExpress have wildly inconsistent color codes;
  trust the continuity test, not the silkscreen.
- Mechanical overload permanently ruins the cell — choose capacity with 2× headroom.
- The red PCB "green HX711" breakout has a known layout bug (traces too long);
  the "green PCB HX711" (SparkFun / Sparkfun-style) is the one you want.

## Typical use cases

- Kitchen / lab scale (1 kg / 5 kg cell).
- Beehive weight monitoring (bar cells under each corner, or a single large
  platform cell).
- 3D-printer filament scale (low-end; extruder weight monitoring).
- Fishing-line / rope tension measurement (S-type cells).
- Livestock / hydroponics nutrient-tank level.

## Pairs well with

- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md) — weight log over time.
- [`../04-light-display/ssd1306-oled.md`](../04-light-display/ssd1306-oled.md) — live kg display.
- [`../08-communication/sx1276-rfm95-lora.md`](../08-communication/sx1276-rfm95-lora.md) — offsite beehive telemetry.
- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) — if you also need general-purpose ADC channels.

## Where to buy

SparkFun and Adafruit sell matched HX711 + load-cell kits. AliExpress: HX711 +
5 kg cell for ~$5.

## Software / libraries

- `HX711` (bogde) — the original, solid Arduino driver.
- `HX711` (Rob Tillaart) — more features, better noise handling.
- ESPHome `sensor: platform: hx711`.

## Notes for WeftOS

Model as a **ScalarSource** in kg (or N), with built-in `tare`, `calibrate`,
and `temperature_compensation` pipeline stages. Capability metadata should
record cell full-scale so higher layers can enforce overload refusal.
