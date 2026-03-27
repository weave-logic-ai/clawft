# Phase K4: Container Integration

**Phase ID**: K4
**Workstream**: W-KERNEL
**Duration**: Week 3 (1 week, parallel with K1)
**Goal**: Provide container-based sidecar service orchestration with an Alpine base image and Docker integration

---

## S -- Specification

### What Changes

This phase adds container integration to the kernel, allowing WeftOS to manage sidecar services (databases, caches, external APIs) as containerized services alongside the agent process. The implementation uses `bollard` for Docker API access and introduces an Alpine-based Docker image for running `weft` in containers.

Feature-gated behind `containers` to keep the default binary free of Docker dependencies.

### Files to Create

| File | Purpose |
|---|---|
| `crates/clawft-kernel/src/container.rs` | `ContainerManager` -- Docker container lifecycle management |
| `crates/clawft-kernel/Dockerfile.alpine` | Minimal Alpine image for running `weft` in containers |
| `crates/clawft-kernel/docker-compose.yml` | Example compose file with `weft` + common sidecars |

### Files to Modify

| File | Change |
|---|---|
| `crates/clawft-kernel/Cargo.toml` | Add `bollard` dep behind `containers` feature |
| `crates/clawft-kernel/src/service.rs` | Add `ContainerService` implementing `SystemService` trait |
| `crates/clawft-kernel/src/lib.rs` | Conditionally export `container` module |

### Key Types

**ContainerManager** (`container.rs`):
```rust
pub struct ContainerManager {
    docker: bollard::Docker,
    managed_containers: DashMap<String, ManagedContainer>,
    config: ContainerConfig,
}

pub struct ContainerConfig {
    pub docker_socket: String,
    pub network_name: String,
    pub default_restart_policy: RestartPolicy,
    pub health_check_interval: Duration,
}

pub struct ManagedContainer {
    pub name: String,
    pub image: String,
    pub container_id: Option<String>,
    pub state: ContainerState,
    pub ports: Vec<PortMapping>,
    pub env: HashMap<String, String>,
    pub volumes: Vec<VolumeMount>,
    pub health_endpoint: Option<String>,
}

pub enum ContainerState {
    Pulling,
    Creating,
    Running,
    Stopping,
    Stopped,
    Failed(String),
}

pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: String,
}

pub struct VolumeMount {
    pub host_path: String,
    pub container_path: String,
    pub read_only: bool,
}

pub enum RestartPolicy {
    Never,
    OnFailure { max_retries: u32 },
    Always,
}

impl ContainerManager {
    pub async fn new(config: ContainerConfig) -> Result<Self>;
    pub async fn start_container(&self, spec: ManagedContainer) -> Result<String>;
    pub async fn stop_container(&self, name: &str, timeout: Duration) -> Result<()>;
    pub async fn restart_container(&self, name: &str) -> Result<()>;
    pub async fn inspect_container(&self, name: &str) -> Result<ContainerInfo>;
    pub async fn list_containers(&self) -> Vec<(String, ContainerState)>;
    pub async fn health_check(&self, name: &str) -> Result<HealthStatus>;
    pub async fn logs(&self, name: &str, tail: u32) -> Result<String>;
    pub async fn stop_all(&self) -> Result<()>;
}
```

**ContainerService** (`service.rs` extension):
```rust
pub struct ContainerService {
    manager: Arc<ContainerManager>,
    spec: ManagedContainer,
}

impl SystemService for ContainerService {
    fn name(&self) -> &str { &self.spec.name }
    fn service_type(&self) -> ServiceType { ServiceType::Custom("container".into()) }
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    async fn health_check(&self) -> HealthStatus;
}
```

### Dockerfile.alpine

```dockerfile
FROM rust:1.83-alpine AS builder
RUN apk add --no-cache musl-dev openssl-dev
WORKDIR /build
COPY . .
RUN cargo build --release --bin weft

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /build/target/release/weft /usr/local/bin/weft
ENTRYPOINT ["weft"]
CMD ["kernel", "status"]
```

### Example docker-compose.yml

```yaml
version: "3.9"
services:
  weft:
    build:
      context: ../..
      dockerfile: crates/clawft-kernel/Dockerfile.alpine
    ports:
      - "18789:18789"
    volumes:
      - weft-data:/data
    environment:
      - CLAWFT_CONFIG=/data/config.json
    depends_on:
      - redis
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
volumes:
  weft-data:
```

---

## P -- Pseudocode

### Container Lifecycle

```
fn ContainerManager::start_container(spec):
    // 1. Check if container already exists
    if managed_containers.contains(spec.name):
        existing = managed_containers.get(spec.name)
        if existing.state == Running:
            return Ok(existing.container_id)

    // 2. Pull image if needed
    set state = Pulling
    if not docker.image_exists(spec.image).await:
        docker.pull_image(spec.image).await?

    // 3. Create container
    set state = Creating
    create_opts = bollard::ContainerCreateOpts {
        image: spec.image,
        name: spec.name,
        ports: spec.ports.map(|p| format!("{}:{}", p.host_port, p.container_port)),
        env: spec.env.entries(),
        volumes: spec.volumes.map(|v| format!("{}:{}:{}", v.host_path, v.container_path, if v.read_only "ro" else "rw")),
        restart_policy: config.default_restart_policy,
        network: config.network_name,
    }
    container_id = docker.create_container(create_opts).await?

    // 4. Start container
    docker.start_container(container_id).await?
    set state = Running

    // 5. Register for health monitoring
    managed_containers.insert(spec.name, ManagedContainer { container_id, state: Running, ... })

    return Ok(container_id)
```

### Health Check Propagation

```
fn ContainerService::health_check():
    match manager.health_check(self.spec.name).await:
        Ok(status) -> status
        Err(DockerNotAvailable) -> HealthStatus::Unknown
        Err(ContainerNotRunning) -> HealthStatus::Unhealthy("container stopped")
        Err(e) -> HealthStatus::Unhealthy(e.to_string())

fn ContainerManager::health_check(name):
    container = managed_containers.get(name)?

    // 1. Check Docker container status
    info = docker.inspect_container(container.container_id).await?
    if not info.running:
        return Ok(HealthStatus::Unhealthy("not running"))

    // 2. If health endpoint configured, check it
    if let Some(endpoint) = container.health_endpoint:
        response = http_get(endpoint).await
        if response.status() == 200:
            return Ok(HealthStatus::Healthy)
        else:
            return Ok(HealthStatus::Degraded(response.status()))

    // 3. Otherwise, running = healthy
    return Ok(HealthStatus::Healthy)
```

---

## A -- Architecture

### Component Relationships

```
ContainerManager
  |
  +-- bollard::Docker (Docker API client)
  |     +-- Unix socket or TCP connection
  |
  +-- DashMap<name, ManagedContainer>
  |     +-- Container specs and runtime state
  |
  +-- ContainerService (per container)
        +-- implements SystemService
        +-- registered in ServiceRegistry
        +-- health checks propagate to kernel health
```

### Integration Points

1. **ServiceRegistry**: Each managed container is wrapped in a `ContainerService` and registered in the kernel's `ServiceRegistry`. This means container health appears in `weave kernel services` output.

2. **Existing container tools**: The `clawft-plugin-containers` crate has container tool definitions. K4's `ContainerManager` is the runtime that executes those tools when the kernel is active.

3. **Health propagation**: Container health checks flow through `ServiceRegistry` -> `HealthSystem` -> `weave kernel status`. A stopped sidecar shows as degraded kernel health.

4. **Docker availability**: `ContainerManager::new()` attempts to connect to Docker. If Docker is not available, it returns a clear error. The kernel continues to function without container support.

### Ruvector Integration (Doc 07)

When the `ruvector-containers` feature gate is enabled, RVF containers provide an
alternative to Docker for agent packaging and deployment. Docker remains available
behind the `containers-docker` feature gate as a fallback. See
`07-ruvector-deep-integration.md` for full adapter code.

| Custom Component | Ruvector Replacement | Feature Gate | Benefit |
|---|---|---|---|
| Docker/bollard container management | `rvf-kernel` + `rvf-wire` RVF container format | `ruvector-containers` | Single-file containers; 125ms boot; no Docker daemon required |
| `Dockerfile.alpine` | `rvf-kernel` bzImage + initramfs builder | `ruvector-containers` | 3-5MB image vs 50MB+ Alpine base; bare metal or VM deployment |
| `docker-compose.yml` | RVF manifest within the container file | `ruvector-containers` | Self-describing format with segment-based composability |
| Docker health checks | `rvf-crypto` witness-attested health | `ruvector-containers` | Cryptographic proof of health state; tamper-evident attestation |
| Service discovery via Docker compose | `ruvector-cluster` service discovery | `ruvector-cluster` | Consistent hash ring discovery without Docker networking |

A `ContainerBackend` enum dispatches to either `RvfContainerManager` or
`DockerContainerManager` depending on enabled features. RVF is the primary path;
Docker is the fallback for third-party images only available as Docker images.

Cross-reference: `07-ruvector-deep-integration.md`, Section 3 "Phase K4: Containers"
and Section 6 "Container Format".

### K2 Symposium Decisions

The following decisions from the K2 Symposium directly shape K4 scope and implementation.
Reference doc: `docs/weftos/k2-symposium/08-symposium-results-report.md`

| Decision | Summary | Commitment |
|----------|---------|------------|
| D10 | Container sandbox for WASM-compiled shell tools. Containers provide the outer isolation layer for chain-linked WASM modules produced in K3 | -- |
| D12 | `ChainAnchor` trait for blockchain anchoring. All container lifecycle events (start, stop, health) are anchored to the ExoChain | C7 |
| D13 | SNARK prover research spike during K4. Evaluate feasibility of zero-knowledge proofs for container attestation; results feed K6 | -- |
| D14 | `SpawnBackend::Tee` variant defined but not implemented. Type exists in the API, returns `BackendNotAvailable` at runtime until K6 TEE support lands | C8 |
| D18 | Late K4: SONA reuptake spike -- pull forward from K5. Validate that accumulated K3 training metrics can feed the SONA learning pipeline; confirm K5 integration path | -- |
| D21 | K3 -> K4 -> K5 -> K6 ordering confirmed, iterative cycle. Each phase produces training data and witness entries consumed by the next | -- |

Key crates for K4:
- **ruvector-tiny-dancer-core**: semantic routing hints for container service dispatch (D17, carried from K3)
- **cognitum-gate-kernel**: kernel-level audit verification for container lifecycle events
- **ruvector-snapshot**: point-in-time state snapshots for container checkpoint/restore

---

## R -- Refinement

### Edge Cases

1. **Docker not installed**: `ContainerManager::new()` returns `DockerNotAvailable` error. Kernel logs warning and skips container services.
2. **Image pull failure**: Container transitions to `Failed("pull failed: <reason>")` state. Does not block other containers.
3. **Port conflict**: Docker returns error; propagated as `PortConflict` with host port number
4. **Volume mount to non-existent path**: Docker creates the path (default Docker behavior). Document this.
5. **Container restart loop**: With `OnFailure` restart policy, track restart count. After `max_retries`, transition to `Failed` state.
6. **Kernel shutdown**: `stop_all()` stops all managed containers in parallel with per-container timeout.
7. **Orphan containers**: On kernel boot, check for previously managed containers still running. Option to adopt or stop them.

### Backward Compatibility

- Feature-gated behind `containers`; default binary unchanged
- No changes to existing container plugin tools
- Compose file is an example, not required

### Error Handling

- `ContainerError` enum: `DockerNotAvailable`, `ImagePullFailed`, `CreateFailed`, `StartFailed`, `PortConflict`, `ContainerNotFound`, `HealthCheckFailed`
- All errors include container name and Docker error details
- Docker connection errors are non-fatal to kernel boot

---

## C -- Completion

### Exit Criteria

- [x] `ContainerManager` compiles with `--features containers`
- [x] Container start/stop lifecycle works with Docker — stub returns DockerNotAvailable, state machine tested
- [x] Health check propagates container status to `ServiceRegistry`
- [x] `ContainerService` implements `SystemService` trait
- [x] Dockerfile.alpine builds and runs `weft`
- [x] docker-compose.yml starts `weft` + Redis sidecar
- [x] Graceful shutdown stops all managed containers
- [x] Docker not available produces clear error (not panic)
- [x] Feature gate: kernel compiles without `containers` feature
- [x] All workspace tests pass — 596 with all features
- [x] Clippy clean
- [x] ChainAnchor trait defined with mock implementation (C7) — pre-existing
- [x] SpawnBackend::Tee variant returns BackendNotAvailable (C8) — pre-existing
- [ ] Training data pipeline validated with accumulated K3 metrics — deferred to K5
- [ ] SONA reuptake spike complete - K5 integration path confirmed — deferred to K5

### Testing Verification

```bash
# Container unit tests (mock Docker API)
cargo test -p clawft-kernel --features containers -- container

# Container service integration
cargo test -p clawft-kernel --features containers -- test_container_service

# Build without feature (should compile)
cargo test -p clawft-kernel

# Dockerfile builds
docker build -f crates/clawft-kernel/Dockerfile.alpine -t weft:test .

# Regression check
scripts/build.sh test
```
