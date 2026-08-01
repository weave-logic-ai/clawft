# Task: ruvllm-sona-optional (UPSTREAM / deferred)

**Mode:** UPSTREAM (not product BUILD)  
**rUv:** ruvllm SONA / MicroLoRA micro-loop under flywheel macro-loop (ADR-234)

WeftOS does **not** need weight-level adaptation for kernel maturity. Macro-loop
is enough: SEE→WIRE→BUILD→UPSTREAM + MH flywheel measure/promote.

If product later wants local adaptive serving:

1. Optional dep only (ADR-150 removable).  
2. Still clear flywheel gate + frozen anchors.  
3. Never mutate ECC/LeWM R1–R5 via MicroLoRA.

Agents: treat as **out of critical path**. Prefer GEPA (ADR-017) for prompt
evolution and ViewSpec flywheel for fusion policy.
