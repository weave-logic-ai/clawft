---
title: "Positioning, Navigation & Motion Modules"
slug: "02-positioning-navigation"
category: "02-positioning-navigation"
tags: [gps, gnss, imu, lidar, tof, uwb, compass, barometer]
source: "../../weftos_sensors.md"
status: "draft"
---

# Category 02 — Positioning, Navigation & Motion

"Where am I, which way am I pointing, and how fast am I moving?" Between GNSS, IMUs, ranging
sensors, LiDAR, UWB, and compass/baro, this is the densest sensor category for ESP32 robotics,
drones, and wearables.

## Modules

| Module | One-liner |
|---|---|
| [NEO-6M GPS](neo-6m-gps.md) | Basic u-blox GPS, 1 Hz, $10 — the default outdoor fix. |
| [NEO-M8N GNSS](neo-m8n-gnss.md) | Multi-constellation (GPS/Galileo/GLONASS/BeiDou), 10 Hz, far better fix quality. |
| [MPU-6050 IMU](mpu6050-imu.md) | Classic 6-axis (accel+gyro+temp) — cheap and everywhere. |
| [MPU-9250 IMU](mpu9250-imu.md) | 9-axis (adds magnetometer); EOL but stocked. |
| [LSM6DS3 / LSM9DS1](lsm6dsx-imu.md) | STMicro IMU family — cleaner specs, better stock than Invensense. |
| [BNO055](bno055-orientation.md) | Sensor-fusion IMU with absolute orientation output. |
| [HC-SR04 ultrasonic](hc-sr04-ultrasonic.md) | $1 ultrasonic range finder, 2–400 cm. |
| [JSN-SR04T](jsn-sr04t-ultrasonic.md) | Waterproof ultrasonic, transducer on a cable. |
| [A02YYUW UART](a02yyuw-ultrasonic.md) | Waterproof UART ultrasonic — cleaner interface than SR04T. |
| [VL53L0X / VL53L1X ToF](vl53l0x-vl53l1x-tof.md) | ST ToF laser rangers, 20 mm–4 m. |
| [TOF050C / TOF10120](tof050c-tof10120.md) | Cheap I²C ToF minis, ≤1.2 m. |
| [TFmini-S / TF-Luna](tfmini-tfluna-lidar.md) | Benewake 12-m / 8-m single-beam LiDAR. |
| [LD06 / LD19 360° LiDAR](ld06-ld19-lidar.md) | Rotating 2-D LiDAR, SLAM-grade for ~$80. |
| [DWM3000 UWB](dwm3000-uwb.md) | Qorvo UWB tag/anchor for RTLS indoor positioning. |
| [HMC5883L / QMC5883L](hmc5883l-qmc5883l-compass.md) | 3-axis magnetometer / digital compass. |
| [BMP180/280/388 baro](bmp180-280-388-baro.md) | Bosch barometric pressure / altimeter family. |

## Fused-navigation pairing guidance

Cheap pose comes from stacking sensors and fusing them. Common recipes:

- **Outdoor drone** — NEO-M8N + BNO055 + BMP388. GNSS for position, BNO055 for orientation,
  BMP388 for high-rate altitude correction.
- **Indoor robot** — LD19 360° LiDAR + MPU-6050 + wheel encoders (host-side SLAM).
- **Indoor room-scale** — DWM3000 UWB anchors + tag on robot, with MPU-6050 for dead-reckoning
  between anchor updates.
- **Pedestrian dead-reckoning** — BNO055 (step heading) + BMP388 (floor change) + NEO-M8N (drift reset when outside).
- **Obstacle layer** — VL53L1X ToF on a servo sweep for close, HC-SR04 or TFmini for longer-range
  backup.

## Cross-category pairings

- Servo / stepper mounts for scanning LiDAR and ToF → [`../07-control-actuators/`](../07-control-actuators/index.md).
- Power budgeting for GPS cold-start and LiDAR motors → [`../10-storage-timing-power/`](../10-storage-timing-power/index.md).
- I²C bus contention with vision / environmental sensors → [`../08-communication/tca9548a-i2c-mux.md`](../08-communication/tca9548a-i2c-mux.md).
