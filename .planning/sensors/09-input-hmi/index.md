---
title: "Input & Human Interface"
slug: "09-input-hmi"
category: "09-input-hmi"
description: "Buttons, rotary encoders, joysticks, capacitive touch pads, matrix keypads, and potentiometers for ESP32-based HMI."
tags: [input, hmi, buttons, encoders, joysticks, capacitive, keypad, potentiometer]
source: "../../weftos_sensors.md"
status: "draft"
---

# 09 — Input & Human Interface

This category covers the physical controls a human pokes at to talk to an ESP32:
momentary buttons, rotary encoders, analog thumbsticks, capacitive-touch pads,
matrix keypads, and pots. These are "user side" of the sensor graph and typically
generate discrete events (GPIO edges) or quantized analog values.

## Modules

| Module | File | Interface | Notes |
|--------|------|-----------|-------|
| Tact switches / button boards | [`momentary-buttons.md`](momentary-buttons.md) | GPIO | Debounce, pull-ups |
| Rotary encoders (KY-040 / EC11) | [`rotary-encoders.md`](rotary-encoders.md) | GPIO / PCNT | Quadrature, push |
| Analog joystick (thumbstick) | [`joysticks.md`](joysticks.md) | ADC x2 + GPIO | 2-axis + button |
| TP223 capacitive touch | [`tp223-capacitive-touch.md`](tp223-capacitive-touch.md) | GPIO | Single-pad switch |
| 4×4 matrix keypad | [`4x4-matrix-keypad.md`](4x4-matrix-keypad.md) | 8× GPIO | Needs scan / expander |
| Potentiometers | [`potentiometers.md`](potentiometers.md) | ADC | Rotary + slide pots |

## Debouncing cheat sheet

- Tact switches bounce 1–20 ms on press **and** release — always debounce.
- Hardware debounce: RC filter (10 kΩ + 100 nF) + Schmitt-trigger input, or a
  dedicated IC like MAX6816/MC14490.
- Software debounce: sample at 1 kHz; require N stable samples, or use a state
  machine with a timeout. `Bounce2` (Arduino) and `ButtonDebouncer` (ESPHome)
  both work well.
- Rotary encoders bounce worse than switches — prefer ESP32's **PCNT**
  (pulse-counter) peripheral to sink the jitter in hardware.

## Input expansion

ESP32 GPIOs run out quickly once you add a camera, SD card, and display. Options:

- **PCF8574 / PCF8575** (I²C GPIO expander) — cheap, 8 or 16 pins per chip,
  interrupt pin for change notification. Ideal for matrix keypads and LED
  annunciators. See [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md)
  for bus-level multiplexing.
- **MCP23017 / MCP23S17** (I²C / SPI) — 16 pins per chip, per-pin interrupts,
  higher drive strength.
- **74HC165 / 74HC595** shift registers — dirt cheap, good for many buttons or
  LEDs, but no per-pin IRQ.

## Creative Lego ideas

- "Control deck" for a WeftOS headless node: encoder for menu, 4×4 keypad for
  preset recall, capacitive pad for "wake", joystick for pan/tilt camera.
- Low-effort hand controller: 2× thumbsticks + 4× tact buttons over I²C
  expander → Bluetooth HID device.
- Accessibility input: large TP223 pads for single-switch scanning interfaces.

## Pairs with

- [`../04-light-display/ssd1306-oled.md`](../04-light-display/ssd1306-oled.md) — menu UI.
- [`../07-control-actuators/hobby-servos.md`](../07-control-actuators/hobby-servos.md) — joystick-driven gimbal.
- [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md) — bus expansion.
