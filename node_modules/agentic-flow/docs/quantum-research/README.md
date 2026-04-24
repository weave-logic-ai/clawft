# Quantum Research Documentation

This directory contains research documentation, literature reviews, and implementation plans for quantum-related experiments and studies.

## Literature Reviews

- **[QUANTUM_RESEARCH_LITERATURE_REVIEW.md](./QUANTUM_RESEARCH_LITERATURE_REVIEW.md)** - Comprehensive literature review covering:
  - Quantum Measurement Theory (POVM formalism, Born rule, delayed-choice experiments)
  - Rust Quantum Computing Libraries (nalgebra, num-complex, RustQIP)
  - Experimental Design (photonic quantum eraser, SPDC, interferometers)
  - Best Practices (pre-registration, reproducibility, error analysis)
  - Implementation Considerations (scientific Rust projects, testing strategies)

## Active Projects

### Observer-Agnostic Measurement Experiment

Implementation plans and documentation for testing whether consciousness affects quantum measurement outcomes.

**Location:** `../../examples/Observer-Agnostic Measurement and No-Retrocausal Marginals Theorem/`

**Key Documents:**
- Master Implementation Plan
- GOAP Analysis (Goal-Oriented Action Planning)
- Executive Summary

**Timeline:** 28-40 weeks (6-10 months)
**Budget:** $280,000
**Expected Outcome:** Null result confirming observer-agnostic measurement

## Research Areas

### Quantum Foundations
- Observer-agnostic measurement theorem
- No-retrocausal marginals
- Wave-particle duality
- Quantum erasure experiments

### Computational Simulation
- Rust-based quantum simulators
- POVM (Positive Operator-Valued Measure) implementations
- Bell state simulations
- Delayed-choice quantum eraser modeling

### Experimental Physics
- Photonic quantum optics
- Type-II BBO crystal SPDC
- Single-photon detection
- Interferometer configurations

### Statistical Analysis
- Chi-squared hypothesis testing
- TOST equivalence testing
- Power analysis and sample sizing
- Falsification criteria

## Key Technologies

**Simulation Stack:**
- Rust 1.70+ (performance, safety, reproducibility)
- nalgebra 0.32 (complex matrix operations)
- num-complex 0.4 (complex number handling)
- statrs 0.16 (statistical tests)
- rayon 1.8 (parallelization)
- proptest 1.4 (property-based testing)

**Experimental Hardware:**
- 405nm pump laser
- Type-II BBO crystal (SPDC)
- Single-photon avalanche diodes (SPADs)
- Time-tagging modules (Swabian, PicoQuant)
- Quantum RNG (ID Quantique)

## Related Resources

- **Implementation Examples:** `../../examples/`
- **GOAP Analysis:** `../quantum-goap/`
- **Architecture:** `../architecture/`

## References

1. Nielsen & Chuang - *Quantum Computation and Quantum Information*
2. Kim et al. (2000) - "A Delayed-Choice Quantum Eraser"
3. Walborn et al. (2002) - "Double-slit quantum eraser"
4. Englert (1996) - "Fringe Visibility and Which-Way Information"

## Contributing

For quantum research contributions, please ensure:
- ✅ Pre-registration of experimental protocols
- ✅ Open data with DOI archiving
- ✅ Reproducible analysis code
- ✅ Clear falsification criteria
- ✅ Peer review compliance
