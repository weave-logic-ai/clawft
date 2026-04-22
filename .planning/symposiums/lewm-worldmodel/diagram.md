# LeWM × ECC × weftos — System Infographic

**Reference diagram for ADRs 048–058.** Two views:
①  Full-system panel diagram · ②  The H-O-E-A real-time training cycle through DEMOCRITUS

---

## ①  System panels · sensor-primary data flow · world model as additive consumer

```
╔═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
║                                                                                                                 ║
║                 W E F T O S   ×   L e W M   ·   J E P A   U N D E R   T H E   D A G                            ║
║                                                                                                                 ║
║      Sensor network  =  primary & self-sufficient       ·       World model  =  additive consumer, optional    ║
║      SIGReg isotropic-Gaussian latent manifold (ADR-050)  ·  decoupling invariant (ADR-058)  ·  ADR-048…058    ║
║                                                                                                                 ║
╚═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╝


┌──────────────────────── ❶  SENSOR PLANE  (per node, always-on) ────────────────────────┐
│                                                                                           │
│     RGB-D  │  IMU  │  Proprio  │  LiDAR  │  Audio  │  … future sensor classes            │
│        │       │        │         │         │                                             │
│        ▼       ▼        ▼         ▼         ▼      raw reads (per-sensor cadence)         │
│                                                                                           │
│     ┌────────────── SENSOR PIPELINE  (weftos-sensor-pipeline, ADR-056) ──────────────┐   │
│     │                                                                                 │   │
│     │   ❶ COLLECT ─────▶  ❷ AGGREGATE ─────▶  ❸ ENCODE ★                              │   │
│     │   transmit-gate      multi-sensor fusion    SIGReg-manifold ViT-tiny            │   │
│     │   (novelty · quality     (trainable)         z_t ∈ ℝ¹⁹²                         │   │
│     │    · salience · summary)                     zero-mean, unit-cov per dim        │   │
│     │                                                                                 │   │
│     │   Three trainable RVF-hosted small models per sensor class  (ADR-057)           │   │
│     │   hot-swap at tick alignment · auto-rollback on sigreg_health < 0.85 / 30s      │   │
│     │                                                                                 │   │
│     │   In-process (strong nodes) · OR ·  Pipeline Service (weak-sensor fan-in)       │   │
│     │   — identical wire format in both modes — delegated-key attestation             │   │
│     └─────────────────────────────────────────────────┬───────────────────────────────┘   │
│                                                        │                                   │
│                              emit (per-frame Ed25519) ▼                                   │
└────────────────────────────────────────────────────────┼───────────────────────────────────┘
                                                         │
                                                         ▼
┌────────────────────── ❷  OBSERVATION WIRE  (mesh-native, ADR-053) ────────────────────────┐
│                                                                                             │
│    ══▶ mesh.sensor.v1.encoded.{cluster}     PRIMARY stream · 10 Hz · burst-able 100 Hz     │
│           ★ cultivated transition tuples · SIGReg partial sums · quality flags             │
│           ★ deterministic burst_episode_id · CBOR + extension fields · ≤ 2 KB / frame      │
│                                                                                             │
│    ◀── mesh.sensor.v1.consensus.{cluster}   CONDITIONAL · z_cluster(t), ẑ_{t+1}, health  │
│                                             (present only if a world model is running)    │
│                                                                                             │
│   ◀─▶ mesh.sensor.v1.control.{cluster}      MULTI-PUB · drift · surprise · plan results   │
│                                                                                             │
│    packets are OBSERVATIONAL-ONLY · no gradient proxies · no consumer-specific framing     │
│    every frame indexed into EXOCHAIN (ADR-022) → replay is the single source of truth      │
└──────────┬──────────────────────────────────────────────────────┬───────────────────────────┘
           │ subscribe                                            │ subscribe
           │                                                      │
           ▼                                                      ▼
┌── ❸  LOCAL CONSUMERS (authoritative, per node) ──┐    ┌── ❹  WORLDMODELSERVICE (optional, cluster) ──┐
│                                                    │    │                                                │
│   hnsw_eml  (ruvector-core)                        │    │   Fusion Transformer                           │
│   ─────────────────────────                        │    │   ├── learned temporal-attention mask          │
│   192-dim HNSW · SIGReg-preconditioned             │    │   ├── cross-sender skew tolerance              │
│                                                    │    │   └── SIGReg-merge weighting (window-size)     │
│   ECC CAUSAL GRAPH  (causal.rs 3417 LOC)           │    │                  │                              │
│   ────────────────────────────────────             │    │                  ▼                              │
│   CMVG forest (ADR-046)                            │    │   Predictor  pred_φ                            │
│   StructureTag::LatentWorldModel (ADR-048)         │    │   z_t , a_t  ──▶  ẑ_{t+1}                      │
│   LatentHandle(Uuid) in CausalNode.metadata        │    │                  │                              │
│                                                    │    │                  ▼                              │
│   HYPOTHESIZE · OBSERVE · EVALUATE · ADJUST        │◀═══│   LatentPlanner  (ADR-051)                    │
│   (see diagram ② for the real-time cycle)          │    │   CEM (default) / MPPI-warm / gradient         │
│                                                    │    │   10 Hz background thread · never inline       │
│                                                    │    │                                                │
│   DEMOCRITUS  (1 kHz servo, 1 ms tick)             │    │   Deployment: Single · Raft-standby · P2P      │
│   ADR-047 self-calibrated heartbeat                │    │   (ADR-054)                                    │
│   ArcSwap<ActionPlan>  ◀───── 10 Hz plan ──────────┤    │                                                │
│                                                    │    │   Exposes LATTICE API  (ADR-052)              │
│                  │                                  │    │   observe · observe_node · predict · plan     │
│                  ▼                                  │    │   recall · subscribe_surprise ·                │
│   Actuators / Servos / Effectors                   │    │   subscribe_drift (local-derivable) · ✱        │
│                  │                                  │    │                                                │
│                  └─────── closes loop ◀─ sensors   │    │   ServiceUnavailable ⇒ caller falls back to    │
│                                                    │    │   local-ECC-only paths (decoupling invariant)  │
└────────────────────────────────────────────────────┘    └────────────────────────────────────────────────┘
           ▲                                                                     │
           │   drift / surprise (control stream)                                 │
           │                                                                     │
           └──────── ◀── mesh.sensor.v1.control ───────────────────────────  ────┘


┌────────────────────── ❺  TRAINING LAYER  (two surfaces, decoupled cadences) ───────────────────────┐
│                                                                                                       │
│   ╭─ EDGE INTELLIGENCE (ADR-057) · offline · per-sensor-class ─╮                                     │
│   │  data: ExoChain replay + synthetic augmentation             │                                     │
│   │  venue: training-equipped node  │  trigger: drift alert     │                                     │
│   │  deliver: RVF segment (WASM_SEG + WITNESS) · governance     │──▶ hot-swap pipeline @ tick align  │
│   │  rollback: sigreg_health < 0.85 for 30 s → prior segment    │◀── per-encoder SIGReg monitor       │
│   ╰───────────────────────────────────────────────────────────╯                                     │
│                                                                                                       │
│   ╭─ WORLD-MODEL STREAMING MERGE (ADR-055) · online · in-service ╮                                   │
│   │  src: mesh.sensor.v1.encoded (live) + ExoChain (deep history) │                                   │
│   │  per-class importance-weighted replay (rare-event residency)  │                                   │
│   │  streaming checkpoint via RVF · ExoChain-attested              │                                   │
│   │                                                                │                                   │
│   │  FOUR-CONDITION AND ROLLBACK GATE:                             │                                   │
│   │    · cluster SIGReg health (merged partial sums)               │                                   │
│   │    · held-out probing accuracy                                 │                                   │
│   │    · VoE surprise differentiation                              │                                   │
│   │    · temporal-straightening score                              │                                   │
│   ╰──────────────────────────────────────────────────────────────╯                                   │
└───────────────────────────────────────────────────────────────────────────────────────────────────────┘


┌────────────────────── ❻  EXOCHAIN  ·  attestation & replay spine  (ADR-022, ADR-033) ─────────────┐
│                                                                                                    │
│   ✦ every sensor frame   ✦ every checkpoint   ✦ every RVF segment activation                      │
│   ✦ every rollback       ✦ every governance topology change  (ADR-033 three-branch)               │
│   ✦ every LatticeApi capability grant                                                              │
│                                                                                                    │
│   Replay is the single source of truth — standby / peer WorldModelServices catch up from replay   │
│   Training data comes from ExoChain, never from the live bus                                       │
└───────────────────────────────────────────────────────────────────────────────────────────────────┘
```

**Legend**

```
   ══▶    primary observation stream (always present, required)
   ◀──    conditional / optional stream (only when world model running)
   ◀─▶   multi-publisher bidirectional (control plane)
   ◀═══   subscription (consumer pulls from wire)
    ★    SIGReg-manifold contract (ADR-050) — load-bearing invariant
    ✦    ExoChain-attested event (ADR-022)
    ✱    lone "local" LatticeApi method — derivable without world model
```

---

## ②  Real-time H-O-E-A cycle · training happens continuously through DEMOCRITUS

```
                                                         ⟲  CONTINUOUS · NO BATCHES · NO STOPS  ⟲

                 ┌───────────────────────────┐
    ┌────────────│   SENSOR PIPELINE         │◀──────────────────────────┐
    │            │   z_t ∈ ℝ¹⁹² ★            │                             │
    │            └──────────────┬────────────┘                             │
    │                           │ observation                               │
    │                           │ frame (wire)                              │
    │                           ▼                                           │
    │            ┌─────────────────────────────────────────────────────┐    │
    │            │     L O C A L   E C C   C A U S A L   G R A P H     │    │   (environment /
    │            │              (causal.rs · CMVG · ADR-046)            │    │    actuators /
    │            │                                                       │    │    effectors)
    │            │   ┌────────────┐                                      │    │
    │            │   │ HYPOTHESIZE│  propose  do(a_t)  intervention      │    │
    │            │   └──────┬─────┘                                      │    │
    │            │          │ candidate action                            │    │
    │            │          ▼                                             │    │
    │            │   ┌────────────┐    predictor pred_φ (ADR-049)         │    │
    │            │   │  OBSERVE   │─▶  ẑ_{t+1} = f(z_t, a_t)             │    │
    │            │   └──────┬─────┘           │                           │    │
    │            │          │                  │  next real latent z_{t+1}│    │
    │            │          │   ┌──────────────┴──────────────┐          │    │
    │            │          ▼   ▼                              │          │    │
    │            │   ┌─────────────┐   VoE surprise = ‖ẑ−z‖²  │          │    │
    │            │   │  EVALUATE   │   Δλ_Fiedler shift          │          │    │
    │            │   └──────┬──────┘                             │          │    │
    │            │          │ residuals                          │          │    │
    │            │          ▼                                    │          │    │
    │            │   ┌─────────────┐   CausalCollapseModel       │          │    │
    │            │   │   ADJUST    │   · edge-weight regression   │──────────┼──┐ │
    │            │   │             │   · predictor online step    │          │  │ │
    │            │   │             │   · SIGReg health update     │          │  │ │
    │            │   └──────┬──────┘                             │          │  │ │
    │            │          │                                     │          │  │ │
    │            │   impulse: EvidenceFor / Contradicts /         │          │  │ │
    │            │   SurpriseSignal   (confidence-decayed)        │          │  │ │
    │            │          │                                     │          │  │ │
    │            │          └─────── re-enters graph ──────────────          │  │ │
    │            └──────────────────────┬──────────────────────────┘          │  │ │
    │                                   │ chosen action a_t                    │  │ │
    │                                   ▼                                      │  │ │
    │            ┌────────────────────────────────────────────┐                 │  │ │
    │            │   LatentPlanner (ADR-051) @ 10 Hz          │                 │  │ │
    │            │   (cluster-only; optional · ADR-058)       │                 │  │ │
    │            │   CEM / MPPI-warm / gradient-shooting      │                 │  │ │
    │            │          │                                  │                 │  │ │
    │            │          ▼                                  │                 │  │ │
    │            │   ArcSwap<ActionPlan>  ◀─ publish ──┐      │                 │  │ │
    │            └──────────────┬──────────────────────┘      │                 │  │ │
    │                           │                              ▲                 │  │ │
    │                           │ 1 kHz interpolation          │ fallback:       │  │ │
    │                           ▼                              │ symbolic        │  │ │
    │            ┌────────────────────────────┐                │ DEMOCRITUS       │  │ │
    │            │   DEMOCRITUS servo loop    │────────────────┘ rank_evidence   │  │ │
    │            │   1 ms tick (ADR-047)      │                  _by_impact      │  │ │
    │            │   interp waypoints         │                                   │  │ │
    │            └──────────────┬─────────────┘                                   │  │ │
    │                           │ motor commands                                  │  │ │
    │                           ▼                                                 │  │ │
    │            ┌────────────────────────────┐                                    │  │ │
    │            │   ACTUATORS · EFFECTORS    │────┐                              │  │ │
    │            └────────────────────────────┘     │                              │  │ │
    │                                               │ environment changes         │  │ │
    │                                               ▼                              │  │ │
    │            ┌────────────────────────────┐                                    │  │ │
    │            │   WORLD (physical)         │─────────── new sensor readings ──┐ │  │ │
    │            └────────────────────────────┘                                   │ │  │ │
    └──────────────────────────────────────────────────────────────────────────────┘ │  │ │
                                                                                      │  │ │
                                                                                      ▼  ▼ ▼
                                                         ┌────────────────────────────────────────┐
                                                         │  ExoChain · ADR-022                    │
                                                         │  every (a_t, z_t, z_{t+1}, surprise)    │
                                                         │  tuple attested · feeds replay training │
                                                         └────────────────────────────────────────┘

    The loop is THREE timescales running concurrently:
       ·  1 ms   DEMOCRITUS servo tick (ADR-047 self-calibrated heartbeat)
       · 10 Hz   planner replan · predictor step · HNSW insert · graph impulse
       ·  ≪1 Hz  edge-intelligence retrains (offline, RVF-delivered hot-swap)
       · cont.   world-model streaming-merge checkpoints (ADR-055)

    ECC is authoritative per node (weaver A1 amendment).  Cluster fusion is an
    OBSERVATION input to the local graph, not a reasoning substitute.  When
    the cluster service is absent, the loop runs unchanged — just without
    cross-node z_cluster and without learned planning (graceful degradation,
    ADR-058 rule 5).
```

---

## What is actually being built

- **Three new workspace crates** — `weftos-worldmodel-core` (`no_std`, traits), `weftos-worldmodel-impls` (candle-backed), `weftos-worldmodel` (facade). Plus `weftos-sensor-pipeline`, `weftos-sensor-pipeline-wire`, `clawft-worldmodel-service` binary, and in-tree `clawft-delegation`.
- **One learned perceptual substrate** — ViT-tiny encoder + AdaLN-modulated predictor, trained end-to-end from pixels with SIGReg (Epps-Pulley on M random 1D projections). No EMA, no stop-gradient scaffolding.
- **One universal latent contract** — isotropic-Gaussian `N(0, I)` in 192 dims, version-tagged, ExoChain-attested, measurable in production via Welford-based `sigreg_health`.
- **One observation wire** — three topics under `mesh.sensor.v1.*`, CBOR + Ed25519, observational-only packets, ExoChain-indexed.
- **Two training surfaces** — offline per-sensor-class edge intelligence (RVF-delivered, hot-swappable); online streaming-merge world-model training (four-condition AND rollback).
- **One kernel service** — WorldModelService, separate process, three deployment topologies (single / hot-standby / peer-to-peer), `LatticeApi` exposed via `ServiceApi` registration.
- **One preserved invariant** — ECC remains the authoritative reasoning substrate per node; the latent world model is a sub-layer that publishes impulses, never short-circuits causal edges.
- **One architectural principle** — the sensor network is primary and self-sufficient; the world model is an additive consumer.
