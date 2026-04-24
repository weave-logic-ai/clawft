---
title: "Buzzers – Active vs Passive Piezo (Actuator View)"
slug: "buzzers"
category: "07-control-actuators"
part_numbers: ["active piezo 5V", "passive piezo element"]
manufacturer: "various"
interface: ["GPIO", "PWM"]
voltage: "3.3–5V"
logic_level: "3.3V compatible"
current_ma: "~20–40 mA"
price_usd_approx: "$0.50 – $3"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [buzzer, piezo, actuator, alarm, notification]
pairs_with:
  - "../07-control-actuators/vibration-motors.md"
  - "../04-light-display/ws2812b-neopixel.md"
  - "../06-biometric-presence/pir-hc-sr501.md"
buy:
  - { vendor: Adafruit,   url: "https://www.adafruit.com/?q=piezo+buzzer" }
  - { vendor: SparkFun,   url: "https://www.sparkfun.com/search/results?term=buzzer" }
  - { vendor: Seeed,      url: "https://www.seeedstudio.com/catalogsearch/result/?q=buzzer" }
  - { vendor: DFRobot,    url: "https://www.dfrobot.com/search-buzzer.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-piezo-buzzer.html" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A buzzer is the simplest audio actuator in the catalog. Two kinds:

- **Active** buzzer: has an internal oscillator. Apply DC → it beeps at a fixed
  frequency. One GPIO, no PWM.
- **Passive** piezo: just a piezo disc. Needs you to drive it with a square
  wave at the desired frequency. PWM → pitched tones / simple melodies.

This file is the **actuator-centric** take; see also the audio-focused entry
at [`../03-audio-sound/buzzers.md`](../03-audio-sound/buzzers.md) if it
exists — think of this one as "I need to beep on an alarm", not "I need a
speaker".

## Key specs

| Spec | Active | Passive |
|------|--------|---------|
| Drive | DC on/off | Square-wave PWM (1–5 kHz typical) |
| SPL at 10 cm | ~85 dB | ~80 dB (depends on resonance) |
| Frequency | fixed (~2.3 kHz) | user-selected |
| Supply | 3–5 V | 3–5 V |
| Current | ~30 mA | ~30 mA peak |

## Interface & wiring

**Active:** `VCC / GND` and a single GPIO — treat it like a LED you can hear.
**Passive:** wire to an ESP32 LEDC PWM channel; consider a small NPN / MOSFET
if you want more volume than the GPIO's drive limit allows. A ~100 Ω series
resistor protects the GPIO in either case.

## Benefits

- Cheapest way to annoy a human from across a room.
- Reliable, low power, drop-anywhere component.
- Passive piezos can play recognizable tunes with no extra hardware.

## Limitations / gotchas

- **Audio-grade they are not.** Don't expect waveforms with any fidelity out
  of a passive piezo — it's a narrow-band resonator.
- Passive buzzers are often returned as "broken" because the user drove them
  as DC. Without PWM they're silent.
- Sealed enclosures muffle the sound dramatically; cut a vent hole.
- Some buzzers are polarity-sensitive (marked `+`); passive discs usually
  aren't.

## Typical use cases

- Alarm / fault notification on IoT devices.
- UI click / confirm sound on a button press.
- Intruder / smoke-alarm siren paired with a relay cut-off.
- Teaching RTTTL-style melody playback.

## Pairs well with

- [`vibration-motors.md`](vibration-motors.md) — audible + tactile notification
  for accessibility.
- [`../04-light-display/ws2812b-neopixel.md`](../04-light-display/ws2812b-neopixel.md) —
  strobe + beep for alarm clarity.
- [`../06-biometric-presence/pir-hc-sr501.md`](../06-biometric-presence/pir-hc-sr501.md) —
  intrusion-trigger sirens.

## Where to buy

See `buy:`. Both types ship in hobby-kit assortments; passive discs in bulk
are pennies each on AliExpress.

## Software / libraries

Arduino's `tone()` works on ESP32 via the LEDC driver. ESPHome has
`output: ledc` + `rtttl` component for melody playback.

## Notes for WeftOS

Model a buzzer as a notification sink with a **max on-time** and **min
inter-event interval** — otherwise rule storms produce literal screaming. For
alarms, interlock with a user-acknowledge rule so the buzzer can always be
silenced.
