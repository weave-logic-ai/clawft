# SPARC Implementation Plan: Phase 3B - Polish + CI/CD

**Stream**: 3B - Polish + CI/CD (Week 11-14)
**Timeline**: 4 weeks (parallel with 3A)
**Deliverables**: Multi-platform binaries, automated release pipeline, performance benchmarks

---

## 1. Agent Instructions

### Documentation References
- **Primary**: `.planning/01-business-requirements.md` (Success criteria, binary targets)
- **Primary**: `.planning/02-technical-requirements.md` (Section 7: Binary size targets)
- **Primary**: `.planning/03-development-guide.md` (Phase 3 checklist)
- **Supporting**: `.planning/05-ruvector-crates.md` (Size profiling techniques)

### Repository Structure
```
.github/
  workflows/
    ci.yml              # Modified - add build matrix
    release.yml         # New - release automation
    wasm-build.yml      # New - WASM validation
    benchmarks.yml      # New - performance benchmarks
scripts/
  build/
    cross-compile.sh    # New - cross-compilation wrapper
    size-check.sh       # New - binary size validation
    docker-build.sh     # New - minimal container image
  bench/
    startup-time.sh     # New - startup benchmark
    memory-usage.sh     # New - memory benchmark
    throughput.sh       # New - message throughput
docs/
  deployment/
    docker.md           # New - container deployment
    release.md          # New - release process
  benchmarks/
    results.md          # New - benchmark results
```

### Branch Strategy
- **Feature Branch**: `feature/phase-3b-cicd`
- **PR Target**: `develop`
- **Sub-branches** (optional):
  - `feature/binary-profiling` (Week 11)
  - `feature/ci-matrix` (Week 12)
  - `feature/release-pipeline` (Week 13)
  - `feature/benchmarks` (Week 14)

### Agent Coordination
- **DevOps Agent**: Configure GitHub Actions, cross-compilation
- **Performance Agent**: Binary profiling, benchmarks
- **Tester Agent**: CI validation, size assertions
- **Documentation Agent**: Deployment guides, release notes

---

## 2. Specification

### 2.1 Week 11: Binary Profiling

#### Requirements
1. Profile binary size with `twiggy` tool
2. Identify top 20 contributors to binary size
3. Generate size breakdown report
4. Document optimization opportunities
5. Profile both native (`weft`) and WASM (`clawft.wasm`) binaries

#### Acceptance Criteria
- `twiggy top -n 20` report generated for release binary
- Size breakdown by crate/module documented
- Optimization opportunities documented with estimated savings
- Report includes comparison: debug vs release vs release-lto
- WASM binary profiled separately

#### Tools
- `twiggy` (binary size profiler)
- `cargo bloat` (alternative size analysis)
- `wasm-opt` (WASM optimization)

### 2.2 Week 12: CI Build Matrix

#### Requirements
1. GitHub Actions: build matrix for 6 targets
   - `x86_64-unknown-linux-musl` (Linux x86_64)
   - `aarch64-unknown-linux-musl` (Linux ARM64)
   - `x86_64-apple-darwin` (macOS Intel)
   - `aarch64-apple-darwin` (macOS ARM64)
   - `x86_64-pc-windows-msvc` (Windows x86_64)
   - `wasm32-wasip2` (WASM)
2. Cross-compile using `cross` tool for Linux targets
3. Native builds for macOS and Windows (GitHub-hosted runners)
4. WASM build with size assertions (fail if > 300 KB)
5. Upload binaries as artifacts for each target

#### Acceptance Criteria
- All 6 targets build successfully in CI
- Binaries uploaded as GitHub Actions artifacts
- WASM size assertion enforces < 300 KB limit
- Native Linux binary < 15 MB (static musl)
- Build matrix completes in < 30 minutes

#### GitHub Actions Matrix
```yaml
strategy:
  matrix:
    include:
      - target: x86_64-unknown-linux-musl
        os: ubuntu-latest
        use_cross: true
      - target: aarch64-unknown-linux-musl
        os: ubuntu-latest
        use_cross: true
      - target: x86_64-apple-darwin
        os: macos-13  # Intel runner
        use_cross: false
      - target: aarch64-apple-darwin
        os: macos-14  # M1 runner
        use_cross: false
      - target: x86_64-pc-windows-msvc
        os: windows-latest
        use_cross: false
      - target: wasm32-wasip2
        os: ubuntu-latest
        use_cross: false
```

### 2.3 Week 13: Release Pipeline

#### Requirements
1. Release workflow triggered on git tags (`v*`)
2. Build all 6 targets in parallel
3. Generate changelog from git commits
4. Create GitHub Release with binaries
5. Build minimal Docker image: `FROM scratch` with static binary
6. Push Docker image to GitHub Container Registry (ghcr.io)
7. Generate release notes with benchmark results

#### Acceptance Criteria
- Release triggered automatically on tag push
- All binaries attached to GitHub Release
- Changelog generated with conventional commits
- Docker image size < 20 MB (scratch + static binary)
- Docker image published to `ghcr.io/user/clawft:v0.1.0`
- Release notes include binary sizes and benchmark summary

#### Release Artifacts
- `weft-x86_64-linux-musl` (Linux x86_64)
- `weft-aarch64-linux-musl` (Linux ARM64)
- `weft-x86_64-macos` (macOS Intel)
- `weft-aarch64-macos` (macOS ARM64)
- `weft-x86_64-windows.exe` (Windows)
- `clawft.wasm` (WASM component)
- `clawft.wasm.gz` (compressed WASM)
- `checksums.txt` (SHA256 checksums)

### 2.4 Week 14: Benchmarks

#### Requirements
1. Benchmark: Startup time (Rust vs Python)
   - Measure `weft --version` vs `nanobot --version`
   - Cold start from disk
2. Benchmark: Memory usage (RSS)
   - Measure idle agent memory footprint
   - Compare Rust vs Python
3. Benchmark: Message throughput
   - Measure messages/second for simple echo task
   - Compare Rust vs Python
4. Benchmark: Cold start to first response
   - Measure end-to-end latency from CLI invocation to first LLM response
   - Compare Rust vs Python

#### Acceptance Criteria
- Startup time: Rust < 50ms, Python > 500ms (10x improvement)
- Memory RSS: Rust < 10 MB, Python > 50 MB (5x improvement)
- Message throughput: Rust > 1000 msg/s, Python < 200 msg/s (5x improvement)
- Cold start: Rust < 2s, Python > 5s (2.5x improvement)
- Benchmark results documented in `docs/benchmarks/results.md`
- CI runs benchmarks on every release

#### Benchmark Matrix
| Metric | Rust (weft) | Python (nanobot) | Improvement |
|--------|-------------|------------------|-------------|
| Startup time | < 50ms | > 500ms | 10x faster |
| Memory RSS (idle) | < 10 MB | > 50 MB | 5x less |
| Message throughput | > 1000/s | < 200/s | 5x faster |
| Cold start to response | < 2s | > 5s | 2.5x faster |
| Binary size | < 15 MB | N/A (50 MB venv) | 3.3x smaller |
| WASM size | < 300 KB | N/A | Portable |

---

## 3. Pseudocode

### 3.1 Binary Size Profiling (Week 11)

```bash
#!/bin/bash
# scripts/build/size-check.sh

set -e

TARGET=$1  # e.g., x86_64-unknown-linux-musl
BINARY=$2  # e.g., target/x86_64-unknown-linux-musl/release/weft

echo "=== Binary Size Analysis for $TARGET ==="

# 1. Get binary size
SIZE_BYTES=$(stat -c %s "$BINARY" 2>/dev/null || stat -f %z "$BINARY")
SIZE_MB=$(echo "scale=2; $SIZE_BYTES / 1048576" | bc)
echo "Binary size: $SIZE_MB MB ($SIZE_BYTES bytes)"

# 2. Check against target (15 MB for native, 300 KB for WASM)
if [[ "$TARGET" == "wasm32-wasip2" ]]; then
    MAX_SIZE=307200  # 300 KB
    if [ "$SIZE_BYTES" -gt "$MAX_SIZE" ]; then
        echo "ERROR: WASM binary exceeds 300 KB limit"
        exit 1
    fi
else
    MAX_SIZE=15728640  # 15 MB
    if [ "$SIZE_BYTES" -gt "$MAX_SIZE" ]; then
        echo "ERROR: Native binary exceeds 15 MB limit"
        exit 1
    fi
fi

# 3. Profile with twiggy (if available)
if command -v twiggy &> /dev/null; then
    echo ""
    echo "=== Top 20 Size Contributors ==="
    twiggy top -n 20 "$BINARY"
fi

# 4. Profile with cargo bloat (if wasm-opt not applied)
if [[ "$TARGET" != "wasm32-wasip2" ]]; then
    echo ""
    echo "=== Cargo Bloat Analysis ==="
    cargo bloat --release --target "$TARGET" -n 20
fi

echo ""
echo "✓ Binary size validation passed"
```

### 3.2 CI Build Matrix (Week 12)

```yaml
# .github/workflows/ci.yml

name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  build:
    name: Build (${{ matrix.target }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
            use_cross: true
          - target: aarch64-unknown-linux-musl
            os: ubuntu-latest
            use_cross: true
          - target: x86_64-apple-darwin
            os: macos-13
            use_cross: false
          - target: aarch64-apple-darwin
            os: macos-14
            use_cross: false
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            use_cross: false
          - target: wasm32-wasip2
            os: ubuntu-latest
            use_cross: false

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross
        if: matrix.use_cross
        run: cargo install cross --git https://github.com/cross-rs/cross

      - name: Build
        run: |
          if [ "${{ matrix.use_cross }}" = "true" ]; then
            cross build --release --target ${{ matrix.target }}
          else
            cargo build --release --target ${{ matrix.target }}
          fi

      - name: Validate binary size
        run: |
          bash scripts/build/size-check.sh ${{ matrix.target }} \
            target/${{ matrix.target }}/release/weft${{ matrix.os == 'windows-latest' && '.exe' || '' }}

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: weft-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/weft${{ matrix.os == 'windows-latest' && '.exe' || '' }}
```

### 3.3 Release Pipeline (Week 13)

```yaml
# .github/workflows/release.yml

name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  create-release:
    name: Create Release
    runs-on: ubuntu-latest
    outputs:
      upload_url: ${{ steps.create_release.outputs.upload_url }}
    steps:
      - uses: actions/checkout@v4

      - name: Generate changelog
        id: changelog
        run: |
          # Generate changelog from git commits since last tag
          PREV_TAG=$(git describe --tags --abbrev=0 HEAD^ 2>/dev/null || echo "")
          if [ -z "$PREV_TAG" ]; then
            CHANGELOG=$(git log --pretty=format:"- %s" HEAD)
          else
            CHANGELOG=$(git log --pretty=format:"- %s" $PREV_TAG..HEAD)
          fi
          echo "changelog<<EOF" >> $GITHUB_OUTPUT
          echo "$CHANGELOG" >> $GITHUB_OUTPUT
          echo "EOF" >> $GITHUB_OUTPUT

      - name: Create release
        id: create_release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: ${{ github.ref }}
          release_name: Release ${{ github.ref_name }}
          body: |
            ## Changes
            ${{ steps.changelog.outputs.changelog }}

            ## Binaries
            - Linux x86_64 (musl static)
            - Linux ARM64 (musl static)
            - macOS Intel
            - macOS ARM64
            - Windows x86_64
            - WASM (wasm32-wasip2)

            ## Docker
            ```bash
            docker pull ghcr.io/${{ github.repository }}:${{ github.ref_name }}
            ```
          draft: false
          prerelease: false

  build-and-upload:
    name: Build and Upload (${{ matrix.target }})
    needs: create-release
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
            use_cross: true
            asset_name: weft-x86_64-linux-musl
          - target: aarch64-unknown-linux-musl
            os: ubuntu-latest
            use_cross: true
            asset_name: weft-aarch64-linux-musl
          - target: x86_64-apple-darwin
            os: macos-13
            use_cross: false
            asset_name: weft-x86_64-macos
          - target: aarch64-apple-darwin
            os: macos-14
            use_cross: false
            asset_name: weft-aarch64-macos
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            use_cross: false
            asset_name: weft-x86_64-windows.exe
          - target: wasm32-wasip2
            os: ubuntu-latest
            use_cross: false
            asset_name: clawft.wasm

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross
        if: matrix.use_cross
        run: cargo install cross --git https://github.com/cross-rs/cross

      - name: Build
        run: |
          if [ "${{ matrix.use_cross }}" = "true" ]; then
            cross build --release --target ${{ matrix.target }}
          else
            cargo build --release --target ${{ matrix.target }}
          fi

      - name: Prepare asset
        run: |
          if [ "${{ matrix.target }}" = "wasm32-wasip2" ]; then
            # Optimize WASM
            wasm-opt -Oz -o ${{ matrix.asset_name }} \
              target/${{ matrix.target }}/release/clawft-wasm.wasm
            gzip -9 -c ${{ matrix.asset_name }} > ${{ matrix.asset_name }}.gz
          else
            # Copy and strip binary
            cp target/${{ matrix.target }}/release/weft \
              ${{ matrix.asset_name }}
            strip ${{ matrix.asset_name }} || true
          fi

      - name: Upload release asset
        uses: actions/upload-release-asset@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          upload_url: ${{ needs.create-release.outputs.upload_url }}
          asset_path: ./${{ matrix.asset_name }}
          asset_name: ${{ matrix.asset_name }}
          asset_content_type: application/octet-stream

  docker:
    name: Build Docker Image
    needs: create-release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build static binary
        run: |
          cross build --release --target x86_64-unknown-linux-musl

      - name: Build Docker image
        run: |
          docker build -f Dockerfile.scratch -t clawft:latest .

      - name: Push to GHCR
        run: |
          echo "${{ secrets.GITHUB_TOKEN }}" | \
            docker login ghcr.io -u ${{ github.actor }} --password-stdin
          docker tag clawft:latest \
            ghcr.io/${{ github.repository }}:${{ github.ref_name }}
          docker push ghcr.io/${{ github.repository }}:${{ github.ref_name }}
```

### 3.4 Performance Benchmarks (Week 14)

```bash
#!/bin/bash
# scripts/bench/startup-time.sh

set -e

RUST_BIN=${1:-target/release/weft}
PYTHON_BIN=${2:-nanobot}
ITERATIONS=100

echo "=== Startup Time Benchmark ==="
echo "Rust binary: $RUST_BIN"
echo "Python binary: $PYTHON_BIN"
echo "Iterations: $ITERATIONS"
echo ""

# Benchmark Rust
rust_times=()
for i in $(seq 1 $ITERATIONS); do
    start=$(date +%s%N)
    $RUST_BIN --version > /dev/null
    end=$(date +%s%N)
    elapsed=$((($end - $start) / 1000000))  # Convert to ms
    rust_times+=($elapsed)
done

rust_avg=$(printf '%s\n' "${rust_times[@]}" | awk '{s+=$1}END{print s/NR}')
echo "Rust average: ${rust_avg}ms"

# Benchmark Python
python_times=()
for i in $(seq 1 $ITERATIONS); do
    start=$(date +%s%N)
    $PYTHON_BIN --version > /dev/null
    end=$(date +%s%N)
    elapsed=$((($end - $start) / 1000000))
    python_times+=($elapsed)
done

python_avg=$(printf '%s\n' "${python_times[@]}" | awk '{s+=$1}END{print s/NR}')
echo "Python average: ${python_avg}ms"

# Calculate improvement
improvement=$(echo "scale=1; $python_avg / $rust_avg" | bc)
echo ""
echo "Improvement: ${improvement}x faster"

# Validate against targets
if (( $(echo "$rust_avg < 50" | bc -l) )); then
    echo "✓ Rust startup < 50ms target met"
else
    echo "✗ Rust startup exceeds 50ms target"
    exit 1
fi

if (( $(echo "$improvement >= 10" | bc -l) )); then
    echo "✓ 10x improvement target met"
else
    echo "✗ 10x improvement target not met"
    exit 1
fi
```

---

## 4. Architecture

### 4.1 CI/CD Pipeline Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Developer Push                        │
└───────────────┬─────────────────────────────────────────┘
                │
                ├─── Push to branch ────► CI Workflow
                │                          ├─ Build matrix (6 targets)
                │                          ├─ Size validation
                │                          ├─ Unit tests
                │                          └─ Upload artifacts
                │
                └─── Push tag (v*) ──────► Release Workflow
                                            ├─ Build all targets
                                            ├─ Generate changelog
                                            ├─ Create GitHub Release
                                            ├─ Upload binaries
                                            ├─ Build Docker image
                                            └─ Push to GHCR
```

### 4.2 Cross-Compilation Strategy

| Target | Method | Runner | Notes |
|--------|--------|--------|-------|
| x86_64-linux-musl | cross | ubuntu-latest | Static binary |
| aarch64-linux-musl | cross | ubuntu-latest | Static binary |
| x86_64-macos | native | macos-13 | Intel runner |
| aarch64-macos | native | macos-14 | M1 runner |
| x86_64-windows | native | windows-latest | MSVC toolchain |
| wasm32-wasip2 | cargo | ubuntu-latest | WASM component |

### 4.3 Docker Image Layers

```dockerfile
# Dockerfile.scratch

FROM scratch
COPY target/x86_64-unknown-linux-musl/release/weft /weft
ENTRYPOINT ["/weft"]
CMD ["gateway"]
```

**Size breakdown**:
- Base image (`scratch`): 0 bytes
- Static binary (`weft`): ~15 MB
- **Total**: ~15 MB

### 4.4 Binary Size Tracking

```
┌─────────────────────────────────────────────────────────┐
│              Binary Size Budget (Native)                 │
├─────────────────────────────────────────────────────────┤
│ clawft-core (agent, tools, LLM)         ~4 MB           │
│ clawft-cli (terminal UI, commands)      ~2 MB           │
│ tokio runtime                            ~3 MB           │
│ reqwest (HTTP client)                    ~2 MB           │
│ serde, serde_json                        ~1 MB           │
│ other dependencies                       ~3 MB           │
├─────────────────────────────────────────────────────────┤
│ Total (release)                          ~15 MB          │
│ Budget                                   < 15 MB  ✓      │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│              Binary Size Budget (WASM)                   │
├─────────────────────────────────────────────────────────┤
│ clawft-core (agent, tools)               ~100 KB        │
│ sona (WASM subset)                       ~30 KB         │
│ rvf-types                                ~30 KB         │
│ WASI HTTP client                         ~50 KB         │
│ micro-hnsw-wasm                          ~12 KB         │
│ ruvector-temporal-tensor                 ~10 KB         │
│ cognitum-gate-kernel                     ~10 KB         │
├─────────────────────────────────────────────────────────┤
│ Total (release-wasm)                     ~242 KB        │
│ Budget                                   < 300 KB  ✓     │
│ Gzipped                                  < 120 KB  ✓     │
└─────────────────────────────────────────────────────────┘
```

### 4.5 Benchmark Test Suite

```
benchmarks/
├── startup_time.rs         # Cold start benchmark
├── memory_usage.rs         # RSS footprint benchmark
├── message_throughput.rs   # Messages/sec benchmark
├── cold_start_e2e.rs       # CLI to LLM response benchmark
└── baseline/
    ├── rust_results.json   # Rust baseline
    └── python_results.json # Python baseline
```

---

## 5. Refinement (TDD Test Plan)

### 5.1 Week 11: Binary Profiling Tests

#### Test Files
- `scripts/build/test_size_check.sh`

#### Test Cases

```bash
#!/bin/bash
# scripts/build/test_size_check.sh

# Test 1: Valid binary under limit
test_valid_binary() {
    # Create dummy binary (10 MB)
    dd if=/dev/zero of=/tmp/test_binary bs=1M count=10

    # Run size check
    bash scripts/build/size-check.sh x86_64-unknown-linux-musl /tmp/test_binary

    # Expect success
    assertEquals 0 $?
}

# Test 2: Binary exceeds limit
test_oversized_binary() {
    # Create dummy binary (20 MB)
    dd if=/dev/zero of=/tmp/test_binary bs=1M count=20

    # Run size check
    bash scripts/build/size-check.sh x86_64-unknown-linux-musl /tmp/test_binary

    # Expect failure
    assertNotEquals 0 $?
}

# Test 3: WASM binary validation
test_wasm_binary() {
    # Create dummy WASM (250 KB)
    dd if=/dev/zero of=/tmp/test.wasm bs=1K count=250

    # Run size check
    bash scripts/build/size-check.sh wasm32-wasip2 /tmp/test.wasm

    # Expect success
    assertEquals 0 $?
}

# Test 4: Oversized WASM
test_oversized_wasm() {
    # Create dummy WASM (350 KB)
    dd if=/dev/zero of=/tmp/test.wasm bs=1K count=350

    # Run size check
    bash scripts/build/size-check.sh wasm32-wasip2 /tmp/test.wasm

    # Expect failure
    assertNotEquals 0 $?
}
```

### 5.2 Week 12: CI Build Matrix Tests

#### Test Files
- `.github/workflows/ci.yml` (integration test via GitHub Actions)

#### Test Cases

```yaml
# Test matrix runs automatically on push
# Validate:
# 1. All 6 targets build successfully
# 2. Binaries uploaded as artifacts
# 3. Size checks pass for all targets
# 4. No cross-compilation failures
# 5. WASM build includes optimization

# Manual validation:
on:
  workflow_dispatch:
    inputs:
      target:
        description: 'Target to test'
        required: true
        type: choice
        options:
          - x86_64-unknown-linux-musl
          - aarch64-unknown-linux-musl
          - x86_64-apple-darwin
          - aarch64-apple-darwin
          - x86_64-pc-windows-msvc
          - wasm32-wasip2
```

### 5.3 Week 13: Release Pipeline Tests

#### Test Files
- `scripts/release/test_release.sh`

#### Test Cases

```bash
#!/bin/bash
# scripts/release/test_release.sh

# Test 1: Changelog generation
test_changelog_generation() {
    # Create test git history
    git init /tmp/test_repo
    cd /tmp/test_repo
    git commit --allow-empty -m "feat: add feature A"
    git commit --allow-empty -m "fix: fix bug B"
    git tag v0.1.0
    git commit --allow-empty -m "feat: add feature C"

    # Generate changelog
    CHANGELOG=$(git log --pretty=format:"- %s" v0.1.0..HEAD)

    # Validate contains "add feature C"
    assertContains "$CHANGELOG" "add feature C"
    assertNotContains "$CHANGELOG" "add feature A"
}

# Test 2: Docker image size
test_docker_image_size() {
    # Build Docker image
    docker build -f Dockerfile.scratch -t test-clawft .

    # Get image size
    SIZE=$(docker image inspect test-clawft --format='{{.Size}}')
    SIZE_MB=$((SIZE / 1048576))

    # Validate < 20 MB
    assertTrue "[ $SIZE_MB -lt 20 ]"
}

# Test 3: Asset naming
test_asset_naming() {
    # Build asset for linux
    ASSET_NAME="weft-x86_64-linux-musl"

    # Validate naming convention
    assertMatches "$ASSET_NAME" "^weft-[a-z0-9_]+-[a-z0-9_]+-[a-z]+$"
}
```

### 5.4 Week 14: Benchmark Tests

#### Test Files
- `scripts/bench/test_benchmarks.sh`

#### Test Cases

```bash
#!/bin/bash
# scripts/bench/test_benchmarks.sh

# Test 1: Startup time benchmark
test_startup_time_benchmark() {
    # Run benchmark script
    bash scripts/bench/startup-time.sh \
        target/release/weft \
        nanobot

    # Expect exit code 0 (targets met)
    assertEquals 0 $?
}

# Test 2: Memory usage benchmark
test_memory_usage() {
    # Start Rust agent
    target/release/weft gateway &
    RUST_PID=$!
    sleep 2

    # Measure RSS
    RUST_RSS=$(ps -o rss= -p $RUST_PID)
    RUST_MB=$((RUST_RSS / 1024))

    # Validate < 10 MB
    assertTrue "[ $RUST_MB -lt 10 ]"

    # Cleanup
    kill $RUST_PID
}

# Test 3: Message throughput
test_message_throughput() {
    # Run throughput benchmark
    bash scripts/bench/throughput.sh \
        target/release/weft \
        1000  # 1000 messages

    # Parse result
    RESULT=$(cat benchmark_results.txt | grep "Messages/sec" | awk '{print $2}')

    # Validate > 1000 msg/s
    assertTrue "[ $RESULT -gt 1000 ]"
}
```

### 5.5 Test Coverage Targets

| Component | Target Coverage |
|-----------|-----------------|
| scripts/build/*.sh | > 80% |
| .github/workflows/*.yml | Manual validation |
| Docker build | Manual validation |
| Benchmarks | > 90% |

---

## 6. Completion (Phase 3B Milestone Checklist)

### 6.1 Code Deliverables

- [ ] `scripts/build/size-check.sh` - Binary size validation script
- [ ] `scripts/build/cross-compile.sh` - Cross-compilation wrapper
- [ ] `scripts/build/docker-build.sh` - Docker image build script
- [ ] `scripts/bench/startup-time.sh` - Startup time benchmark
- [ ] `scripts/bench/memory-usage.sh` - Memory usage benchmark
- [ ] `scripts/bench/throughput.sh` - Message throughput benchmark
- [ ] `.github/workflows/ci.yml` - CI build matrix
- [ ] `.github/workflows/release.yml` - Release automation
- [ ] `.github/workflows/wasm-build.yml` - WASM validation
- [ ] `.github/workflows/benchmarks.yml` - Benchmark automation
- [ ] `Dockerfile.scratch` - Minimal Docker image

### 6.2 Build Artifacts

- [ ] Linux x86_64 binary (`weft-x86_64-linux-musl`)
- [ ] Linux ARM64 binary (`weft-aarch64-linux-musl`)
- [ ] macOS Intel binary (`weft-x86_64-macos`)
- [ ] macOS ARM64 binary (`weft-aarch64-macos`)
- [ ] Windows x86_64 binary (`weft-x86_64-windows.exe`)
- [ ] WASM component (`clawft.wasm`)
- [ ] Compressed WASM (`clawft.wasm.gz`)
- [ ] Docker image (`ghcr.io/user/clawft:v0.1.0`)
- [ ] SHA256 checksums (`checksums.txt`)

### 6.3 CI/CD Pipeline

- [ ] CI workflow builds all 6 targets on every push
- [ ] Size validation enforces < 15 MB for native, < 300 KB for WASM
- [ ] Release workflow triggers on git tags
- [ ] Changelog generated automatically from commits
- [ ] Binaries uploaded to GitHub Releases
- [ ] Docker image pushed to GHCR
- [ ] Benchmark workflow runs on releases

### 6.4 Performance Benchmarks

- [ ] Startup time: Rust < 50ms ✓
- [ ] Startup time: 10x faster than Python ✓
- [ ] Memory RSS: Rust < 10 MB ✓
- [ ] Memory: 5x less than Python ✓
- [ ] Message throughput: Rust > 1000 msg/s ✓
- [ ] Throughput: 5x faster than Python ✓
- [ ] Cold start: Rust < 2s ✓
- [ ] Cold start: 2.5x faster than Python ✓

### 6.5 Documentation

- [ ] `docs/deployment/docker.md` - Container deployment guide
- [ ] `docs/deployment/release.md` - Release process documentation
- [ ] `docs/benchmarks/results.md` - Benchmark results
- [ ] Binary size profiling report (`twiggy` output)
- [ ] Cross-compilation guide
- [ ] CI/CD architecture diagram

### 6.6 Testing

- [ ] Size check script tests pass
- [ ] CI build matrix validates all targets
- [ ] Release pipeline tested with pre-release tags
- [ ] Docker image size < 20 MB validated
- [ ] All benchmark tests pass
- [ ] Benchmark targets met (10x, 5x, 2.5x improvements)

### 6.7 Integration Points

- [ ] Phase 3A WASM binary integrated into CI
- [ ] WASM size assertions enforce < 300 KB
- [ ] Release pipeline publishes WASM alongside native binaries
- [ ] Docker image uses static binary from Phase 2
- [ ] Benchmarks compare against Python nanobot baseline

### 6.8 Edge Cases Handled

- [ ] Build failure notifications via GitHub Actions
- [ ] Oversized binary fails CI with clear error message
- [ ] Missing changelog handled gracefully
- [ ] Docker build failure does not block release
- [ ] Benchmark failures reported but do not fail CI
- [ ] Cross-compilation errors provide platform-specific guidance

### 6.9 Phase 3B Exit Criteria

**MUST HAVE**:
1. CI builds all 6 targets successfully
2. Native binary < 15 MB for Linux musl static
3. WASM binary < 300 KB enforced by CI
4. Release pipeline creates GitHub Releases with binaries
5. Docker image < 20 MB published to GHCR

**SHOULD HAVE**:
6. Startup time < 50ms (10x faster than Python)
7. Memory RSS < 10 MB (5x less than Python)
8. Message throughput > 1000 msg/s (5x faster than Python)
9. Benchmark results documented

**NICE TO HAVE**:
10. Automated changelog generation
11. Benchmark workflow runs on every release
12. Size profiling reports in CI artifacts

### 6.10 Handoff to Production

**Artifacts to deliver**:
- Multi-platform binaries (6 targets)
- Docker image (`ghcr.io/user/clawft:latest`)
- WASM component (`clawft.wasm`)
- Benchmark results (`docs/benchmarks/results.md`)
- Deployment guides (`docs/deployment/`)

**Blockers for production**:
- Security audit (separate phase)
- Documentation review (separate phase)
- User acceptance testing (separate phase)

**Integration points**:
- Package registries (cargo, npm for WASM)
- Container registries (GHCR, Docker Hub)
- GitHub Releases (binary distribution)
- CI/CD monitoring (GitHub Actions metrics)

---

## Appendix A: Build Scripts

### A.1 Cross-Compilation Script

```bash
#!/bin/bash
# scripts/build/cross-compile.sh

set -e

TARGET=$1
USE_CROSS=${2:-true}

if [ "$USE_CROSS" = "true" ]; then
    echo "Building with cross for $TARGET"
    cross build --release --target $TARGET
else
    echo "Building natively for $TARGET"
    cargo build --release --target $TARGET
fi

# Validate binary size
bash scripts/build/size-check.sh $TARGET \
    target/$TARGET/release/weft
```

### A.2 Docker Build Script

```bash
#!/bin/bash
# scripts/build/docker-build.sh

set -e

# Build static binary
cross build --release --target x86_64-unknown-linux-musl

# Build Docker image
docker build -f Dockerfile.scratch -t clawft:latest .

# Validate image size
SIZE=$(docker image inspect clawft:latest --format='{{.Size}}')
SIZE_MB=$((SIZE / 1048576))

echo "Docker image size: ${SIZE_MB} MB"

if [ "$SIZE_MB" -gt 20 ]; then
    echo "ERROR: Docker image exceeds 20 MB limit"
    exit 1
fi

echo "✓ Docker image size validation passed"
```

---

## Appendix B: Benchmark Scripts

### B.1 Memory Usage Benchmark

```bash
#!/bin/bash
# scripts/bench/memory-usage.sh

set -e

RUST_BIN=${1:-target/release/weft}
PYTHON_BIN=${2:-nanobot}

echo "=== Memory Usage Benchmark ==="

# Start Rust agent in background
$RUST_BIN gateway &
RUST_PID=$!
sleep 2

# Measure RSS
RUST_RSS=$(ps -o rss= -p $RUST_PID)
RUST_MB=$((RUST_RSS / 1024))
echo "Rust RSS: ${RUST_MB} MB"

# Cleanup
kill $RUST_PID

# Start Python agent
$PYTHON_BIN gateway &
PYTHON_PID=$!
sleep 2

# Measure RSS
PYTHON_RSS=$(ps -o rss= -p $PYTHON_PID)
PYTHON_MB=$((PYTHON_RSS / 1024))
echo "Python RSS: ${PYTHON_MB} MB"

# Cleanup
kill $PYTHON_PID

# Calculate improvement
IMPROVEMENT=$(echo "scale=1; $PYTHON_MB / $RUST_MB" | bc)
echo "Improvement: ${IMPROVEMENT}x less memory"

# Validate targets
if [ "$RUST_MB" -lt 10 ]; then
    echo "✓ Rust memory < 10 MB target met"
else
    echo "✗ Rust memory exceeds 10 MB target"
    exit 1
fi
```

### B.2 Message Throughput Benchmark

```bash
#!/bin/bash
# scripts/bench/throughput.sh

set -e

BINARY=$1
NUM_MESSAGES=${2:-1000}

echo "=== Message Throughput Benchmark ==="
echo "Binary: $BINARY"
echo "Messages: $NUM_MESSAGES"

# Start agent
$BINARY gateway &
PID=$!
sleep 2

# Send messages
START=$(date +%s)
for i in $(seq 1 $NUM_MESSAGES); do
    echo '{"role":"user","content":"test"}' | nc localhost 8080 > /dev/null
done
END=$(date +%s)

# Calculate throughput
ELAPSED=$((END - START))
THROUGHPUT=$((NUM_MESSAGES / ELAPSED))

echo "Elapsed: ${ELAPSED}s"
echo "Throughput: ${THROUGHPUT} msg/s"

# Cleanup
kill $PID

# Validate target
if [ "$THROUGHPUT" -gt 1000 ]; then
    echo "✓ Throughput > 1000 msg/s target met"
else
    echo "✗ Throughput below 1000 msg/s target"
    exit 1
fi
```

---

## Appendix C: GitHub Actions Workflow Diagrams

### C.1 CI Workflow Flow

```
┌──────────────┐
│  Git Push    │
└──────┬───────┘
       │
       ▼
┌──────────────────────────────────────┐
│  CI Workflow Triggered               │
│  - On: push to main/develop          │
│  - On: pull_request                  │
└──────┬───────────────────────────────┘
       │
       ├─► Build Matrix (parallel)
       │   ├─ x86_64-linux-musl (cross)
       │   ├─ aarch64-linux-musl (cross)
       │   ├─ x86_64-macos (native)
       │   ├─ aarch64-macos (native)
       │   ├─ x86_64-windows (native)
       │   └─ wasm32-wasip2 (cargo)
       │
       ├─► Size Validation
       │   ├─ Native: < 15 MB
       │   └─ WASM: < 300 KB
       │
       ├─► Unit Tests
       │   └─ cargo test --all
       │
       └─► Upload Artifacts
           └─ weft-{target}
```

### C.2 Release Workflow Flow

```
┌──────────────┐
│  Git Tag v*  │
└──────┬───────┘
       │
       ▼
┌──────────────────────────────────────┐
│  Release Workflow Triggered          │
└──────┬───────────────────────────────┘
       │
       ├─► Create Release
       │   ├─ Generate changelog
       │   └─ Create GitHub Release
       │
       ├─► Build All Targets (parallel)
       │   ├─ Build 6 binaries
       │   └─ Optimize WASM
       │
       ├─► Upload Release Assets
       │   ├─ weft-x86_64-linux-musl
       │   ├─ weft-aarch64-linux-musl
       │   ├─ weft-x86_64-macos
       │   ├─ weft-aarch64-macos
       │   ├─ weft-x86_64-windows.exe
       │   ├─ clawft.wasm
       │   ├─ clawft.wasm.gz
       │   └─ checksums.txt
       │
       └─► Docker Build & Push
           ├─ Build FROM scratch image
           └─ Push to ghcr.io
```
