# Android splat capture edge node (ADR-077 / WEFT-705–707)

Native Android capture client + WeftOS edge core for phone → Mac/cloud splatd.

| Ticket | Phase | What |
|--------|-------|------|
| **WEFT-705** | A1 | CameraX stills, rotation-vector poses, coverage bins UI, session ZIP |
| **WEFT-706** | A2 | Pair (URL/token/QR deep link), upload ZIP → `job_id`, progress, SOG WebView |
| **WEFT-707** | A3 | `crates/clawft-android-edge` UniFFI: Ed25519 node id, pair store, upload client |

**Normative protocol:** [`docs/weftos/capture-protocol-v1.md`](../weftos/capture-protocol-v1.md)  
**ADR:** [`docs/adr/adr-077-android-splat-capture-edge-node.md`](../adr/adr-077-android-splat-capture-edge-node.md)

---

## Repo layout

```
apps/android-splat-capture/     # Gradle / Kotlin Compose app
  app/src/main/java/ai/weftos/splatcapture/
  tools/mock_splatd.py          # Local HTTP stub for A2 without GPU
crates/clawft-android-edge/     # Rust UniFFI edge core
docs/guides/android-splat-capture-edge.md  # this file
```

---

## Build: Rust edge core (always)

Workspace member; no Android SDK required.

```bash
# From repo root
scripts/build.sh check
# or package-scoped:
cargo test -p clawft-android-edge
cargo check -p clawft-android-edge
```

### UniFFI → Kotlin bindings (optional NDK path)

Default feature `uniffi` enables scaffolding. After installing the NDK and
`cargo-ndk`:

```bash
# Example: generate Kotlin bindings next to the app module
cargo build -p clawft-android-edge --release
# With uniffi-bindgen (version-matched to crate uniffi 0.29):
# cargo install uniffi-bindgen --version 0.29.*
# uniffi-bindgen generate \
#   --library target/release/libclawft_android_edge.dylib \
#   --language kotlin \
#   --out-dir apps/android-splat-capture/app/src/main/java

# Cross-compile for device (illustrative):
# cargo ndk -t arm64-v8a -o apps/android-splat-capture/app/src/main/jniLibs \
#   build -p clawft-android-edge --release
```

Until the `.so` is packaged, the app uses a **Kotlin fallback** identity
(`EdgeCore`) that persists a stable node id under app-private storage and
mirrors the pair-store / upload HTTP surface. Production phones should link
the Rust crate for real Ed25519 (`node_identity.json` seed).

### Edge core API (Rust / UniFFI)

| Function | Role |
|----------|------|
| `edge_load_or_create_identity(data_dir)` | Ed25519 load-or-create; stable `node_id_hex` |
| `edge_parse_pair` / `edge_save_pair` / `edge_load_pair` / `edge_clear_pair` | Host URL + capability token |
| `edge_upload_session_zip` | `POST /v1/jobs` multipart `session` |
| `edge_get_job_status` | Poll job; best-effort SOG artifact URL |
| `edge_health_check` | `GET /healthz` |
| `edge_sign` | Sign with node key (pair proofs later) |

Persisted files under `data_dir`:

- `node_identity.json` — seed + public key (mode 0600 on Unix)
- `host_pair.json` — base URL, token, optional host node id

This is the phone as **`Kernel<AndroidPlatform>`** edge step: identity +
capability-scoped host pair before full mesh/QUIC (A4).

---

## Build: Android app (needs SDK)

**Requirements**

- Android Studio Ladybug+ or cmdline tools
- JDK 17
- Android SDK 35, minSdk 29
- Optional: device or emulator with camera

```bash
cd apps/android-splat-capture

# If gradlew is missing, generate the wrapper once:
# gradle wrapper --gradle-version 8.9

./gradlew :app:assembleDebug
# Install:
./gradlew :app:installDebug
```

Open the project folder `apps/android-splat-capture` in Android Studio if you
prefer the IDE.

### Operator flow (A1 offline)

1. Grant camera.
2. **New session** → **SNAP** stills while walking the room.
3. Watch coverage bins fill; follow “Missing: turn left · look up” hints.
4. **Stop + ZIP** → package under app files (`session.json`, `poses.jsonl`, `frames/`).
5. **Sessions** → **Share ZIP** (USB/manual train) or **Upload** (A2).

### Operator flow (A2 host)

1. On Mac: run real splatd **or** the mock:

   ```bash
   python3 apps/android-splat-capture/tools/mock_splatd.py --port 7860
   ```

2. Phone **Pair** tab: `http://<mac-lan-ip>:7860` (+ token if used), or deep link  
   `weftos://pair?host=<ip>&port=7860&token=…&v=1`.
3. **Probe /healthz** → should return healthy.
4. **Sessions → Upload** → progress `stage` / `status` → **Open SOG** (WebView).

---

## Session package (weft.capture.v1)

```
session-<uuid>/
  session.json
  poses.jsonl
  frames/000001.jpg
  …
```

Pose line:

```json
{"frame_id":1,"t_ns":…,"path":"frames/000001.jpg","quat_wxyz":[w,x,y,z],"coverage_bin":12}
```

Upload: `POST /v1/jobs` with multipart field **`session`** = ZIP of that tree
(see capture-protocol-v1 §6.2).

---

## Residual / follow-ups

| Item | Notes |
|------|--------|
| Android SDK on CI agents | This worktree had **no** `ANDROID_HOME`; ship complete Gradle sources; assemble on a machine with SDK. |
| UniFFI `.so` in APK | Documented; Kotlin fallback until NDK artifacts are checked in or CI-built. |
| A4 | QUIC/Noise stream, resumable chunks, foreground service. |
| A5–A6 | Host hole map; ARCore VIO priors. |
| Plane | Claim/close WEFT-705/706/707 when `PLANE_API_KEY` available. |
