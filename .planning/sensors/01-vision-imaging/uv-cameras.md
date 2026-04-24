---
title: "UV / Hyperspectral Cameras (niche)"
slug: "uv-cameras"
category: "01-vision-imaging"
part_numbers: ["(various)"]
manufacturer: "various / DIY"
interface: ["SPI", "MIPI-CSI (not ESP32)", "USB"]
voltage: "varies"
logic_level: "varies"
current_ma: "varies"
price_usd_approx: "$60 – $2000+"
esp32_compat: ["ESP32-S3 (SPI mini-cams)", "host-PC (USB)"]
tags: [uv, hyperspectral, niche, experimental]
pairs_with:
  - "./uv-illuminators.md"
  - "./guva-s12sd-uv.md"
  - "./arducam-noir.md"
source: "../../weftos_sensors.md"
status: "draft"
---

## Overview

Genuine UV imaging on ESP32 is firmly in the "here be dragons" zone. This page is a pointer map
rather than a part-number catalog: what options exist, what "kind of works," and what you
actually need a PC or FPGA for. Mark anything here as experimental and verify data sheets before
buying.

## Key specs (by approach)

| Approach | Resolution | Notes |
|---|---|---|
| IR-cut-removed visible CMOS | 2–5 MP | "Full-spectrum" conversion; some UV-A response with UV-pass filter. |
| Dedicated UV CMOS / CCD | VGA–2 MP | Expensive, often machine-vision USB3. Not ESP32-friendly. |
| Hyperspectral line-scan | 100s of bands × N pixels | Ximea, Specim, etc. — lab equipment. |
| DIY diffraction grating + mono CMOS | 1D spectra | Cheap; useful science-fair tier. |

## Interface & wiring

Most real UV / hyperspectral sensors are USB3, GigE, or MIPI-CSI — none of which ESP32 speaks.
The practical ESP32 path is: (a) a "full-spectrum" modified OV2640 + UV-pass filter (Wratten 18A
or similar), or (b) ESP32 acts as controller/trigger for a real UV camera hosted on a PC / Pi.

## Benefits

- Access to wavelengths normal cameras can't see — forensics, fluorescence, plant health.
- "Full-spectrum" mod is cheap to try.
- Hyperspectral data is rich enough to do chemometrics.

## Limitations / gotchas

- Real UV sensors are expensive and often export-controlled.
- UV-pass filters can cost more than the camera body.
- ESP32 is almost certainly the wrong host — use it as a sidekick, not the imager.
- UV lenses matter; most glass blocks below ~350 nm. Need quartz / fused silica for serious work.

## Typical use cases

- Fluorescence macro photography (UV illuminator + UV-pass or even stock RGB camera).
- Plant / leaf inspection (combine with NIR channel).
- Forensic / authenticity lighting on documents and currency.

## Pairs well with

- [`./uv-illuminators.md`](./uv-illuminators.md) — you'll almost always need a source.
- [`./guva-s12sd-uv.md`](./guva-s12sd-uv.md) for "is the UV lamp actually on?" interlock.
- [`./arducam-noir.md`](./arducam-noir.md) as a cheaper "modified CMOS" starting point.

## Where to buy

- Ximea, Specim, IDS, Basler — industrial UV / hyperspectral (search vendor sites directly).
- Kolari Vision / LifePixel — full-spectrum camera conversions.
- UV-pass filters: ThorLabs, Edmund Optics.

## Software / libraries

- Heavy workflows live on the host (Python + OpenCV, HyperSpy, scikit-image).
- ESP32 side is usually just a trigger / shutter / illuminator controller.

## Notes for WeftOS

Speculative and honest: don't plan a first-class WeftOS UV surface yet. Treat UV/hyperspectral as
an experimental plug-in class — the node is likely a PC-side adapter, with ESP32 only owning
illuminator + trigger.
