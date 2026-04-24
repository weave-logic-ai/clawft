---
title: "Solar Charge Controllers (CN3791 & friends)"
slug: "solar-charge-controllers"
category: "10-storage-timing-power"
part_numbers: ["CN3791", "CN3065", "CN3722", "SPV1040"]
manufacturer: "Consonance / ST"
interface: ["—"]
voltage: "PV input 4 – 28 V; 1S LiPo output"
logic_level: "—"
current_ma: "up to ~1 A charge"
price_usd_approx: "$2 – $8"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: [power, solar, mppt, lipo, charger, cn3791, off-grid]
pairs_with:
  - "./tp4056-lipo-charger.md"
  - "./max17048-fuel-gauge.md"
  - "./ina219-ina260.md"
  - "../05-environmental/bme280.md"
  - "../08-communication/sx1276-rfm95-lora.md"
buy:
  - { vendor: Adafruit,    url: "https://www.adafruit.com/?q=solar+charger" }
  - { vendor: SparkFun,    url: "https://www.sparkfun.com/search/results?term=solar+charger" }
  - { vendor: Seeed,       url: "https://www.seeedstudio.com/catalogsearch/result/?q=cn3791" }
  - { vendor: DFRobot,     url: "https://www.dfrobot.com/search-solar%20charger.html" }
  - { vendor: AliExpress,  url: "https://www.aliexpress.com/w/wholesale-cn3791.html" }
datasheet: "http://www.consonance-elec.com/pdf/datasheet/DSE-CN3791.pdf"
libraries: []
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

Cheap "MPPT" 1-cell LiPo solar chargers — the CN3791 is the most common — are
**MPPT-lite**: they implement an input-voltage regulation loop that holds the
solar panel at a preset voltage (typically 70–85% of V<sub>OC</sub>), which is a
rough approximation of true maximum-power-point tracking. Much better than
running a TP4056 straight from a panel (where input-UVLO chatter cripples
yield), good enough for ESP32 / LoRa nodes on a 1–6 W panel.

## Key specs

| Spec | CN3791 |
|------|--------|
| PV input | 4.5 – 28 V |
| Battery | 1S LiPo / Li-ion, 4.2 V float |
| Charge current | programmable, up to ~1 A with `R_CS` |
| MPPT voltage | set by resistor divider on `VINREG` pin |
| Efficiency | ~85% at 500 mA |

## Interface & wiring

- Solar panel to `VIN`/`GND`; pick a panel whose MPP voltage matches the
  `VINREG` set point on the board (default 6 V for a 6 V panel, 9 V for 9 V).
- LiPo to `BAT+`/`BAT–`.
- `PROG` (`R_CS`) resistor sets charge current. 2 kΩ ≈ 500 mA.
- Some boards include a load output with automatic disconnect at low battery
  (Vbat < 3.0 V) — exactly what you want for an ESP32 node that should not
  deep-discharge the cell.
- Pair with a [`MAX17048`](./max17048-fuel-gauge.md) for SoC and an
  [`INA219`](./ina219-ina260.md) for live solar current monitoring.

## Benefits

- Keeps working at low light where TP4056 bounces on UVLO.
- Many boards include load switch + UVLO — full "off-grid sensor node in a
  can".
- Cheap (~$3) and widely available.

## Limitations / gotchas

- **Not true MPPT** — just a fixed input-voltage clamp. For 20 W+ panels you
  want a real MPPT IC (BQ24650, SPV1040 with dithering, or a Victron module).
- The `VINREG` resistor divider is preset for a specific panel V<sub>OC</sub>.
  Using a mismatched panel can drop yield by half.
- 1-cell LiPo only. No lead-acid, no LiFePO4 (different float voltage), no
  multi-cell.
- Cheap boards may omit input reverse-polarity protection — add an input diode
  or a P-FET if users can hot-plug the panel.
- Charge current tops out around 1 A — for larger systems, parallel two or move
  to a proper charge controller.

## Typical use cases

- Solar LoRa weather station (panel + CN3791 + 18650 + ESP32 + LoRa).
- Off-grid game-camera / trail-cam trigger.
- Remote agriculture / tank-level monitor.
- Outdoor air-quality node (PMS5003 is hungry — size accordingly).

## Pairs well with

- [`./tp4056-lipo-charger.md`](./tp4056-lipo-charger.md) — USB-backup path when panel can't keep up.
- [`./max17048-fuel-gauge.md`](./max17048-fuel-gauge.md) — SoC telemetry.
- [`./ina219-ina260.md`](./ina219-ina260.md) — monitor PV current.
- [`../05-environmental/bme280.md`](../05-environmental/bme280.md) — typical payload for a solar node.
- [`../08-communication/sx1276-rfm95-lora.md`](../08-communication/sx1276-rfm95-lora.md) — low-duty-cycle radio for solar budgets.

## Where to buy

CN3791 modules on AliExpress ~$3. Adafruit sells nicer "solar LiPo charger"
boards based on bq24074 / bq25896 for higher-end designs.

## Software / libraries

None — analog part. Instrument with INA219 / MAX17048 for visibility.

## Notes for WeftOS

A solar-powered node should expose a `power_path` entity combining PV
controller, battery, and fuel gauge. WeftOS can then schedule sampling bursts
relative to SoC and solar insolation forecast.
