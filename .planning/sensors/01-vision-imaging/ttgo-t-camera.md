---
title: "LilyGo TTGO T-Camera"
slug: "ttgo-t-camera"
category: "01-vision-imaging"
part_numbers: ["TTGO T-Camera", "T-Camera Plus", "T-Camera Mini"]
manufacturer: "LilyGo"
interface: ["I2C", "SPI", "UART"]
voltage: "5V"
logic_level: "3.3V"
current_ma: "~110–220 mA streaming"
price_usd_approx: "$14 – $28"
esp32_compat: ["ESP32"]
tags: [camera, ttgo, lilygo, oled, pir]
pairs_with:
  - "./esp32-cam-ov2640.md"
  - "../06-biometric-presence/index.md"
  - "../10-storage-timing-power/microsd.md"
buy:
  - { vendor: LilyGo,   url: "https://www.lilygo.cc/search?type=product&q=t-camera" }
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=TTGO+camera" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-ttgo-t-camera.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The TTGO T-Camera family is LilyGo's integrated take on ESP32-CAM: OV2640 sensor plus an onboard
0.96" SSD1306 OLED and, on most variants, an AS312 PIR motion sensor — all on one PCB with
USB-C. Useful when you want a self-contained "motion triggers frame + status on OLED" node.

## Key specs

| Spec | Value |
|---|---|
| Sensor | OV2640 (2 MP) |
| Display | 0.96" 128×64 OLED (I²C SSD1306) |
| PIR | AS312 (standard T-Camera + T-Camera Plus) |
| USB | USB-C with CH9102/CP2104 |
| Extras (Plus) | 1.3" display, speaker/mic on some revs |

## Interface & wiring

Internal I²C bus links OLED + optional sensors; camera bus uses the ESP32 DVP pins. PIR output is
a plain GPIO — you can deep-sleep and wake on PIR without any glue. Check the specific revision's
pinout — LilyGo ships several that share the name.

## Benefits

- Motion-wake out of the box (PIR + deep sleep).
- Onboard status OLED is great for debugging / QR display / IP reporting.
- USB-C programming.

## Limitations / gotchas

- Multiple hardware revisions under the same name — pin maps drift.
- Not every variant has an SD slot; check before buying.
- PIR on the same PCB as Wi-Fi radio can pick up RF noise in edge cases.

## Typical use cases

- Hallway / entrance motion cam with OLED status.
- Deep-sleep battery node that only streams on motion.
- Compact kiosk with OLED + small CV task.

## Pairs well with

- [`./esp32-cam-ov2640.md`](./esp32-cam-ov2640.md) for the bare-board comparison.
- [`../06-biometric-presence/index.md`](../06-biometric-presence/index.md) for deeper PIR / mmWave alternatives.
- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md) on SD-equipped variants.

## Where to buy

- LilyGo official storefront.
- Adafruit for US stock.
- AliExpress for all variants including EOL ones.

## Software / libraries

- LilyGo example repos on GitHub (search `Xinyuan-LilyGO/TTGO-Camera-*`).
- `espressif/esp32-camera` + SSD1306 OLED lib (e.g. `olikraus/u8g2`).

## Notes for WeftOS

Speculative: T-Camera is a natural "edge-triggered surface producer" node — PIR-gated frame
emission would map nicely to a WeftOS cyclic source with an external activation predicate.
