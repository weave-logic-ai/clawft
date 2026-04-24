---
title: "Analog pH Probe + BNC Signal Board (DFRobot style)"
slug: "ph-sensor"
category: "05-environmental"
part_numbers: ["DFR0300", "SEN0161", "PH-4502C"]
manufacturer: "DFRobot / generic"
interface: ["ADC"]
voltage: "5V"
logic_level: "3.3V"
current_ma: "~10 mA"
price_usd_approx: "$25 – $60 (probe + board)"
esp32_compat: ["ESP32", "ESP32-S3", "ESP32-C3"]
tags: ["ph", "water-quality", "hydroponics", "aquarium"]
pairs_with:
  - "./ds18b20.md"
  - "./ec-tds.md"
  - "./turbidity-sensor.md"
  - "../10-storage-timing-power/ads1115-ads1015.md"
buy:
  - { vendor: Adafruit, url: "https://www.adafruit.com/?q=ph+sensor" }
  - { vendor: SparkFun, url: "https://www.sparkfun.com/search/results?term=ph" }
  - { vendor: Seeed,    url: "https://www.seeedstudio.com/catalogsearch/result/?q=ph" }
  - { vendor: DFRobot,  url: "https://www.dfrobot.com/search-ph.html" }
  - { vendor: AliExpress, url: "https://www.aliexpress.com/w/wholesale-ph-sensor-module.html" }
libraries:
  - "no dedicated MCU driver — ADC + linear 2-point calibration at pH 4.0 and 7.0 (or 7.0 & 10.0)"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

A pH sensor is a glass electrode connected via BNC to a signal-conditioning board that translates the ± 414 mV pH-dependent signal into a MCU-friendly 0–3 V analog output. The common DFRobot / "PH-4502C" kits pair an industrial probe with an op-amp board. For WeftOS, pH is the #1 hydroponics control variable — nutrient availability depends directly on pH in the root zone.

## Key specs

- Range: typically 0–14 pH; effective 2–12 pH.
- Accuracy: ± 0.1 pH with fresh calibration; drifts to ± 0.3 after weeks.
- Temperature dependence: Nernstian slope shifts ~0.06 pH / °C — **must compensate with a temperature sensor** ([DS18B20](./ds18b20.md)) in the same fluid.
- Probe life: 6–12 months with proper care (never dry, never stored in distilled water).
- Output: analog, ~2.5 V at pH 7.

## Interface & wiring

- Probe → BNC → signal board → analog out to ESP32 ADC, preferably through an [ADS1115](../10-storage-timing-power/ads1115-ads1015.md) for 16-bit resolution.
- Calibration potentiometer on the signal board sets the pH-7 output to 2.5 V (do this with pH 7.0 buffer solution first).
- 5 V supply; analog output is within 0–3.3 V after calibration, safe for ESP32.
- Keep BNC cable short (<1 m) and away from switching loads (relays, motors) — pH signal is high-impedance and noisy cables bias the reading.

## Benefits

- The only reliable way to measure pH — no optical / MOX shortcut.
- Industrial-grade probes are affordable.
- Signal board abstracts the high-impedance electrode so a regular ADC can read it.

## Limitations / gotchas

- **Two-point calibration is mandatory** at install and roughly monthly: pH 4.00 and pH 7.00 buffer (or 7.00 + 10.00 for alkaline work). Store calibration offsets in NVS per probe.
- **Probe storage matters.** Keep in storage solution (3 M KCl) — not distilled water. Drying out kills the probe permanently.
- Temperature compensation is essential; pair with DS18B20 in the same fluid.
- Probe drift is real — re-calibrate weekly for tight control.
- The glass bulb is fragile; a careless install cracks it silently and readings go to noise.
- Signal board clones vary in op-amp quality; cheap boards are noisier and drift more.

## Typical use cases

- Hydroponics nutrient tank.
- Aquarium / aquaponics monitoring.
- Pool / hot tub automation.
- Soil slurry testing (with a specialized soil probe).
- Fermentation / brewing.

## Pairs well with

- [`./ds18b20.md`](./ds18b20.md) — temperature compensation, mandatory.
- [`./ec-tds.md`](./ec-tds.md) — pH + EC is the hydroponics duo.
- [`./turbidity-sensor.md`](./turbidity-sensor.md) — water clarity often tracks with pH shifts.
- [`../10-storage-timing-power/ads1115-ads1015.md`](../10-storage-timing-power/ads1115-ads1015.md) — stable reads off a noisy analog line.

## Where to buy

- DFRobot Gravity Analog pH meter (v1 / pro).
- Atlas Scientific (far more accurate, far more expensive — worth it for lab work).
- AliExpress "PH-4502C" — OK with fresh calibration; expect to replace the probe every 6 months.

## Software / libraries

- No driver — read ADC, apply linear calibration, apply temperature slope correction.
- Many DFRobot sample sketches are the canonical starting point.

## Notes for WeftOS

- HAL exposes `WaterQuality::pH { value, temp_c, last_cal_ts }`. UI must prominently show "calibration age" and warn after 14 days.
- Include a calibration wizard in WeftOS Admin app that walks the user through 7/4 buffer steps and stores the offsets.
- Alarm on pH excursion (configurable per deployment): a hydroponics tank out of range for > N minutes is a pump-shutoff event routed through [relay modules](../07-control-actuators/relay-modules.md).
