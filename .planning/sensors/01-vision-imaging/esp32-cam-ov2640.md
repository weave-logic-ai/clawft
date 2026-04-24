---
title: "ESP32-CAM (AI-Thinker, OV2640)"
slug: "esp32-cam-ov2640"
category: "01-vision-imaging"
part_numbers: ["ESP32-CAM", "AI-Thinker ESP32-CAM", "OV2640"]
manufacturer: "AI-Thinker / OmniVision"
interface: ["SPI", "I2C", "GPIO", "UART"]
voltage: "5V (board) / 3.3V (MCU)"
logic_level: "3.3V"
current_ma: "idle ~80 mA, streaming 180–310 mA (brownouts common on weak USB)"
price_usd_approx: "$6 – $12"
esp32_compat: ["ESP32"]
tags: [camera, rgb, ov2640, wifi, bluetooth, sd-card]
pairs_with:
  - "./ir-illuminators.md"
  - "./arducam-noir.md"
  - "../10-storage-timing-power/microsd.md"
  - "../04-light-display/ws2812b-neopixel.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=ESP32-CAM" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=ESP32-CAM" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=ESP32-CAM" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-ESP32-CAM.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-ESP32--CAM.html" }
libraries:
  - { name: "espressif/esp32-camera", url: "https://github.com/espressif/esp32-camera" }
  - { name: "arduino-esp32 Camera examples", url: "https://github.com/espressif/arduino-esp32" }
  - { name: "esphome", url: "https://github.com/esphome/esphome" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The AI-Thinker ESP32-CAM pairs an original ESP32 (dual-core, PSRAM, Wi-Fi/BT) with an OmniVision
OV2640 2 MP sensor and a microSD slot on a single ~$8 board. It is the default, cheapest way to
get "take a picture over Wi-Fi" into any project. Variants with a short flex to the sensor,
90°/120° fish-eye lenses, and long flex-cable "snake" probes are all pin-compatible.

## Key specs

| Spec | Value |
|---|---|
| Sensor | OmniVision OV2640 |
| Max resolution | 1600 × 1200 (UXGA), JPEG |
| Frame rate (typical) | ~12–25 fps at 640×480, 2–5 fps at UXGA |
| Lens | M12, ~65° default; 90°/120°/160° fisheye swap |
| PSRAM | 4 MB (needed for JPEG at UXGA) |
| Storage | microSD via 1-bit SDIO |
| Flash LED | GPIO 4 (shared with SD DAT1) |

## Interface & wiring

OV2640 is driven via the ESP32's dedicated camera peripheral (DVP, 8-bit parallel + I²C SCCB for
config). The board has **no USB bridge** — you need an external USB-UART adapter and must short
IO0→GND to enter flash mode. Power: 5 V at ≥500 mA is required; a flaky USB port is the #1 cause
of "brownout detector" reboots when streaming. Add a 470–1000 µF cap between 5 V and GND close to
the board.

## Benefits

- Best price-per-feature of any ESP32 camera board.
- PSRAM present, so UXGA JPEG works out of the box.
- microSD built in — good for motion-triggered recording.
- Huge community, tons of tutorials and forks.

## Limitations / gotchas

- No USB programming bridge. Expect to wire IO0/EN manually.
- Flash LED pin collides with SD (can't flash and write to SD at the same trivial setup).
- OV2640 IR sensitivity is mediocre without removing the IR-cut filter (see `arducam-noir.md`).
- Power noise and antenna layout hurt Wi-Fi range; external IPEX antenna helps.
- No hardware H.264 — you ship motion JPEG.

## Typical use cases

- Wi-Fi doorbell / yard cam.
- Time-lapse with SD buffering.
- QR / simple CV with ESP-WHO examples.
- Low-rate video to a server doing the real ML.

## Pairs well with

- [`./ir-illuminators.md`](./ir-illuminators.md) for NoIR night vision.
- [`./arducam-noir.md`](./arducam-noir.md) for IR-cut-removed sensor swap.
- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md) for on-device clip buffering.
- [`../04-light-display/ws2812b-neopixel.md`](../04-light-display/ws2812b-neopixel.md) for status or fill light.
- [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) to switch a bright IR array cleanly.

## Where to buy

- Adafruit — search "ESP32-CAM".
- SparkFun — search "ESP32-CAM".
- Seeed — search "ESP32-CAM".
- DFRobot — search "ESP32-CAM".
- AliExpress — bulk, cheapest, widely cloned.

## Software / libraries

- `espressif/esp32-camera` — the low-level driver; used by everything.
- `arduino-esp32` CameraWebServer example.
- `esphome` — declarative YAML, great for Home Assistant.

## Notes for WeftOS

Speculative: the OV2640 is a natural "raw frame" sensor node whose output surface could be a
JPEG / RGB888 buffer exposed as a WeftOS canonical stream (like `camera.ov2640.frame`), consumed
by effects (motion, classify, overlay) before hitting a display surface. Brownout behavior should
surface as a sensor-graph fault channel, not a silent reset.
