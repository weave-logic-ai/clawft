# Splat capture sensor head (Pi + lab sensors + 3D print)

**Status:** Design proposal (2026-07-30)  
**Related:** [splat-multimodal-sensing.md](./splat-multimodal-sensing.md), [splat-to-world-model.md](./splat-to-world-model.md), [ADR-077](../adr/adr-077-android-splat-capture-edge-node.md), [ADR-078](../adr/adr-078-splat-feeds-world-model.md), [splat-pipeline-design.md](./splat-pipeline-design.md)

Build a **3D-printed capture head** that reuses WeftOS / lab sensors (ToF, IMU, optional radar/sonar) with a **Raspberry Pi** (or ESP32 sidecar) for preprocess and stream to Mac/cloud **splatd**.

This is **not** “radar that replaces the camera.” It is:

> **RGB capture + depth/IMU coprocessor**, speaking the same capture package idea as the Android edge node.

---

## 1. Why build this

| Goal | How the head helps |
|------|---------------------|
| Better than Gallery MP4 | Per-frame IMU + structured session package |
| Reuse Stu / edge kit | VL53-class ToF, IMUs, ESP32 bridges, mmWave if on hand |
| WeftOS-native | `substrate/sensor/tof`, sensor bridge pattern, later edge identity |
| Cost | Often **cheaper than PortalCam** if parts already owned |
| Control | Step–shoot overlap you design, not hope |

---

## 2. What lab sensors are good for (honest)

| Sensor (typical kit) | Good for | Weak for Gaussian RGB splats |
|----------------------|----------|------------------------------|
| **ToF multi-zone (VL53L5CX / L7CX)** | Near depth, occupancy, hole flags, scale hints | Dense far texture, glass/black |
| **mmWave radar** | Presence, range, some clutter | Appearance / dense mesh |
| **Ultrasonic / sonar** | Cheap range; **domain** (e.g. acoustic/water stacks) | Indoor air multipath rooms |
| **IMU (MPU / BNO)** | Attitude, coverage bins, pose sidecar | Scale alone |
| **RGB (Pi Cam / USB)** | **Primary splat input** | Needs multi-view path |
| **Stereo pair** | Metric depth if calibrated | Calibration + sync cost |

WeftOS already surfaces ToF:

- `substrate/sensor/tof` (4×4 / 8×8 heatmaps, VL53L5CX / L7CX)
- Tray chip fixture `weftos-chip-tof.toml`
- Historical pattern: ESP32 sensor nodes → `clawft-sensor-bridge`-style publish to substrate

**Sonar / hydrophone expertise** (from other projects) pays off when the *domain* is acoustic—not as the default indoor room texture sensor.

---

## 3. Architecture

```
        ┌─────────────────────────────────────┐
        │  3D-printed head (fixed or pan/tilt) │
        │  · Pi Cam / HQ / global-shutter cam  │
        │  · VL53L7CX (8×8 ToF)                │
        │  · BNO085 / ICM-class IMU            │
        │  · optional mmWave (side mount)      │
        │  · LED ring (SfM texture)            │
        └───────────────┬─────────────────────┘
                        │ CSI / USB / I²C / UART
        ┌───────────────▼─────────────────────┐
        │  Raspberry Pi 5 (or CM4)             │
        │  Preprocess:                         │
        │   · frames + t_ns                    │
        │   · IMU → quat                       │
        │   · ToF → sparse depth bin           │
        │   · coverage bins                    │
        │   · package / stream session         │
        │  WeftOS edge client (upload/identity)│
        └───────────────┬─────────────────────┘
                        │ Wi‑Fi / Ethernet
                        ▼
                 Mac splatd / cloud SPLAT_RUNNER
                 (COLMAP + Brush + SOG)
```

### Optional split (matches existing ESP32 bridges)

| MCU | Role |
|-----|------|
| **ESP32-S3** | ToF + IMU + radar UART @ high rate → serial/Wi‑Fi to Pi or substrate |
| **Pi 5** | Camera + packaging + HTTPS/QUIC to host |
| **Mac** | Heavy train only |

Do **not** put full COLMAP/Brush on the Pi for v1.

---

## 4. Mechanical / gimbal guidance

| Style | When |
|-------|------|
| **Fixed wide + human walk** | Best first build (no motor blur) |
| **2-axis pan/tilt step–shoot** | Automated angular coverage |
| **Continuous spin** | Avoid unless true stop–shoot–stop |

Gimbal blur kills SfM faster than missing ToF.

**Print design notes:**

- Rigid **camera + ToF + IMU** co-mount (extrinsics must not flex)
- Cable strain relief; LED ring outside lens FOV glare
- Tripod 1/4-20 + handheld grip
- Leave mmWave / sonar as modular side pods (don’t force into optical axis)

---

## 5. Session package (host-friendly)

Align with capture protocol (ADR-077 / `weft.capture.v1` when frozen):

```
session-<uuid>/
  session.json              # id, device, extrinsics, started_at
  poses.jsonl               # one JSON object per line
  frames/
    000001.jpg
    ...
  depth/                    # optional
    000001.tof.bin          # 8×8 or serialized frame
  radar/                    # optional
    samples.jsonl
  optional/
    preview.mp4
    coverage.bin
```

Example `poses.jsonl` line:

```json
{
  "frame_id": 1,
  "t_ns": 123456789,
  "path": "frames/000001.jpg",
  "quat_wxyz": [0.7, 0, 0, 0.7],
  "coverage_bin": 12,
  "tof_min_mm": 820
}
```

Host uses:

1. **RGB** → COLMAP / Brush (appearance)  
2. **ToF / radar** → scale, hole map, occupied volumes, “missed ceiling” QA  
3. **Structure stage** → `WM_OBJECT` / `WM_VOLUME` / `WM_SURFACE` leaves (ADR-078)  
4. **BVH publish** → agent-queryable world model (ADR-056)—not day-one train, but product target

---

## 6. Calibration (mandatory)

Without this, multi-sensor data is noise:

| Transform | Meaning |
|-----------|---------|
| `T_cam_imu` | IMU frame → camera |
| `T_cam_tof` | ToF frame → camera |
| `T_cam_radar` | If present |
| Timebase | Shared `t_ns` or explicit offsets |

Document in `session.json`. Re-calibrate after any mechanical change.

---

## 7. Capture schedule (solid layer-A overlap)

1. Operator (or pan/tilt) aims until coverage bin empty.  
2. **Stop** motion.  
3. Grab RGB + IMU sample + ToF frame.  
4. Advance Δθ / Δx (e.g. 10–15° or 20–40 cm).  
5. Require min coverage % before “complete.”  
6. Optional: radar confirms “return at ~2 m” where ToF is blind.  

**Do not** start host train on radar/ToF alone.

---

## 8. BOM sketch (indicative)

| Item | Notes |
|------|--------|
| Raspberry Pi 5 + NVMe/SD | Preprocess + Wi‑Fi |
| Camera Module 3 / HQ + lens | Fixed focus preferred for rooms |
| VL53L7CX breakout | 8×8 ToF (WeftOS UI already expects this class) |
| BNO085 or similar | Rotation vector / fused quat |
| LED ring 5 V | Texture for SfM |
| Optional mmWave module | UART/SPI from lab stock |
| Optional ESP32-S3 | Sidecar if I²C load / legacy bridge |
| Print + fasteners + 1/4-20 insert | Rigid optical bench |

**Cost:** often **~$80–200** if sensors are already owned; more if buying new depth cams.

Compare: Insta360 ~$400–600 (RGB sphere only); PortalCam ~$5k (integrated LiDAR+multi-cam).

---

## 9. Software on the Pi (v1)

| Component | Responsibility |
|-----------|----------------|
| Camera capture loop | JPEG/YUV, timestamps |
| IMU reader | Quat / raw rates |
| ToF reader | Zone depths → bin file |
| Coverage accumulator | View bins from quat |
| Packager | ZIP or stream to splatd |
| Edge client | Pairing, job_id, progress (later UniFFI/Rust) |

Publish optional live metrics to substrate when on LAN with WeftOS daemon (`substrate/sensor/tof`, future `substrate/sensor/capture/*`).

---

## 10. Build phases

| Phase | Deliverable | Exit |
|-------|-------------|------|
| **H0** | Inventory lab bins (VL53, radar, IMUs, Pis, mounts) | BOM reality check |
| **H1** | Fixed head: Cam + BNO + VL53; no motors; session ZIP | Offline package → manual splatd |
| **H2** | Stream/upload to Mac splatd; coverage UI on phone/laptop | End-to-end |
| **H3** | Optional pan/tilt step-shoot; printed shell v2 | Automated angular fill |
| **H4** | Host uses depth for hole hints back to operator | Closed-loop guidance |

Aligns with ADR-077 phases: head is another **edge peer**, not a second product language.

---

## 11. Limits (do not oversell)

1. VL53 8×8 is **not** LiDAR.  
2. mmWave ≠ dense splat texture.  
3. Gimbal blur > missing ToF for explode.  
4. Uncalibrated extrinsics make depth useless.  
5. Sonar only when the domain needs it.  

---

## 12. Relationship to other capture paths

| Path | Use when |
|------|----------|
| Android app (ADR-077) | Phone-first field capture |
| **This sensor head** | Lab reuse, fixed install, multi-sensor research |
| Insta360 | Cheap full-sphere RGB without building hardware |
| Gallery MP4 only | Legacy; weakest pose/coverage story |

All paths should converge on the **same session package shape** and host splatd.

---

## 13. History

- 2026-07-30: Proposal from multi-modal sensing discussion + lab ToF/radar/sonar reuse + Pi gimbal idea.
