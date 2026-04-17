# Bug: weave.toml is not read by the kernel

**Severity**: Medium (config mismatch, not data loss)
**Status**: Documented, fix planned
**Discovered**: 2026-04-17

## Problem

The kernel's `load_config()` only reads:
1. `~/.clawft/config.json` (default)
2. `$CLAWFT_CONFIG` env var (override)

It does NOT read `weave.toml` from the project root, despite:
- `weaver init` generating a `weave.toml` template
- All documentation referencing `weave.toml` as the project config
- The configuration reference listing `[kernel.mesh]`, `[kernel.chain]`,
  `[tick]`, `[sources]` etc as `weave.toml` sections

This means `[kernel.mesh] enabled = true` in `weave.toml` is a dead
field — the mesh boot integration we just added reads from
`KernelConfig` which comes from the JSON config path, not TOML.

## Evidence

```
clawft-platform/src/config_loader.rs:
  load_config_raw() → searches for config.json, not weave.toml

clawft-weave/src/commands/mod.rs:
  load_config() → calls config_loader which reads JSON

weaver init generates:
  weave.toml with [kernel], [tick], [sources], [mesh] sections
  → none of these are consumed by the kernel boot
```

## Workaround

Use the JSON config path:
```bash
# Set kernel.mesh config via JSON
cat > ~/.clawft/config.json << 'EOF'
{
  "kernel": {
    "mesh": {
      "enabled": true,
      "transport": "tcp",
      "listen_addr": "0.0.0.0:9470"
    }
  }
}
EOF
```

Or set `CLAWFT_CONFIG` to point to a JSON file with kernel config.

## Fix Required

`load_config_raw()` in `clawft-platform/src/config_loader.rs` needs
to also check for `weave.toml` in the current directory and merge
it with the JSON config. TOML fields should map to the same
`Config` struct via serde.

The merge order should be:
1. `weave.toml` (project root) — project-level settings
2. `~/.clawft/config.json` — user-level settings
3. `$CLAWFT_CONFIG` — explicit override

This matches the Docker-config composability model from the
symposium decisions.

## Impact

- `[kernel.mesh]` config in weave.toml is ignored
- `[kernel.chain]` config in weave.toml is ignored
- `[tick]` and `[sources]` may work via a different code path
  (the CLI commands parse weave.toml directly in some places)
- Users following the configuration reference will think their
  settings aren't working
