# NVIDIA Cosmos3-Edge — world-model evaluation for the splat / BVH spatial track

> Research note. DESIGN/EVAL only — no code lands from this file.
> Raised by the user 2026-07-31 from the ~/llm model-lab session ("interesting for the 3d
> world stuff — inform weftos"). Subject: `nvidia/Cosmos3-Edge` on HuggingFace.
> Grounded against: ADR-077 (Android splat capture edge node), ADR-078 (splat feeds a
> structured world model, not appearance-only), ADR-056 (BVH world model),
> ADR-087 (spatio-temporal dual-branch sensors, K-STEMIT), LeWM latent world-model track.

## TL;DR verdict

**Runnable — on NVIDIA hardware we have or are acquiring. Not on the Mac.**

*(Revised 2026-07-31 after the hardware picture was corrected: a Jetson is being acquired,
an RTX 4070 12 GB is already on hand, and H100-class rental is available. The first draft
of this note said "cannot run on our hardware" — that was wrong about the fleet, not about
the Mac.)*

Cosmos3-Edge is a 4B omnimodal *world model* whose thesis overlaps ADR-078's almost
exactly: reconstruction is input to **prediction and action**, not appearance. It is an
NVIDIA-stack artifact end to end, so it is permanently out of reach for the Apple-Silicon
model lab — but it lands squarely on the hardware track WeftOS edge nodes are heading
toward. Treat it as a **real candidate for the Jetson node**, not merely a design reference.

### Where it fits, by machine

| hardware | verdict |
|---|---|
| **M5 Max (~/llm lab)** | ✗ never — no MLX/CoreML path, BF16-only forecloses quantising to fit |
| **RTX 4070 12 GB** | ~ partial. 4B @ BF16 ≈ 8 GB of weights leaves ~4 GB for activations/KV. The **reasoner / action path** is plausibly in reach; **480p video generation (189 frames)** almost certainly is not. Worth a real test rather than a guess. |
| **Jetson (incoming)** | ✓ explicitly in NVIDIA's tested-hardware list. This is the interesting one — it is the only path where a WeftOS *edge node* runs a world model locally rather than shipping frames home. |
| **Rented H100-class** | ✓ this is where the published numbers come from; the right venue for evaluating the video/forward-dynamics paths before committing to edge deployment. |

## What it actually is

Two transformer towers — an autoregressive one for discrete tokens, a diffusion one for
continuous multimodal generation. Notable for us:

| | |
|---|---|
| **Inputs** | text (≤4096 tok), images (256p/480p), video, **action trajectories (16–400 seq len)** |
| **Outputs** | images, video (12–30 fps, 50–150 frames), **action sequences (JSON)**, text |
| **Params** | 4B, BF16 |
| **Licence** | OpenMDW-1.1, **not gated**, commercial use permitted |
| **Target** | robotics, autonomous vehicles, smart spaces / factory-scale |

The action-trajectory I/O is the part worth attention: it consumes *and emits* action
sequences, which is the "world understanding → embodied policy" loop rather than a
renderer.

## The blocker — hard, not a matter of effort

- **NVIDIA GPUs only**: Ampere / Blackwell / Hopper. Tested on H100, H20, RTX PRO 6000,
  DGX, Jetson.
- **Linux only** — its card states other operating systems are untested.
- **Runtimes**: vLLM-Omni, vLLM, PyTorch. **No MLX, CoreML, or Apple-Silicon path stated.**
- **BF16 only** — "Other precisions like FP4, FP8, and FP16 are not officially supported",
  so the usual quantise-to-fit escape hatch is explicitly off the table.

Published latency is all H100-class: image→video (480p, 189 frames) 27.6 s on vLLM-Omni /
23.9 s PyTorch; forward dynamics 3.9 s. Reasoner TTFT 142–374 ms on RTX PRO 6000 Blackwell.
None of that transfers to an M5 Max.

**Jetson is the only interesting hardware note** — if a WeftOS edge node were ever
NVIDIA-based rather than ESP32/Android, this is the class of model that would run there.
That is a hardware-strategy question, not a model question.

## Why it still matters to ADR-078

ADR-078 argues reconstruction must feed a *structured* world model (objects, surfaces,
volumes in the BVH) rather than an appearance-only SOG. Cosmos3 is an independent, well-
funded bet on the same premise, and it goes one step further: the structured
representation is consumed by a policy that emits actions. Two things worth lifting:

1. **The dual-tower split** (discrete autoregressive + continuous diffusion) is a concrete
   answer to a question our splat→BVH→LeWM path also faces: how to keep symbolic entity
   structure and continuous geometry in one model without collapsing either. Compare
   against ADR-087's dual-branch spatio-temporal framing — same shape of answer, arrived
   at independently.
2. **Action trajectories as a first-class modality.** ADR-077/078 currently stop at
   capture → entities. If the roadmap ever reaches "the world model proposes an action",
   this is the interface shape to study.

## What I'd do

- **Do not** make it a dependency of anything that must run on the Mac. No Apple-Silicon
  path exists, and BF16-only means quantising to fit is explicitly unsupported.
- **Do** read the model card's I/O contract when specifying LeWM's action interface —
  particularly the action-trajectory sequence format and the 16–400 length window. That
  costs nothing and is useful regardless of whether we ever serve this model.
- **On the incoming Jetson**: this is the concrete thing to try first. It is on NVIDIA's
  own tested list, and it would answer a question ADR-077/078 currently leaves open —
  whether an edge node can hold a world model locally, or must remain a capture device
  that ships frames to a host. That is an architecture question, not a benchmark one.
- **Cheap intermediate step**: run the *reasoner* path on the RTX 4070 (12 GB) to sanity-
  check the I/O contract and action-sequence format before Jetson hardware lands. Expect
  the video-generation path not to fit; that is fine, it is not the part ADR-078 cares about.
- **Rent H100-class** only for evaluating video / forward-dynamics quality, if that ever
  becomes load-bearing. Do not rent to answer "does the action interface look right" —
  the 4070 can answer that.

## Provenance / confidence

Model card read directly 2026-07-31. Hardware, licence, runtime and latency claims are
quoted from that card. **Not verified by running anything** — we have no compatible
hardware, so every performance figure here is NVIDIA's own, on NVIDIA silicon.
