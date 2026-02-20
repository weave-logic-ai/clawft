# Phase C2: WASM Plugin Host -- Decisions

## WIT component model vs custom ABI
- **Decision**: Used WIT (WebAssembly Interface Types) component model
- **Rationale**: WIT provides typed interfaces with automatic bindings generation. This is the standardized approach for WASM component model and avoids manual serialization/deserialization at the host-guest boundary.

## Security-first sandbox implementation
- **Decision**: Implemented all sandbox security validation (NetworkAllowlist, path canonicalization, rate limiting) before wasmtime engine integration
- **Rationale**: Security enforcement code must be correct and well-tested before any plugin can execute. Building the sandbox first means the attack surface is validated before any WASM code can call host functions.

## Rate limiting approach: fixed-window counter
- **Decision**: Used simple fixed-window rate counter instead of sliding window or token bucket
- **Rationale**: Plugins are short-lived and rate limits are primarily DoS prevention. The precision difference between fixed-window and sliding-window is irrelevant for plugin lifetimes. Simpler implementation, lower overhead.

## Private IP detection: parsed octets vs regex
- **Decision**: Parse IP addresses and check against RFC 1918 ranges numerically
- **Rationale**: String-based matching leads to false positives (e.g., `172.160.0.1` matching `172.1` prefix). Numeric comparison is correct for all edge cases. Reuses the A6 pattern from Element 03.

## Sandbox containment: canonicalize + check vs chroot
- **Decision**: Path canonicalization with prefix checking rather than OS-level chroot
- **Rationale**: WASM plugins don't have direct filesystem access -- all I/O goes through host functions. Path validation at the host function level provides equivalent security to chroot without requiring root privileges or OS-specific APIs.
