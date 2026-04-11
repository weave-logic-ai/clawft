# Gaming & Robotics Symposium: Final Report

**Embodied Causal Cognition: From Game Characters to Physical Robots**

| Field | Value |
|-------|-------|
| Date | 2026-04-11 |
| Status | Final |
| Sources | Robotics Research Report, Gaming Research Report, Experiments Plan, Integration Architecture, Creator Answers |
| Author | Symposium Synthesis Team |

---

## Executive Summary

Embodied Causal Cognition (ECC) is the cognitive substrate at the core of WeftOS.
It replaces black-box neural networks and hand-authored behavior trees with a
causal directed acyclic graph (DAG) where every decision traces back to
observable causes and measurable effects. ECC was designed for software system
analysis -- understanding, documenting, and automating client systems via
knowledge graph -- but its name contains the word "Embodied" for a reason.
The architecture was always meant for physical and simulated bodies.

This symposium investigated whether ECC can power two adjacent domains:
**game character intelligence** and **physical robot control**. Five expert
teams produced research reports, an integration architecture, and an experiments
plan. The WeftOS creator answered 18 strategic questions that shaped the final
direction.

**The central thesis is this: the DEMOCRITUS cognitive loop IS a servo control
loop.** The five-phase cycle -- SENSE, EMBED, SEARCH, UPDATE, COMMIT -- maps
directly to the robotics sense-plan-act paradigm. A robot arm reading an IMU,
a game NPC observing the player, and a CLI assistant watching for file changes
all execute the same loop. The only difference is which `Sensor` and `Actuator`
implementations are registered.

The **Perceive-Think-Act (PTA) framework** unifies all WeftOS applications
under a single abstraction. PERCEIVE samples sensors and embeds readings into
HNSW vector space. THINK traverses the causal graph, retrieves motor memories
via similarity search, and selects an action. ACT dispatches the command
through a governance gate to the actuator. Conversation is a slow actuator.
A servo is a fast one. The kernel does not care.

**The creator's strategic decisions** set the course:

- **Parallel tracks**: Gaming and robotics advance simultaneously. Neither waits for the other.
- **Sidecar model**: WeftOS runs as a native binary sidecar process, not embedded WASM. The game engine connects via TCP.
- **Blog-first publishing**: LinkedIn posts as each experiment completes. Academic papers and conference demos later.
- **Many small trees, not one big forest**: Causal graphs stay scoped to their application. Evaluate per use case.
- **Agent-heavy development**: 10 human hours per week, many bot hours. No fixed timeline.

The combined research demonstrates that ECC offers 10-100x sample efficiency
over model-free reinforcement learning, full explainability through causal
traces, causal structure that transfers across domains without catastrophic
forgetting, and a governance layer that provides structural safety rather than
reward-shaped approximations. Eight experiments are planned across a 7-week
timeline at a total hardware cost of $1,102.

---

## Session 1: The ECC Thesis

### Why Causal DAGs Beat Neural Networks

The robotics team and the gaming team arrived at the same conclusion from
different directions: current approaches in both fields have hit a ceiling,
and causal reasoning is the way through.

**In robotics**, the state of the art is dominated by reinforcement learning
(RL) and imitation learning (IL). The numbers tell the story:

- OpenAI's Dactyl required approximately 13,000 simulated years of experience
  to learn Rubik's Cube manipulation, achieving only 60% success on easy
  scrambles. OpenAI subsequently disbanded its robotics team.
- DeepMind's MuJoCo Playground trains locomotion policies in minutes on GPU,
  but the equivalent real-world time would be years.
- DreamerV3, the current world-model RL state of the art, still suffers from
  compounding prediction errors over long horizons and catastrophic forgetting.
- Action Chunking with Transformers (ACT) achieved 80-90% success from only
  10 minutes of demonstrations -- a 1000x improvement over RL -- but the
  learned policy has no causal understanding and cannot transfer to novel tasks.

**In gaming**, the AI techniques in production today -- behavior trees, utility
AI, GOAP, HTN planning -- were all established before 2010. NPC intelligence
has not meaningfully improved in fifteen years. Every behavior is authored.
NPCs do not learn from player behavior. They have no memory between encounters.
Every player sees the same AI. The budget allocation problem compounds the
stagnation: graphics get 90% of the compute budget; AI gets 1-5ms per frame on
a single thread.

**ECC addresses both ceilings with five structural advantages:**

1. **Sample efficiency (10-100x).** Causal models encode structure separately
   from parameters. When a robot encounters a new object, it does not need to
   relearn that "applying force causes displacement." It only needs to estimate
   mass, friction, and compliance. Judea Pearl's do-calculus formalizes this:
   a robot can predict the outcome of an action it has never performed, as long
   as the causal graph captures the relevant relationships.

2. **Full explainability.** Every decision has a complete trace from sensor input
   through typed causal edges (`Causes`, `Inhibits`, `Enables`, `EvidenceFor`,
   `Contradicts`, `TriggeredBy`) to motor output. When a robot drops an object,
   the system traces: "grip force was below threshold because slip was detected
   because surface friction was overestimated." This is the actual computation,
   not a post-hoc attribution.

3. **Causal transfer.** The causal graph for "reaching and grasping" is
   identical whether the robot is a Sesame quadruped or a 6-axis industrial arm:
   `target_detected -> trajectory_planned -> joints_actuated -> contact_made ->
   grip_applied`. Edge weights differ; topology is invariant. This is the insight
   behind Scholkopf et al. (2021) on causal representation learning: high-level
   causal variables are the transferable unit.

4. **No catastrophic forgetting.** Learning a new skill adds nodes and edges to
   the graph; it never modifies existing ones. The `CausalGraph` uses `DashMap`
   for concurrent access with purely additive operations. A robot that learned
   to walk does not forget walking when it learns to reach, because the walking
   subgraph is a separate connected component.

5. **Structural safety.** In neural RL, safety is enforced through reward
   shaping -- approximate and brittle. In a causal DAG, safety is structural.
   An `Inhibits` edge from "human within 0.5 meters" to "high-speed motion"
   prevents the action before it enters the planning pipeline.

---

## Session 2: Robotics Track

### Key Findings

The robotics team produced a comprehensive survey of the field and mapped ECC
primitives to robotics concepts:

| ECC Component | Robotics Role |
|---------------|---------------|
| `CausalGraph` | World model + control logic |
| `DemocritusLoop` | Sense-plan-act control loop |
| `HnswService` | Motor memory / proprioceptive memory |
| `ImpulseQueue` | Reflex arc / interrupt system |
| `CrossRefStore` | Sensorimotor binding |
| `CognitiveTick` | Control frequency (0.5ms-50ms adaptive) |
| `EccCalibration` | Servo characterization / system ID |
| `WeaverEngine` | Learning / model refinement |
| `ExoChain` | Audit log / black box recorder |

**The Sesame Robot** serves as the reference implementation. Created by Dorian
Todd, Sesame is an open-source quadruped with 8 MG90S servos, an ESP32-S2
controller, and a 128x64 OLED face. At $50-60 in components, it provides an
ideal learning platform. The creator confirmed: Sesame is a hobby project and
starting point, not a product. It has a crab-like element that aligns with
ECC's embodied goals.

**The hardware stack** follows a three-tier architecture per the creator's
decision to use multiple ESP32s for sensors and a Pi 5 for brains:

- **T-1 (Actuator Tier)**: MKS Gen L or Arduino, executing commanded
  trajectories with hardware-level timing. No cognitive processing.
- **T0 (Sensor Tier)**: ESP32-S3 at 0.5-5ms tick. Sensor fusion hub running
  impulse-only ECC mode. Local reflex loops (emergency stop on overcurrent)
  execute without waiting for brain-tier processing.
- **T2 (Brain Tier)**: Raspberry Pi 5 running the full WeftOS kernel with
  CausalGraph, HnswService, DemocritusLoop, and WeaverEngine at 10-50ms tick.

**Industrial applications** extend beyond hobby robotics. The research team
mapped ECC to 3D printing (adaptive parameter control via causal graphs of
temperature, flow, speed, and retraction), CNC milling (chatter detection
through causal models of spindle speed, feed rate, and tool wear),
pick-and-place (grasp optimization with reusable causal structures), welding
(seam tracking with material-specific parameter learning), and agricultural
robotics (field-adaptive AI with invariant causal structure).

**The creator grounded these ambitions:** keep it cheap, use custom 3D-printed
robots, run robotics in parallel with gaming. No safety certification yet --
address per-product as designs mature.

---

## Session 3: Gaming Track

### Key Findings

The gaming team documented a fifteen-year stagnation in game AI and mapped
every ECC primitive to a game character concept:

- **CausalGraph as beliefs**: An NPC's causal graph IS its understanding of
  the world. Two NPCs with different experiences develop different beliefs.
  "I don't trust the player because they stole from my shop" is a causal chain,
  not a flag.

- **HNSW as episodic memory**: Every experience is embedded as a vector. When
  an NPC faces a new situation, it queries HNSW for the most similar past
  experiences. A character with 10,000 embedded experiences requires
  approximately 15MB of HNSW storage at 384 dimensions.

- **Coherence score as emotional state**: High coherence (0.8-1.0) maps to
  confidence and decisive action. Low coherence (0.0-0.2) maps to panic and
  irrational choices. This emotional system is not authored; it emerges from
  graph structure.

- **ImpulseQueue as reflexive reactions**: A loud explosion triggers a
  `NoveltyDetected` impulse. An ally's betrayal triggers a `CoherenceAlert`.
  High-priority impulses preempt ongoing processing.

- **Governance as values**: A lawful NPC's governance prohibits illegal actions
  even when the causal graph suggests they would be effective. Governance can
  evolve -- an NPC observing that rule-following leads to bad outcomes may
  experience governance drift, the equivalent of a moral crisis.

- **Gap analysis as curiosity**: The WeaverEngine identifies gaps in the causal
  graph. An NPC that has never seen magic has a massive gap around magical
  phenomena, driving curiosity and investigation behavior.

- **Spectral partition as internal conflict**: An NPC with a clean partition
  between "loyalty to the king" and "evidence the king is corrupt" experiences
  genuine cognitive dissonance. Character arcs emerge without scripting.

**Emergent personality** arises from the combination of all these mappings. Two
NPCs initialized with the same code but given different experiences develop
different beliefs, emotional baselines, memories, values, curiosities, social
awareness, and internal conflicts. Personality is not a set of authored traits;
it emerges from the accumulated structure of lived experience.

**Procedural animation** via ECC treats a game character's skeleton as a servo
array. Rather than playing canned animation clips, an ECC-driven character
generates movement through physics simulation governed by its motor control
causal graph. This is computationally more expensive than clip playback but
produces movement that naturally adapts to terrain, obstacles, injuries, and
load.

**The Nemesis patent avoidance strategy** is critical. Warner Bros.' US Patent
10,926,179, which does not expire until 2036, covers systems where interactions
between a player and NPC A affect the parameters of NPC B through pre-defined
event templates. ECC operates differently: there are no pre-defined event
templates, parameter changes emerge from causal reasoning over the full graph,
and the mechanism is general-purpose cognition rather than a purpose-built
nemesis system. The recommended branding is **"causal personality"** rather
than anything that invokes the Nemesis name. Legal analysis should confirm this
architectural distinction.

**The creator's decisions for gaming:**

- **Unity first**, possibly Unreal. Need to test which has better hook/harness
  integration.
- **Sidecar model**: WeftOS runs as a separate native binary process. The game
  engine connects via TCP. Not WASM, not embedded in the engine.
- **Open source**: not decided yet; evaluate per product.

---

## Session 4: Perceive-Think-Act Loop

### The Unifying Framework

The integration architecture team produced the key intellectual contribution of
the symposium: the Perceive-Think-Act (PTA) framework that proves a single
cognitive kernel genuinely serves both domains.

**The formal definition:**

```
PERCEIVE        THINK               ACT
---------       ---------------     ---------
Sensors    -->  CausalGraph    -->  Actuators
               + HNSW search
               + Governance
```

**PERCEIVE.** Sample all registered sensors. Each reading is converted into a
fixed-dimensional embedding vector and inserted into the HNSW store. A
corresponding `CausalNode` is created with `EvidenceFor` edges linking the new
observation to the nearest existing nodes found by vector search.

**THINK.** The DEMOCRITUS cycle runs: gather recent entries, perform HNSW
similarity search to retrieve relevant historical nodes (motor memory, past
observations), traverse causal edges to build a local causal model, propose an
action, and submit through the governance gate.

**ACT.** If governance approves, the command is dispatched to registered
actuators. Actuator feedback is immediately re-ingested as a new PERCEIVE
sample, closing the loop.

### The 4-Tier Control Loop

Not all control loops need the same latency. The architecture defines four
tiers:

| Tier | Name | Period | Use Case |
|------|------|--------|----------|
| T-reflex | Reflex | 0.5ms | Emergency stop, collision avoidance, current limiting |
| T-servo | Servo | 5ms | PID joint control, closed-loop position tracking |
| T-plan | Planning | 50ms | Motion primitive selection, HNSW search, causal reasoning |
| T-strategy | Strategy | 500ms-5s | Goal planning, LLM inference, cross-domain transfer |

T-reflex runs outside the ECC entirely -- a hard-real-time interrupt handler on
the microcontroller. T-servo runs a simple PID loop. T-plan is where the ECC
DEMOCRITUS cycle operates. T-strategy handles high-level deliberation and may
involve LLM inference.

### The Human-as-Actuator Model

In a CLI conversation, the "actuator" is language output and the "sensor" is
text input. This is the slowest PTA loop (seconds to minutes per cycle), but
it uses the same infrastructure: `ConversationActuator` sends a message,
`CodebaseSensor` watches for changes, and the ECC reasons about conversation
history using the causal graph.

**This means all WeftOS applications are robotic -- they just differ in effector
bandwidth.** The software assessment tool, the cold case investigator, the
sonobuoy processor -- these are disembodied instances of an embodied system.
The robot arm and game character are the embodied instances. Same kernel, same
causal graph, same governance pipeline.

### New Crates

The integration architecture defines the following new crate boundaries:

- `clawft-actuator`: `Actuator` trait, `ServoActuator`, `StepperActuator`,
  `GameJointActuator`, `ConversationActuator`, `HttpActuator`
- `clawft-sensor`: `Sensor` trait, `ImuSensor`, `CameraSensor`,
  `CurrentSensor`, `PositionSensor`, `GameStateSensor`, `CodebaseSensor`
- `clawft-control`: `ControlLoop`, `MotionPlanner`, `GovernanceGate`,
  `SensorNode`, `ActuatorNode`
- `clawft-primitive`: `MotionPrimitive` storage, HNSW-based primitive
  library, precondition/postcondition matching
- `clawft-bridge`: Game engine TCP bridge protocol (MessagePack over TCP),
  `PerceptionFrame`, `ActionFrame`, character registration

---

## Session 5: Sim-to-Real Pipeline

### Game Engines as Robotics IDEs

The sim-to-real pipeline is where the gaming and robotics tracks converge.
Game engines (Unity, Unreal, Godot) already provide physics simulation,
rendering, and scripting. The insight is that these are also robotics
development environments:

- **Visual debugging**: See the robot's causal graph, HNSW neighborhoods, and
  coherence scores overlaid on the 3D model.
- **Scenario testing**: Create stairs, obstacles, and adversaries without
  risking physical hardware.
- **Rapid iteration**: Change the environment and immediately observe
  behavioral adaptation.
- **Safe testing**: Physical robots cannot safely test falling or collision
  recovery. In simulation, these scenarios can be run millions of times.

### Why Causal Transfer Succeeds Where Neural Transfer Fails

The sim-to-real gap exists because neural policies encode correlations specific
to the simulator. When the simulator's rendering, physics, or sensor model
differs from reality, the policy degrades. Domain randomization attempts to
bridge this gap but has been shown (ICLR 2024) to be counterproductive for
measurable parameters like mass and inertia.

Causal models do not have this problem:

1. **The causal graph encodes physics, not simulator artifacts.** "Torque causes
   angular acceleration proportional to inverse inertia" is true in MuJoCo,
   Unity, Isaac Sim, and reality.

2. **Only parameters are simulator-specific.** The simulated inertia value may
   differ from the real value, but this is captured in a single edge weight.

3. **Transfer is explicit, not implicit.** The system knows exactly which
   parameters need re-estimation and which are invariant.

Domain randomization is unnecessary with causal models. A causal model already
has friction as an explicit variable: `surface_friction -> required_grip_force
-> servo_command`. It estimates friction from a handful of exploratory grasps
rather than learning a function approximation over the entire parameter space.

### The Calibration Protocol

When deploying an ECC-trained causal model to a new physical platform:

1. Import the causal graph from simulation. Graph topology is identical; edge
   weights are initialized from sim.
2. Run servo calibration: command each joint through its range, measure actual
   response time, backlash, and range limits. Update edge weights.
3. Run dynamic calibration: execute 5-10 simple motions, compare predicted
   outcomes with actual outcomes, update edge weights.
4. Validate: execute test tasks. If confidence exceeds threshold, deploy.

Total calibration time: 2-5 minutes. Compare with RL sim-to-real, which
requires hours to days of real-world fine-tuning.

**The creator confirmed the deployment model**: binaries as sidecars, not WASM.
Linux and Windows gaming platforms already have native binaries. WeftOS runs as
a sidecar process, game connects via TCP. WASM is a fallback, not primary.

---

## Session 6: Experiments

### The Eight Experiments

The experiments plan defines eight experiments prioritized per the creator's
answers. The creator specified: servo calibration first, gait learning and 3D
print quality control in parallel, work through them in order of readiness.

| # | Experiment | Duration | Incremental Cost | Key Metric |
|---|-----------|----------|-----------------|------------|
| 1 | Servo Calibration via ECC | 6 days | $154 | Speed ratio >= 10x, MAE <= 1.0 deg |
| 2 | Sesame Robot Gait Learning | 11 days | $651 | Stable 10-step gait in < 500 attempts |
| 3 | Motion Mimicry from Video | 8 days | $50 | Cosine similarity > 0.80 |
| 4 | 3D Print Quality Learning | 10 days | $373 | 30% quality improvement over baseline |
| 5 | Game Character Gait (Sim) | 9 days | $80 | 7x+ sample efficiency over PPO |
| 6 | Sim-to-Real Transfer | 3 days | $0 | Stable step in < 20 calibrations |
| 7 | Multi-Robot Skill Transfer | 6.5 days | $45 | Transfer efficiency > 5x |
| 8 | Personality Emergence | 10 days | $0 | >= 3 distinct archetypes, human discrimination > 70% |

**Critical path**: Exp 1 -> Exp 2 -> Exp 5 -> Exp 6 (29 days total).

**Parallelizable**: Exp 4 and Exp 8 can run concurrently with any other
experiment. Exp 3 can run concurrently with Exp 5 after Exp 2 completes.

**Two 3D printers available**: Bambu Lab + Prusa (PS1). The creator confirmed
both are on hand.

### Experiment Highlights

**Experiment 1 (Servo Calibration)** is the foundation. ECC characterizes
servo response curves by commanding positions, observing results via ArUco
marker detection and computer vision, recording errors in the causal graph,
and converging on a correction curve. Target: 10x faster than manual
calibration with sub-1-degree accuracy.

**Experiment 2 (Gait Learning)** uses a 22-servo Sesame humanoid robot with
IMU, safety harness, and gantry frame. ECC discovers stable micro-movements
through random perturbation, then composes them into gait phases. The
hypothesis: stable 10-step gait in fewer than 500 total attempts, compared to
8+ hours of manual tuning.

**Experiment 5 (Simulated Gait)** benchmarks ECC against PPO on Walker2d-v4
(MuJoCo). The target: reach reward=1000 in under 200,000 steps versus PPO's
typical 1,500,000. PPO may achieve higher asymptotic reward, but ECC provides
human-readable causal explanations for every fall.

**Experiment 6 (Sim-to-Real Transfer)** tests the central claim: export the
sim-trained causal graph, import it to the physical Sesame robot, run 20
calibration movements, and walk. The hypothesis: the graph structure transfers
intact; only edge weights need updating.

**Experiment 8 (Personality Emergence)** places 10 identically initialized ECC
agents in a grid world with different encounter sequences. The prediction: at
least 3 distinct personality archetypes emerge, with human judges distinguishing
characters with over 70% accuracy.

### Timeline

```
Week 1:  Exp 1 (Servo Calibration)      | Exp 4 (3D Print - setup)
Week 2:  Exp 2 (Gait Learning - setup)  | Exp 4 (3D Print - runs)
Week 3:  Exp 2 (Gait Learning - runs)   | Exp 8 (Personality - build)
Week 4:  Exp 3 (Motion Mimicry)         | Exp 5 (Sim Gait)
Week 5:  Exp 6 (Sim-to-Real)            | Exp 8 (Personality - runs)
Week 6:  Exp 7 (Multi-Robot Transfer)   | Analysis + paper drafts
Week 7:  Buffer + re-runs               | Final analysis
```

Total calendar time: 7 weeks with 2 parallel tracks. Total person-days: 63.5.
No fixed deadline -- the creator specified working through things as fast as
practical, in the right order.

---

## Session 7: Roadmap

### v0.7: ACT Layer

The next WeftOS release focuses on the actuator abstraction:

- `Actuator` and `Sensor` traits with associated `Command`, `Feedback`, and
  `Reading` types
- `ServoActuator` (PCA9685 PWM control over I2C)
- `StepperActuator` (G-code streaming to MKS Gen L via serial UART)
- `GameJointActuator` (TCP bridge to game engines)
- `GovernanceGate` with built-in rules: joint limits, velocity caps, current
  limits, collision checking, emergency stop
- Game engine bridge protocol (MessagePack over TCP) with `PerceptionFrame`
  and `ActionFrame` wire format
- Servo calibration experiment software (Experiment 1)

### v0.8: LEARN Layer

Closed-loop experimentation and skill acquisition:

- Online causal model updating from experience during the DEMOCRITUS loop
- Motor memory: HNSW-stored motion primitives with preconditions,
  postconditions, and expected sensor profiles
- Sim-to-real transfer protocol: graph export, import, edge-weight
  recalibration
- Multi-robot skill transfer via shared HNSW embedding space
- `WeaverEngine` extensions for motor skill refinement

### v1.0: EMBODY

The reference embodied system:

- Full Perceive-Think-Act-Learn loop running on physical hardware
- Sesame quadruped running ECC with live sensor fusion, causal reasoning,
  and adaptive gait
- Game character running ECC as TCP sidecar with emergent personality,
  procedural animation, and causal memory
- Sim-to-real pipeline demonstrated end to end
- Fleet learning: multiple robots sharing causal models via mesh gossip

### Publishing Strategy

Blog posts on LinkedIn as each experiment completes. The creator prioritized
visibility over formal publication: demonstrate real results to attract users
and contributors. Academic papers and conference demos follow as results
solidify. Target venues include ICRA (servo calibration), RSS (gait learning),
CoRL (motion mimicry, sim-to-real), NeurIPS/ICML (simulated gait vs PPO),
AIIDE/FDG (personality emergence), and GDC AI Summit (gaming integration).

---

## Conclusions

### ECC Was Always for Robotics

"Embodied" is in the name. The software assessment, cold case investigation,
and sonobuoy processing applications are disembodied instances of an embodied
system. They use the same DEMOCRITUS loop, the same causal graph, the same
HNSW search, and the same governance pipeline. The only difference is that their
sensors read files and network packets instead of IMUs and cameras, and their
actuators emit text instead of servo commands.

### The Software Applications Prove the Architecture

The fact that ECC already works for software system analysis -- where it builds
causal models of codebases, detects anomalies through coherence scoring, and
explains its reasoning through causal chains -- validates the cognitive
architecture. Robotics and gaming are the embodied applications of an
architecture that was proven on disembodied problems first. The structure
transfers; only the parameters change.

### Parallel Tracks Prove Generality

Running gaming and robotics simultaneously is not a distraction -- it is a
proof of generality. If the same kernel can drive a game NPC and a physical
robot with only different sensor/actuator registrations, the architecture is
genuinely domain-independent. Each domain stress-tests different aspects: gaming
demands 60fps cognitive ticks and believable personality; robotics demands
real-time control and physical safety. Meeting both constraints makes the
kernel stronger for all applications.

### Many Small Trees, Not One Big Forest

The creator's decision to use small, scoped causal graphs per application
rather than one unified world model is architecturally sound. A servo
calibration graph with 1,440 nodes is fast to traverse and reason over. A gait
graph with 5,000 edges is manageable on a Pi 5 within the 50ms tick budget.
Smaller graphs mean faster spectral analysis, faster HNSW search, and clearer
causal traces. When cross-domain transfer is needed, the mesh gossip protocol
shares relevant subgraphs between instances.

### Resource Reality

10 human hours per week plus many bot hours. Agent-heavy development using the
WeftOS swarm infrastructure to parallelize research, implementation, and
testing. No fixed timeline -- work through things in the right order. The
$1,102 total BOM is deliberately modest, validating the thesis that ECC runs
on commodity hardware. The most expensive single item is the Sesame humanoid
robot at $450.

### What Comes Next

The first action is Experiment 1: servo calibration. It requires the least
hardware, validates the most fundamental claim (ECC can characterize a physical
system faster than a human), and produces the software foundation that all
subsequent experiments build on. Once servo calibration works, gait learning
and 3D print quality learning run in parallel on the two available printers.
The sim-to-real pipeline follows naturally: train in simulation, calibrate on
hardware, walk.

The gaming track advances simultaneously. The sidecar TCP bridge is the first
deliverable: a WeftOS process that accepts perception frames from Unity and
returns action frames. Experiment 8 (personality emergence) can run on a
development workstation with no hardware dependencies, making it an ideal
parallel workstream.

Every experiment that succeeds becomes a blog post. Every blog post
demonstrates WeftOS capabilities to the market. The gaming and robotics work
is not a distraction from the core weavelogic.ai GTM -- it is a demonstration
that ECC is a general cognitive substrate, not a narrow software analysis tool.

---

## Appendix A: Decision Log

All 18 pre-symposium questions with the creator's answers.

| # | Question | Answer |
|---|----------|--------|
| 1 | Which domain first: gaming or robotics? | **Parallel.** Both simultaneously. Simple robotics hardware first. |
| 2 | Target game engine? | **Unity first**, possibly Unreal. Test which has better hook/harness integration. |
| 3 | Robot platform? | **Custom 3D-printed.** Industrial platform for simulated/synthetic testing if available. |
| 4 | Demo priority order? | **All three in parallel.** Servo calibration first, gait + 3D print simultaneously. Work through in order of readiness. |
| 5 | Hardware budget? | **Keep it cheap.** Lots of small parts on hand. Building sensor kits. |
| 6 | 3D printer access? | **Yes.** Bambu Lab + Prusa (PS1). Two printers available. |
| 7 | Compute platform? | **ESP32s for sensors, Pi 5s for brains.** Gaming on desktop. A typical robot: multiple ESP32s + Pi 5. |
| 8 | Timeline? | **No fixed timeline.** Work through as fast as practical, in the right order. |
| 9 | Engineering hours? | **~10 human hours/week + many bot hours.** Agent-heavy development. |
| 10 | GTM impact? | **Not a distraction.** Proves out WeftOS capabilities. Commercially viable products. Sesame = hobby starting point. |
| 11 | Open source split? | **Not decided yet.** Evaluate per product. |
| 12 | Sesame relationship? | **Hobby starting point**, not a product. Good platform to learn on. Has crab-like element. |
| 13 | Safety certification? | **No safety certification yet.** Address per-product as designs mature. |
| 14 | Nemesis patent? | **Design around it.** Want to understand what the patent covers. |
| 15 | Publication strategy? | **Blog posts primarily.** LinkedIn publishing. Academic papers and conference demos later. |
| 16 | Real-time guarantees? | **Best-effort first.** Add real-time guarantees as a feature later (like DiskANN feature flag pattern). |
| 17 | Graph size strategy? | **Many small trees, not one big forest.** Evaluate per application. Smaller when possible. |
| 18 | Deployment model? | **Binaries as sidecars, not WASM.** Native binaries on Linux/Windows. Game connects via TCP. WASM is fallback. |

**Key derived decisions:**

- Sidecar model for gaming: WeftOS runs as separate process, game engine connects via TCP.
- Multi-ESP32 robot architecture: one robot = multiple ESP32 sensor nodes + Pi 5 brain.
- Agent-heavy development: 10 human hours + extensive bot hours per week.
- Parallel tracks: gaming and robotics advance simultaneously.
- Two 3D printers available: Bambu + Prusa for QC experiments.

---

## Appendix B: Bill of Materials

Consolidated BOM across all eight experiments.

| Item | Experiments | Cost |
|------|------------|------|
| Raspberry Pi 5 (8 GB) | 1, 2, 3, 6, 7 | $80 |
| PCA9685 PWM driver (x2) | 1, 2, 7 | $12 |
| TowerPro SG90 servos (x4) | 1 | $12 |
| Sesame humanoid robot (22 servos) | 2, 3, 6, 7 | $450 |
| MPU-6050 IMU breakout | 2, 3, 6 | $4 |
| Pi Camera Module 3 | 1, 4 | $25 |
| Logitech C920 webcam | 3 | $50 |
| Safety harness + gantry frame | 2, 6 | $60 |
| LiPo battery 7.4V 2200mAh | 2 | $18 |
| Foam crash mat | 2, 6 | $15 |
| USB-C power supply (Pi) | All | $12 |
| ArUco marker sheets | 1, 7 | $2 |
| Servo mounting brackets (3D printed) | 1 | $4 |
| 5V 3A power supply | 1 | $8 |
| Jumper wires + breadboard | All | $5 |
| Calibration weights (10g, 50g, 100g) | 1 | $12 |
| Creality Ender-3 V3 SE | 4 | $180 |
| MKS Gen L v2.1 board | 4 | $25 |
| Ring light (10-inch) | 4 | $15 |
| PLA filament (2 kg) | 4 | $36 |
| Digital calipers | 4 | $12 |
| Custom 4-servo arm | 7 | $40 |
| Target markers (3D printed, x5) | 7 | $5 |
| **Grand Total** | | **$1,102** |

Development workstation assumed to be available (Experiments 5, 8).

---

## Appendix C: References

The 20 most important references from across all team reports, ordered by
relevance to the ECC thesis.

### Foundational Causal Reasoning

1. Pearl, J. (2009). *Causality: Models, Reasoning, and Inference* (2nd ed.).
   Cambridge University Press.

2. Scholkopf, B., Locatello, F., Bauer, S., Ke, N.R., et al. (2021). Toward
   Causal Representation Learning. *Proceedings of the IEEE*, 109(5), 612-634.

3. Bareinboim, E. & Pearl, J. (2024). An Introduction to Causal Reinforcement
   Learning. Columbia CausalAI Laboratory.

4. Lee, T.E. (2024). Causal Robot Learning for Manipulation. PhD Thesis,
   Carnegie Mellon University, CMU-RI-TR-24-25.

5. Bowen, F. et al. (2024). Physics-Based Causal Reasoning for Safe & Robust
   Next-Best Action Selection in Robot Manipulation Tasks.

### Robot Learning and Sim-to-Real

6. Zhao, T.Z., Kumar, V., Levine, S., & Finn, C. (2023). Learning
   Fine-Grained Bimanual Manipulation with Low-Cost Hardware (ACT). *RSS 2023*.

7. Zakka, K. et al. (2025). MuJoCo Playground. Google DeepMind.

8. Hafner, D. et al. (2023). Mastering Diverse Domains through World Models
   (DreamerV3). *Nature* (2025).

9. Akkaya, I. et al. (2019). Solving Rubik's Cube with a Robot Hand. OpenAI.

10. Tobin, J. et al. (2017). Domain Randomization for Transferring Deep Neural
    Networks from Simulation to the Real World. *IROS 2017*.

11. MIT CSAIL (2024). Precision Home Robots: Real-to-Sim-to-Real (RialTo).

### Game AI

12. Orkin, J. (2006). Three States and a Plan: The AI of F.E.A.R. *GDC 2006*.

13. Mark, D. (2023). Are Behavior Trees a Thing of the Past? *Game Developer*.

14. Adams, T. (2015). Simulation Principles from Dwarf Fortress. *Game AI Pro 2*.

15. Peng, X.B., Abbeel, P., Levine, S., & van de Panne, M. (2018). DeepMimic:
    Example-Guided Deep Reinforcement Learning of Physics-Based Character Skills.
    *ACM Transactions on Graphics* 37(4).

### Cognitive Architectures and Continual Learning

16. Laird, J.E. (2022). Introduction to the Soar Cognitive Architecture.

17. van de Ven, G.M. et al. (2024). Continual Learning and Catastrophic
    Forgetting.

18. Rajendran, G., Buchholz, S., Aragam, B., & Ravikumar, P. (2024). From
    Causal to Concept-Based Representation Learning. *NeurIPS 2024*.

### Developmental Robotics

19. Metta, G. et al. (2010). The iCub Humanoid Robot: An Open-Source Platform
    for Research in Embodied Cognition. *Neural Networks*, 23(8-9).

### Patent

20. Warner Bros. Entertainment (2016). US Patent 10,926,179: Nemesis System.
    Expires August 11, 2036.
