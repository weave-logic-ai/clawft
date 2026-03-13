# Browser Deployment

This guide covers deploying clawft-wasm to production web environments.

## Static Hosting Options

clawft-wasm consists of static files (HTML, JS, WASM) and can be deployed
to any static hosting provider.

### Vercel

```bash
# Build
cd crates/clawft-wasm
wasm-pack build --target web --release --no-default-features --features browser -- --no-default-features --features browser

# vercel.json in project root
```

```json
{
  "buildCommand": "wasm-pack build crates/clawft-wasm --target web --release --no-default-features --features browser -- --no-default-features --features browser",
  "outputDirectory": "crates/clawft-wasm/www",
  "headers": [
    {
      "source": "**/*.wasm",
      "headers": [
        { "key": "Content-Type", "value": "application/wasm" },
        { "key": "Cross-Origin-Embedder-Policy", "value": "require-corp" },
        { "key": "Cross-Origin-Opener-Policy", "value": "same-origin" }
      ]
    }
  ]
}
```

### Netlify

Create `netlify.toml`:

```toml
[build]
  command = "wasm-pack build crates/clawft-wasm --target web --release --no-default-features --features browser -- --no-default-features --features browser"
  publish = "crates/clawft-wasm/www"

[[headers]]
  for = "/*.wasm"
  [headers.values]
    Content-Type = "application/wasm"
    Cross-Origin-Embedder-Policy = "require-corp"
    Cross-Origin-Opener-Policy = "same-origin"
```

### S3 + CloudFront

1. Build locally and upload the `www/` and `pkg/` directories to an S3 bucket.
2. Configure CloudFront with the bucket as origin.
3. Add a custom response header policy for `.wasm` files:
   - `Content-Type: application/wasm`
4. Set cache TTLs appropriately (WASM files are immutable per build, so long
   TTLs with cache-busting hashes work well).

```bash
# Upload
aws s3 sync crates/clawft-wasm/www s3://my-bucket/clawft/ --delete
aws s3 sync crates/clawft-wasm/pkg s3://my-bucket/clawft/pkg/ --delete

# Invalidate cache
aws cloudfront create-invalidation --distribution-id EDIST123 --paths "/clawft/*"
```

### GitHub Pages

Add a GitHub Actions workflow:

```yaml
name: Deploy WASM
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - run: cargo install wasm-pack
      - run: |
          cd crates/clawft-wasm
          wasm-pack build --target web --release --no-default-features --features browser -- --no-default-features --features browser
      - uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: crates/clawft-wasm/www
```

## CORS Proxy Setup

LLM provider APIs that do not support browser CORS require a proxy. clawft
supports per-provider CORS proxy configuration via the `corsProxy` field.

### Config example

```json
{
  "providers": {
    "openrouter": {
      "apiKey": "sk-or-...",
      "corsProxy": "https://cors-proxy.example.com",
      "browserDirect": false
    },
    "anthropic": {
      "apiKey": "sk-ant-...",
      "browserDirect": true
    }
  }
}
```

When `browserDirect` is `true`, the `BrowserHttpClient` sends requests
directly to the provider API. When `false`, requests are routed through
the specified `corsProxy` URL.

### Minimal CORS proxy (Cloudflare Worker)

```javascript
export default {
  async fetch(request) {
    const url = new URL(request.url);
    const target = url.searchParams.get("url");
    if (!target) {
      return new Response("Missing ?url= parameter", { status: 400 });
    }

    const resp = await fetch(target, {
      method: request.method,
      headers: request.headers,
      body: request.body,
    });

    const headers = new Headers(resp.headers);
    headers.set("Access-Control-Allow-Origin", "*");
    headers.set("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
    headers.set("Access-Control-Allow-Headers", "Content-Type, Authorization, x-api-key, anthropic-version");

    return new Response(resp.body, {
      status: resp.status,
      headers,
    });
  },
};
```

## HTTPS Requirement

Browser WASM features require a **secure context** (HTTPS or `localhost`):

- **OPFS** (Origin Private File System) -- Only available in secure contexts.
  clawft-wasm uses OPFS for persistent storage when available.
- **Service Workers** -- Required for offline caching of the WASM binary.
- **Fetch API** -- Mixed-content restrictions prevent HTTP API calls from
  HTTPS pages.
- **SharedArrayBuffer** (future) -- Requires `Cross-Origin-Isolation` headers,
  which only work over HTTPS.

For local development, `http://localhost` is treated as a secure context by
all major browsers. For production, always deploy behind HTTPS.

## Required Headers for WASM Files

The server must set the following headers for `.wasm` files:

### Mandatory

| Header | Value | Reason |
|--------|-------|--------|
| `Content-Type` | `application/wasm` | Required for `WebAssembly.compileStreaming()`. Without this, the browser falls back to the slower `WebAssembly.compile()` path. |

### Recommended

| Header | Value | Reason |
|--------|-------|--------|
| `Cross-Origin-Embedder-Policy` | `require-corp` | Enables `SharedArrayBuffer` for future multi-threaded WASM. |
| `Cross-Origin-Opener-Policy` | `same-origin` | Required alongside COEP for cross-origin isolation. |
| `Cache-Control` | `public, max-age=31536000, immutable` | WASM binaries are immutable per build. Use content hashing in filenames for cache busting. |

### Example nginx config

```nginx
location ~ \.wasm$ {
    types { application/wasm wasm; }
    add_header Cross-Origin-Embedder-Policy "require-corp" always;
    add_header Cross-Origin-Opener-Policy "same-origin" always;
    add_header Cache-Control "public, max-age=31536000, immutable" always;
}
```

## Security Considerations

- **Never embed API keys in the WASM binary or static HTML.** Inject keys
  at runtime via `set_env()` or a server-side token exchange.
- **Use a CORS proxy with allowlisting.** Do not deploy an open CORS proxy.
  Restrict the `target` parameter to known LLM API domains.
- **Enable CSP headers.** Add `script-src 'self' 'wasm-unsafe-eval'` to
  allow WASM execution while blocking inline scripts.
- **Audit binary size.** Large WASM binaries may indicate unintended
  native dependencies being compiled in.
