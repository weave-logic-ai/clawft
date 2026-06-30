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
