# WEFT-604 result — Unify local-LLM endpoint/model config

**Ticket:** WEFT-604  
**Branch:** `wave0k/weft-604-llm-config`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4dd-9e94-7350-bce2-783dc76c9dca`  
**Date:** 2026-07-30

## Problem

No single source of truth for “which local LLM endpoint + model to use.” Three
consumers used three mechanisms:

1. **Daemon** → `[kernel.llm]` / `LLM_SERVICE_URL` → `clawft-service-llm` default **`:8111` / `local`**
2. **`weft agent`** → `agents.defaults.model` default **`deepseek/deepseek-chat`** → cloud provider registry (API key required); `LLM_SERVICE_URL` ignored
3. **Voice / tests** → `LocalProvider::hermes_serving()` hardcoded **`:8090` / `hermes-4.3-36b`** (ADR-060)

Zero-config Hermes required machine-local patches. `[providers.local]` overrides
were silently dropped. Winning config layer for `agents.defaults.model` was opaque.

## Fix

### 1. Single source of truth (`clawft-types`)

New module `crates/clawft-types/src/config/local_llm.rs`:

| Constant | Value |
|----------|--------|
| `DEFAULT_LOCAL_LLM_SERVICE_URL` | `http://127.0.0.1:8090` |
| `DEFAULT_LOCAL_LLM_API_BASE` | `http://127.0.0.1:8090/v1` |
| `DEFAULT_LOCAL_LLM_MODEL` | `hermes-4.3-36b` |
| `DEFAULT_LOCAL_LLM_MODEL_ROUTED` | `local/hermes-4.3-36b` |

Helpers: `service_url_to_api_base`, `api_base_to_service_url`,
`is_default_local_model`, `is_legacy_cloud_default_model`.

### 2. Aligned defaults

- **`clawft-service-llm`**: `DEFAULT_LLM_SERVICE_URL` → `:8090`; `DEFAULT_LLM_MODEL` → `hermes-4.3-36b`; `LlmConfig::from_env` also reads `LLM_MODEL`
- **`agents.defaults.model`**: default → `local/hermes-4.3-36b` (keyless local route)
- **`clawft-llm` builtin `local`**: base → ADR-060 API base; default model → Hermes
- **`LocalProvider::hermes_serving`**: constants re-export types constants
- Lockstep CI test: `service_llm_defaults_match_types_constants`

### 3. Bridge `[kernel.llm]` → CLI agent path

`clawft-core::local_llm_bridge`:

- Resolve env → `[kernel.llm]` → ADR-060 defaults
- Stamp `agents.defaults.model` when default / legacy cloud
- Stamp `providers.local.api_base` when unset
- Preserve explicit non-default agent models and existing local `api_base`

Wired in `weft agent` (`clawft-cli/src/commands/agent.rs`) before loop bootstrap.

### 4. Provider overrides + keyless local adapter

- `ProvidersConfig` gains `local` + `ollama` slots
- `apply_config_overrides` honors them (and `vllm`); unknown names **warn** instead of silent return
- `create_adapter_from_config` uses **`LocalProvider`** (keyless) for `local` / `ollama` / bare Hermes alias, with Hermes `num_ctx`

### 5. Model-source logging

- `LoadedConfig.agents_model_source` names the layer that set `agents.defaults.model`
  (`default` | `global:weave.toml|home-config` | `workspace:.clawft/config.json` | `cli:--config` | `cli:--model`)
- Agent logs `local llm endpoint resolved` with `url_source` / `model_source` / `agents_model_source` (mirrors daemon)

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-types/src/config/local_llm.rs` | **new** — constants + helpers |
| `crates/clawft-types/src/config/mod.rs` | default model, `providers.local`/`ollama`, tests |
| `crates/clawft-types/src/config/kernel.rs` | docs → ADR-060 defaults |
| `crates/clawft-service-llm/src/lib.rs` | defaults `:8090` / hermes |
| `crates/clawft-service-llm/src/client.rs` | `from_env` reads `LLM_MODEL` |
| `crates/clawft-llm/src/local_provider.rs` | Hermes constants from types |
| `crates/clawft-llm/src/config.rs` | builtin `local` → ADR-060 |
| `crates/clawft-core/src/local_llm_bridge.rs` | **new** — resolve + stamp + tests |
| `crates/clawft-core/src/lib.rs` | export module |
| `crates/clawft-core/src/pipeline/llm_adapter.rs` | LocalProvider + overrides |
| `crates/clawft-core/src/pipeline/router.rs` | default-router test |
| `crates/clawft-core/src/bootstrap.rs` / workspace / phase1 | default model asserts |
| `crates/clawft-cli/src/commands/mod.rs` | model-source provenance |
| `crates/clawft-cli/src/commands/agent.rs` | bridge + logging |
| `crates/clawft-weave/src/daemon.rs` | comment (defaults) |
| `tests/fixtures/config.json` | model → local Hermes |

## Tests

```bash
scripts/build.sh check
cargo test -p clawft-types --lib config::
cargo test -p clawft-core --lib local_llm
cargo test -p clawft-core --lib pipeline::llm_adapter
cargo test -p clawft-llm --lib hermes_serving
cargo test -p clawft-service-llm --lib config_from_env
```

- **check:** pass  
- **WEFT-604 unit tests:** all pass (bridge 6, llm_adapter 23, types local_llm 2, hermes defaults, from_env)  
- Note: pre-existing `workspace::config::tests::load_merged_config_mcp_servers` fails on null MCP overlay deserialize (unrelated)

## Acceptance

| Criterion | Status |
|-----------|--------|
| Serve Hermes (ADR-060 `:8090`) → daemon, weft agent, voice reach it with zero machine-local config | **Met** — shared defaults + bridge + keyless `LocalProvider` for `local/` |
| Config layer that wins `agents.defaults.model` visible in logs | **Met** — `agents_model_source` + agent info log |
| `[providers.local]` / `[providers.ollama]` overrides work or rejected loudly | **Met** — overrides apply; unknown providers warn |

## How to verify manually

```bash
# With Hermes on :8090 (serve-llamacpp --port 8090 --alias hermes-4.3-36b):
# No weave.toml / no OPENAI_API_KEY required:
weft agent -m "What is 17 * 23?"
# Expect exit 0, answer 391; logs show local llm endpoint resolved url_source=default:local-adr060

# Explicit override still works:
# [kernel.llm] service_url / model, or LLM_SERVICE_URL / LLM_MODEL env
# [providers.local] api_base = "http://…"
```

## Commit

Branch tip: `feat(weft-604): unify local-LLM endpoint/model config (ADR-060)`  
(`git log -1 --oneline` on `wave0k/weft-604-llm-config`)
