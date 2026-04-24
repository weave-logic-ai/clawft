---
title: "Composed Builds"
slug: "12-builds"
category: "12-builds"
description: "End-to-end build recipes composed from catalog modules — not single-part files."
source: "../../weftos_sensors.md"
status: "draft"
---

# Composed Builds

This category is different from categories 1 – 11. Those describe **single
modules** — one sensor, one amp, one driver, one IC. This category describes
**end-to-end builds**: recipes that combine several catalog modules into a
working instrument, rig, or capability.

A build file:

- Starts from a goal ("measure a reflectance spectrum", "watch a bird feeder
  at night", "log soil moisture over a season"), not a part.
- Has a `uses_modules:` frontmatter array of paths to the per-module files
  it depends on.
- Can name specific part combinations without re-describing those parts — it
  links to them in the module categories.
- Can include build-specific concerns (mechanical, optical, 3D-print, safety
  interlocks, firmware patterns) that wouldn't fit in a per-module file.

## Builds in this catalog

| File | Path | One-liner |
|------|------|-----------|
| [spectrograph.md](./spectrograph.md) | ESP32 Spectrograph / Spectrometer | Two paths — AS7265x 18-channel triad, or ESP32-CAM + diffraction grating — with honest caveats for quantitative work. |

## Planned / open slots

These are natural next builds, each a composition of modules already in the
catalog. Not yet written:

- **Air-quality node** — BME680 + MH-Z19 + PMS5003 + OLED + optional LoRa.
- **Hydroponics node** — pH + EC/TDS + turbidity + capacitive soil moisture
  + relay bank + RTC + SD.
- **Doorbell / porch cam** — ESP32-CAM + PIR + IR flood + LoRa or MQTT.
- **Plant-fluorescence rig** — UV LED array + GUVA-S12SD + AS7265x or
  ESP32-CAM + enclosure interlock.
- **Battery weather station** — BME280 + BH1750 + BMP388 + anemometer +
  rain/water level + solar charge controller + LoRa + deep-sleep.
- **Power-monitored relay bank** — INA219 per channel + 8-ch relay + SD log.

When you write one of these, follow `spectrograph.md` as the template.

## Writing a new build

1. Create `<build-name>.md` in this folder.
2. Frontmatter must include `category: "12-builds"`, `kind: "build"`, and
   `uses_modules:` listing every catalog file the build depends on.
3. Body structure (recommended, not rigid):
   - Overview / terminology.
   - One *bill of materials* table per build path, each row a link to its
     module file.
   - Build notes (mechanical, optical, wiring, firmware).
   - Critical caveats — the non-obvious failure modes.
   - Shared tips / extensions.
   - "Notes for WeftOS" on how the build surfaces as a capability.
4. Update the table above.
5. Run the dead-link scan in `../index.md`'s convention.
