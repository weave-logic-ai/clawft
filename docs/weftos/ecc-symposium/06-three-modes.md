# The Three Modes: Act, Analyze, Generate

**The same engine operates in three modes that compose into a continuous loop.**

---

## The Insight

A conversation engine that can run a live conversation can also analyze an existing one — and can also generate new ones toward a goal. These aren't three different systems. They're three operating modes of the same engine, sharing the same data structures, the same cognitive tick, the same witness chain, and the same scoring system.

---

## Mode 1: Act (Real-Time Conversation)

The original ClawStage use case. Actors (human and AI) produce utterances in real-time. The engines process each contribution within the cognitive tick:

- DCTE creates nodes, advances the wavefront, merges branches
- DSTE updates beliefs and goals based on what was said
- RSTE classifies discourse relations, scores coherence
- EMOT tracks affect, modulates behavior
- SCEN tracks dramatic arc, injects goal seeds

The tree of possibilities collapses as speculative branches are pruned, merged, or committed. The ExoChain records every committed moment with full provenance.

**Input**: Live utterances from participants
**Output**: The conversation itself (committed MainLine + full causal history)

---

## Mode 2: Analyze (Post-Hoc Understanding)

Given an existing corpus — a transcript, a PR history, a research paper, a sprint's commits, planning documentation — run the engines in read-only mode:

- DCTE reconstructs the contribution tree (who said what, in response to what)
- DSTE infers participant goals and beliefs from their contributions (what were they trying to achieve? what did they assume?)
- RSTE maps discourse coherence (does the argument hold together? where are the gaps? which questions were left unanswered?)
- EMOT tracks affect trajectory (where did frustration peak? when did engagement drop?)
- SCEN identifies lifecycle position (is this project in rising action or falling action? did the sprint reach resolution?)
- Scoring quantifies quality across all dimensions

**Input**: Existing corpus modeled as a conversation
**Output**: Structural analysis — goal completion status, coherence scores, causal decision graph, affect trajectory, lifecycle position, quality metrics, gap identification

---

## Mode 3: Generate (Goal-Directed Conversation)

Set a goal (DSTE), spawn expert agent-processes, and let them have a conversation that produces the desired output. The conversation IS the development process:

1. **Goal setting**: Define DSTE objectives — "produce a comprehensive architecture plan for system X"
2. **Agent spawning**: Spawn expert actor-processes (architect, security reviewer, performance engineer, domain expert) as WeftOS kernel processes
3. **Guided conversation**: Agents converse toward the goal. The SCEN engine structures the discussion into phases (requirements gathering, option analysis, trade-off evaluation, decision, documentation). The floor manager ensures each expert contributes. The RSTE enforces coherence.
4. **Tree collapse**: Speculative branches (alternative approaches considered) are evaluated and pruned. The winning approach advances past the wavefront into committed history.
5. **Output extraction**: The committed MainLine IS the planning documentation. Not a summary of a process — the process itself, with full causal provenance.

**Input**: Goal definition + expert agent configurations
**Output**: The goal artifact (planning docs, research report, code design) + full causal history of how it was produced

---

## Composition: The Continuous Loop

The three modes compose because they share the same ExoChain witness chain:

```
GENERATE (expert agents produce a plan)
    |
    | ExoChain links generation to analysis
    v
ANALYZE (evaluate the plan for coherence, completeness, gaps)
    |
    | ExoChain links analysis to execution
    v
ACT (human participants join, execute the plan, adapt in real-time)
    |
    | ExoChain links execution back to generation
    v
GENERATE (refine the plan based on execution feedback)
    |
    ...continuous loop
```

Each transition is a causal edge in the CMVG. The analysis of the generated plan is provably linked to the generation. The execution is provably linked to the analysis. When execution reveals that the plan was wrong, the causal graph shows exactly which generation-time decision led to the problem and what evidence was available at the time.

---

## Training Material Falls Out Naturally

Every scored witness entry in the conversation IS a training sample:

| Conversation Artifact | Training Artifact | Automatically Produced |
|----------------------|-------------------|----------------------|
| Committed MainLine | The output document/plan/code | Yes — it's the wavefront history |
| Scored witness entries | Training samples for SONA | Yes — every operation has a score delta |
| Causal edges (decisions → outcomes) | Decision matrix | Yes — the CausalGraph IS the matrix |
| DSTE goal tree (ACHIEVED/FAILED/ABANDONED) | Results metrics | Yes — goal lifecycle is tracked |
| RSTE coherence scores | Quality metrics | Yes — per-section, per-contribution |
| Speculative branches (explored but pruned) | Alternative analysis | Yes — the crown (above wavefront) preserves what was considered |
| CrossRefs (cross-engine links) | Traceability matrix | Yes — typed links between all artifacts |

You don't create these artifacts separately. They ARE the conversation's structural metadata. The witness chain ensures they're tamper-evident. The Merkle proofs ensure they're verifiable. The Universal Node IDs ensure they're addressable.

---

## Example: Generate a Project Plan

**Goal**: "Produce an architecture plan for a real-time bidding system"

**Spawned agents** (WeftOS kernel processes):
- `architect` — system design expertise, high dominance
- `security-reviewer` — threat modeling, flags risks
- `performance-eng` — latency requirements, capacity planning
- `domain-expert` — ad-tech domain knowledge
- `devil-advocate` — challenges assumptions, raises edge cases

**Conversation flow** (SCEN-structured):

**Act 1: Requirements** (setup)
- `domain-expert` states business requirements (DCTE utterances)
- `architect` asks clarifying questions (RSTE: QAP relations)
- DSTE tracks: requirements → goals, assumptions → beliefs

**Act 2: Design Options** (rising action)
- `architect` proposes 3 approaches (DCTE: 3 speculative branches)
- `security-reviewer` evaluates each for threat surface (RSTE: Evaluation relations)
- `performance-eng` evaluates each for latency (RSTE: Evaluation relations)
- `devil-advocate` challenges weakest assumptions (RSTE: Contrast/Correction)
- DSTE tracks: evidence for/against each option → belief confidence updates

**Act 3: Decision** (climax)
- Floor manager grants `architect` the floor for final proposal
- Merge engine: SemanticDedup merges the evaluated options into a single recommendation
- Wavefront advances: the chosen approach commits to MainLine
- Pruned branches (rejected approaches) remain in the crown as alternatives-considered

**Act 4: Documentation** (falling action)
- `architect` produces the architecture document (committed MainLine segments)
- `security-reviewer` annotates with threat mitigations (CrossRefs to design decisions)
- `performance-eng` annotates with capacity numbers (CrossRefs to requirements)

**Act 5: Review** (resolution)
- Mode switches to ANALYZE: run coherence scoring on the produced document
- RSTE identifies: 2 unanswered questions, 1 section with weak Evidence relation
- Mode switches back to ACT: agents address the gaps
- Final commit to MainLine

**Outputs**:
1. The architecture document (committed MainLine)
2. Causal decision graph (why each choice was made, with evidence links)
3. Alternatives-considered (pruned branches with rejection reasoning)
4. Threat analysis (security CrossRefs)
5. Capacity analysis (performance CrossRefs)
6. Quality score (RSTE coherence + DSTE goal completion + scoring trajectory)
7. Training data (scored trajectories for every agent decision, ready for SONA LoRA)
