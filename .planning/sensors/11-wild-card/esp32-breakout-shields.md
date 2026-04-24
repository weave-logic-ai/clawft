---
title: "ESP32 Breakout, Shields & Connector Systems"
slug: "esp32-breakout-shields"
category: "11-wild-card"
part_numbers: ["ESP32 30-pin screw breakout", "ESP32 38-pin screw breakout", "FeatherWing (various)", "Qwiic / Stemma QT", "Grove Base Shield", "Gravity"]
manufacturer: "Adafruit / SparkFun / Seeed / DFRobot / generic"
interface: ["passive breakout", "I2C (Qwiic / Stemma QT / Grove-I2C)", "analog (Gravity / Grove)"]
voltage: "3.3V / 5V"
logic_level: "3.3V / 5V depending on shield"
current_ma: "—"
price_usd_approx: "$2 – $15"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [breakout, shield, qwiic, stemma-qt, grove, gravity, feather, expansion]
pairs_with:
  - "./ky-series.md"
  - "../08-communication/tca9548a-i2c-mux.md"
  - "../09-input-hmi/4x4-matrix-keypad.md"
  - "../10-storage-timing-power/microsd.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/category/946" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/qwiic" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/category/Grove-c-1003.html" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/gravity" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-esp32-breakout-board.html" }
datasheet: ""
libraries: []
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

"Shields" and "breakouts" are the mechanical glue of an ESP32 project — boards
whose only job is to give you clean access to the module's pins, a friendlier
connector system, or a stackable form factor. They rarely do anything
electrically interesting, but they make the difference between "rats-nest of
jumpers" and "actual product".

## Families

### Screw-terminal breakouts (30-pin / 38-pin)

Generic carrier PCB that clones the ESP32 DevKitC pinout to screw terminals
plus a 5 V / 3.3 V rail. Perfect for "plug & forget" field installs where a
breadboard won't survive.

- **30-pin** fits short ESP32 NodeMCU boards.
- **38-pin** fits the more common 38-pin ESP32 dev boards (including ESP32-S3
  clones).
- Usually includes a 5 V barrel jack and an onboard LDO → 3.3 V (redundant but
  harmless).
- ~$3 on AliExpress; buy with the matching DevKit or measure pin-spacing first
  because variants exist.

### FeatherWing-style shields

Adafruit's **Feather** form factor is a 28-pin stacking header. "Wings" are
shields that plug on top to add: OLED, SD + RTC, LoRa, CAN, motor driver,
airlift, etc. ESP32 Feather variants (HUZZAH32, ESP32-S3 Feather, ESP32-S2
Feather, ESP32-C3) all share the same pinout — a wing you buy today works on
next year's board.

### Qwiic / Stemma QT (I²C connector system)

- **Qwiic** (SparkFun) — 4-pin polarized JST-SH cable: GND, 3.3 V, SDA, SCL.
  Daisy-chainable. Every SparkFun I²C breakout has two Qwiic ports for
  chaining.
- **Stemma QT** (Adafruit) — electrically identical to Qwiic; Adafruit
  rebranded their I²C breakouts to use the same connector. The ecosystems
  interoperate.

Use these when you want "plug-together" I²C without soldering. Pair with
[`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md)
when you chain > 5 sensors and need address / isolation control. Qwiic/Stemma
QT is **3.3 V only** — don't plug a 5 V-signalled device like PCF8591 into it
without a level shifter.

### Grove (Seeed)

4-pin polarized connector used by Seeed Studio. Variants:

- **Grove-I2C** (yellow) — SDA/SCL/Vcc/GND.
- **Grove-UART** (blue) — TX/RX/Vcc/GND.
- **Grove-Digital** (black) — two GPIOs.
- **Grove-Analog** (red) — two ADC pins.

The Grove Base Shield gives you a bunch of these connectors on a stacking
board. Compatible with XIAO ESP32-S3 / C3 via Seeed's own adapter boards.

### Gravity (DFRobot)

DFRobot's alternative to Grove — 3-pin connector for analog/digital sensors,
4-pin for I²C. Color-coded (blue = digital, red = analog, green = I²C, black
= UART). Slightly higher price than Grove but consistent polarity and good
documentation.

## Benefits

- **No soldering** (Qwiic / Stemma QT / Grove / Gravity) = prototypes in
  minutes instead of hours.
- **Polarized connectors** = no reversed-polarity disasters.
- **Stacking form factors** (Feather, screw-terminal) = clean deployments.
- **Ecosystems are crossing over** — Adafruit Stemma QT interoperates with
  SparkFun Qwiic; Seeed XIAO boards now often include a Qwiic/Stemma port.

## Limitations / gotchas

- **Every ecosystem is 3.3 V, 400 kHz I²C.** Plugging a 5 V device damages
  everything. Verify voltage domain.
- **Cable length matters.** I²C over 20+ cm of thin Qwiic cable adds
  capacitance and can kill the bus — use shorter cables or add active pull-ups.
- **Address collisions don't care about connectors.** Two I²C sensors on the
  same bus at the same address = bus broken regardless of fancy cables. Use a
  mux ([`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md)).
- Screw-terminal breakouts **don't filter / protect the rail** — a short on
  your field wiring can take the ESP32 with it. Add a PTC + TVS on any
  user-accessible terminal.
- Feather and Grove and Qwiic connectors are **not mechanically
  interchangeable** — pick one ecosystem per project.

## Typical use cases

- "Dev-to-deployed" — screw-terminal breakout for a field install of a
  prototype.
- Classroom / demo rigs — Qwiic / Grove lets students plug sensors together
  without tools.
- Stackable expansion — Feather + FeatherWings for mass-produced / consistent
  layouts.
- Multi-sensor bus with mux — Qwiic hub + TCA9548A + 8 identical BME280s.

## Pairs well with

- [`./ky-series.md`](./ky-series.md) — adapt 3-pin KY modules to Grove / Gravity.
- [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md) — chain many Qwiic sensors.
- [`../09-input-hmi/4x4-matrix-keypad.md`](../09-input-hmi/4x4-matrix-keypad.md) — screw-terminal keypad wiring.
- [`../10-storage-timing-power/microsd.md`](../10-storage-timing-power/microsd.md) — SD Feather Wing combo.

## Where to buy

- Adafruit Feather + FeatherWings catalog — [adafruit.com/category/946](https://www.adafruit.com/category/946).
- SparkFun Qwiic — [sparkfun.com/qwiic](https://www.sparkfun.com/qwiic).
- Seeed Grove — [seeedstudio.com/category/Grove-c-1003.html](https://www.seeedstudio.com/category/Grove-c-1003.html).
- DFRobot Gravity — [dfrobot.com/gravity](https://www.dfrobot.com/gravity).
- Generic screw-terminal breakouts on AliExpress ~$3.

## Software / libraries

None — these are connector / carrier boards. Their job is mechanical.

## Notes for WeftOS

Record connector ecosystem (`ecosystem: qwiic | stemma_qt | grove | gravity |
feather | bare`) in node metadata so WeftOS's planner can suggest compatible
sensors when the user adds hardware. Bus-level properties (logic level, default
pull-ups) should surface up to the sensor-graph substrate so drivers don't
have to rediscover them per sensor.
