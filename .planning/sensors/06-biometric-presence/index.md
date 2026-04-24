---
title: "06 – Biometric, Health & Presence"
slug: "06-biometric-presence"
category: "06-biometric-presence"
description: "Modules that measure people: heart rate, blood oxygen, fingerprint, and presence across the PIR → microwave → mmWave spectrum."
tags: [biometric, presence, health, pir, mmwave, esp32, index]
source: "../../weftos_sensors.md"
status: "draft"
---

# 06 – Biometric, Health & Presence

This category covers sensors that measure **humans**: vital signs, identity, and presence.
The "presence" problem in particular shows up everywhere in WeftOS (smart lights, HVAC,
access control, occupancy analytics), and is solved across a spectrum of technologies
with very different tradeoffs.

## Modules

| Module | Type | Interface | One-liner |
|--------|------|-----------|-----------|
| [MAX30102 / MAX30105](max30102-max30105.md) | Pulse-ox + HR | I²C | IR + red LED + photodiode, optical HR/SpO₂. |
| [AS608 / R307](as608-fingerprint.md) | Fingerprint | UART | Embedded capacitive-scan + matching engine. |
| [PIR HC-SR501](pir-hc-sr501.md) | Passive IR | GPIO | Classic low-power motion trigger. |
| [RCWL-0516](rcwl-0516-microwave.md) | Microwave Doppler | GPIO | Sees through thin walls; very cheap; crude. |
| [LD2410 / LD2410C / LD2410S](ld2410-mmwave.md) | 24 GHz mmWave | UART | Presence + micro-motion zones; seated humans. |
| [C4001 mmWave](c4001-mmwave.md) | 24 GHz mmWave | UART / I²C | Breathing, heart-rate, fall detection. |
| [AD8232 ECG](ecg-modules.md) | Single-lead ECG | ADC | Analog biopotential front-end. |
| [FSR 402 / 406](fsr.md) | Force-sensitive resistor | ADC | Pressure / touch / weight-in-seat. |

## The presence spectrum

WeftOS projects typically climb this ladder when "is a person there?" needs to get smarter:

1. **PIR (HC-SR501)** — detects *motion* of a warm body. Fails the moment someone sits still.
2. **Microwave Doppler (RCWL-0516)** — detects *any* motion via Doppler shift, even through
   thin walls. Still motion-based, and notoriously triggered by insects, HVAC airflow,
   and fluorescent fixtures.
3. **mmWave 24 GHz (LD2410-family)** — detects both macro motion and **micro-motion**
   (breathing, small fidgets). Enables "someone is seated on the couch" detection.
4. **mmWave health-grade (C4001)** — on top of presence, extracts heart-rate and
   breathing-rate, and can flag falls. This is starting to blur into biometric.

## Privacy implications

Every module in this category produces data that can be re-identifying. WeftOS's
stance:

- Biometric identifiers (**fingerprint templates**, **ECG traces**, **HR/SpO₂ streams**)
  must never leave the device plaintext, and must never be stored in cloud-backed
  memory namespaces without explicit user scope.
- mmWave radar does **not** produce camera frames, but it *does* let you infer room
  occupancy, sleep schedule, and breathing rate. Treat it as biometric-adjacent.
- PIR and microwave Doppler leak a coarse "someone is moving here right now" signal;
  safer, but still worth rate-limiting off-device.

## Creative Lego ideas (from source)

- Fingerprint + relay → DIY smart lock.
- mmWave + ESP32 + MQTT → bed-occupancy sensor for smart HVAC.
- MAX30102 + OLED → wearable HR display.

## Cross-links

- [`../01-vision-imaging/esp32-cam-ov2640.md`](../01-vision-imaging/esp32-cam-ov2640.md) —
  pair presence detection with a camera trigger to save storage / bandwidth.
- [`../07-control-actuators/relay-modules.md`](../07-control-actuators/relay-modules.md) —
  presence → actuation (lights, locks, HVAC).
- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) —
  upgrade noisy analog biopotential / FSR reads.
