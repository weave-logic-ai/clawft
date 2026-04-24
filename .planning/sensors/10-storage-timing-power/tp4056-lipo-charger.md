---
title: "TP4056 1-Cell LiPo Charger"
slug: "tp4056-lipo-charger"
category: "10-storage-timing-power"
part_numbers: ["TP4056", "TP4056 + DW01 + FS8205A (protection)"]
manufacturer: "Top Power ASIC / NanJing Top Power"
interface: ["—"]
voltage: "4.0 – 8.0 V input; 4.2 V LiPo output"
logic_level: "—"
current_ma: "programmable charge current up to 1 A"
price_usd_approx: "$0.30 – $1"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [power, lipo, charger, tp4056, battery, protection]
pairs_with:
  - "./max17048-fuel-gauge.md"
  - "./ina219-ina260.md"
  - "./solar-charge-controllers.md"
  - "./buck-boost-converters.md"
  - "../07-control-actuators/mosfet-drivers.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=lipo+charger" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=lipo+charger" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=tp4056" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-tp4056.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-tp4056.html" }
datasheet: "https://dlnmh9ip6v2uc.cloudfront.net/datasheets/Prototyping/TP4056.pdf"
libraries: []
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

The TP4056 is the ubiquitous single-cell LiPo/Li-ion linear charger. It
implements the CC/CV profile a LiPo wants (constant current to ~4.2 V, then
constant voltage until current tapers), with a `PROG` resistor setting the
charge current up to 1 A. On almost every cheap module, two status LEDs show
charge state (red = charging, green = done). The "improved" variant adds a
**DW01 + FS8205A** protection pair for over/undervoltage, overcurrent, and
short-circuit protection — this is the **only** variant you should use if the
battery is exposed to users.

## Key specs

| Spec | TP4056 |
|------|--------|
| Input | 4.0 – 8.0 V (USB 5 V typical) |
| Output | 4.2 V ±1% float |
| Charge current | up to 1 A, set by `PROG` resistor (1.2 kΩ = 1 A) |
| Topology | Linear (dissipates heat on IC) |
| Termination | 1/10 of programmed current |

## Interface & wiring

- 5 V in on `IN+`/`IN–` (micro-USB, USB-C, or bare pads).
- LiPo on `B+`/`B–`.
- Load on `OUT+`/`OUT–` (on the *protected* boards) — protection MOSFETs sit
  between battery and load.
- `PROG` resistor sets I<sub>CHG</sub>. Default 1.2 kΩ ≈ 1 A. For a 500 mAh
  cell, change it to 2.4 kΩ for 500 mA (0.5C) charge.
- Status pins `STDBY` and `CHRG` are open-drain — can drive LEDs or MCU GPIOs.

## Benefits

- Dirt cheap — entire charger board < $1.
- Proper CC/CV profile for 1S LiPo / Li-ion.
- Protected variant (DW01 + FS8205A) is safe-by-default.
- Works directly from USB 5 V.

## Limitations / gotchas

- **Unprotected (no DW01) modules are dangerous.** Buy the protected variant
  **always** unless you are adding external protection.
- **Linear charger** — dissipates `(V_IN – V_BAT) × I_CHG` as heat. At 1 A from
  5 V into a 3.6 V cell that is ~1.4 W on a tiny SOIC, which easily hits
  thermal shutdown. Lower charge current for dense cells, or add a heatsink.
- Charges to a fixed 4.2 V — **not safe for LiFePO4** (3.6 V float) unless you
  hack the resistor network.
- Can't charge multi-cell packs.
- Many cheap boards have USB connector cold joints — re-solder if USB is
  unreliable.
- Not a fuel gauge — pair with an [`MAX17048`](./max17048-fuel-gauge.md) for SoC.

## Typical use cases

- Battery-powered ESP32 nodes with a single 18650 or pouch LiPo.
- Portable displays / badges.
- Solar-powered nodes — pair with a [`solar charge controller`](./solar-charge-controllers.md)
  rather than TP4056 directly (TP4056's input undervoltage lockout fights MPPT).

## Pairs well with

- [`./max17048-fuel-gauge.md`](./max17048-fuel-gauge.md) — battery state-of-charge.
- [`./ina219-ina260.md`](./ina219-ina260.md) — charge current monitoring.
- [`./buck-boost-converters.md`](./buck-boost-converters.md) — boost 3.7 V LiPo → 5 V rail.
- [`./solar-charge-controllers.md`](./solar-charge-controllers.md) — solar input.

## Where to buy

AliExpress 10-packs for a few dollars. Adafruit PowerBoost-charger boards are
TP4056-class with nicer protection and USB-C.

## Software / libraries

None — it's an analog part. Read the `STDBY`/`CHRG` pins with GPIO if you want
charge-state telemetry.

## Notes for WeftOS

A TP4056 should surface as **power-path capability metadata** (charger present,
max charge current, protection variant Y/N). Protection status is critical:
WeftOS should refuse battery-level automation on unprotected boards.
