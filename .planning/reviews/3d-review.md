# Expert Review: Phase 3D -- WASI HTTP/FS + Docker Multi-Arch

## Reviewer: Systems Architect (WASM/Docker specialist)
## Date: 2026-02-17
## Verdict: APPROVE_WITH_CHANGES

---

## Scores

| Dimension | Score | Notes |
|-----------|-------|-------|
| Completeness | 4/5 | Covers WASI HTTP, FS, Docker multi-arch, and WAMR fallback well. Missing: streaming HTTP responses, TLS certificate handling for WASI HTTP, and WASM Component Model WIT declaration details. |
| Feasibility | 3/5 | The `std::fs` insight for WASI FS is correct and solid. However, the `wasi-http-client` dependency is risky -- see critical issues. The wasip2 target for bin crates has known rough edges. |
| Timeline | 3/5 | 3 weeks is tight but possible IF the HTTP crate evaluation goes smoothly. The 12-18 day LOE estimate is honest, but does not account for debugging wasip2 toolchain issues which could easily consume 2-3 extra days. |
| Dependencies | 4/5 | Inter-phase dependencies correctly identified. Minor conflict with 3E on size budgets (see below). No conflicts with 3F. |
| Test Coverage | 4/5 | Good FS integration tests (9 tests, well-designed). HTTP tests are necessarily limited on native. Missing: error-path tests, timeout tests, large-body tests, concurrent FS access tests. |
| Risk Assessment | 3/5 | Correctly identifies wasi-http-client compatibility risk and binary size risk. Misses several risks: Component Model adapter tooling, wasmtime CLI flag stability, and the `bin` vs `cdylib` preopens issue is mentioned but underweight. |
| Code Quality | 4/5 | Pseudocode is clean and idiomatically Rust. Minor issues with `Box<dyn Error>` vs custom error types, and the `resolve()` path method has a security concern. |
| Size Budget | 2/5 | The 350KB/140KB targets are likely unrealistic with wasi-http-client + serde_json. See critical issue below. |

---

## Strengths

- **Correct `std::fs` insight**: The plan correctly identifies that on `wasm32-wasip2`, Rust's `std::fs` transparently uses WASI filesystem. This means the FS implementation needs zero conditional compilation, which is elegant and reduces maintenance burden. This is the single best decision in the plan.

- **Two-strategy HTTP approach**: Having both Strategy A (wasi-http-client, high-level) and Strategy B (raw wasi crate, low-level) with a documented decision tree is excellent engineering practice. The plan to start with A and fall back to B if size exceeds budget is pragmatic.

- **Docker multi-arch design is solid**: Using per-arch build matrix with digest-based manifest merge is the correct modern approach. Avoids QEMU emulation for ARM builds since native runners are already in the release matrix (the aarch64 build already uses `cross`).

- **WAMR fallback strategy is realistic**: Acknowledging that WAMR cannot run wasip2 components and providing a wasip1 fallback for FS-only operations is the right trade-off. Not trying to force full compatibility.

- **Feature flag design**: Using `wasi-http-client` and `wasi-raw` as mutually exclusive optional features keeps the binary small and the dependency tree explicit.

- **Wasmtime validation script**: The E2E validation script (`test-wasm-wasmtime.sh`) is well-structured with proper error handling and temp directory cleanup.

- **Preserving existing test suite**: Explicit requirement to keep all 41 existing tests passing throughout is good discipline.

- **Appendix C feature matrix**: The feature/capability matrix across wasip2, wasip1, and native is a valuable reference that will prevent confusion during implementation.

---

## Issues Found

### Critical (must fix before implementation)

**C1: Size budget is almost certainly unrealistic with wasi-http-client**

The plan estimates wasi-http-client at 30-60 KB with deps at 20-40 KB, totaling 135-185 KB pre-wasm-opt. This is optimistic. The `wasi-http-client` crate pulls in:
- `wasi` crate (the full wasip2 bindings, not just HTTP): this alone is 50-100 KB of generated Component Model glue code
- `url` crate: ~30-40 KB (URL parsing, IDNA encoding)
- Possibly `serde_urlencoded` and other utilities

Combined with `serde_json` (~40 KB) and `clawft-types` (~25 KB), the realistic pre-opt size is **250-400 KB**. After `wasm-opt -Oz` with aggressive dead code elimination, you might get 30-40% reduction, landing at **175-280 KB**. The 350 KB uncompressed budget might be met, but the 140 KB gzipped target is the real constraint -- gzip typically achieves 50-60% compression on WASM, which means 350 KB uncompressed would gzip to ~140-210 KB.

**Recommendation**:
1. Run the actual build with `wasi-http-client` BEFORE committing to Week 15. Add a "Week 14.5" spike task: `cargo add wasi-http-client`, build, measure.
2. Raise the gzipped budget to 180 KB for this phase. The 140 KB target was set for stubs, not for real HTTP.
3. If Strategy A blows the budget, switch to Strategy B (raw `wasi` crate) immediately. The raw approach avoids the `url` crate entirely.
4. Update 3E size targets to be consistent: 3E says <= 300 KB / 120 KB gzipped (for stubs + optimization), but 3D relaxes to 350 KB / 140 KB (for real HTTP). 3E needs to know the 3D post-state.

**C2: `wasi-http-client` crate maturity and Rust 1.93 compatibility is unverified**

The plan acknowledges this as a "Low likelihood" risk. Based on the WASI ecosystem state, this should be **Medium-High likelihood**. The `wasi-http-client` crate is maintained by the Bytecode Alliance but:
- It targets `wasi:http@0.2.x` which is stable, but the Rust bindings are generated by `wit-bindgen` which has rapid churn
- The crate's dependency on `wasi` (the bindings crate) must be compatible with Rust 1.93's Component Model support
- There is no evidence the plan author has actually verified that `wasi-http-client 0.2.x` compiles against `wasm32-wasip2` on Rust 1.93.1

**Recommendation**: Make Task 15.1 (evaluate wasi-http-client) a strict go/no-go gate. If it does not compile cleanly within 1 day, immediately pivot to Strategy B. Do not spend multiple days debugging compatibility.

**C3: The `bin` target for wasip2 has implications not fully addressed**

The plan correctly notes that `cdylib` does not get preopens automatically and proposes a `bin` target. However:
- A `bin` target produces a WASM Component with a `wasi:cli/command` world export (specifically `wasi:cli/run@0.2.0`). This is fundamentally different from a library/reactor component.
- The plan's `main.rs` reads from stdin and writes to stdout -- this is a "one-shot command" model, not the "exported function" model described in the use case (UC-3D-001 step 3: "Component receives user message via exported function").
- If the goal is host instantiation with function calls (as described in the use case), you need a **reactor component**, which requires either `cdylib` with explicit WIT exports, or the newer `wasi:cli/reactor` approach.

**Recommendation**: Clarify the component model architecture. Either:
(a) Accept the command-line one-shot model (stdin/stdout) for this phase and defer the reactor/exported-function model to a later phase, OR
(b) Use `cargo-component` or `wit-bindgen` to produce a proper reactor component with exported functions. This adds significant complexity but matches the UC-3D-001 flow.

Given the timeline, option (a) is the pragmatic choice, but it must be documented as a limitation.

### Major (should fix)

**M1: WasiFileSystem `resolve()` method has a path traversal vulnerability**

```rust
fn resolve(&self, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()  // BUG: allows escaping base_path!
    } else {
        self.base_path.join(path)
    }
}
```

If a caller passes an absolute path like `/etc/passwd`, the `resolve()` method will return it unchanged, bypassing the `base_path` sandbox. While WASI preopens provide host-level sandboxing, the Rust-level API should not give a false sense of security.

The `self.base_path.join(path)` path is also vulnerable to `../` traversal: `base_path.join("../../etc/passwd")` resolves outside the base.

**Recommendation**: Replace with a sandboxing resolver:
```rust
fn resolve(&self, path: &Path) -> PathBuf {
    let resolved = self.base_path.join(path);
    // Canonicalize to eliminate .. components
    // On WASI, use the resolved path directly (canonicalize may not work)
    // but at minimum strip .. components
    resolved
}
```
Or better yet, reject any path containing `..` or starting with `/` when a base_path is set. The WASI host provides the real sandbox, but the Rust API should enforce its own contract.

**M2: Error type should not be `Box<dyn Error>` in WasiHttpClient**

The HTTP client returns `Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>>`. This is fine for prototyping but:
- It makes error matching impossible for callers
- It does not implement `Display` consistently for user-facing messages
- The existing `clawft-types::ClawftError` or a new `WasmError` enum should be used

**Recommendation**: Define a `WasmHttpError` enum (or reuse an existing error type) with variants for: NetworkUnavailable, Timeout, InvalidUrl, ResponseError(u16), DeserializationError, and WasiHostError(String). This also makes the native stub path cleaner.

**M3: Missing `tempfile` dev-dependency for FS tests**

The FS test pseudocode uses `tempfile::tempdir()` but `tempfile` is not listed as a dev-dependency in the Cargo.toml changes. This is a minor omission but will cause compilation failure of the test suite.

**Recommendation**: Add `[dev-dependencies] tempfile = "3"` to the Cargo.toml changes section (3.2).

**M4: Docker Dockerfile uses `TARGETARCH` which maps to `amd64`/`arm64`, but artifacts use `x86_64`/`aarch64` naming**

The Dockerfile expects `weft-linux-${TARGETARCH}` which resolves to `weft-linux-amd64` and `weft-linux-arm64`. But the release workflow produces `weft-linux-x86_64` and `weft-linux-aarch64`.

The plan mentions this mismatch (Task 17.1 note about renaming) but does not specify which solution to use. This is a guaranteed build failure if not addressed.

**Recommendation**: Add a mapping step in the Docker workflow:
```yaml
- name: Map architecture
  run: |
    case ${{ matrix.arch }} in
      amd64) echo "ARTIFACT=weft-linux-x86_64" >> $GITHUB_ENV ;;
      arm64) echo "ARTIFACT=weft-linux-aarch64" >> $GITHUB_ENV ;;
    esac
```
Then `COPY docker-build/${ARTIFACT} /usr/local/bin/weft` in a templated Dockerfile, or use `--build-arg`.

**M5: Wasmtime validation script uses `--wasi http` flag which is not guaranteed stable**

The `wasmtime run --wasi http` flag syntax has changed between wasmtime versions. In older versions it was `--wasi-modules=experimental-wasi-http`, in current versions it might be `--wasi http` or `S=wasi:http/proxy`. The plan should pin a specific wasmtime version and document the exact flag syntax.

**Recommendation**: Pin wasmtime version in CI (e.g., `WASMTIME_VERSION=28.0.0`), install that specific version, and test the exact flag syntax. Add a comment in the validation script noting which wasmtime versions are known-good.

**M6: No discussion of TLS/HTTPS handling in WASI HTTP**

The use case (FR-3D-001, FR-3D-002) requires HTTPS connections to `api.openai.com` and `api.anthropic.com`. In WASI preview2, TLS is handled by the host runtime (wasmtime), not by the guest. However:
- The guest must trust that the host properly verifies TLS certificates
- Some wasmtime configurations may not enable TLS by default
- The `--wasi http` flag in wasmtime may have TLS-specific sub-options

**Recommendation**: Add a note in section 1.3 (Constraints) about TLS delegation to host, and add a test case that specifically validates an HTTPS request (not just HTTP) in the wasmtime validation.

### Minor (nice to have)

**m1: `WasiFileSystem::home_dir()` returns `Some(base_path)` which is semantically incorrect**

The current stub returns `None`, which is correct -- WASI has no home directory concept. The plan changes it to return `Some(base_path)`, which conflates "home directory" with "root of preopened directory". If a caller uses `home_dir()` to construct paths like `~/.clawft/config.json`, they will get `/config/.clawft/config.json` (assuming base_path is `/config`), which is wrong.

**Recommendation**: Keep returning `None` and let callers handle the WASM case explicitly. Or rename to `base_dir()` / `root_dir()` to avoid the semantic confusion.

**m2: Missing `with_timeout()` tests in HTTP test plan**

The test `wasi_http_client_timeout_configurable` accesses `client.default_timeout` directly, but the struct field is not public in the pseudocode. Either make it public (breaking encapsulation) or add a getter method.

**m3: `process_message()` in main.rs is called but never defined with the right signature**

The `main.rs` pseudocode calls `clawft_wasm::process_message(input.trim())` but the current `process_message()` in `lib.rs` takes `&str` and returns `String`. The plan should verify the config is actually passed to the processing function -- currently the config is loaded but never wired to anything.

**m4: The `.cargo/config.toml` changes should be additive, not replacement**

The plan shows a full `.cargo/config.toml` that adds `wasm32-wasip2` section and aliases. The existing file already has `wasm32-wasip1` config. Ensure the plan is explicit about adding to (not replacing) the existing file.

**m5: Week 17 Task 17.2 WAMR validation -- `cargo build --target wasm32-wasip1 --release` will conflict with `--profile release-wasm`**

The existing CI and 3E plan use `--profile release-wasm` for WASM builds. The WAMR validation task uses `--release` instead. This inconsistency means different optimization settings. Should use `--profile release-wasm` consistently.

---

## Cross-Phase Conflicts

### 3D vs 3E (Optimization)

**Size budget conflict**: 3E targets <= 300 KB uncompressed / 120 KB gzipped for the WASM binary. 3D relaxes this to 350 KB / 140 KB to accommodate real HTTP. If 3D executes first (Weeks 15-17) and 3E executes in parallel or after, 3E needs to know that:
1. The baseline binary will be significantly larger (real HTTP + serde)
2. The 300 KB target in 3E's NFR-01/02 must be updated to reflect post-3D reality
3. The allocator comparison in 3E should use the 3D binary (with real HTTP), not the stub binary

**`wasm32-wasip1` vs `wasm32-wasip2` target conflict**: 3E's constraint C-02 says "WASM target remains `wasm32-wasip1`". But 3D changes the primary target to `wasm32-wasip2`. If 3E scripts and CI reference `wasm32-wasip1`, they will be operating on the wrong binary after 3D completes.

**`.cargo/config.toml` conflict**: Both 3D and 3E modify `.cargo/config.toml`. 3D adds `wasm32-wasip2` section and aliases. 3E adds `wasm32-wasip1` runner. These are compatible but must be merged carefully.

**Recommendation**:
- 3E must be re-baselined AFTER 3D completes
- Update 3E constraint C-02 to acknowledge wasip2 as primary target
- Update 3E size targets to 350 KB / 140 KB (or whatever 3D achieves)
- Ensure all 3E scripts accept target as parameter (not hardcoded wasip1)

### 3D vs 3F (RVF Integration)

**No direct conflict**. 3F does not modify clawft-wasm's HTTP or FS implementations. 3F adds `rvf-wasm` and `micro-hnsw-wasm` as optional dependencies, which will further increase WASM binary size. However, 3F's WASM size budget (section 6) already accounts for this at ~250 KB total, which fits within 3D's 350 KB budget.

**Minor dependency concern**: 3F plans to use `wasi-http-client` (noted as "~50 KB" in 3F section 6 WASM Size Budget). If 3D's evaluation determines that raw `wasi` crate is better than `wasi-http-client`, 3F's size estimates need updating.

### 3D vs 3G, 3H (no conflicts identified)

### 3D vs 3I (Gap Analysis)

3I identifies GAP-28: "WASM stubs (HTTP, FS) not functional. Deferred per Phase 3 exit criteria." 3D directly resolves GAP-28 and GAP-34 (wasip2 target validation). 3I also notes GAP-36: "Multi-arch Docker images (buildx). Deferred from 3B." 3D resolves this as well.

---

## Missing Requirements

### From 02-technical-requirements.md

1. **`wasm32-wasip2` target with `--import-memory` linker flag**: The tech spec (section 8) specifies `rustflags = ["-C", "link-arg=--import-memory"]` for the wasip2 target. The 3D plan does not include this flag in its `.cargo/config.toml`. This flag is important for hosts that want to control WASM linear memory allocation. May need investigation whether wasip2 components need this or if it is a wasip1-era concern.

2. **`talc` allocator recommended in tech spec**: Section 8 of the tech spec recommends `talc` as the WASM allocator (< 1 KB overhead) instead of `dlmalloc` (8 KB). The 3D plan keeps `dlmalloc` as default. While 3E addresses allocator comparison, the tech spec's recommendation should be acknowledged in 3D.

3. **`waki` crate as alternative**: The tech spec's research sources do not mention `waki` as an alternative WASI HTTP client. The `waki` crate is a lighter-weight alternative to `wasi-http-client` that provides both client and server. It should be evaluated alongside Strategy A and B.

### From gap-analysis (3I)

1. **GAP-28 resolution verification**: The gap analysis lists WASM stubs as a P2 item with 8h LOE. The 3D plan allocates 12-18 days. The LOE difference suggests the gap analysis underestimated the scope, OR that 3D includes more than just stub replacement (Docker, WAMR, CI -- which it does). No action needed, just noting the discrepancy is explained.

2. **Streaming HTTP not addressed**: 3I identifies GAP-11 (SSE streaming not implemented) as P0. The 3D plan does not address streaming HTTP responses in WASM. The `WasiHttpClient` API is synchronous (returns full body). For real LLM usage, streaming would dramatically improve UX. This should at minimum be listed as a "Remaining for future phases" item (it is partially noted at line 1022: "Streaming HTTP responses in WASM").

### From tech spec not in 3D

1. **`wasi:cli/reactor` world**: For WASM components that should be long-lived (hosting an agent that processes multiple messages), the reactor pattern is more appropriate than the command pattern used in `main.rs`. This is an architectural decision that should be explicitly deferred with rationale.

2. **`Config` struct compatibility**: The `main.rs` pseudocode deserializes a `clawft_types::Config`, but it is unclear whether the full `Config` struct (with all channel, tool, gateway fields) is appropriate for the WASM binary. A `WasmConfig` subset might be more appropriate and would reduce serde code bloat.

---

## Recommendations

### Immediate (before implementation begins)

1. **Spike: build wasi-http-client against wasm32-wasip2 on Rust 1.93** -- This should take 2-4 hours and will immediately answer whether Strategy A is viable. Do this before any other work.

2. **Spike: also evaluate `waki` crate** -- It may be lighter than `wasi-http-client` with better size characteristics.

3. **Fix path traversal in `resolve()`** -- Either reject `..` components and absolute paths, or document that WASI host preopens provide the real sandbox.

4. **Add `tempfile` as dev-dependency** to the Cargo.toml plan.

### During implementation

5. **Define `WasmHttpError` enum** instead of `Box<dyn Error>`. This will pay dividends in error handling quality.

6. **Pin wasmtime version in CI** to a specific release that is known to support `--wasi http` with the expected flag syntax.

7. **Address Dockerfile architecture mapping** explicitly in the workflow YAML.

8. **Keep `home_dir()` returning `None`** for WASI semantic correctness.

### After implementation

9. **Re-baseline 3E size targets** to reflect 3D's post-state.

10. **Document the command-model limitation** (stdin/stdout one-shot) and plan for reactor-model in a future phase.

11. **Consider `WasmConfig` subset** to reduce serde code pulled into the WASM binary.

---

## Timeline Assessment

The 3-week / 12-18 day estimate is **optimistic but achievable** with the following caveats:

**Week 15 (HTTP implementation) is the highest-risk week.** The wasi-http-client evaluation could go sideways. If it does not compile against Rust 1.93 + wasm32-wasip2, switching to raw `wasi` crate bindings requires implementing the full HTTP request/response flow manually (constructing `OutgoingRequest`, reading `IncomingResponse` body stream, handling headers). This is ~200-300 lines of non-trivial code that interacts with generated Component Model bindings. Budget 4-5 days instead of 3-5.

**Week 16 (FS + binary production) is low-risk.** The `std::fs` approach is straightforward. Binary production with a `bin` target is standard. Budget 3-4 days as planned.

**Week 17 (Docker + WAMR + CI) is medium-risk.** Docker multi-arch is well-documented, but the release.yml refactoring is non-trivial (matrix + digest + merge pattern). WAMR validation may reveal unexpected issues with the wasip1 fallback build if http.rs conditional compilation is not clean. Budget 4-5 days.

**Suggested adjustments:**
- Add a 0.5-day spike at the start of Week 15 for the wasi-http-client build test
- Move WAMR validation (Task 17.2) to a "nice-to-have" if time runs short -- it is listed as SHOULD HAVE in exit criteria, not MUST HAVE
- If Week 15 takes 5 days (HTTP is hard), steal 1 day from Week 17 WAMR/polish

**Overall: 3 weeks is achievable if the HTTP crate evaluation goes well. Budget a 1-week buffer if it does not.**

---

## Summary

Phase 3D is a well-structured plan that correctly identifies the key technical challenges (WASI HTTP, Component Model, Docker multi-arch) and provides pragmatic fallback strategies. The main risks are around binary size budget (likely too aggressive) and the wasi-http-client crate compatibility (unverified). The FS implementation approach using `std::fs` is elegant and correct. The Docker multi-arch design follows best practices.

With the changes recommended above -- particularly the size budget adjustment, the path traversal fix, the pre-implementation spike, and the 3E re-baseline -- this plan is ready for implementation.
