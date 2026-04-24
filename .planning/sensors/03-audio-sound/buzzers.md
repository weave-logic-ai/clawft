---
title: "Buzzers (Active vs Passive Piezo)"
slug: "buzzers"
category: "03-audio-sound"
part_numbers: ["active piezo buzzer", "passive piezo buzzer", "magnetic buzzer"]
manufacturer: "various"
interface: ["GPIO (active)", "PWM / tone (passive)"]
voltage: "3.3V / 5V"
logic_level: "3.3V direct (active), PWM for passive"
current_ma: "~15–40 mA"
price_usd_approx: "$0.20 – $2"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [buzzer, piezo, beep, tone, alarm]
pairs_with:
  - "./max98357a-amp.md"
  - "../04-light-display/index.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=piezo+buzzer" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=buzzer" }
  - { vendor: DigiKey, url: "https://www.digikey.com/en/products/search?keywords=piezo%20buzzer" }
  - { vendor: Mouser, url: "https://www.mouser.com/c/?q=piezo%20buzzer" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-piezo-buzzer.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A buzzer is the cheapest possible sound output. Two flavors: **active** (has its own oscillator
inside — drive HIGH to beep at a fixed frequency) and **passive** (needs a PWM / square-wave
input, frequency-controlled). Passive buzzers can play simple tunes; active buzzers are louder
per dollar but monotone.

## Key specs

| Type | Frequency | Drive |
|---|---|---|
| Active piezo | ~2.7 kHz fixed | HIGH on GPIO (sometimes via transistor) |
| Passive piezo | ~31 Hz – 10 kHz | PWM / `tone()` |
| Magnetic active | ~2 kHz | Needs ~30 mA; use transistor/MOSFET |

## Interface & wiring

Active piezo buzzers can often be driven straight from a GPIO if current is <10 mA; louder
variants need a small NPN transistor or MOSFET. Passive buzzers want a square wave at the
desired tone frequency; the ESP32 LEDC peripheral is the clean way to do this.

## Benefits

- Dirt cheap.
- Draws minimal board real estate.
- Good for feedback beeps, alarms, and "I'm alive" indicators.

## Limitations / gotchas

- Passive piezos are loud at their resonant frequency and *quiet* elsewhere; pick one whose
  resonance matches your target tone.
- Active piezos have zero tonal control — one note.
- Some magnetic buzzers are polarity-sensitive — orientation matters.
- Continuous operation creates annoying high-pitched overtones; duty-cycle them for alerts.

## Typical use cases

- Keypad / UI click feedback.
- Low-battery / error alarm.
- Simple melodies (ESP32 `tone` / `ledcWriteTone`).
- Ultrasonic rat / pest repeller (30 kHz passive piezo).

## Pairs well with

- [`./max98357a-amp.md`](./max98357a-amp.md) when the buzzer isn't enough and you need real audio.
- [`../04-light-display/index.md`](../04-light-display/index.md) for light+sound combined feedback.

## Where to buy

- Adafruit / SparkFun / DigiKey / Mouser for characterized parts.
- AliExpress for 10-packs of unspecified but usable buzzers.

## Software / libraries

- `tone()` on Arduino (passive buzzers).
- `ledcWriteTone` on ESP32 for PWM-controlled tones.

## Notes for WeftOS

Speculative: a buzzer is the minimal "audio event sink" — a tiny surface that only handles
timed beeps. WeftOS shouldn't upgrade it to a full audio pipeline; a dedicated "notification
tone" effect type keeps the contract small and cheap to schedule.
