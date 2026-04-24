---
title: "Color Sensors (TCS3200 / TCS34725)"
slug: "tcs3200-color"
category: "11-wild-card"
part_numbers: ["TCS3200", "TCS3210", "TCS34725"]
manufacturer: "TAOS / ams / ScioSense"
interface: ["GPIO frequency (TCS3200)", "I2C (TCS34725)"]
voltage: "2.7 – 5.5 V"
logic_level: "3.3V / 5V"
current_ma: "~5 – 15 mA (LEDs dominate)"
price_usd_approx: "$3 – $8"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [color, rgb, sensor, tcs3200, tcs34725, spectrometer-lite]
pairs_with:
  - "../04-light-display/ws2812b-neopixel.md"
  - "../01-vision-imaging/esp32-cam-ov2640.md"
  - "../07-control-actuators/hobby-servos.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=tcs34725" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=color+sensor" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=tcs34725" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-tcs34725.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-tcs3200.html" }
datasheet: "https://ams.com/tcs34725"
libraries:
  - { name: "Adafruit_TCS34725", url: "https://github.com/adafruit/Adafruit_TCS34725" }
  - { name: "ESPHome tcs34725",  url: "https://esphome.io/components/sensor/tcs34725.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

Two distinct color sensors go under the same "color sensor" hobby banner:

- **TCS3200 / TCS3210** — array of 64 filtered photodiodes (16 each of R, G,
  B, and clear). Output is a **square wave whose frequency is proportional to
  the sampled intensity**; you select the color filter via two digital pins and
  count pulses on a third. 5 V part, cheap, Arduino-kit staple.
- **TCS34725** — integrated RGB + clear ADC over I²C, with an onboard IR
  blocking filter and white LED. Much easier to use — just read four 16-bit
  values. This is the one Adafruit and ams recommend.

## Key specs

| Spec | TCS3200 | TCS34725 |
|------|---------|----------|
| Channels | R, G, B, Clear | R, G, B, Clear |
| Output | Frequency on OUT pin | I²C registers |
| Resolution | ~10 kHz/unit of light (frequency count) | 16-bit per channel |
| IR filter | No | Yes |
| Onboard LED | 4× white | 1× white (controllable) |
| Supply | 2.7 – 5.5 V | 2.7 – 3.3 V |

## Interface & wiring

TCS3200:
- Pins: S0/S1 (freq scaling), S2/S3 (filter select), OUT (freq output), OE.
- Count pulses on OUT for a fixed window (e.g. 100 ms) → per-channel intensity.
- Use ESP32's **PCNT** peripheral to count pulses without interrupts.
- 5 V-friendly but works fine at 3.3 V.

TCS34725:
- I²C at 0x29.
- Read R, G, B, Clear registers; convert to sRGB with a simple normalization.
- `LED` pin drives the onboard white LED via MOSFET — turn off for ambient-light
  readings, on for surface-color identification.

## Benefits

- Cheap, standalone surface-color identification with no camera needed.
- TCS34725 gives near-instant 16-bit RGB without frequency counting.
- Built-in illumination solves the "is the surface lit consistently?" problem.

## Limitations / gotchas

- **Surface-color only, not spectroscopy.** Two physically different pigments
  that look the same will read the same. For real spectroscopy use an AS7262 /
  AS7341.
- White LED is the reference — ambient light, especially sunlight, throws
  results off. Build a light shield around the sensor and surface.
- TCS3200 is 5 V nominal and quite noisy; prefer TCS34725 for new designs.
- Calibrate against known-color reference cards before trusting values.
- Reflective vs. translucent surfaces give very different readings — fix sensor
  distance (~5–10 mm typical) and geometry.

## Typical use cases

- Sorting colored objects on a conveyor / servo arm (candy, Lego bricks,
  recycling).
- Gamified teaching kits.
- Mood lights that match the color of a reference object (pair with
  [`../04-light-display/ws2812b-neopixel.md`](../04-light-display/ws2812b-neopixel.md)).
- Water / solution colorimetry (shaded cuvette).

## Pairs well with

- [`../04-light-display/ws2812b-neopixel.md`](../04-light-display/ws2812b-neopixel.md) — mirror the seen color on an LED strip.
- [`../01-vision-imaging/esp32-cam-ov2640.md`](../01-vision-imaging/esp32-cam-ov2640.md) — ground-truth one-pixel readings against an image.
- [`../07-control-actuators/hobby-servos.md`](../07-control-actuators/hobby-servos.md) — drive a sorter arm.

## Where to buy

Adafruit 1334 (TCS34725). TCS3200 clones on AliExpress for $3. SparkFun sells
both in Qwiic form.

## Software / libraries

- `Adafruit_TCS34725` — includes color-temp (CCT) and lux calculations.
- ESPHome `sensor: platform: tcs34725`.
- TCS3200: custom sketches using `pulseIn()` or ESP32 PCNT.

## Notes for WeftOS

Model as a 4-channel **ScalarSource** (R, G, B, Clear) with optional derived
outputs (hue, lux, CCT). Flag TCS3200 metadata with "no IR filter" so the
pipeline applies IR-rejection math.
