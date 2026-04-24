---
title: "Audio & Sound Modules"
slug: "03-audio-sound"
category: "03-audio-sound"
tags: [audio, mic, speaker, amp, dac, i2s]
source: "../../weftos_sensors.md"
status: "draft"
---

# Category 03 — Audio & Sound

I2S is king on ESP32 (and especially ESP32-S3 with its AI instructions) for clean digital audio
capture and playback. This category covers input microphones, DACs, amplifiers, and the
simple-but-useful fallbacks like buzzers and offline voice ICs.

## Modules

| Module | One-liner |
|---|---|
| [INMP441 I²S mic](inmp441-i2s-mic.md) | The default ESP32 digital mic — $3, clean, Omni. |
| [SPH0645LM4H](sph0645lm4h-mic.md) | Knowles I²S mic, alternate to INMP441, Adafruit breakout. |
| [MAX9814 analog mic](max9814-analog-mic.md) | Analog electret + AGC, for ADC-only builds. |
| [Electret mic breakouts](electret-mic-breakouts.md) | Generic $1 boards — the cheap fallback. |
| [Dual / multi-mic arrays](dual-mic-arrays.md) | 2-mic / 4-mic for beamforming, AEC on ESP32-S3. |
| [MAX98357A I²S amp](max98357a-amp.md) | Mono 3 W Class-D, easiest ESP32 speaker driver. |
| [PAM8403 stereo amp](pam8403-amp.md) | Cheap analog 2×3 W for driving small speakers. |
| [TAS5805M hi-fi amp](tas5805m-amp.md) | TI hi-fi stereo I²S amp with DSP. |
| [PCM5102 / PCM5122 DAC](pcm5102-pcm5122-dac.md) | TI I²S DACs for line-out / external amp. |
| [Buzzers](buzzers.md) | Active / passive piezo — the $0.50 "beep". |
| [LD3320 voice recog](ld3320-voice.md) | Legacy offline voice module (availability caveat). |

## Creative "Lego" prompts

- **Walkie-talkie / intercom** — ESP32 + INMP441 + MAX98357A + Wi-Fi streaming.
- **SD-card jukebox** — ESP32 + PCM5102 DAC + PAM8403 + microSD MP3 decoder.
- **Always-listening doorbell** — ESP32-S3 + dual-mic array running on-device wake-word.
- **Ultrasonic rat-repeller** — ESP32 + piezo driver + 30 kHz passive piezo sweep.

## Cross-category pairings

- Power budgeting for Class-D amps (brownout city) → [`../10-storage-timing-power/`](../10-storage-timing-power/index.md).
- Enclosure / microphone port design affects SNR; keep analog audio traces away from Wi-Fi antennas
  and consider shielding — see [`../08-communication/`](../08-communication/index.md) for RF neighbors.
- Wake-on-sound power management and RTC → [`../10-storage-timing-power/`](../10-storage-timing-power/index.md).
- Audio visualizer pairings → [`../04-light-display/ws2812b-neopixel.md`](../04-light-display/ws2812b-neopixel.md).
