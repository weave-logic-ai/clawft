# Agent harness (WeftOS)

Triple loop + master controller. Guide:
`docs/guides/agent-harness-triple-loop.md`. ADR-098: no environment overlay.

Every turn: score (`scripts/metaharness/score.sh`), research
(`crosscut.mjs` + `plane-dag.sh ready`), then at most one development
claim. Jobs live in `process-compose.yaml` (`job_*`). This PC listens on
`:18090` / `:18091`, never Forge's `:18081`.
