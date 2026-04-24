---
title: "Analog Joystick / Thumbstick"
slug: "joysticks"
category: "09-input-hmi"
part_numbers: ["PSP-style thumbstick", "KY-023"]
manufacturer: "ALPS / generic"
interface: ["ADC", "GPIO"]
voltage: "3.3V / 5V"
logic_level: "3.3V"
current_ma: "< 5 mA"
price_usd_approx: "$2 – $5"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [input, joystick, thumbstick, analog, adc, gaming]
pairs_with:
  - "./momentary-buttons.md"
  - "./potentiometers.md"
  - "../07-control-actuators/hobby-servos.md"
  - "../10-storage-timing-power/ads1115-ads1015.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=thumbstick" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=joystick" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=joystick" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-joystick.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-ky-023-joystick.html" }
datasheet: ""
libraries:
  - { name: "ESP32 BLE Gamepad", url: "https://github.com/lemmingDev/ESP32-BLE-Gamepad" }
  - { name: "ESPHome adc",       url: "https://esphome.io/components/sensor/adc.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A thumbstick is two perpendicular potentiometers on a gimbal, plus an optional
"click-down" momentary switch. Each axis returns an analog voltage from 0 V
(full one way) through mid-rail (centered) to Vcc (full the other way). The
KY-023 is the generic PSP-style joystick module seen in every Arduino starter kit.

## Key specs

| Spec | Typical |
|------|---------|
| Axes | 2 (X, Y) |
| Button | 1 (press-down) |
| Supply | 3.3–5 V |
| Centered output | Vcc/2 ± 5–10% |
| Resistance | 10 kΩ per axis |
| Travel | ±20° gimbal |

## Interface & wiring

- 5 pins: Vcc, GND, VRx (ADC), VRy (ADC), SW (GPIO).
- Run it from **3.3 V**, not 5 V, so the ADC never exceeds 3.3 V full-scale.
- Use ADC1 channels on the ESP32 (GPIO 32–39). ADC2 is disabled while Wi-Fi is
  active, which will randomly break your controller.
- Button is open-drain-to-ground; enable internal pull-up.
- Calibrate center at boot; never assume exactly Vcc/2 — cheap sticks drift
  5–10% off center.

## Benefits

- Proportional 2D input in a tiny footprint.
- Cheap and universally available.
- Great for pan/tilt, rover drive, cursor control.

## Limitations / gotchas

- ESP32's built-in ADC is noisy and non-linear — deadzone and apply a rolling
  average or low-pass filter. For precise control, move to an
  [`ADS1115`](../10-storage-timing-power/ads1115-ads1015.md) external ADC.
- Mechanical center-spring is weak; dead-zone of ±5–10% is mandatory to avoid
  drift when untouched.
- Contact wear causes "scratchy" zones over time (10k+ actuations).
- The diagonal range is √2 × the axial range; normalize to a circle if you need
  circular travel.

## Typical use cases

- Remote-control rover / drone.
- Pan-tilt camera gimbal.
- Menu navigation on UIs too cramped for a D-pad + buttons.
- Bluetooth HID gamepad (`ESP32-BLE-Gamepad`).

## Pairs well with

- [`../07-control-actuators/hobby-servos.md`](../07-control-actuators/hobby-servos.md) — direct gimbal control.
- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) — for precise, noise-free readings.
- [`../08-communication/sx1276-rfm95-lora.md`](../08-communication/sx1276-rfm95-lora.md) — long-range RC link (if present).

## Where to buy

KY-023 clones on AliExpress for < $1; Adafruit sells better-quality ALPS sticks
with break-outs for a few dollars.

## Software / libraries

- `analogRead()` / `adc1_get_raw()` directly.
- ESP-IDF **OneShot** ADC API with calibration fuses for linearized readings.
- `ESP32-BLE-Gamepad` to expose it as a standard BLE HID gamepad.

## Notes for WeftOS

Model as a pair of **ScalarSources** (X, Y, normalized to [-1, 1]) plus one
**DiscreteEventSource** for the button. A deadzone + low-pass filter should be
part of the default pipeline because the raw signal is always noisy.
