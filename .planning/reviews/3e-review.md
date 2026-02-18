# Expert Review: Phase 3E -- WASM Allocator & Size Optimization

## Reviewer: Performance Engineer (WASM optimization specialist)
## Date: 2026-02-17
## Verdict: APPROVE_WITH_CHANGES

---

## Scores

| Dimension | Score | Notes |
|-----------|-------|-------|
| 1. Completeness | 5/5 | Exceptional coverage: allocator selection, LTO, strip, wasm-opt, size budgets, CI gates, PGO, eh_frame, twiggy, alloc-tracing -- all present. No meaningful omission. |
| 2. Technical Accuracy | 4/5 | Allocator comparisons are well-researched. dlmalloc/talc/lol_alloc analysis is sound. wasm-opt flags correct. Minor issue with talc locking API and dlmalloc version pin. |
| 3. Benchmarking Plan | 4/5 | Methodology is rigorous: size buckets, fragmentation stress, serde workloads. Missing: latency benchmarks for allocation throughput (not just size), no mention of criterion or hyperfine for statistical rigor. |
| 4. Timeline | 3/5 | 3 weeks is generous for the MUST-HAVE scope but the plan tries to fit PGO, eh_frame, thin-LTO comparison, musl optimization, and memory pooling into the same window. PGO alone can consume a week if profiling workloads are non-trivial. Recommend hard triage. |
| 5. Dependencies | 4/5 | Good awareness of 3D and 3F. One conflict identified below (target mismatch with 3D's wasip2 migration). No blocking circular deps. |
| 6. Size Targets | 3/5 | <300 KB uncompressed is aggressive given serde_json alone can be 40-60 KB in WASM. Currently stubs only at ~142 KB rlib. A cdylib with clawft-types + serde + serde_json + allocator will likely land 200-350 KB pre-wasm-opt. Post wasm-opt <300 KB is plausible but tight. <120 KB gzipped is very aggressive -- gzip typically achieves 50-60% compression on WASM, so 300 KB -> ~150 KB gzipped. 120 KB requires the uncompressed to be ~240 KB. |
| 7. CI Integration | 4/5 | Size gate design is solid (wasm-size.sh + regression-check.sh + baseline.json). Missing: hysteresis/ratchet mechanism to prevent size creep over time (once you hit 280 KB, baseline resets and you lose the alarm). |
| 8. Memory Profiling | 3/5 | alloc_trace.rs design is practical for WASM. However, `clawft_alloc_stats()` returning a raw pointer with `todo!()` needs a concrete memory layout plan. WASM linear memory profiling via Wasmtime is mentioned but the script (`wasm-memory.sh`) is listed as NEW with no pseudocode. |

**Weighted Average: 3.8/5**

---

## Strengths

1. **Thorough allocator comparison framework.** The feature-gated allocator selection pattern in Section 2.1 is exactly the right approach. Using `#[cfg(all(target_arch = "wasm32", not(feature = "alloc-talc"), not(feature = "alloc-lol")))]` ensures mutual exclusivity at compile time. The decision matrix in Appendix A with weighted scoring is a disciplined methodology.

2. **wasm-opt integration is production-quality.** The `wasm-opt.sh` script (Section 2.3) handles both macOS and Linux `stat` syntax, validates output with `wasmtime compile`, and reports size reduction percentages. The flag set (`-Oz --strip-debug --strip-dwarf --strip-producers --zero-filled-memory --converge`) is exactly right for size optimization. The `--converge` flag is particularly good -- it runs optimization passes until no further reduction is possible.

3. **PGO pipeline is well-structured.** The 4-step approach (instrument -> profile -> merge -> rebuild) in Section 2.4 is correct. The workload scenarios in Appendix B (startup, help, config, message roundtrip, long conversation) cover the critical hot paths for a CLI tool with an agentic loop pattern.

4. **Non-functional requirements are specific and measurable.** NFR-01 through NFR-10 have concrete numerical targets with current baselines. The 20% build-time regression threshold for PGO (NFR-07) is pragmatic.

5. **Constraint discipline.** Section 1.3 correctly pins the Rust toolchain, WASM target, dependency budget, and unsafe policy. C-08 (feature flags for allocator selection) prevents accidental allocator conflicts.

6. **Risk assessment is realistic.** The risk of `wasm-opt` producing invalid modules (mitigated by wasmtime compile validation) and PGO workload representativeness are genuine concerns handled correctly.

7. **Detailed section-level native binary analysis.** Appendix C provides a section-by-section breakdown (`.text`, `.rodata`, `.eh_frame`, `.rela.dyn`, `.data.rel.ro`) with targeted reduction techniques for each. This demonstrates genuine binary analysis expertise.

---

## Issues Found

### Critical

1. **Target mismatch with Phase 3D (wasip2 migration).**

   - The 3E plan targets `wasm32-wasip1` exclusively (Constraint C-02: "WASM target remains wasm32-wasip1").
   - Phase 3D explicitly migrates the primary target to `wasm32-wasip2` and builds a bin target (main.rs) for the .wasm component.
   - Phase 3D also relaxes the WASM size budget from 300 KB to 350 KB (uncompressed) and from 120 KB to 140 KB (gzipped) to accommodate real HTTP/FS implementations.
   - **Impact**: If 3D executes before or concurrently with 3E, the allocator benchmarks, wasm-opt pipeline, and size budgets in 3E become invalid because the target and binary composition change fundamentally. If 3E executes first, its measurements become throwaway work once 3D lands.
   - **Recommendation**: Add explicit sequencing -- 3E MUST run after 3D's target migration is complete, OR 3E must benchmark both wasip1 and wasip2 targets. The size budgets must align with 3D's relaxed targets (350 KB / 140 KB) rather than the original 300 KB / 120 KB. Update Constraint C-02 to acknowledge wasip2 as the eventual primary target.

2. **`alloc_trace.rs` has a `todo!()` in an exported function.**

   - `clawft_alloc_stats()` (Section 2.2, line 198) has `todo!("serialize stats")` which will panic at runtime if called.
   - This is a `#[no_mangle] pub extern "C"` function that a WASM host can call. A panic in WASM traps the module.
   - **Recommendation**: Either (a) define the serialization format (e.g., write a fixed-layout struct to a static buffer and return its offset/length), or (b) mark the function as `SHOULD HAVE` and gate it behind the `alloc-tracing` feature so it never ships in production builds. The pseudocode should at minimum show the buffer layout.

### Major

3. **talc API may not compile as written.**

   - Section 2.1 shows:
     ```rust
     static ALLOCATOR: Talck<talc::locking::AssumeUnlockable, ClaimOnOom> =
         Talc::new(unsafe { ClaimOnOom::new(Span::empty()) }).lock();
     ```
   - The `talc` crate's API has changed across major versions. In `talc 4.x`, the locking wrapper API uses `Talck` as a type alias, but the constructor pattern and `AssumeUnlockable` may differ from what is shown. The 02-technical-requirements.md document (Section 8) shows a different talc initialization pattern.
   - **Recommendation**: Pin `talc` to an exact version (e.g., `talc = "=4.4"`) and verify the constructor compiles against the actual API before committing the pseudocode. Add a spike task in Week 1 Day 1: "Verify talc 4.x API compiles for wasm32-wasip1."

4. **Size targets may be unachievable with serde_json in the binary.**

   - The current binary is ~142 KB as an rlib (stubs only). The 3E plan acknowledges the cdylib size is "TBD."
   - `serde_json` alone contributes roughly 40-70 KB to a WASM cdylib (depending on features used and how much gets dead-code-eliminated). `clawft-types` with all its serde derive impls will add another 20-40 KB.
   - After wasm-opt, a realistic floor for `clawft-types + serde + serde_json + allocator` is approximately 150-250 KB.
   - The 300 KB target is achievable IF the pipeline and tool modules are not wired (stubs only). Once 3D adds real HTTP/FS implementations, the budget will be exceeded.
   - The 120 KB gzipped target requires the uncompressed binary to be roughly 200-240 KB (gzip achieves 50-60% compression on WASM binaries, not the 40% implied by 300->120).
   - **Recommendation**: (a) Measure the actual cdylib size NOW (before 3E begins) by building with `--crate-type cdylib`. (b) Adjust the gzipped target to 150 KB (more realistic with serde_json). (c) Document that these targets apply to the stub/scaffold binary and will need revision after 3D.

5. **No cdylib build verification in current workspace.**

   - The `clawft-wasm` Cargo.toml does not specify `crate-type = ["cdylib"]`. It is currently built as an rlib (library).
   - The 3E plan assumes a cdylib output (`clawft_wasm.wasm`) but the Cargo.toml shown in Section 2.6 only adds features -- it does not add the `[lib]` section with `crate-type`.
   - Without `crate-type = ["cdylib"]`, `cargo build --target wasm32-wasip1` produces an `.rlib`, not a `.wasm` binary that wasm-opt can process.
   - **Recommendation**: Add to the 3E plan (Week 1, Task 1): Update `clawft-wasm/Cargo.toml` to include:
     ```toml
     [lib]
     crate-type = ["cdylib", "rlib"]
     ```
     The `rlib` is kept for `cargo test` (which needs it). The `cdylib` is the WASM delivery artifact. Alternatively, the 3D plan adds a `[[bin]]` target (main.rs) which produces the .wasm -- clarify which approach 3E assumes.

6. **`release` profile shared between WASM and native with `opt-level = "z"`.**

   - The workspace `Cargo.toml` has `[profile.release]` with `opt-level = "z"`. This applies to ALL native builds, not just WASM.
   - `opt-level = "z"` for native binaries is unusual -- it sacrifices runtime performance for size. For a CLI tool, `opt-level = 2` or `opt-level = 3` is more typical unless binary size is the primary concern.
   - The plan adds `[profile.release-pgo]` inheriting from `release` (which uses `opt-level = "z"`). PGO with `opt-level = "z"` is an unusual combination -- PGO benefits are most visible at `opt-level = 2` or `3` where the optimizer has more latitude to inline hot functions.
   - **Recommendation**: Change the workspace release profile to `opt-level = 2` (or `3`) and keep `opt-level = "z"` only in `[profile.release-wasm]`. The PGO profile should inherit from a profile with `opt-level = 2` for maximum PGO benefit. This is a breaking change to the current profile setup and needs coordination with the team.

### Minor

7. **Missing `--no-default-features` in feature-exclusive allocator builds.**

   - Section 2.1 uses `#[cfg(all(target_arch = "wasm32", not(feature = "alloc-talc"), not(feature = "alloc-lol")))]` for the dlmalloc default. However, `cargo build --features alloc-talc` will enable BOTH the talc feature AND the default features (if `default = ["alloc-dlmalloc"]` or similar is ever added). The current `default = []` is correct, but the cfg guards should use a positive default feature rather than double-negation for clarity and robustness.
   - **Recommendation**: Consider a positive feature `alloc-dlmalloc` (enabled by default) instead of the double-negation pattern. This is a style issue, not a correctness bug, since `default = []` means dlmalloc is the no-feature fallback.

8. **`bc` command may not be available in CI.**

   - `wasm-opt.sh` (line 254) and `pgo-build.sh` (line 370) use `bc` for floating-point arithmetic. Alpine-based CI images and some minimal Docker images do not include `bc`.
   - **Recommendation**: Replace `bc` with `awk` (universally available) or shell arithmetic. Example: `PCT=$(awk "BEGIN {printf \"%.1f\", $REDUCTION * 100 / $PRE_SIZE}")`.

9. **Appendix B PGO workload uses `weft --stdin` which may not exist.**

   - The message roundtrip scenario (`echo '{"content":"hi"}' | weft --stdin`) references a `--stdin` flag. The CLI command listing (from the gap analysis) shows `weft agent -m "msg"` but no `--stdin` flag.
   - **Recommendation**: Verify the flag exists or update the workload to use `weft agent -m "hi"` (which is a valid command per the codebase).

10. **Shell scripts lack `shellcheck` validation.**

    - All new shell scripts should pass `shellcheck` (not just `bash -n` syntax check). The acceptance criteria mention `bash -n` but not `shellcheck`, which catches common issues like unquoted variables and POSIX portability.
    - **Recommendation**: Add `shellcheck` validation to the acceptance criteria alongside `bash -n`.

---

## Cross-Phase Conflicts

### 3D (WASI + Docker Multi-Arch)

- **Target Migration**: 3D migrates clawft-wasm from `wasm32-wasip1` to `wasm32-wasip2`. If 3E runs first, all size measurements are against wasip1 and become invalid when 3D changes the target. If 3D runs first, 3E needs to benchmark against wasip2 and the binary will include real HTTP/FS implementations, changing the allocator workload profile significantly.
- **Size Budget Conflict**: 3D relaxes the size budget to 350 KB / 140 KB. 3E targets 300 KB / 120 KB. These numbers must be reconciled.
- **Binary Type**: 3D adds a `[[bin]]` target (main.rs) producing a WASM component. 3E assumes a cdylib library target. The wasm-opt pipeline must target the correct artifact.
- **Allocator**: 3D states "dlmalloc allocator already configured -- UNCHANGED." 3E changes the allocator configuration significantly. These must be sequenced: 3E allocator work first, then 3D inherits the chosen allocator.
- **Recommendation**: Execute 3E's allocator comparison and wasm-opt pipeline work BEFORE 3D's real HTTP/FS implementation. After 3D completes, re-run 3E's size benchmarks against the wasip2 binary with real implementations. Update the 3E plan to include a "re-benchmark after 3D" task.

### 3F (RVF Integration)

- **WASM Size Budget**: 3F plans to add rvf-wasm (<8 KB), micro-hnsw-wasm (11.8 KB), ruvector-temporal-tensor FFI (<10 KB), and sona WASM subset (~30 KB) to clawft-wasm. This adds roughly 50-60 KB to the WASM binary. Combined with 3D's HTTP/FS additions, the total could easily exceed 300 KB.
- **Allocator Selection Impact**: 3F's vector operations (HNSW, quantization) have very different allocation patterns than serde_json deserialization. The allocator chosen in 3E based on serde workloads may not be optimal for 3F's vector workloads. lol_alloc (leak-only) would be catastrophic for 3F's long-running vector operations.
- **Recommendation**: (a) The allocator decision matrix should include a "vector operations" workload alongside serde workloads. (b) Document that lol_alloc is unsuitable for 3F's use case and should only be considered for ephemeral single-invocation patterns. (c) The 300 KB WASM target may need to become 400+ KB after 3F -- this should be explicitly acknowledged.

### 02-Technical-Requirements.md

- The technical requirements document (Section 8) specifies `talc` as the WASM allocator, not `dlmalloc`:
  ```rust
  #[cfg(target_arch = "wasm32")]
  #[global_allocator]
  static ALLOCATOR: talc::Talck<...> = ...;
  ```
  The 3E plan correctly keeps dlmalloc as the current default and evaluates talc as an alternative, but the tech requirements document already pre-decided on talc. This discrepancy should be acknowledged -- either the tech requirements should be updated to reflect the evaluation approach, or the 3E plan should note this prior decision and explain why re-evaluation is warranted.

---

## Missing Requirements

1. **No `wasm-strip` step.** The plan uses wasm-opt's `--strip-debug --strip-dwarf --strip-producers` flags, which is good, but does not mention `wasm-strip` from the WABT toolkit as a separate validation step. wasm-opt's stripping is usually sufficient, but `wasm-strip` can catch additional custom sections that wasm-opt misses. Low priority but worth mentioning.

2. **No Brotli compression measurement.** The plan measures gzip compression but modern WASM deployment (especially via HTTP) uses Brotli (`br`) which typically achieves 15-25% better compression than gzip on WASM binaries. If the delivery mechanism supports it, Brotli should be the primary compression metric alongside gzip.

3. **No `wasm-tools validate` step.** The plan validates with `wasmtime compile` but does not mention `wasm-tools validate` which checks WASM specification conformance. 3D mentions `wasm-tools validate` in its acceptance criteria -- 3E should include it too.

4. **No allocator benchmarking for deallocation patterns.** The fragmentation stress test (Section 4.1) allocates and drops in reverse order, which is good. However, it does not measure the RSS/linear-memory high-water mark after the test. For WASM, the linear memory can only grow, never shrink. The benchmark should measure peak pages used, not just whether the allocations succeed.

5. **No consideration of `wee_alloc` as a fourth allocator option.** While `wee_alloc` is now in maintenance mode, it is still widely used in production WASM and has a smaller code footprint than dlmalloc. Its inclusion would make the comparison more comprehensive. However, this is optional -- the three allocators chosen cover the important tradeoff space.

6. **Missing WASM binary diff tracking.** The plan tracks absolute sizes in `baseline.json` but does not include a mechanism for understanding WHY size changed between builds. Adding `twiggy diff` between the baseline and current binary to the CI pipeline would make size regression investigation much faster.

---

## Recommendations

### High Priority (address before implementation)

1. **Reconcile size budgets with Phase 3D.** Adopt 3D's relaxed targets (350 KB uncompressed / 150 KB gzipped) as the 3E targets, since 3D adds real HTTP/FS implementations that are non-negotiable. The 300 KB / 120 KB targets should be documented as "stretch goals for the stub-only binary."

2. **Measure actual cdylib baseline immediately.** Before any optimization work begins, build the current clawft-wasm as a cdylib (`crate-type = ["cdylib", "rlib"]`) and measure the uncompressed and gzipped sizes. All optimization work should be measured against this baseline, not the rlib size. This is a 15-minute task that anchors all subsequent work.

3. **Fix the release profile `opt-level` mismatch.** Move `opt-level = "z"` to `[profile.release-wasm]` only. Use `opt-level = 2` for the native release profile. PGO results will be significantly better with `opt-level = 2`. This is a workspace-wide change that needs agreement from the team.

4. **Sequence 3E relative to 3D explicitly.** Either: (a) Run 3E first with wasip1 stubs, document results, then re-benchmark after 3D. Or (b) Wait for 3D to complete and run 3E against the wasip2 binary with real implementations. Option (a) is preferable because 3E's allocator comparison and wasm-opt pipeline are useful infrastructure regardless of target.

5. **Remove the `todo!()` from `clawft_alloc_stats()`.** Either implement the serialization (a packed struct written to a static `[u8; 256]` buffer with known offsets) or remove the export entirely and defer to the `alloc-tracing` feature acceptance criteria.

### Medium Priority (address during implementation)

6. **Add `twiggy diff` to the CI pipeline.** Store the previous build's WASM binary as a CI artifact and run `twiggy diff old.wasm new.wasm` to show which functions grew or shrank. This makes size regression triage fast.

7. **Add Brotli compression measurement alongside gzip.** `brotli -9 < clawft.wasm | wc -c` in the size check script. Report both metrics.

8. **Include a vector-operations workload in the allocator comparison.** Add a test that allocates `Vec<f32>` of varying sizes (128, 384, 1024 elements), performs cosine similarity computations, and deallocates. This simulates 3F's workload and ensures the chosen allocator does not regress for vector use cases.

9. **Replace `bc` with `awk` in shell scripts.** Eliminates a dependency that may not be present in CI images.

10. **Add `shellcheck` to acceptance criteria.** `shellcheck scripts/**/*.sh` alongside `bash -n`.

### Low Priority (nice-to-have)

11. **Consider `wasm-snip` for removing panic formatting code.** If the binary uses `panic = "abort"`, the panic formatting infrastructure (`core::fmt`) may still be linked. `wasm-snip` can replace unreachable panic handlers with `unreachable` instructions, saving 10-20 KB in some cases.

12. **Document the allocator selection as an ADR.** The allocator comparison results should be captured as an Architecture Decision Record (not just a report) so future contributors understand why a particular allocator was chosen and under what conditions the decision should be revisited.

---

## Timeline Assessment

The 3-week timeline is **realistic for the MUST-HAVE scope** but **overloaded with SHOULD-HAVE and NICE-TO-HAVE items**.

### Week 1 (Allocator + wasm-opt): Achievable
- Adding feature-gated allocator selection: 2 days
- Creating wasm-opt.sh and wasm-twiggy.sh: 1 day
- Creating alloc-compare.sh and running benchmarks: 1 day
- Writing allocator tests: 1 day
- **Risk**: talc API compatibility may require a half-day spike

### Week 2 (WASM optimization + Native PGO): Overloaded
- Integrating wasm-opt into CI: 1 day
- Updating baseline.json and regression-check.sh: 1 day
- PGO pipeline: 2-3 days (finding representative workloads is the hard part)
- Thin LTO vs fat LTO comparison: 0.5 day
- eh_frame experiment: 0.5 day
- .cargo/config.toml: 0.5 day
- **Total estimated: 5.5-6.5 days. Risk of slipping into Week 3.**

### Week 3 (Integration + Benchmark Hardening): Tight
- Full benchmark suite run: 1 day
- Update baseline.json: 0.5 day
- alloc_trace.rs implementation: 1-2 days
- Documentation: 1 day
- Reviewer code review: 1 day
- **If Week 2 slips, Week 3 becomes a crunch.**

**Recommendation**: Move PGO (FR-06), thin-LTO comparison (FR-07), and eh_frame reduction (FR-08) to SHOULD-HAVE with explicit scope cut if Week 2 is running behind. The WASM optimization work (allocator + wasm-opt + CI integration) is the critical path and should not be jeopardized by native binary optimization experiments.

The `alloc_trace.rs` module (FR-10) and memory pool pattern (FR-11) should be explicitly marked as Week 3 stretch goals, not Week 3 commitments.

**Revised timeline recommendation**:
- Week 1: Allocator comparison + wasm-opt (unchanged)
- Week 2: WASM CI integration + size gate hardening + baseline update (drop PGO to stretch)
- Week 3: PGO (if time), documentation, code review, benchmark validation

---

## Summary

Phase 3E is a well-structured optimization plan with strong technical foundations. The allocator comparison framework, wasm-opt integration, and CI size gates are all production-quality designs. The primary concerns are:

1. **Cross-phase coordination with 3D** (target migration and size budget reconciliation)
2. **Aggressive size targets** that need empirical validation with a cdylib baseline
3. **Timeline overload** with native PGO work competing against the WASM-critical path
4. **Minor technical issues** (talc API verification, release profile opt-level, todo!() in exported function)

None of these are plan-breaking. With the changes recommended above -- particularly reconciling size targets with 3D and measuring the cdylib baseline upfront -- this plan is ready for implementation.
