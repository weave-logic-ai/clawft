---
title: "Storage, Timing, Power & Management"
slug: "10-storage-timing-power"
category: "10-storage-timing-power"
description: "SD storage, real-time clocks, LiPo charging, power/current monitoring, and precision external ADC/DAC modules for ESP32."
tags: [storage, rtc, power, charger, monitor, adc, dac]
source: "../../weftos_sensors.md"
status: "draft"
---

# 10 — Storage, Timing, Power & Management

This category is the "infrastructure" that everything else rides on:

- **Storage** — microSD cards over SPI or SDMMC for datalogging, OTA blobs,
  images, and config.
- **Timing** — real-time clocks with battery backup so nodes know the wall-clock
  time even without Wi-Fi / NTP.
- **Power delivery** — LiPo chargers, buck/boost converters, and solar charge
  controllers for off-grid / battery-powered ESP32 nodes.
- **Power monitoring** — current / voltage / power measurement and battery
  fuel-gauging.
- **Precision ADC / DAC** — external converters that sidestep the ESP32's noisy,
  Wi-Fi-coupled internal ADC.

## Modules

### Storage

| Module | File | Interface | Notes |
|--------|------|-----------|-------|
| microSD breakout | [`microsd.md`](microsd.md) | SPI or SDMMC | FAT32, brown-out hazards |

### Real-Time Clocks

| Module | File | Interface | Notes |
|--------|------|-----------|-------|
| DS3231 TCXO RTC | [`ds3231-rtc.md`](ds3231-rtc.md) | I²C | ±2 ppm, temp sensor |
| DS1307 RTC | [`ds1307-rtc.md`](ds1307-rtc.md) | I²C | Legacy, drifts |
| PCF8563 RTC | [`pcf8563-rtc.md`](pcf8563-rtc.md) | I²C | NXP, low-power |
| DS1302 RTC | [`ds1302-rtc.md`](ds1302-rtc.md) | 3-wire | Legacy, avoid |

### Power delivery

| Module | File | Interface | Notes |
|--------|------|-----------|-------|
| TP4056 LiPo charger | [`tp4056-lipo-charger.md`](tp4056-lipo-charger.md) | — | 1 A, optional DW01 protection |
| Buck / Boost converters | [`buck-boost-converters.md`](buck-boost-converters.md) | — | MP1584 / LM2596 / MT3608 / XL6009 |
| Solar charge controllers | [`solar-charge-controllers.md`](solar-charge-controllers.md) | — | CN3791 MPPT-lite for off-grid |

### Monitoring / fuel gauge

| Module | File | Interface | Notes |
|--------|------|-----------|-------|
| INA219 / INA260 | [`ina219-ina260.md`](ina219-ina260.md) | I²C | High-side current + power |
| MAX17048 fuel gauge | [`max17048-fuel-gauge.md`](max17048-fuel-gauge.md) | I²C | ModelGauge SoC for LiPo |

### External ADC / DAC

| Module | File | Interface | Notes |
|--------|------|-----------|-------|
| ADS1115 / ADS1015 | [`ads1115-ads1015.md`](ads1115-ads1015.md) | I²C | 16-bit / 12-bit ADC |
| PCF8591 | [`pcf8591-adc-dac.md`](pcf8591-adc-dac.md) | I²C | 8-bit ADC + DAC, 5 V only |

## Why this category matters for WeftOS

Every WeftOS node is a power, timing, and storage envelope around a compute
core. The decisions here — "do we sleep on a LiPo? do we log to SD? do we trust
the ESP32 ADC?" — shape the whole sensor graph's reliability envelope. Every
serious outdoor / off-grid deployment will pull **at least one module from each
sub-section** above.

## Pairs with

- [`../07-control-actuators/mosfet-drivers.md`](../07-control-actuators/mosfet-drivers.md) — power-gate hungry peripherals.
- [`../08-communication/sx1276-rfm95-lora.md`](../08-communication/sx1276-rfm95-lora.md) — low-power radio fits solar + LiPo budgets.
- [`../05-environmental/bme280.md`](../05-environmental/bme280.md) — typical data payload for SD logging.
