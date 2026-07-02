# Voice/ECC graph-walk release — pre-existing gate follow-ups (2026-06-30)

The `feat/voice-native → feat/hermes-loop-base` release merge (ADR-062, commit `ab263d15`)
passed all 10 voice-relevant gate checks (Rust build / clippy / test / WASM / voice-feature).
Three gate items failed that are **pre-existing and NOT introduced by the voice work** — filed
here as cleanup (none block the voice feature):

1. **cargo audit — dependency advisories.** `rustls-webpki` (RUSTSEC-2026-0098/0099/0104) and
   `wasmtime` advisories on base-wide transitive deps (TLS stack + WASM sandbox). `rustls-webpki`
   has two pinned versions (0.101.7 + 0.103.10) so a plain `cargo update` is ambiguous. Action:
   bump/patch the affected crates or extend the gate's audit ignore-list for 0.7.x with a rationale.
2. **clawft-ui tsc build.** `TS2688: Cannot find type definition file for 'vite/client' / 'node'`
   — missing/unresolved `@types/node` + vite client types in `clawft-ui`. Action: restore the dev
   type deps / fix `tsconfig` `types`. (Zero TypeScript changed in the voice work.)
3. **ui-docker container build.** `scripts/build.sh ui-docker` couldn't connect to the Docker
   daemon (OrbStack socket absent) on the build host. Action: run the container build in CI / with
   the daemon up; it is an environment gap, not a code failure.

## Holistic: local-LLM endpoint config is fragmented (needs a single source of truth)

The local-Hermes port/model is specified in **three places that don't agree**, so "serve Hermes
(ADR-060) → bring up the kernel" does NOT work out-of-the-box — the kernel health-checks the wrong
endpoint (`[health] llm: unhealthy … :8111`) until manually reconfigured:

- **ADR-060 recipe / `serve-llamacpp` / `LocalProvider::hermes_serving()`** → `http://127.0.0.1:8090`,
  alias `hermes-4.3-36b` (the canonical Hermes serving).
- **`clawft-service-llm::DEFAULT_LLM_SERVICE_URL`** → `http://127.0.0.1:8111`, default model `"local"`.
- **Generated `weave.toml` `[kernel.llm]`** → shipped `service_url = ":8111"`, `model = "gemma-iq2m"`
  (neither matches the served model).
- **Standalone `weft agent` CLI — a SEPARATE, un-bridged path.** It uses `agents.defaults.model`
  (default `"deepseek/deepseek-chat"`, `config/mod.rs:265` — the `deepseek/` prefix routes to the CLOUD
  deepseek provider and needs `DEEPSEEK_API_KEY`) + `LlmConfig::from_env()` (`bootstrap.rs:483` =
  `LLM_SERVICE_URL` env, default `:8111`). It does NOT read `[kernel.llm]` (only the *daemon* bridges
  `[kernel.llm].model → agents.defaults.model`), and `weft` does NOT read `.env` (only `weaver` does).
  Net: even after `[kernel.llm]` is fixed, `weft agent` still fails `provider not configured: set
  DEEPSEEK_API_KEY` until `agents.defaults.model` is a bare local alias AND `LLM_SERVICE_URL=:8090` is
  exported in the shell.

Immediate local fix applied (2026-06-30/07-01, uncommitted machine-local): `weave.toml [kernel.llm]`
= `:8090`/`hermes-4.3-36b` (daemon path), AND `weave.toml [agents.defaults].model = "hermes-4.3-36b"`
(CLI path), AND `export LLM_SERVICE_URL=http://127.0.0.1:8090` (CLI endpoint — `weft` has no `.env`).

**Holistic action (do properly):** pick ONE source of truth for the local-LLM endpoint+model and
align the others to it — align `DEFAULT_LLM_SERVICE_URL` + the `weave.toml`/`init` template + the
`hermes_serving()`/ADR-060 recipe on the SAME port (8090) and a consistent default model/alias, so a
freshly-served Hermes is discovered with zero config. **Must also cover the CLI agent path**: bridge
`[kernel.llm]`→`agents.defaults.model` + `service_url` in `weft agent` the same way the daemon does
(or route the CLI through the daemon per ADR-021), have the CLI honor the configured local `service_url`
(not only the `LLM_SERVICE_URL` env), and change the hardcoded `deepseek/deepseek-chat` default so it
routes to the local endpoint when one is configured (rather than a cloud provider that needs a key).
Consider a health-check that probes the ADR-060 port, and a clear error that names the expected
`service_url`/`model` vs what's served. Related:
`[embedding] provider` in the generated `weave.toml` defaults to `mock-sha256` (non-semantic recall) —
should default to (or document switching to) the ADR-059 Qwen3 provider for the real graph-walk recall.

## Holistic: every kernel service needs a config enable/disable (not just whisper)

Observed: bringing up the kernel logs `whisper: health probe timeout — degraded mode … :8123` when the
substrate/sensor STT service (`clawft-service-whisper`, ADR-053 — consumes sensor PCM off the substrate,
**unrelated** to the native conversational voice loop) has no backend. It's non-fatal (degraded-but-alive),
but there is **no config way to not start it** — the daemon always constructs + health-probes it. A
runtime control flag exists (`control_flags.register(ControlKind::Service, "whisper", true)`,
`daemon.rs:469/489`; also `classify` at `:487`) but it (a) defaults on and (b) doesn't gate the
boot-time construction/probe, so the WARN fires regardless.

**Decision (user, 2026-07-01): this should be general — EVERY service gets an enable/disable config,
not a one-off whisper toggle.** The control-flag registry (`ControlKind::Service "<name>"`) already
enumerates the services; make their **initial value + boot-time construction** config-driven.

Design sketch:
- A config surface, e.g. `[services]` with per-service `enabled` (or per-service `[kernel.<svc>].enabled`),
  covering the optional services the daemon spins up (whisper, classify, and any future ones registered
  via `control_flags.register(ControlKind::Service, …)`).
- Seed each service's control flag from config; and — crucially — **gate the boot-time construct/probe**
  on it, so a disabled service does NOT start, health-probe, or log degraded-mode noise (fixes the actual
  complaint, which the runtime flag alone does not).
- Defaults preserve today's behavior (all on) → no change for existing setups; operators set
  `enabled = false` in `weave.toml` for services they don't run.
- Pairs with the endpoint-config item above: a service's `enabled` + its endpoint/`service_url` belong on
  the same config block, so "which services run + where they point" is one coherent, discoverable surface.

Immediate: no code change made yet (the degraded WARN is harmless and does not block the Hermes loop or the
native voice loop). Tracked here for the holistic services-config pass.
