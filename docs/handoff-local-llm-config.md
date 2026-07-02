# Handoff — Local-LLM config fragmentation blocks testing the Hermes agent loop

> **RESOLVED 2026-07-02** — the hermes loop is verified end-to-end on both paths. See
> "Resolution" at the bottom. Sub-problem B was NOT a loader bug: the workspace overlay
> `.clawft/config.json` (stale, May 2) was overriding `weave.toml` by design (most-specific
> wins) with `deepseek/deepseek-chat` + tiered cloud routing. Sub-problem C (hang after
> provider error) was a real bug, fixed in `loop_core.rs` + `agent.rs`.

**Date:** 2026-07-01
**Repo:** `/Users/mathewbeane/weftos` (branch `feat/hermes-loop-base`, HEAD `ab263d15` = the full
hermes-loop + ECC graph-walk voice stack merged for release)
**Status:** the whole native-Rust conversational stack is built, merged, and green; **but you can't
actually *run* the agent loop against local Hermes from the CLI** because the local-LLM endpoint/model
config is fragmented across several subsystems that don't share a source of truth. This handoff is
scoped to that blocker.

---

## TL;DR — the underlying problem

There is **no single source of truth for "which local LLM endpoint + model to use,"** and the three
consumers of an LLM each use a *different* selection mechanism:

1. **Daemon (`weaver`)** → `[kernel.llm]` config (`service_url`/`model`) → `clawft-service-llm::LlmClient`
   (which actually reads the `LLM_SERVICE_URL` env, default `:8111`) → keyless local llama.cpp path.
2. **CLI `weft agent` (standalone)** → `agents.defaults.model` (default `deepseek/deepseek-chat`) →
   the **`clawft-llm` provider registry** (openai / anthropic / deepseek / … / `local`@:11434 /
   `ollama`@:11434), which routes by **model name → provider** and each provider needs an **API key**.
   `LLM_SERVICE_URL` is **not** wired into this path.
3. **`LocalProvider::hermes_serving()`** (`clawft-llm`) → hardcoded `:8090` — used by the deterministic
   tests and the voice bridge, but **not** by the `weft agent` provider registry.

So a Hermes served per ADR-060 on `:8090` is reachable by the daemon (after config) and by the direct
tests, but **`weft agent` can't reach it** without either overriding a provider's `base_url` (untested
lead below) or unifying the config.

The desired end state: **serve Hermes (ADR-060, `:8090`) → both the daemon AND `weft agent` (and the
voice loop) discover it with minimal/zero config, and the loop runs and answers.**

---

## What actually happened (error progression)

Hermes IS up and healthy: `curl :8090/v1/models` → 200, `curl :8090/health` → 200, served via
`~/llm/bin/serve-llamacpp … --port 8090 --alias hermes-4.3-36b`.

1. Bringing up the kernel logged `[health] llm: unhealthy … :8111` — the daemon's LLM points at `:8111`
   (default), not `:8090`. **Fixed** by editing `weave.toml [kernel.llm]` → `service_url =
   "http://127.0.0.1:8090"`, `model = "hermes-4.3-36b"`.
2. `weft agent -m "what is 17 * 23?"` → `model=deepseek/deepseek-chat` → `provider not configured: set
   DEEPSEEK_API_KEY` → **then hangs** (does not exit in single-message mode).
3. Added `weave.toml [agents.defaults].model = "hermes-4.3-36b"` → **STILL `deepseek/deepseek-chat`**.
   No `~/.clawft/config.json` exists to override it, so the likely cause is **the `weave.toml` loader
   only recognizes a subset of keys and silently drops `[agents]`** (needs confirmation).
4. `weft agent --model hermes-4.3-36b -m "…"` (the `--model` flag is applied unconditionally at
   `agent.rs:86`) → audit now shows `model=hermes-4.3-36b` (routing fixed!) → but → `provider not
   configured: set OPENAI_API_KEY` — a **bare model routes to the `openai` provider**, which needs a
   key; `LLM_SERVICE_URL` is ignored on this path. Again **hangs** after the error.

So there are **three distinct sub-problems**:
- **(A) Config fragmentation** — can't point `weft agent` at local Hermes (the main problem).
- **(B) `weave.toml [agents]` not loaded** — the loader drops the `[agents.defaults]` block.
- **(C) `weft agent` hangs after a provider error** — single-message mode never exits ("then it stalls").

---

## The fragmentation map (disagreeing defaults, with file:line)

| Thing | Where | Value |
|---|---|---|
| Canonical Hermes serve | ADR-060 / `serve-llamacpp` / `LocalProvider::hermes_serving()` `clawft-llm/src/local_provider.rs:51,144` | `:8090`, alias `hermes-4.3-36b` |
| Kernel LLM service default | `clawft-service-llm/src/lib.rs:68` `DEFAULT_LLM_SERVICE_URL` | `http://127.0.0.1:8111` |
| Kernel LLM env override | `clawft-service-llm` `LLM_SERVICE_URL` env; CLI reads it via `bootstrap.rs:483` `LlmConfig::from_env()` | default `:8111` |
| `weave.toml [kernel.llm]` (shipped) | generated template | `:8111` / `gemma-iq2m` (now locally patched → `:8090`/`hermes-4.3-36b`) |
| Agent-loop model default | `clawft-types/src/config/mod.rs:265` | `"deepseek/deepseek-chat"` (cloud) |
| CLI provider registry | `clawft-llm/src/config.rs` (11 builtins) | `openai` base `https://api.openai.com/v1` key `OPENAI_API_KEY`; `local` (`local/` prefix) base `http://localhost:11434/v1` key `LOCAL_LLM_API_KEY`; `ollama` (`ollama/` prefix) same base |
| Provider specs / routing | `clawft-types/src/provider.rs` (fields `env_key`, `default_api_base`, `model_prefix`, `detect_by_base_keyword`, `is_local`, …) | bare/unknown model → `openai` |
| Whisper (sensor STT, ADR-053) | `clawft-weave/src/daemon.rs:489-491` fallback; `clawft-service-whisper/src/lib.rs` `DEFAULT` | daemon `:8123`, lib default `:8080` (separate warning; unrelated to the loop) |

**Config precedence** (`clawft-platform/src/config_loader.rs`): `weave.toml` (cwd, project) ←
`~/.clawft/config.json` (overrides) ← env vars. There's a typed `config.providers: ProvidersConfig`
(`config/mod.rs:58`) with per-provider fields incl. `api_base` (e.g. `providers.openrouter.api_base`,
`config/mod.rs:622`) — **this is the untested lead for overriding a provider's base_url**.

---

## Untested candidate fixes (for the next thread — pick/verify)

**Fastest local hack to get `weft agent` talking to :8090** (verify these, in order):
1. **Override a provider's `api_base` via `config.providers` in `weave.toml`.** The typed
   `ProvidersConfig` has per-provider `api_base`. Try, e.g.:
   ```toml
   [providers.local]
   api_base = "http://127.0.0.1:8090/v1"
   ```
   then `LOCAL_LLM_API_KEY=dummy weft agent --model local/hermes-4.3-36b -m "…"`. (The `local/` prefix
   selects the `local` provider; llama.cpp ignores the key.) **Unknown:** whether the loader merges
   `[providers.local]` and whether `[agents]` being dropped (sub-problem B) also affects `[providers]`.
   Confirm by reading how `config/mod.rs` deserializes `providers` + how `clawft-llm` builds the
   provider from `config.providers` (does it honor `api_base`, or use the hardcoded builtin `:11434`?).
2. **Or override `openai`:** `[providers.openai].api_base = "http://127.0.0.1:8090/v1"` +
   `OPENAI_API_KEY=dummy` + bare model `hermes-4.3-36b`.
3. Note: Ollama **is** running on `:11434` (returns 200) but does not host `hermes` — so an
   unmodified `local/`/`ollama/` route hits Ollama, not the llama.cpp Hermes.

**Proper fixes (holistic — see `docs/voice-release-followups.md` for the full write-up):**
- **Unify the local-LLM config** to one source of truth: align `DEFAULT_LLM_SERVICE_URL` + the
  `weave.toml`/`init` template + `hermes_serving()`/ADR-060 on the same port (`:8090`) + a consistent
  default model/alias, so a freshly-served Hermes is discovered with zero config.
- **Bridge `[kernel.llm]` → the CLI agent path** (`agents.defaults.model` + provider `api_base`) the
  same way the daemon does at `bootstrap.rs:638`, OR **route `weft agent` through the running daemon**
  per **ADR-021** (CLI must route through the kernel daemon) instead of building its own standalone
  loop + cloud provider registry.
- **Change the `deepseek/deepseek-chat` default** (`config/mod.rs:265`) to route local when a local
  endpoint is configured.
- **Fix sub-problem B:** make the `weave.toml` loader recognize `[agents.defaults]` (and `[providers]`)
  — confirm which keys the loader whitelists/normalizes and why `[agents]` is dropped.
- **Fix sub-problem C:** `weft agent -m` single-message mode should **exit non-zero on a provider
  error**, not hang. Investigate `agent.rs::run_single_message` (`agent.rs:184+`) — the loop is spawned
  on the bus and likely awaits a completion that never arrives when the provider errors.

---

## Known-good reference (this DOES work — the loop's brain on Hermes)

```bash
cargo test -p clawft-llm --test hermes_provider -- --ignored
```
Uses `LocalProvider::hermes_serving()` (`:8090`) directly for a real `<tool_call>`→`tool_calls`
round-trip with `<think>` stripping. Verified green. This confirms the LLM leg works on Hermes; the
problem is purely the **CLI/daemon config plumbing** that selects a provider.

The **daemon path** is also correctly configured now (`weave.toml [kernel.llm]` = `:8090`/
`hermes-4.3-36b`) — bringing up `weaver` should health-check green against Hermes; a daemon-routed
`agent.chat`/`llm.prompt` uses the clean local `LlmClient` path (keyless llama.cpp). That may be the
easiest "real loop on Hermes" test if routing `weft agent` through the daemon is the chosen direction.

---

## Key files / anchors

- CLI agent: `crates/clawft-cli/src/commands/agent.rs` (`:49` args, `:83` `load_config`, `:86-88`
  `--model` override, `:90` `effective_model`, `:113` `enable_live_llm`, `:184` `run_single_message`).
- Config load: `crates/clawft-cli/src/commands/mod.rs:52,70` → `clawft-platform/src/config_loader.rs`
  (`:30` discover, `:67-82` `load_config_raw`; weave.toml ← json ← env).
- Daemon LLM bridge: `crates/clawft-core/src/bootstrap.rs:638` (stamps `[kernel.llm].model` →
  `agents.defaults.model`), `:410,483` `enable_live_llm` / `LlmConfig::from_env()`.
- Provider registry: `crates/clawft-llm/src/config.rs` (11 builtins; `:132` `local`, `:141` `ollama`),
  `crates/clawft-types/src/provider.rs` (`:140-175` spec fields, `:256` `openai`, `:286` `deepseek`),
  `crates/clawft-llm/src/error.rs:102` (the `set OPENAI_API_KEY` NotConfigured error).
- Providers config: `crates/clawft-types/src/config/mod.rs:58` `providers`, `:265` deepseek default,
  `:319` `ProvidersConfig`, `:622` `openrouter.api_base` (proves per-provider `api_base` exists).
- Kernel LLM config: `crates/clawft-types/src/config/kernel.rs:286` `LlmEndpointConfig`
  (`service_url`/`model`, env `LLM_SERVICE_URL`/`LLM_MODEL`).
- Whisper (separate, harmless): `crates/clawft-weave/src/daemon.rs:469,489-491`,
  `crates/clawft-service-whisper/src/lib.rs`.

---

## Local changes already applied (uncommitted, machine-local)

- `weave.toml [kernel.llm]` → `service_url = "http://127.0.0.1:8090"`, `model = "hermes-4.3-36b"`.
- `weave.toml [agents.defaults].model = "hermes-4.3-36b"` (currently **not honored** — sub-problem B).
- `docs/voice-release-followups.md` — updated with the endpoint-fragmentation + per-service-enable
  follow-ups (the holistic write-up; **committable**; `weave.toml` should stay local).
- Working CLI knob today: `--model hermes-4.3-36b` (routing) but the provider still needs a keyless
  local endpoint override — that's the open piece.

## Also open (adjacent, documented in voice-release-followups.md)
- Every kernel service should get a config `enable/disable` (whisper `:8123` degraded WARN is harmless
  but there's no config way to not start it).
- `[embedding] provider = "mock-sha256"` in `weave.toml` → switch to ADR-059 Qwen3 for real semantic
  graph-walk recall.
- Release-gate follow-ups: `rustls-webpki`+`wasmtime` RUSTSEC advisories, `clawft-ui` tsc `@types`,
  `ui-docker` needs the Docker daemon.

---

## Resolution (2026-07-02)

**Sub-problem B — solved (config, not code).** The `weave.toml` loader was fine; the *workspace
overlay* `/Users/mathewbeane/weftos/.clawft/config.json` (dated May 2, pre-Hermes) set
`agents.defaults.model = "deepseek/deepseek-chat"` and `routing.mode = "tiered"` (all-cloud tiers),
and per `config_loader.rs` precedence (weave.toml ← home JSON ← **workspace `.clawft/config.json`**)
the overlay wins. Local edits applied: overlay model → `hermes-4.3-36b`, routing → `static`.

**Sub-problem A — solved via candidate fix #2 (openai `api_base` override).** A bare model routes to
the `openai` builtin in `create_adapter_from_config` (`clawft-core/src/pipeline/llm_adapter.rs:302`),
and `apply_config_overrides` honors `[providers.openai].api_base`. Added to `weave.toml`:
`[providers.openai] api_base = "http://127.0.0.1:8090/v1"`, run with `OPENAI_API_KEY=dummy`
(llama.cpp ignores the key). Note: candidate fix #1 (`[providers.local]`) can NOT work —
`apply_config_overrides` has no `local`/`ollama` arm (`llm_adapter.rs:350-360`).

**Sub-problem C — fixed in code.** `AgentLoop::run` swallowed turn errors (logged, dispatched
nothing), so `run_single_message` blocked forever on `consume_outbound`. Fix: the Err arm now
dispatches an outbound error reply with `metadata.error = true`
(`clawft-core/src/agent/loop_core.rs`), and `weft agent -m` prints it to stderr and exits 1
(`clawft-cli/src/commands/agent.rs::run_single_message`). Channel users now also get an error
reply instead of silence.

**Verified green (all against Hermes 4.3 36B Q8 on `:8090` via `~/llm/bin/serve-llamacpp`):**
- `weft agent -m "What is the capital of France? …"` → `Paris`, exit 0.
- `weft agent -m "what is 17 * 23?"` → `391`, exit 0. (An earlier run drove a genuine
  20-iteration `<tool_call>` loop — `exec_shell` attempts each denied by the security policy —
  proving tool-call parsing/dispatch works end-to-end.)
- Provider-error path: no key → error on stderr, **exit 1 in <1s** (previously hung forever).
- Daemon path: `weaver kernel start` → all 7 services healthy incl. `llm`;
  `llm.prompt` RPC → `"pong"`, `model=hermes-4.3-36b`; `agent.chat` RPC → `"391"`.

**Still open (the proper holistic fixes, unchanged):** unify local-LLM config to one source of
truth (`:8090` everywhere), bridge `[kernel.llm]` → CLI agent path or route `weft agent` through
the daemon per ADR-021, change the `deepseek/deepseek-chat` cloud default, and make the model
tool-visible security policy discoverable (Hermes burned 20 iterations trying denied shell
commands with no hint of the allowlist).
