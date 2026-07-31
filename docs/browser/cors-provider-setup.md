# Browser CORS Provider Setup

Per-provider recipes for calling LLM APIs from clawft-wasm in the browser.

Browser `fetch` is subject to CORS. Most provider APIs do **not** send
`Access-Control-Allow-Origin` for arbitrary web origins, so requests fail
unless you either:

1. Call a provider that **explicitly supports browser access**
   (`browserDirect: true`), or
2. Route traffic through a **CORS proxy** you control
   (`corsProxy: "https://…"`).

Field names accept both `snake_case` and `camelCase`
(`browser_direct` / `browserDirect`, `cors_proxy` / `corsProxy`,
`api_key` / `apiKey`, `api_base` / `apiBase` / `baseUrl`).

Source of truth:

- Transport: `crates/clawft-llm/src/browser_transport.rs`
  (`resolve_url`, `add_browser_headers`, `BrowserLlmClient`)
- Config: `crates/clawft-types/src/config/mod.rs` (`ProviderConfig`)
- Dashboard validator: `clawft-ui/src/lib/url-validator.ts` (WEFT-310)
- UI setup form: `clawft-ui/src/components/wasm/browser-config.tsx`

For static hosting headers and a sample Cloudflare Worker, see
[deployment.md](./deployment.md). For the full annotated config shape, see
[config-schema.md](./config-schema.md).

---

## How routing works

```
agents.defaults.model  (e.g. "openai/gpt-4o")
        │
        ▼
  resolve_provider()  — strip provider prefix, merge user overrides
        │
        ▼
  BrowserLlmClient
        │
        ├─ browser_direct = true  →  POST {base_url}/chat/completions
        │                            (+ Anthropic direct-access header)
        │
        └─ cors_proxy set         →  POST {cors_proxy}/{base_url}/chat/completions
```

`resolve_url` prepends the proxy base (no query-string rewrite):

| Mode | Example final URL |
|------|-------------------|
| Direct | `https://api.openai.com/v1/chat/completions` |
| Proxied | `https://proxy.example.com/https://api.openai.com/v1/chat/completions` |

Your proxy must accept that path form, forward the method/headers/body to the
embedded target URL, and return CORS headers for your page origin.

### Proxy URL validation (dashboard / WEFT-310)

When users enter a CORS proxy in the browser-mode setup screen:

| Input | Accepted? |
|-------|-----------|
| empty (optional until you need a proxy) | yes |
| `https://…` | yes |
| `http://localhost…`, `http://127.0.0.1…`, `http://[::1]…` | yes (dev only) |
| `http://public-host…` | **no** — keys would leave the browser unencrypted |
| other schemes (`ftp:`, etc.) | **no** |

---

## Quick matrix

| Provider | Base URL (default) | Model prefix | Browser-direct? | Typical setup |
|----------|--------------------|--------------|-----------------|---------------|
| **Anthropic** | `https://api.anthropic.com/v1` | `anthropic/` | **Yes** (opt-in header) | `browserDirect: true` |
| **OpenAI** | `https://api.openai.com/v1` | `openai/` | No | `corsProxy` required |
| **OpenRouter** | `https://openrouter.ai/api/v1` | `openrouter/` | No | `corsProxy` required |
| **Groq** | `https://api.groq.com/openai/v1` | `groq/` | No | `corsProxy` required |
| **DeepSeek** | `https://api.deepseek.com/v1` | `deepseek/` | No | `corsProxy` required |
| **Gemini** | `https://generativelanguage.googleapis.com/v1beta/openai` | `gemini/` | No | `corsProxy` required |
| **xAI** | `https://api.x.ai/v1` | `xai/` | No | `corsProxy` required |

Local endpoints (not in the AC list, but same transport):

| Provider | Base URL | Browser-direct? | Notes |
|----------|----------|-----------------|-------|
| Ollama | `http://localhost:11434/v1` | Yes (local) | Enable CORS in Ollama if origin ≠ localhost |
| LM Studio | `http://localhost:1234/v1` | Yes (local) | Same as Ollama |
| Custom OpenAI-compat | your `apiBase` | Usually no | Set `corsProxy` unless the origin allows you |

---

## Anthropic (browser-direct)

Anthropic is the primary cloud provider that supports **direct browser**
calls when you send the opt-in header. clawft adds it automatically when
`browserDirect` is true and the provider name is Anthropic-related:

```
anthropic-dangerous-direct-browser-access: true
```

Built-in config also sends `anthropic-version: 2023-06-01`.

### Config

```json
{
  "providers": {
    "anthropic": {
      "apiKey": "sk-ant-…",
      "browserDirect": true
    }
  },
  "agents": {
    "defaults": {
      "model": "anthropic/claude-sonnet-4-5-20250514",
      "maxTokens": 4096
    }
  }
}
```

### Checklist

- [ ] `browserDirect: true` (required for the header)
- [ ] Prefer a **restricted** API key (browser-visible)
- [ ] No `corsProxy` needed for Anthropic direct mode
- [ ] If you still set `corsProxy`, the client routes through the proxy
      even when direct mode is set — leave it unset for true direct calls

### Common errors

| Symptom | Cause | Fix |
|---------|-------|-----|
| CORS error in DevTools | `browserDirect` false or missing | Set `browserDirect: true` |
| 401 / authentication | missing or placeholder key | Set `apiKey` or inject via `set_env` after init |
| 400 about version | stripped `anthropic-version` | Do not clear built-in headers unless you re-add the version |

---

## OpenAI (CORS proxy)

OpenAI’s public API does not allow arbitrary browser origins. Use a proxy
you control (or a local proxy during development).

### Config

```json
{
  "providers": {
    "openai": {
      "apiKey": "sk-…",
      "browserDirect": false,
      "corsProxy": "https://cors-proxy.example.com"
    }
  },
  "agents": {
    "defaults": {
      "model": "openai/gpt-4o",
      "maxTokens": 4096
    }
  }
}
```

Proxied request URL:

```
https://cors-proxy.example.com/https://api.openai.com/v1/chat/completions
```

### Checklist

- [ ] `corsProxy` is **HTTPS** in production (validator rejects public HTTP)
- [ ] Proxy allowlists `api.openai.com` only
- [ ] Forward `Authorization` and `Content-Type`
- [ ] Answer OPTIONS preflight with `Access-Control-Allow-*`

---

## OpenRouter (CORS proxy)

OpenRouter is the default **fallback** when a model string has no known
prefix and an OpenRouter key is present (see `resolve_provider` in
`clawft-wasm`). It still needs a CORS proxy in the browser.

### Config

```json
{
  "providers": {
    "openrouter": {
      "apiKey": "sk-or-…",
      "browserDirect": false,
      "corsProxy": "https://cors-proxy.example.com"
    }
  },
  "agents": {
    "defaults": {
      "model": "openrouter/meta-llama/llama-3.1-8b-instruct:free",
      "maxTokens": 4096
    }
  }
}
```

Optional base override (same as the test fixture):

```json
"openrouter": {
  "apiKey": "sk-or-…",
  "apiBase": "https://openrouter.ai/api/v1",
  "corsProxy": "https://cors-proxy.example.com"
}
```

### Checklist

- [ ] Prefer the `openrouter/` model prefix so routing is explicit
- [ ] Unprefixed third-party model ids (e.g. `meta-llama/…`) fall back to
      OpenRouter only if that provider has a non-empty `apiKey`
- [ ] Proxy allowlist: `openrouter.ai`

---

## Groq (CORS proxy)

```json
{
  "providers": {
    "groq": {
      "apiKey": "gsk_…",
      "browserDirect": false,
      "corsProxy": "https://cors-proxy.example.com"
    }
  },
  "agents": {
    "defaults": {
      "model": "groq/llama-3.1-70b-versatile",
      "maxTokens": 4096
    }
  }
}
```

| Item | Value |
|------|--------|
| Default base | `https://api.groq.com/openai/v1` |
| Env var (native) | `GROQ_API_KEY` |
| Proxy allowlist host | `api.groq.com` |

---

## DeepSeek (CORS proxy)

```json
{
  "providers": {
    "deepseek": {
      "apiKey": "sk-…",
      "browserDirect": false,
      "corsProxy": "https://cors-proxy.example.com"
    }
  },
  "agents": {
    "defaults": {
      "model": "deepseek/deepseek-chat",
      "maxTokens": 4096
    }
  }
}
```

| Item | Value |
|------|--------|
| Default base | `https://api.deepseek.com/v1` |
| Env var (native) | `DEEPSEEK_API_KEY` |
| Proxy allowlist host | `api.deepseek.com` |

---

## Gemini (CORS proxy)

Uses Google’s **OpenAI-compatible** endpoint (not the raw Generative Language
REST shape).

```json
{
  "providers": {
    "gemini": {
      "apiKey": "AIza…",
      "browserDirect": false,
      "corsProxy": "https://cors-proxy.example.com"
    }
  },
  "agents": {
    "defaults": {
      "model": "gemini/gemini-2.5-flash",
      "maxTokens": 4096
    }
  }
}
```

| Item | Value |
|------|--------|
| Default base | `https://generativelanguage.googleapis.com/v1beta/openai` |
| Env var (native) | `GOOGLE_GEMINI_API_KEY` |
| Proxy allowlist host | `generativelanguage.googleapis.com` |

---

## xAI / Grok (CORS proxy)

```json
{
  "providers": {
    "xai": {
      "apiKey": "xai-…",
      "browserDirect": false,
      "corsProxy": "https://cors-proxy.example.com"
    }
  },
  "agents": {
    "defaults": {
      "model": "xai/grok-3-mini",
      "maxTokens": 4096
    }
  }
}
```

| Item | Value |
|------|--------|
| Default base | `https://api.x.ai/v1` |
| Env var (native) | `XAI_API_KEY` |
| Proxy allowlist host | `api.x.ai` |

---

## Multi-provider example

```json
{
  "providers": {
    "anthropic": {
      "apiKey": "sk-ant-…",
      "browserDirect": true
    },
    "openai": {
      "apiKey": "sk-…",
      "corsProxy": "https://cors-proxy.example.com"
    },
    "openrouter": {
      "apiKey": "sk-or-…",
      "corsProxy": "https://cors-proxy.example.com"
    },
    "groq": {
      "apiKey": "gsk_…",
      "corsProxy": "https://cors-proxy.example.com"
    },
    "deepseek": {
      "apiKey": "sk-…",
      "corsProxy": "https://cors-proxy.example.com"
    },
    "gemini": {
      "apiKey": "AIza…",
      "corsProxy": "https://cors-proxy.example.com"
    },
    "xai": {
      "apiKey": "xai-…",
      "corsProxy": "https://cors-proxy.example.com"
    }
  },
  "agents": {
    "defaults": {
      "model": "anthropic/claude-sonnet-4-5-20250514",
      "maxTokens": 4096
    }
  }
}
```

Switch models at runtime by changing `agents.defaults.model` before `init`,
or by re-initializing with a new config. The active model’s **prefix**
selects which `providers.*` entry supplies `apiKey`, `corsProxy`, and
`browserDirect`.

---

## Building or choosing a CORS proxy

Requirements for clawft’s `{proxy}/{full-target-url}` convention:

1. Parse the path after your proxy origin as the absolute target URL.
2. Forward method, body, and auth headers (`Authorization`, plus any
   provider-specific headers such as `anthropic-version`).
3. Respond to `OPTIONS` with CORS headers for your page origin (prefer an
   explicit origin over `*` when credentials matter).
4. **Allowlist** target hosts (the seven API hosts above). Never deploy an
   open relay — API keys transit the proxy.

A minimal Cloudflare Worker sketch lives in
[deployment.md § CORS Proxy Setup](./deployment.md#cors-proxy-setup).
Adapt it so the path embeds the target URL (matching `resolve_url`), not
only a `?url=` query parameter, **or** change your deployment to match
whichever rewrite style your proxy already implements end-to-end.

### Local development proxies

HTTP is allowed only for loopback hosts:

```text
http://localhost:8080/
http://127.0.0.1:8080/
http://[::1]:8080/
```

Examples: a local Node proxy, `cors-anywhere` pinned to localhost, or a
dev Worker tunnel. Production must use `https://`.

---

## Security notes

- **Keys are visible to page JS.** Prefer scoped / browser-restricted keys;
  never bake secrets into the WASM binary or static HTML.
- **Dashboard storage** encrypts keys with Web Crypto AES-GCM before
  IndexedDB (see `browser-config.tsx`); the live `init` config still needs
  the plaintext key in memory for `Authorization`.
- **Prefer allowlisted proxies.** An open CORS proxy is an API-key exfil
  path.
- **CSP**: allow `connect-src` for provider hosts (direct mode) or only your
  proxy origin (proxied mode). See [deployment.md](./deployment.md).

---

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| `Access-Control-Allow-Origin` error | Provider blocks browser origin; no proxy | Set `corsProxy` or use Anthropic direct |
| Network error to `http://…` from HTTPS page | Mixed content | Use HTTPS proxy or localhost only |
| Validator: “HTTP CORS proxy URLs are only allowed for localhost” | Public `http://` proxy | Switch to `https://` |
| `no API key configured for provider matching model '…'` | Empty `apiKey` for the resolved provider | Fill the matching `providers.<name>.apiKey` |
| `no provider found for model '…'` | Unknown prefix and no fallback key | Use `provider/model` form or configure a key |
| 404 from proxy | Proxy expects `?url=` but client uses path prepend | Align proxy with `{proxy}/{target}` |
| Anthropic CORS despite key | Missing `browserDirect: true` | Enable direct mode |

---

## Related docs

- [config-schema.md](./config-schema.md) — full browser-mode config reference
- [quickstart.md](./quickstart.md) — five-minute WASM harness
- [deployment.md](./deployment.md) — hosting, headers, sample Worker
- [api-reference.md](./api-reference.md) — `init` / `send_message` / `set_env`
- [architecture.md](./architecture.md) — browser vs native platform split
- [ADR-083](../adr/adr-083-browser-wasm-support.md) — browser WASM decision record
- [guides/providers.md](../guides/providers.md) — native provider routing
