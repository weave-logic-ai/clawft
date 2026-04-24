---
title: "Environmental & Gas/Air/Soil/Water Sensors"
slug: "05-environmental"
category: "05-environmental"
source: "../../weftos_sensors.md"
status: "draft"
---

# Environmental & Gas/Air/Soil/Water Sensors

This category covers every sensor that measures the *physical world around the node* — air temperature, humidity, pressure, gas concentrations, particulates, ambient light, soil moisture, water pH/EC/turbidity, wind, rain, flame. In WeftOS these feed the environmental telemetry bus, drive home/greenhouse/hydroponic control loops, and surface as ambient context in the HUD.

## Module list

### Temperature / humidity / pressure

| File | Measures | Interface |
|------|----------|-----------|
| [dht11-dht22.md](./dht11-dht22.md) | T, RH | 1-wire (proprietary) |
| [bme280.md](./bme280.md) | T, RH, P | I²C / SPI |
| [bme680-bme688.md](./bme680-bme688.md) | T, RH, P, VOC (+AI on '688) | I²C / SPI |
| [bmp180-280-388.md](./bmp180-280-388.md) | T, P | I²C / SPI |
| [aht10-aht20-aht30.md](./aht10-aht20-aht30.md) | T, RH | I²C |
| [sht31.md](./sht31.md) | T, RH (precision) | I²C |
| [ds18b20.md](./ds18b20.md) | T (waterproof) | 1-Wire |

### Air quality / gas / particulates

| File | Measures | Interface |
|------|----------|-----------|
| [mq-series.md](./mq-series.md) | CO, CH₄, LPG, H₂, alcohol, smoke, air quality | Analog + heater |
| [ccs811-ens160.md](./ccs811-ens160.md) | eCO₂, TVOC | I²C |
| [mh-z19-co2.md](./mh-z19-co2.md) | CO₂ (NDIR, true) | UART / PWM |
| [pms5003.md](./pms5003.md) | PM1.0 / PM2.5 / PM10 | UART |
| [gp2y1010-dust.md](./gp2y1010-dust.md) | Dust density (analog) | ADC |
| [mics-4514.md](./mics-4514.md) | CO + NO₂ | Analog + heater |

### Ambient light

| File | Measures | Interface |
|------|----------|-----------|
| [bh1750-lux.md](./bh1750-lux.md) | Lux | I²C |
| [tsl2561.md](./tsl2561.md) | Lux (visible + IR) | I²C |
| [apds9960.md](./apds9960.md) | Gesture + RGB + proximity + ambient | I²C |

### Soil / water

| File | Measures | Interface |
|------|----------|-----------|
| [capacitive-soil-moisture.md](./capacitive-soil-moisture.md) | Soil moisture (anti-corrosion) | Analog |
| [resistive-soil-moisture.md](./resistive-soil-moisture.md) | Soil moisture (legacy, corrodes) | Analog |
| [ph-sensor.md](./ph-sensor.md) | pH | Analog + BNC |
| [ec-tds.md](./ec-tds.md) | EC / TDS | Analog |
| [turbidity-sensor.md](./turbidity-sensor.md) | Turbidity (NTU) | Analog |
| [rain-water-level.md](./rain-water-level.md) | Rain / water level | Analog |

### Other outdoor / safety

| File | Measures | Interface |
|------|----------|-----------|
| [anemometer.md](./anemometer.md) | Wind speed + direction | Pulse / analog |
| [leaf-wetness.md](./leaf-wetness.md) | Leaf wetness | Analog |
| [flame-sensor.md](./flame-sensor.md) | Flame (470–1100 nm IR) | Digital / analog |
| [smoke-detector.md](./smoke-detector.md) | Smoke (ionisation / photoelectric) | Digital / analog |

## Canonical combos

### Full air-quality node

The "we want to understand this room" triple, directly from the source brief:

- [BME680/BME688](./bme680-bme688.md) — T, RH, P, and a gas-resistance channel that responds to VOCs (gives a relative IAQ signal; **requires BSEC** for the Bosch index, see gotchas).
- [MH-Z19B](./mh-z19-co2.md) — true NDIR CO₂ (the only reliable way to know ventilation is working).
- [PMS5003](./pms5003.md) — laser PM1.0 / PM2.5 / PM10 counter for particulates.

Layout tip: keep the MH-Z19 and PMS5003 on opposite sides of the enclosure; both pull air actively and cross-pollute if adjacent. Do not share airflow with a screen backlight / CPU — self-heating kills RH accuracy on the BME680.

### Hydroponics / aquaponics node

- [capacitive soil moisture](./capacitive-soil-moisture.md) in growing medium (not resistive — corrosion in 24/7 wet soil).
- [DS18B20 waterproof](./ds18b20.md) in the reservoir.
- [pH sensor](./ph-sensor.md) + [EC/TDS](./ec-tds.md) + [turbidity](./turbidity-sensor.md) for nutrient control.
- [BME280](./bme280.md) for grow-tent ambient.
- [BH1750](./bh1750-lux.md) under the grow light to catch degradation.

Calibrate pH and EC on a schedule — these probes drift weekly, not yearly.

### Weather station

- [BME280](./bme280.md) or [BMP388](./bmp180-280-388.md) for T/RH/P.
- [anemometer + wind vane](./anemometer.md).
- [rain board / tipping bucket](./rain-water-level.md).
- [BH1750](./bh1750-lux.md) for solar.
- [leaf wetness](./leaf-wetness.md) for agri use.

### Fire / safety node

- [flame sensor](./flame-sensor.md) (directional, fast).
- [smoke detector head](./smoke-detector.md) (photoelectric for smouldering fires).
- [MQ-2 / MQ-7 / MQ-9](./mq-series.md) for LPG / CO combustion products.
- [MH-Z19](./mh-z19-co2.md) catches rising CO₂ from occupancy changes.

## Cross-cutting concerns

- **ADC resolution.** ESP32 ADC is noisy and non-linear; for analog probes (pH, EC, TDS, turbidity, GP2Y1010) route through an [ADS1115 16-bit ADC](../10-storage-timing-power/ads1115-ads1015.md).
- **Shared I²C addresses.** BME280, BMP280, BMP388, AHT20, SHT31 all live around 0x76/0x77/0x38/0x44 — use a [TCA9548A mux](../08-communication/tca9548a-i2c-mux.md) when more than one is present.
- **Self-heating.** Any sensor that also has a heater (BME680, MQ-series, CCS811) needs careful thermal isolation from T/RH sensors.
- **Calibration drift.** Wet-chemistry probes (pH, EC) need recalibration weekly; gas sensors need multi-hour burn-in on first power-up and ABC or fresh-air recalibration monthly. This is a first-class WeftOS concern — do not treat readings as ground truth without tracking "last calibrated".
- **Corrosion.** Anything continuously wet (resistive moisture, rain boards) will corrode within weeks. Prefer capacitive or sealed probes where possible.

## Files in this category

See the tables above. Every file follows the same body schema (`Overview` / `Key specs` / `Interface & wiring` / `Benefits` / `Limitations / gotchas` / `Typical use cases` / `Pairs well with` / `Where to buy` / `Software / libraries` / `Notes for WeftOS`).
