---
title: "SW-420 Vibration / Shock Switch"
slug: "sw-420-vibration"
category: "11-wild-card"
part_numbers: ["SW-420", "SW-18010P"]
manufacturer: "Generic"
interface: ["GPIO"]
voltage: "3.3V / 5V"
logic_level: "matches supply"
current_ma: "< 5 mA"
price_usd_approx: "$1 – $2"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [vibration, shock, sw-420, tilt, niche, digital]
pairs_with:
  - "../06-biometric-presence/pir-hc-sr501.md"
  - "../02-positioning-navigation/mpu6050-imu.md"
  - "../08-communication/sx1276-rfm95-lora.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=vibration+switch" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=vibration+sensor" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=sw-420" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-vibration.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-sw-420.html" }
datasheet: ""
libraries:
  - { name: "ESPHome binary_sensor (gpio)", url: "https://esphome.io/components/binary_sensor/gpio.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The SW-420 is a spring / metal-ball switch: under vibration or shock, the ball
momentarily bridges two contacts, closing the switch. On a module it comes
with an LM393 comparator, a sensitivity pot, and a digital output that goes
LOW when a vibration event is detected. Crude but cheap and effective for
"something is moving / something was knocked" detection. Not an
accelerometer — it gives you a binary "event happened" signal, not magnitude.

## Key specs

| Spec | SW-420 |
|------|--------|
| Output | Digital (LOW on vibration) |
| Sensitivity | Adjustable via onboard pot |
| Trigger threshold | ~0.5 – 3 g depending on orientation |
| Response | Immediate (ms scale) |
| Supply | 3.3 – 5 V |

## Interface & wiring

- 3 pins: Vcc, GND, DOUT (to any GPIO).
- Adjust the pot until the LED just stops flickering when the board sits still.
- Mount **rigidly** to whatever should be monitored — loose wiring will make it
  trigger on its own mass swinging.
- Orientation matters — the ball-switch is axially sensitive; rotate to find
  the optimum.
- Attach a hardware interrupt for wake-from-deep-sleep use.

## Benefits

- Dead simple — a switch, a comparator, and a pot.
- Zero power in the "no vibration" state (static switch).
- Good ESP32 wake-on-motion source for battery devices.

## Limitations / gotchas

- **Not a magnitude sensor.** You can't tell "gentle tap" from "dropped onto
  concrete" — just "happened / didn't". For real motion analytics use an
  [`MPU6050`](../02-positioning-navigation/mpu6050-imu.md) or ADXL345.
- Pot calibration drifts with temperature and handling.
- Mechanical — the spring/ball can fatigue after many millions of cycles.
- Only one axis of sensitivity at a time — use 3 for full 3-axis coverage, or
  just use an IMU.
- No debounce — every vibration pulse is a sub-millisecond bounce; handle in
  firmware with a short post-event lockout.

## Typical use cases

- Wake-on-knock for a battery-powered device.
- "Package was moved" logger for shipping / pet monitoring.
- Tripwire / tamper detection on an enclosure.
- Retro pinball / arcade "tilt" switch.

## Pairs well with

- [`../06-biometric-presence/pir-hc-sr501.md`](../06-biometric-presence/pir-hc-sr501.md) — combine
  vibration + motion for better false-alarm rejection.
- [`../02-positioning-navigation/mpu6050-imu.md`](../02-positioning-navigation/mpu6050-imu.md)
  — real accelerometer for quantitative analysis.
- [`../08-communication/sx1276-rfm95-lora.md`](../08-communication/sx1276-rfm95-lora.md) — tamper notifications at range.

## Where to buy

AliExpress SW-420 modules ~$1. SparkFun has a "vibration sensor" analog
equivalent using a piezo strip if you need magnitude.

## Software / libraries

- None — standard GPIO with a short post-trigger lockout in firmware.
- ESPHome `binary_sensor: platform: gpio` with `delayed_off: 500ms` filter.

## Notes for WeftOS

Model as a **DiscreteEventSource** with a configurable debounce/lockout window.
Useful as the world's cheapest wake-on-motion source. For magnitude-bearing
vibration, promote to the IMU.
