---
title: "PCA9685 – 16-Channel I²C PWM / Servo Driver"
slug: "pca9685-servo-driver"
category: "07-control-actuators"
part_numbers: ["PCA9685"]
manufacturer: "NXP Semiconductors"
interface: ["I2C", "PWM"]
voltage: "2.3–5.5V logic, separate V+ rail for servos/LEDs"
logic_level: "3.3V compatible"
current_ma: "chip < 10 mA; aggregate load on V+ is your motors/LEDs"
price_usd_approx: "$3 – $10"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [pwm, servo, i2c, 16-channel, led-driver]
pairs_with:
  - "../07-control-actuators/hobby-servos.md"
  - "../04-light-display/ws2812b-neopixel.md"
  - "../08-communication/tca9548a-i2c-mux.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=PCA9685" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=PCA9685" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=PCA9685" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-PCA9685.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-PCA9685.html" }
datasheet: "https://www.nxp.com/products/PCA9685"
libraries:
  - { name: "Adafruit_PWMServoDriver", url: "https://github.com/adafruit/Adafruit-PWM-Servo-Driver-Library" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The PCA9685 is NXP's 16-channel, 12-bit I²C PWM generator. It offloads all PWM
timing from the MCU — you write "channel 3, duty 2048" over I²C and the chip
keeps the PWM alive until told otherwise. The Adafruit and generic HW-272
breakouts add headers suitable for three-pin servos and a bulk V+ rail with a
reverse-polarity-protected screw terminal.

It is the default way to drive more servos than an ESP32 has PWM channels for
(a problem at 6+ servos) and doubles as a general-purpose LED / fan dimmer.

## Key specs

| Spec | Value |
|------|-------|
| Channels | 16, independent |
| Resolution | 12-bit per channel |
| Frequency | 24 Hz – 1526 Hz, shared across all channels |
| Bus | I²C, up to 1 MHz fast-mode+ |
| Addresses | 6-bit → 62 chips cascaded on one bus |
| Oscillator | 25 MHz internal (or external `EXTCLK`) |

## Interface & wiring

Six pins on the breakout: `GND / OE / SCL / SDA / VCC / V+`. `VCC` is the
logic rail (3.3 V), `V+` is the servo / LED rail (4–6 V for servos, up to 6 V
on the board). `OE` active-LOW turns all outputs off — wire it to a GPIO so
you can hard-cut all servos in one instruction. Each channel is a 3-pin
header ready for a standard servo connector.

## Benefits

- Frees up 16 PWM channels without burning 16 ESP32 pins.
- Global frequency control (50 Hz for servos, kHz for LEDs).
- Hardware `OE` pin is a one-line emergency stop.
- Cascadable — six address pins let you run dozens on one bus.

## Limitations / gotchas

- **Shared frequency.** All 16 channels share the same PWM period. You can't
  mix 50 Hz servo and 1 kHz LED on the same chip — buy two.
- The board's V+ trace is thin; for six servos pulling an amp each, feed V+
  from a beefier supply and use the screw terminal, not the header.
- Servo stall draws enough to collapse a shared rail. Budget for a dedicated
  2–5 A BEC.
- `OE` is active-LOW and pulled low by default on many clones — some boards
  come up enabled regardless of MCU state.

## Typical use cases

- Robot arms, hexapods, animatronics (6–24 servos).
- Multi-channel LED dimming (non-addressable strips).
- Large pan-tilt arrays (camera walls).

## Pairs well with

- [`hobby-servos.md`](hobby-servos.md) — the canonical downstream load.
- [`../04-light-display/ws2812b-neopixel.md`](../04-light-display/ws2812b-neopixel.md) —
  PCA9685 is for *non-addressable* strips; WS2812 has its own protocol.
- [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md) —
  cascade multiple PCA9685s behind a mux when you run out of addresses.

## Where to buy

See `buy:`. Adafruit's breakout is the reference; the Chinese HW-272 clone is
mechanically identical and half the price.

## Software / libraries

- `Adafruit_PWMServoDriver` — simplest API.
- `ESP32Servo` piggy-backs on PCA9685 if you want a consistent `Servo` API.

## Notes for WeftOS

Represent each channel in the sensor graph as a logical actuator with its own
clamp (min/max pulse width per servo). Hard-wire `OE` to a GPIO and expose it
as a "panic off" rule target.
