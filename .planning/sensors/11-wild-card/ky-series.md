---
title: "KY-0xx Series (Arduino Sensor Kit master index)"
slug: "ky-series"
category: "11-wild-card"
part_numbers: ["KY-001", "KY-002", "KY-003", "KY-004", "KY-005", "KY-006", "KY-008", "KY-009", "KY-010", "KY-011", "KY-012", "KY-013", "KY-015", "KY-016", "KY-017", "KY-018", "KY-019", "KY-020", "KY-021", "KY-022", "KY-023", "KY-024", "KY-025", "KY-026", "KY-027", "KY-028", "KY-029", "KY-031", "KY-032", "KY-033", "KY-034", "KY-035", "KY-036", "KY-037", "KY-038", "KY-039", "KY-040"]
manufacturer: "Joy-IT / Keyes / many clones"
interface: ["GPIO", "ADC", "PWM", "1-Wire"]
voltage: "3.3V / 5V (varies)"
logic_level: "mixed — check per module"
current_ma: "varies"
price_usd_approx: "$0.20 – $2 each ($15–$25 for 37-in-1 kit)"
esp32_compat: ["ESP32 (with caveats — see notes)"]
tags: [ky-series, arduino-kit, starter-kit, reference, joy-it]
pairs_with:
  - "./esp32-breakout-shields.md"
  - "../09-input-hmi/momentary-buttons.md"
  - "../04-light-display/ws2812b-neopixel.md"
  - "../05-environmental/dht11-dht22.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=sensor+kit" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=37+in+1+sensor+kit" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=grove+kit" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-sensor%20kit.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-37-in-1-sensor-kit.html" }
datasheet: ""
libraries:
  - { name: "Joy-IT KY wiki",     url: "https://joy-it.net/en/products/SEN-KY" }
  - { name: "SensorKit X40",      url: "https://sensorkit.joy-it.net/en/" }
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

"KY-0xx" is the part-number prefix used by Joy-IT / Keyes for the ubiquitous
**37-in-1 Arduino sensor kit**, sold under countless private labels on
AliExpress. Each module is a single sensor on a small PCB with a 3-pin or
4-pin header. Most are very simple — a transistor plus an LED, a comparator
plus a pot, or a single named IC on a breakout. This file is the **master
index** of the whole series so you don't need a file per KY-XXX.

## Per-module table

| Part   | Function                         | Interface | ESP32 notes |
|--------|----------------------------------|-----------|-------------|
| KY-001 | Temperature (DS18B20)            | 1-Wire    | Use DallasTemperature lib |
| KY-002 | Vibration (SW-18010P)            | GPIO      | See [`sw-420-vibration.md`](./sw-420-vibration.md) |
| KY-003 | Hall-effect switch (A3144)       | GPIO      | Works on 3.3 V |
| KY-004 | Tact button                      | GPIO      | See [`../09-input-hmi/momentary-buttons.md`](../09-input-hmi/momentary-buttons.md) |
| KY-005 | IR transmitter (940 nm)          | PWM       | 38 kHz carrier |
| KY-006 | Passive buzzer (small piezo)     | PWM       | Tone generator |
| KY-008 | Laser diode (5 mW, 650 nm)       | GPIO      | **Eye safety — Class IIIa** |
| KY-009 | RGB LED (common-anode SMD)       | PWM × 3   | 5 V preferred |
| KY-010 | Photo-interrupter                | GPIO      | Works on 3.3 V |
| KY-011 | Dual-color LED (red/green)       | PWM × 2   | Common cathode |
| KY-012 | Active buzzer                    | GPIO      | Drives on Vcc |
| KY-013 | Analog temperature (NTC thermistor) | ADC    | Noisy on ESP32 ADC |
| KY-015 | DHT11 temp + humidity            | 1-Wire-ish| Prefer DHT22 — see [`../05-environmental/dht11-dht22.md`](../05-environmental/dht11-dht22.md) |
| KY-016 | RGB LED (through-hole CA)        | PWM × 3   | 5 V preferred |
| KY-017 | Mercury tilt switch              | GPIO      | Avoid — mercury |
| KY-018 | Light-dependent resistor (LDR)   | ADC       | Cheap ambient light |
| KY-019 | 5 V relay                        | GPIO (5 V)| Needs opto-iso for ESP32 safety |
| KY-020 | Tilt ball switch                 | GPIO      | Like SW-420 on one axis |
| KY-021 | Reed switch                      | GPIO      | Magnet-triggered |
| KY-022 | IR receiver (VS1838B)            | GPIO      | 38 kHz demodulator |
| KY-023 | Analog joystick (thumbstick)     | ADC × 2 + GPIO | See [`../09-input-hmi/joysticks.md`](../09-input-hmi/joysticks.md) |
| KY-024 | Linear Hall (49E)                | ADC       | Magnetic field magnitude |
| KY-025 | Reed switch (bigger)             | GPIO      | Door sensor style |
| KY-026 | Flame sensor (IR photodiode)     | ADC / GPIO | False-triggers on sun |
| KY-027 | "Magic light cup" (LED + mercury)| GPIO      | Avoid — mercury |
| KY-028 | Digital temperature (NTC + comp) | GPIO      | Use KY-013 for analog |
| KY-029 | Dual-color LED (red/green 3 mm)  | PWM × 2   | Through-hole variant of KY-011 |
| KY-031 | Knock / shock sensor             | GPIO      | Piezo; digital threshold |
| KY-032 | IR obstacle avoidance            | GPIO      | Adjustable distance pot |
| KY-033 | Line-following TCRT5000          | GPIO      | Robot line-follower |
| KY-034 | 7-color auto-flash LED           | —         | Self-driving LED, no MCU control |
| KY-035 | Analog Hall                      | ADC       | Similar to KY-024 |
| KY-036 | Metal touch sensor               | GPIO      | Conductive-touch, needs contact |
| KY-037 | High-sensitivity mic + comparator| GPIO / ADC| Poor audio quality; good "sound detected" trigger |
| KY-038 | Low-sensitivity mic + comparator | GPIO / ADC| Shorter-range clap detector |
| KY-039 | Heartbeat (finger-clip IR)       | ADC       | Noisy — see [`../06-biometric-presence/max30102-max30105.md`](../06-biometric-presence/max30102-max30105.md) |
| KY-040 | Rotary encoder (EC11 variant)    | GPIO × 3  | See [`./../09-input-hmi/rotary-encoders.md`](../09-input-hmi/rotary-encoders.md) |

## Benefits

- Buys you a **whole curriculum of sensors for ~$20**.
- Every module is documented on the Joy-IT wiki with ESP32 / Arduino examples.
- Great for rapid prototyping, classroom use, and "did I pick the right
  sensor?" ahead of a real-part order.
- Most modules expose the underlying IC — if you want a DHT22 breakout, the
  KY-015 gives you one for pennies.

## Limitations / gotchas

- **Logic-level mix.** Many modules are 5 V only (KY-019 relay, KY-006 buzzer
  at max volume, KY-009 RGB). Check before wiring to a 3.3 V ESP32.
- **Quality is inconsistent.** Cheap clones replace the named IC with a
  lookalike; "DHT11" on KY-015 can be a no-name knockoff.
- **Mercury modules (KY-017, KY-027)** use metallic mercury; responsibly
  source / dispose, or skip them entirely.
- **The laser (KY-008) is Class IIIa** — eye hazard. Treat accordingly.
- ESP32 ADC + cheap analog sensors (KY-013, KY-018, KY-026) is a noisy combo —
  promote to [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md)
  for precision.
- The heartbeat module (KY-039) is a toy; real HR/SpO₂ comes from a MAX30102.

## Typical use cases

- Teaching / demo / curriculum bundles.
- "Which sensor do we actually need?" triage before a BOM lock.
- One-off props (haunted-house motion triggers, escape-room puzzles).

## Pairs well with

- [`./esp32-breakout-shields.md`](./esp32-breakout-shields.md) — screw-terminal / Grove adapters.
- Any category file referenced in the table above (KY-0xx is a superset index).

## Where to buy

The "37-in-1 Arduino sensor kit" on AliExpress / Amazon — $15–$25 for the
whole set. Joy-IT sells the original branded sets with reliable QC.

## Software / libraries

- Most modules need **no library** — GPIO / PWM / ADC only.
- Per-module examples at [sensorkit.joy-it.net](https://sensorkit.joy-it.net/en/).

## Notes for WeftOS

The KY series is ideal as the "hello world" sensor set for a new WeftOS node —
every datatype in the graph (digital event, scalar, PWM output, 1-Wire temp)
has a < $1 KY-0xx representative you can pin to a GPIO. Tag each imported
KY sensor with `provenance: ky-series` in metadata so the WeftOS planner
knows accuracy is lower than the dedicated breakouts in other categories.
