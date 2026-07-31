# Browser WASM Threat Model: API Keys in JS-Readable Memory

**Issue**: [WEFT-406](../plans/plane-board-inventory.md)  
**Source**: `.planning/reviews/0.7.0-release-gate/16-browser-wasm.md` — cross-cutting risks  
**Related**: [architecture.md](./architecture.md), [deployment.md](./deployment.md),
[cors-provider-setup.md](./cors-provider-setup.md), [config-schema.md](./config-schema.md),
[ADR-083](../adr/adr-083-browser-wasm-support.md), `clawft-types::SecretString`,
`clawft-llm::browser_transport::BrowserLlmClient`

This note documents the residual risk that provider API keys, once injected into
clawft-wasm via `init(config_json)`, live in WebAssembly linear memory that is
**fully readable from JavaScript** on the same origin. It is documentation-first
(mitigation product work is out of scope for WEFT-406).

---

## Summary (for operators)

| Fact | Implication |
|------|-------------|
| WASM linear memory is not a secret store | Any script on the host page can scan or export the module’s `WebAssembly.Memory` and recover plaintext keys |
| `SecretString` redacts logs / `Debug` / serde only | It does **not** encrypt, isolate, or zeroize memory against a same-origin attacker |
| Host UI may encrypt at rest (IndexedDB + Web Crypto) | Encryption stops at rest; `init` still needs the **plaintext** key in JS heap and then in WASM |
| XSS on the host page = key theft | CSP, dependency hygiene, and never embedding long-lived secrets are the primary controls |
| Production should prefer a backend / proxy | Keep durable provider keys off the browser; use short-lived session tokens when a key must touch the tab |

**Never embed API keys in the WASM binary, static HTML/JS, or public config
checked into git.**

---

## Trust boundaries

```
┌──────────────────────────────────────────────────────────────────┐
│  Browser origin (same page / same process)                         │
│                                                                    │
│  JS heap                    WASM linear memory                     │
│  ────────                   ──────────────────                     │
│  · config form / React      · BrowserLlmClient.api_key: String     │
│  · init(config_json)        · Config / provider structs            │
│  · decrypted IndexedDB key  · Authorization header scratch buffers │
│  · any third-party script   · temporary String clones on complete()│
│                                                                    │
│  Attacker with XSS / malicious extension of same origin            │
│  can read ALL of the above.                                        │
└──────────────────────────────────────────────────────────────────┘
                              │ HTTPS fetch
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Network edge                                                       │
│  · Provider API (browserDirect)  OR  operator-controlled CORS proxy │
│  · Must not be an open relay; keys appear in Authorization headers  │
└──────────────────────────────────────────────────────────────────┘
                              │ (recommended production path)
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│  Operator backend (trusted)                                         │
│  · Holds long-lived provider keys                                   │
│  · Issues short-lived tokens / proxies LLM calls                    │
│  · Browser never sees the durable secret                            │
└──────────────────────────────────────────────────────────────────┘
```

| Zone | Trusted for durable secrets? | Notes |
|------|------------------------------|--------|
| Host page JS + WASM (same origin) | **No** | Shared attack surface; treat as one principal |
| Browser extensions with page access | **No** | Can read page/WASM memory depending on permissions |
| IndexedDB ciphertext + non-extractable CryptoKey | **At rest only** | Defends disk theft / casual inspection; not live XSS |
| Operator backend / private CORS proxy | **Yes** (if locked down) | Preferred vault for long-lived keys |
| Third-party CDN scripts on the page | **No** | Supply-chain XSS equivalent |

---

## How keys enter the browser path today

### 1. `init(config_json)` (wasm-bindgen)

Browser mode has no process environment for secrets. Config JSON must carry a
non-empty `providers.<name>.api_key` / `apiKey` for the model’s provider or
`init` fails. See [config-schema.md](./config-schema.md).

Source: `crates/clawft-wasm/src/lib.rs` (browser entry) resolves the matching
provider, reads `user_cfg.api_key.expose()`, and constructs:

```text
BrowserLlmClient::with_api_key(config, api_key.to_string(), …)
```

### 2. `BrowserLlmClient` field

```text
// crates/clawft-llm/src/browser_transport.rs
pub struct BrowserLlmClient {
    …
    api_key: Option<String>,   // plaintext for the lifetime of the client
    …
}
```

On each `complete()` / `complete_stream_callback()`, `resolve_api_key()` returns
a **clone** of that string for the `Authorization: Bearer …` header. Debug
formatting masks the key as `***`; the bytes remain in linear memory.

### 3. `SecretString` (config type only)

```text
// crates/clawft-types/src/secret.rs
// Debug/Display → [REDACTED]; Serialize → ""; expose() → &str
```

`SecretString` prevents accidental logging and config re-serialization of
secrets. It is **not** a cryptographic enclave:

- No encryption at rest inside WASM
- No hardware isolation (no SGX / Secure Enclave)
- No guaranteed zeroization on drop (plain `String`)
- `expose()` yields a normal UTF-8 view over WASM memory

The release-gate note described the client as holding `api_key: SecretString`;
the live field is `Option<String>`. The threat is the same either way: **same-
origin JS can read the bytes.**

### 4. Host UI (clawft-ui) — partial mitigation at rest

`clawft-ui/src/components/wasm/browser-config.tsx`:

- Encrypts the API key with Web Crypto **AES-256-GCM** before IndexedDB
- Stores a **non-extractable** `CryptoKey` in IndexedDB for decrypt
- Still passes **plaintext** `api_key` into the config object handed to
  `onConfigured` → wasm `init`

At-rest encryption reduces risk if the user’s profile directory is copied
offline. It does **not** protect against XSS while the tab is live.

### 5. Test harness

`crates/clawft-wasm/www/index.html` accepts keys in a JSON editor for local
demos. Treat harness keys as disposable; never use production account keys in
public demos.

---

## Threat scenarios

### T1 — XSS on the host page (primary)

**Attacker capability**: Inject or load script on the clawft origin (stored XSS,
compromised dependency, malicious browser extension with host access, open
`innerHTML` sink, etc.).

**Impact**:

1. Call exported wasm-bindgen helpers if any re-expose config (or patch them).
2. Read `WebAssembly.Memory` buffer(s) and scan for known key prefixes
   (`sk-ant-`, `sk-or-`, `sk-…`, `gsk_…`, …).
3. Intercept `fetch` / monkey-patch `reqwest`’s underlying Fetch to log
   `Authorization` headers.
4. Exfiltrate to an attacker-controlled origin.

**Severity**: High for any deployment that injects a **long-lived, high-
privilege** provider key into the tab.

### T2 — Memory scraping without classic XSS

DevTools, compromised shared machines, malicious co-tenants in poorly
isolated multi-tenant embeddings, or extensions that can debug the page can
inspect heap/WASM memory even without a durable XSS bug.

### T3 — Open or hostile CORS proxy

If `corsProxy` points at an untrusted or open relay, the proxy sees full
request headers including `Authorization` / `x-api-key`. This is network-edge
exfil independent of WASM memory. Dashboard validation (WEFT-310) rejects
public `http://` proxies; operators must still **allowlist destinations** on
the proxy itself. See [cors-provider-setup.md](./cors-provider-setup.md).

### T4 — Embedded secrets in static assets

Keys compiled into the `.wasm` data section, hardcoded in `www/*.html`, or
committed in playground JSON are world-readable via View Source / download.
This is always out of policy.

### T5 — Supply-chain script on the page

Any third-party script with same-origin execution (analytics, widgets, hotjar-
style heatmaps with full DOM access) inherits T1.

### Non-goals / weaker threats

| Scenario | Notes |
|----------|--------|
| Cross-origin iframe without `postMessage` leaks | Same-origin policy still applies; do not widen with overly broad COOP/COEP mistakes or `document.domain` |
| Pure network eavesdropper on HTTPS | TLS protects transit; threat is client-side principal, not MITM on well-configured HTTPS |
| Provider-side breach | Out of scope for this note; still argue for short-lived / scoped keys |

---

## What WASM does **not** give you

Common misconception: “the key is in Rust/WASM, so JavaScript cannot see it.”

False. In the browser:

1. Modules share the page’s origin and process.
2. `WebAssembly.Memory` is a JS-accessible `ArrayBuffer` (or SharedArrayBuffer
   when cross-origin isolated).
3. wasm-bindgen passes strings across the boundary as ordinary JS strings
   during `init` and often retains copies on both sides.
4. There is no browser API that makes linear memory opaque to same-origin
   script.

Compare:

| Runtime | Key material isolation |
|---------|------------------------|
| Native CLI (`std::env` / OS secret stores) | Process boundary; not readable by random web scripts |
| Server-side agent | Keys stay on the host; browser gets only session UX |
| Browser WASM agent (this path) | **No isolation from host JS** |

---

## Mitigations (recommended)

Ordered from **strongest / preferred for production** to **defense-in-depth
on the playground path**.

### M1 — Never embed durable secrets (mandatory)

- Do not put production keys in git, CI artifacts, static HTML, or the WASM
  binary.
- Demo harnesses: use free-tier / throwaway keys with hard spend caps.
- Document “bring your own key” only when the user understands T1.

### M2 — Backend proxy / BFF holds the long-lived key (preferred production)

```text
Browser WASM  ── short-lived session token ──▶  Your API / proxy
                                                   │
                                                   ▼
                                            Provider API (secret stays server-side)
```

Patterns:

- **LLM reverse proxy**: browser calls only your origin; server attaches the
  real provider key.
- **Session minting**: user authenticates to your backend; backend returns a
  **short-lived, audience-bound** token used as `api_key` in browser config
  (or only as a Bearer to your proxy, not the upstream provider).
- Aligns with ADR-083 hybrid model: production “server-attached” mode already
  exists via dashboard / axum paths — prefer that when secrets matter.

Configure browser providers with `corsProxy` / `apiBase` pointing at **your**
HTTPS endpoint, not an open public CORS worker.

### M3 — Short-lived / scoped provider tokens

When a secret must enter the tab:

| Property | Guidance |
|----------|----------|
| TTL | Minutes to hours, not months; rotate aggressively |
| Scope | Single model / project / spend limit where the provider allows |
| Blast radius | Dedicated “browser playground” key, never org-root key |
| Revocation | Ability to kill the key without rotating all infrastructure |
| Logging | Provider audit logs + your proxy access logs |

Ephemeral keys limit damage if T1 occurs after the token expires.

### M4 — Content Security Policy (CSP)

Deployment should ship a strict CSP so arbitrary script injection is harder:

```http
Content-Security-Policy:
  default-src 'self';
  script-src 'self' 'wasm-unsafe-eval';
  connect-src 'self' https://api.anthropic.com https://your-proxy.example.com;
  object-src 'none';
  base-uri 'self';
  frame-ancestors 'none';
```

Notes:

- `'wasm-unsafe-eval'` (or equivalent) is typically required to compile WASM.
- Prefer **no** `'unsafe-inline'` / `'unsafe-eval'` for scripts; use nonces or
  hashes if inline bootstrapping is unavoidable.
- `connect-src` should list only provider origins (direct mode) or **only**
  your proxy (proxied mode).
- Pair with COOP/COEP only when you need cross-origin isolation; do not
  weaken isolation casually. See [deployment.md](./deployment.md).

CSP is necessary but not sufficient (XSS gadgets, browser bugs, extensions).

### M5 — Trusted CORS proxy only

- Allowlist upstream hosts (provider API domains only).
- Require HTTPS for non-localhost proxies (dashboard already enforces this).
- Do not set `Access-Control-Allow-Origin: *` with credentialed key traffic in
  production; pin to your app origin.
- Log and rate-limit; treat proxy compromise as key compromise.

### M6 — Host UI hygiene

- Keep encrypting keys at rest (IndexedDB + Web Crypto) for local UX.
- Minimize time plaintext sits in React state; clear form fields after
  `init` when possible.
- Avoid logging config objects that include `api_key`.
- Subresource integrity (SRI) for any third-party scripts; prefer zero
  third-party script on the agent origin.
- Separate “playground” origin from authenticated product origin when
  feasible.

### M7 — Key handling inside WASM (future / optional code work)

Not required to close WEFT-406 (docs-only). Useful follow-ups:

| Idea | Benefit | Caveat |
|------|---------|--------|
| Zeroize `api_key` buffers after each `complete()` | Shrinks window for opportunistic scrapes | XSS can still hook `complete` / Fetch; clones may linger in allocator free lists |
| Hold key only in JS, pass per-request via a host callback | Reduces long-lived WASM copies | JS heap still exposed to same XSS |
| Prefer proxy token that is useless upstream if stolen | Limits value of scraped material | Requires backend |
| Avoid retaining full config JSON strings after parse | Fewer residual copies in linear memory | Init path must still parse once |

`SecretString` improvements (zeroize-on-drop, `Zeroizing<String>`) help native
forensics and accidental logs; they **do not** close T1 in the browser.

### M8 — Operational controls

- Per-environment keys; never share prod keys with local WASM demos.
- Alert on unusual spend / geo / User-Agent from browser-tagged keys.
- Document key rotation in runbooks when XSS or proxy incidents occur.
- For enterprise: force server-side agent mode; disable “paste API key into
  browser” entirely.

---

## Production deployment checklist

Use this when shipping clawft-wasm (or clawft-ui browser mode) beyond a local
demo:

- [ ] **No long-lived provider keys** in the browser; backend proxy or short-
      lived mint path is default.
- [ ] If BYOK is required: scoped keys, spend caps, easy revoke, user education
      that XSS steals the key.
- [ ] CSP with `script-src` locked down and `connect-src` limited to proxy /
      provider.
- [ ] CORS proxy (if any) is first-party, allowlisted, HTTPS, origin-pinned.
- [ ] HTTPS everywhere (secure context for OPFS / modern APIs).
- [ ] No secrets in static assets or WASM data sections (binary audit /
      `strings` on artifacts if needed).
- [ ] Dependency and XSS review on the host SPA; SRI or bundler-owned scripts
      only.
- [ ] Separate demo keys from production org keys.
- [ ] Incident plan: revoke browser-scoped keys, rotate proxy credentials.

---

## Developer / playground guidance

Acceptable for local development:

```json
{
  "providers": {
    "anthropic": {
      "apiKey": "sk-ant-…-dev-only",
      "browserDirect": true
    }
  },
  "agents": {
    "defaults": { "model": "anthropic/claude-sonnet-4-5-20250514" }
  }
}
```

Not acceptable for production static hosting:

- Baking that JSON into `index.html`
- Committing real keys next to the harness
- Pointing `corsProxy` at a public open CORS service

Prefer:

```bash
# Local only — inject at runtime, never commit
# (harness: paste into config editor; do not save to git)
```

Or run the **native** CLI / server-attached UI so keys stay in the process
environment or server secret store.

---

## Residual risk statement

Even with perfect CSP, first-party proxy, and short-lived tokens:

> Any secret that must be presented by the browser to call an upstream API is
> readable by code running in that page. WASM does not create a second trust
> domain inside the tab.

The residual risk is **accepted for demo / BYOK playgrounds** when users
supply their own limited keys, and **should not be accepted for org-root or
unbounded production keys**. Server-side key custody is the control that
actually changes the threat model.

---

## Code map (for audits)

| Component | Path | Role |
|-----------|------|------|
| Browser entry / `init` | `crates/clawft-wasm/src/lib.rs` | Parses config, injects key into client |
| LLM client | `crates/clawft-llm/src/browser_transport.rs` | Holds `api_key: Option<String>`, attaches Bearer |
| Secret wrapper | `crates/clawft-types/src/secret.rs` | Redacts logs/serde; not isolation |
| Provider config | `crates/clawft-types/src/config/mod.rs` | `api_key`, `cors_proxy`, `browser_direct` |
| UI encrypt-at-rest | `clawft-ui/src/components/wasm/browser-config.tsx` | AES-GCM + IndexedDB; plaintext to `init` |
| Proxy URL rules | `clawft-ui/src/lib/url-validator.ts` | WEFT-310 HTTPS / localhost checks |
| Platform env redaction | `crates/clawft-platform/src/browser/env.rs` | Sensitive env key naming (Debug) |

---

## Related docs

| Doc | Why |
|-----|-----|
| [architecture.md](./architecture.md) | Browser vs native split, data flow |
| [deployment.md](./deployment.md) | Hosting headers, sample proxy, short security bullets |
| [cors-provider-setup.md](./cors-provider-setup.md) | Per-provider CORS + proxy security notes |
| [config-schema.md](./config-schema.md) | `apiKey` required in browser |
| [quickstart.md](./quickstart.md) | Local harness |
| [../reference/security.md](../reference/security.md) | Native tool / prompt-injection policy (different threat model) |
| [ADR-083](../adr/adr-083-browser-wasm-support.md) | Hybrid browser WASM decision |

---

## Acceptance (WEFT-406)

| Criterion | Status |
|-----------|--------|
| Threat-model note under `docs/browser/` | This document |
| XSS exposure documented | § Threat scenarios T1–T2, T5 |
| WASM-memory readability documented | § Trust boundaries, § What WASM does not give you |
| Mitigations: CSP, key scoping, ephemeral keys | § M3, M4, checklist |
| Mitigations: proxy, never embed secrets | § M1, M2, M5 |
| Production recommendations | § Production deployment checklist |
| Optional zeroize-after-`complete()` prototype | Deferred — noted under § M7 as follow-up engineering |
