---
title: "Light, Illumination & Display Modules"
slug: "04-light-display"
category: "04-light-display"
source: "../../weftos_sensors.md"
status: "draft"
---

# Light, Illumination & Display Modules

This category covers anything that emits or renders light for WeftOS — addressable LED arrays, single indicator LEDs, laser dot modules, graphical and character displays (OLED, TFT, e-paper, 7-seg, LED matrix), plus the specialty illuminators (UV and IR floods) that pair with the cameras in [`../01-vision-imaging/`](../01-vision-imaging/).

## Why separate illumination from sensing?

A huge number of "sensor" problems are actually illumination problems:

- A UV sensor in [`../05-environmental/`](../05-environmental/) is almost useless without a [UV LED array](./uv-led-arrays.md) next to the specimen.
- A [NoIR camera](../01-vision-imaging/) needs an [850/940 nm IR flood](./ir-flood-arrays.md) to see in the dark.
- A capacitive fingerprint sensor or 3D-printed resin cure station needs a controlled 365 nm source.

Grouping emitters here keeps sensing files focused on transduction.

## Display technology tradeoffs

| Tech | Power (idle) | Power (active) | Refresh | Viewing | Sunlight | Best for |
|------|--------------|----------------|---------|---------|----------|----------|
| OLED ([SSD1306](./ssd1306-oled.md)/[SH1106](./sh1106-oled.md)) | low | medium | 60+ Hz | wide angle | poor | status HUD, menus |
| TFT IPS ([ILI9341](./ili9341-tft.md)/[ST7789](./st7789-st7735-tft.md)) | medium | high (backlight) | 30–60 Hz | wide angle | ok w/ brightness | dashboards, thumbnails |
| e-Paper ([Waveshare/CrowPanel](./eink-epaper.md)) | ~0 W | pulsed | 1–15 s | wide angle | excellent | name badges, signage, slow telemetry |
| 7-seg ([TM1637](./tm1637-7seg.md)) | low | low | 50 Hz | narrow | ok | numeric counters |
| 8×8 matrix ([MAX7219](./max7219-matrix.md)) | low | low | 50–200 Hz | narrow | ok | retro scrollers, dot clocks |
| Addressable RGB ([WS2812B](./ws2812b-neopixel.md)) | low | high (per px) | data-rate bound | any | ok | accent, status, animations |

Rule of thumb: if the UI changes less than once per second **and** needs to survive power loss visibly, pick e-paper. Otherwise pick OLED for monochrome or TFT for color.

## Module list

### Emitters

| File | Tech | Typical use |
|------|------|-------------|
| [ws2812b-neopixel.md](./ws2812b-neopixel.md) | Addressable RGB (WS2812B/WS2813/SK6812) | strips, rings, matrices, status |
| [single-rgb-leds.md](./single-rgb-leds.md) | Discrete RGB LED on PWM | per-module status light |
| [laser-diode-modules.md](./laser-diode-modules.md) | 650 nm 5 mW laser dot | pointing, alignment, cheap TOF |
| [uv-led-arrays.md](./uv-led-arrays.md) | 365 / 395 nm UV LEDs | plant fluorescence, cure, inspection |
| [ir-flood-arrays.md](./ir-flood-arrays.md) | 850 / 940 nm IR LEDs | night vision for NoIR cameras |

### Displays

| File | Tech | Typical use |
|------|------|-------------|
| [ssd1306-oled.md](./ssd1306-oled.md) | 0.96" / 1.3" monochrome OLED | status HUD |
| [sh1106-oled.md](./sh1106-oled.md) | 1.3" mono OLED | SSD1306 alt (watch column offset) |
| [ili9341-tft.md](./ili9341-tft.md) | 2.8" 320×240 SPI TFT + touch | rich UI, camera preview |
| [st7789-st7735-tft.md](./st7789-st7735-tft.md) | 1.3"–2.0" IPS TFT | small dashboards |
| [eink-epaper.md](./eink-epaper.md) | 1.54"–7.5" B/W or 3-color | always-on signage |
| [tm1637-7seg.md](./tm1637-7seg.md) | 4-digit 7-seg + keypad | counters, clocks |
| [max7219-matrix.md](./max7219-matrix.md) | 8×8 LED matrix + daisy chain | scrolling text |
| [led-bar-graphs.md](./led-bar-graphs.md) | 10-seg LED bar | VU meters, level indicators |

## Canonical combos

- **Hybrid status wall** — WS2812B [16×16 matrix](./ws2812b-neopixel.md) for ambient/animation + [e-paper](./eink-epaper.md) for the "last known good" numeric readout that survives power cycles.
- **Plant fluorescence rig** — [UV LED array](./uv-led-arrays.md) + [UV-sensitive camera](../01-vision-imaging/) + [GUVA-S12SD UV sensor](../01-vision-imaging/guva-s12sd-uv.md) for intensity logging.
- **Night-vision node** — [850 nm IR flood](./ir-flood-arrays.md) + [NoIR Pi camera](../01-vision-imaging/) + [PIR motion trigger](../06-biometric-presence/pir-hc-sr501.md).

## Cross-cutting concerns

- **Power budget.** A single full-white WS2812 is ~60 mA. A 16×16 matrix at full brightness can pull >15 A; plan rails and derate brightness aggressively. See [`../10-storage-timing-power/`](../10-storage-timing-power/).
- **Level shifting.** WS2812 wants 5 V data; ESP32 GPIO is 3.3 V. A 74HCT125 or SN74AHCT1G125 on the first pixel's DIN solves most glitches.
- **Driving relays / MOSFETs.** UV and IR LED arrays usually need [low-side MOSFET drivers](../07-control-actuators/mosfet-drivers.md) or [relay modules](../07-control-actuators/relay-modules.md) rather than direct GPIO.
- **Shared I²C bus.** OLEDs, SH1106, and small TFT touch controllers often collide at 0x3C / 0x3D / 0x27. Use a [TCA9548A I²C mux](../08-communication/tca9548a-i2c-mux.md) for >1 same-address display.
