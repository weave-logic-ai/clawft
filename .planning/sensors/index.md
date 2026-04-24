---
title: "WeftOS ESP32 Sensor & Module Catalog"
description: "Structured catalog of ESP32-compatible sensors, actuators, cameras, displays, and communication modules usable as 'Lego bricks' for WeftOS-based projects."
source: "../weftos_sensors.md"
scope: "ESP32 / ESP32-S3 / ESP32-C3 family"
logic_levels: ["3.3V", "5V-tolerant-via-level-shifter"]
interfaces: ["GPIO", "I2C", "SPI", "UART", "I2S", "PWM", "ADC", "1-Wire", "CAN"]
category_count: 11
module_count_approx: 200
tags: [catalog, esp32, sensors, actuators, modules, index]
version: "0.1.0"
status: "draft"
---

# WeftOS ESP32 Sensor & Module Catalog

This directory is the structured, navigable form of [`../weftos_sensors.md`](../weftos_sensors.md).
Each **category** is a folder. Inside each category folder is an `index.md` plus one
file per **solution** (part number, family, or tightly-coupled variant group).

All files carry YAML frontmatter so they can be machine-indexed later by WeftOS's own
planning / knowledge-graph tooling.

---

## Categories

| #  | Category | Folder | Focus |
|----|----------|--------|-------|
| 01 | Vision & Imaging (Cameras + Paired Lighting) | [`01-vision-imaging/`](01-vision-imaging/index.md) | RGB, NoIR, thermal, UV cameras; IR/UV illuminators |
| 02 | Positioning, Navigation & Motion | [`02-positioning-navigation/`](02-positioning-navigation/index.md) | GPS, IMU, ToF, ultrasonic, LiDAR, UWB, compass, baro |
| 03 | Audio & Sound | [`03-audio-sound/`](03-audio-sound/index.md) | I²S mics, amps, DACs, speakers, buzzers |
| 04 | Light, Illumination & Display | [`04-light-display/`](04-light-display/index.md) | NeoPixel, OLED, TFT, e-paper, 7-seg, lasers |
| 05 | Environmental / Gas / Air / Soil / Water | [`05-environmental/`](05-environmental/index.md) | T/RH/P, VOC, CO₂, PM, soil, water quality |
| 06 | Biometric, Health & Presence | [`06-biometric-presence/`](06-biometric-presence/index.md) | HR/SpO₂, fingerprint, PIR, mmWave |
| 07 | Control Mechanisms & Actuators | [`07-control-actuators/`](07-control-actuators/index.md) | Relays, MOSFETs, motor drivers, servos, steppers |
| 08 | Communication & Wireless Expansion | [`08-communication/`](08-communication/index.md) | LoRa, RFID/NFC, Ethernet, CAN, cellular, sub-GHz |
| 09 | Input & Human Interface | [`09-input-hmi/`](09-input-hmi/index.md) | Buttons, encoders, joysticks, capacitive touch, keypads |
| 10 | Storage, Timing, Power & Management | [`10-storage-timing-power/`](10-storage-timing-power/index.md) | SD, RTC, LiPo charging, power monitors, external ADCs |
| 11 | Niche / Creative "Wild Card" | [`11-wild-card/`](11-wild-card/index.md) | Load cells, color, Geiger, KY-series, breakouts |

---

## How this catalog is organized

### Per-module file schema

Every `*.md` in a category folder describes **one solution** (one part, one family,
or one tightly related variant group). It uses this YAML frontmatter shape:

```yaml
---
title: "Human-readable module name"
slug: "file-stem-matching-path"
category: "NN-category-slug"
part_numbers: ["PRIMARY_PN", "ALIAS_PN"]
manufacturer: "Bosch / STMicro / Invensense / ..."
interface: ["I2C", "SPI", "UART", "I2S", "GPIO", "ADC", "PWM", "1-Wire"]
voltage: "3.3V"      # or "3.3V / 5V"
logic_level: "3.3V"  # or "5V - needs level shifter"
current_ma: "typ X mA, peak Y mA"
price_usd_approx: "$2 – $8"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [sensor, imu, i2c, ...]
pairs_with:
  - "../NN-other/other-module.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=..." }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=..." }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=..." }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-..." }
datasheet: "https://vendor.example/part.pdf"
libraries:
  - { name: "Adafruit_XYZ",     url: "https://github.com/adafruit/..." }
  - { name: "esphome.io",       url: "https://esphome.io/components/..." }
source: "../../weftos_sensors.md"
status: "draft"
---
```

Each file's body then follows this structure:

1. **Overview** – what it is, what it senses/actuates, why it's on ESP32 radar.
2. **Key specs** – table of the numbers that actually matter.
3. **Interface & wiring** – pins, bus, level shifting, power notes.
4. **Benefits** – what it's good at.
5. **Limitations / gotchas** – what to watch for (drift, MCU load, licensing, etc.).
6. **Typical use cases** – project patterns it fits.
7. **Pairs well with** – cross-references to other catalog files.
8. **Where to buy** – vendor links (generic search URLs where a specific SKU is volatile).
9. **Software / libraries** – drivers, ESPHome components, Arduino examples.
10. **Notes for WeftOS** – how this fits the WeftOS sensor graph model (optional).

### Category `index.md` files

Each category folder contains an `index.md` that:

- Links to every module file in that folder.
- Gives a one-liner per module.
- Surfaces the **"Creative Lego Idea"** prompts from the source doc.
- Flags known pairings into other categories (e.g. cameras ↔ illuminators).

---

## Cross-cutting design themes

These themes repeat across categories and are treated as first-class cross-references:

- **Camera ↔ Illumination pairings** — every low-light imaging path pairs an
  imager in [`01-vision-imaging/`](01-vision-imaging/index.md) with an LED
  source (IR / UV / RGB) in the same folder or in
  [`04-light-display/`](04-light-display/index.md).
- **Sensor-fusion positioning** — GPS + IMU + UWB + barometer modules in
  [`02-positioning-navigation/`](02-positioning-navigation/index.md) are
  typically fused, not used alone.
- **Power gating** — MOSFET / relay modules in
  [`07-control-actuators/`](07-control-actuators/index.md) are routinely used
  to cut power to hungry peripherals (cameras, mmWave radar, cellular modems).
- **I²C bus expansion** — `TCA9548A` in [`08-communication/`](08-communication/index.md)
  is the escape hatch when too many I²C sensors collide on addresses.
- **Precision ADC** — external ADCs (`ADS1115` / `ADS1015`) in
  [`10-storage-timing-power/`](10-storage-timing-power/index.md) upgrade any
  analog sensor in this catalog when the ESP32's built-in ADC isn't good enough.

---

## Buying sources (canonical)

All modules in this catalog are sourced from the same short list of vendors.
Per-file `buy:` entries link to vendor search URLs for robustness.

- **Adafruit** – [adafruit.com](https://www.adafruit.com/) — best docs / libraries.
- **SparkFun** – [sparkfun.com](https://www.sparkfun.com/) — Qwiic ecosystem.
- **Seeed Studio** – [seeedstudio.com](https://www.seeedstudio.com/) — Grove + XIAO.
- **DFRobot** – [dfrobot.com](https://www.dfrobot.com/) — Gravity ecosystem.
- **Pimoroni** – [pimoroni.com](https://shop.pimoroni.com/).
- **AliExpress / Amazon** – generic & cloned modules; cheapest, least curated.
- **Waveshare** – [waveshare.com](https://www.waveshare.com/) — especially displays.
- **M5Stack** – [m5stack.com](https://shop.m5stack.com/).
- **LilyGo / TTGO** – [lilygo.cc](https://www.lilygo.cc/) — integrated boards.

## Reference tutorials & catalogs

- [Random Nerd Tutorials – ESP32 projects & sensor guides](https://randomnerdtutorials.com/projects-esp32/)
- [espboards.dev – ESP32 module / board catalog](https://espboards.dev/)
- [Adafruit Learning System](https://learn.adafruit.com/)
- [SparkFun Tutorials](https://learn.sparkfun.com/tutorials)
- [ESPHome – drop-in sensor components](https://esphome.io/components/sensor/index.html)

---

## Status

- 2026-04-21 — initial breakout from `weftos_sensors.md`, structure scaffold and
  per-module files generated. Fine-grained variants (LED matrix sizes, relay
  channel counts, MQ gas variants) are grouped into family files and covered
  inside a single file rather than exploded per-SKU, to keep the graph navigable.
