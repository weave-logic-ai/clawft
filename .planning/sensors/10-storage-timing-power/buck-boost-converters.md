---
title: "Buck / Boost Converters (MP1584, LM2596, MT3608, XL6009)"
slug: "buck-boost-converters"
category: "10-storage-timing-power"
part_numbers: ["MP1584", "LM2596", "MT3608", "XL6009"]
manufacturer: "MPS / TI / XLSEMI / generic"
interface: ["—"]
voltage: "varies — see topology table below"
logic_level: "—"
current_ma: "up to 2–4 A output depending on part"
price_usd_approx: "$0.50 – $3"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [power, dcdc, buck, boost, converter, mp1584, lm2596, mt3608, xl6009]
pairs_with:
  - "./tp4056-lipo-charger.md"
  - "./ina219-ina260.md"
  - "./solar-charge-controllers.md"
  - "../07-control-actuators/hobby-servos.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=buck+converter" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=buck+converter" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=dc-dc" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-dc-dc.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-mp1584.html" }
datasheet: "https://www.monolithicpower.com/en/mp1584.html"
libraries: []
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

"Generic Chinese DC-DC module" is a category, not a part. This file covers the
four most common switching-regulator modules you'll see in every hobby bin —
the first two are **bucks** (step-down), the last two are **boosts** (step-up).
Pick the right topology for your rail, then pick the chip that handles your
worst-case current with margin.

## Topology cheat sheet

| Part   | Topology | V<sub>IN</sub> | V<sub>OUT</sub> | I<sub>OUT</sub> (max) | Notes |
|--------|----------|---------------|-----------------|-----------------------|-------|
| MP1584 | Buck     | 4.5 – 28 V    | 0.8 – 20 V      | 3 A                   | Small, efficient, PWM-adjustable |
| LM2596 | Buck     | 4.5 – 40 V    | 1.5 – 35 V      | 2 – 3 A               | Classic, 150 kHz, low-efficiency variant |
| MT3608 | Boost    | 2.0 – 24 V    | up to 28 V      | 2 A (input-limited)   | Common 3.7 V → 5 V / 12 V |
| XL6009 | Boost    | 3 – 32 V      | up to 35 V      | 3 – 4 A peak          | Beefier than MT3608 |

## Interface & wiring

- Solder `IN+`/`IN–` to the source, turn the trim-pot to set V<sub>OUT</sub>,
  then solder `OUT+`/`OUT–` to the load. Measure first; trim pots are cheap
  single-turn and bump around.
- Add your own output capacitance (10 – 100 µF) if the load has fast transients
  (ESP32 Wi-Fi transmit, motors).
- Most boards have a power LED that leaks current — remove it for battery use.

## Benefits

- Dirt cheap.
- Switching efficiency (80–95%) vs. a 7805 linear (can be < 30% efficient).
- Covers the whole hobby power range in 4 parts.

## Limitations / gotchas

- **Pick the right topology.** A buck cannot step up; a boost cannot step down.
  A LiPo at 3.0 V (nearly empty) through an MT3608 can still push 5 V; a buck
  cannot.
- **Boost converters can't isolate the load from input on shutdown** — the
  output diode still conducts when enabled = off. Add a MOSFET switch
  downstream if you need a true off state.
- **EMI:** the cheap modules are unshielded. Running a 2.4 GHz ESP32 within a
  few cm of an unshielded boost at 500 kHz can desense the receiver.
- **Trim-pot drift** — the single-turn pots are jittery. Use 25-turn pots on
  critical rails, or swap to fixed feedback resistors.
- **Inrush current** on a boost with a big output cap can latch-off the
  upstream charger; add a soft-start or slow-start cap.
- LM2596 is slower and less efficient than MP1584; for new designs start with
  MP1584.

## Typical use cases

- **LiPo (3.0–4.2 V) → stable 3.3 V buck** for ESP32 + peripherals. (Use a
  low-Iq LDO for ultra-low-power sleep, or a buck if active current is high.)
- **LiPo → 5 V boost** for servos, LED strips, USB accessories.
- **12 V automotive → 5 V buck** for in-car ESP32 projects.
- **Single AA (1.5 V) → 3.3 V boost** for ultra-low-power demos (needs a
  purpose-built sub-1 V start-up converter, not MT3608).

## Pairs well with

- [`./tp4056-lipo-charger.md`](./tp4056-lipo-charger.md) — charge path upstream.
- [`./ina219-ina260.md`](./ina219-ina260.md) — verify output current under load.
- [`../07-control-actuators/hobby-servos.md`](../07-control-actuators/hobby-servos.md) — servos need a beefy 5 V / 6 V rail.
- [`./solar-charge-controllers.md`](./solar-charge-controllers.md) — converter downstream of solar.

## Where to buy

AliExpress 10-packs of MP1584 / MT3608 for ~$5. Adafruit and Pololu carry
better-shielded / higher-efficiency alternatives (TPS6291x, TPS61023).

## Software / libraries

None — they're analog power parts. Instrument the rail with an INA219 for
telemetry.

## Notes for WeftOS

Track the upstream power-path topology in node metadata so WeftOS can estimate
efficiency and battery run-time. Boost stages should be tagged with
`no_true_off` capability — the sensor scheduler should not assume cutting
`enable` kills current.
