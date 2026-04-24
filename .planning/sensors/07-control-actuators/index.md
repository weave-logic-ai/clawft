---
title: "07 – Control Mechanisms & Actuators"
slug: "07-control-actuators"
category: "07-control-actuators"
description: "Devices that flip bits into physical motion, heat, light, or switched power: relays, MOSFETs, motor drivers, servos, steppers, triacs, solenoids, buzzers."
tags: [actuators, relays, mosfet, motor-driver, servo, stepper, triac, esp32, index]
source: "../../weftos_sensors.md"
status: "draft"
---

# 07 – Control Mechanisms & Actuators

This category is the **output** side of WeftOS: everything that turns a logical
decision into physical state. It is also where most electrical-safety and
thermal mistakes happen, so the category index stresses power budgets, isolation,
and efficiency.

## Modules

| Module | Role | Interface | One-liner |
|--------|------|-----------|-----------|
| [Relay modules (1/2/4/8/16-ch)](relay-modules.md) | Switch high-current DC/AC loads | GPIO | Optocoupler-isolated mechanical relays. |
| [SSR relays](ssr-relays.md) | Silent AC switching | GPIO | Solid-state relays, zero-cross variants. |
| [MOSFET drivers (IRF520, dual PWM)](mosfet-drivers.md) | PWM-controlled DC loads | PWM | Power-gate or dim up to 15+ A. |
| [BSS138 level shifter](bss138-level-shifter.md) | 3.3V ↔ 5V I²C / logic | I²C / GPIO | Bidirectional, 4-channel. |
| [L298N driver](l298n-driver.md) | Dual H-bridge DC motor | PWM + DIR | Classic; hot and lossy. |
| [TB6612FNG / L9110S](tb6612-l9110s.md) | Efficient dual H-bridge | PWM + DIR | Modern replacement for L298N. |
| [A4988 stepper driver](a4988-driver.md) | Stepper motor | STEP / DIR | 16× microstepping, up to 2 A/phase. |
| [TMC2209 stepper driver](tmc2209-driver.md) | Silent stepper driver | STEP / DIR + UART | StealthChop, coolStep, quiet. |
| [PCA9685 servo driver](pca9685-servo-driver.md) | 16-ch PWM / servo | I²C | Offload servo PWM to a chip. |
| [Hobby servos (SG90/MG90S/MG996R)](hobby-servos.md) | 3-wire angle actuator | PWM | 180° or continuous rotation. |
| [28BYJ-48 stepper + ULN2003](28byj-48-stepper.md) | Geared unipolar stepper | 4× GPIO | Slow, cheap, high torque. |
| [Solenoids](solenoids.md) | Push/pull linear | MOSFET-driven | 5–24 V with flyback diode. |
| [Vibration motors](vibration-motors.md) | Haptic feedback | PWM | Coin / LRA, MOSFET-driven. |
| [Triac AC dimmer](triac-ac-dimmer.md) | Phase-cut AC dimming | zero-cross + PWM | RobotDyn-style mains dimmer. |
| [Buzzers](buzzers.md) | Audible indicator / actuator | GPIO / PWM | Active vs passive piezo. |

## Power budget & isolation (read this first)

Actuators are where "works on the bench" stops working in the field. Three
rules that show up in every file:

1. **Isolate mains.** Anything touching 110 / 230 V AC goes behind an
   optocoupler-isolated relay or a zero-cross SSR — never a bare MOSFET.
2. **Back-EMF kills MCUs.** Every inductive load (motor, solenoid, relay coil)
   needs a flyback diode or a snubber. The L298N and relay boards have them;
   the IRF520 board does not — you supply the diode.
3. **Budget current, not just voltage.** An ESP32 3.3 V rail can source maybe
   500 mA. A servo stall is 1 A. A 28BYJ-48 at full draw is ~240 mA. Use a
   separate motor / servo supply and common the grounds.

## Efficiency notes

- **L298N** is a BJT H-bridge from the 1990s. It drops ~2 V per leg and runs
  hot. Prefer TB6612 / L9110S / DRV8833 for anything battery-powered.
- **IRF520** modules are fine for logic-level 5 V drive but not true
  logic-level MOSFETs; performance degrades below ~8 V gate drive. For 3.3 V
  ESP32 PWM, use a board with a gate driver or pick an AO3400 / IRLZ44N
  alternative.
- **TMC2209** UART mode unlocks current-sense, stallGuard, and silent
  StealthChop operation — worth the extra wiring.

## Creative Lego ideas (from source)

- Relay / MOSFET bank to power-cycle other modules on failure.
- PCA9685 + servo array for robotic arms / pan-tilt rigs.
- Triac dimmer + mmWave presence → ambient-response room lighting.

## Cross-links

- [`../01-vision-imaging/esp32-cam-ov2640.md`](../01-vision-imaging/esp32-cam-ov2640.md) —
  MOSFET-gate the camera to save power between shots.
- [`../06-biometric-presence/index.md`](../06-biometric-presence/index.md) —
  presence → actuator is the canonical WeftOS automation shape.
- [`../10-storage-timing-power/index.md`](../10-storage-timing-power/index.md) —
  current monitors and power-path ICs for closing the loop on actuator power.
