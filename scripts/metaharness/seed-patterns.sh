#!/usr/bin/env bash
# Seed AgentDB / ruflo memory patterns for MetaHarness foundation (WEFT-726).
# Idempotent: same keys overwrite with current text. Never stores secrets.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
ONLY="${1:-}"
if [[ "${1:-}" == "--only" ]]; then
  ONLY="${2:-}"
fi

NS="patterns"
store() {
  local key="$1"
  local value="$2"
  if [[ -n "$ONLY" && "$ONLY" != "$key" && "$ONLY" != "${key#pattern-}" ]]; then
    return 0
  fi
  echo "→ store $NS/$key"
  if npx --yes ruflo@3.32.38 memory store --key "$key" --namespace "$NS" --value "$value" 2>/dev/null; then
    return 0
  fi
  if npx --yes @claude-flow/cli@3.32.38 memory store --key "$key" --namespace "$NS" --value "$value" 2>/dev/null; then
    return 0
  fi
  echo "WARN: ruflo/claude-flow memory store failed for $key (CLI absent?)" >&2
  return 0
}

store "pattern-metaharness-foundation" \
  "ADR-096/097: MetaHarness foundational for agent+fusion+ALL data surfaces under same governance. Freeze model; evolve harness/ViewSpec/policies. Flywheel evaluate→receipt→promote. Grok: .grok/rules/metaharness.md. Scripts: scripts/metaharness/*. weft runtime must not require MH. Plane WEFT-724+"

store "pattern-graph-views-fusion" \
  "Sensor fusion operationally = Graph Views (docs/research/graph-views.md F1-F10). Purpose-scoped multi-source live graphs; caps; promote to BVH+chain (ADR-078). Batch WCC/PageRank per View edge table (ADR-095). Not one global unbounded graph. rUv WorldGraph analogue."

store "pattern-release-gate" \
  "Release readiness: scripts/build.sh gate before commit claims. MetaHarness task weft-gate. Dual cargo-audit+npm-audit via scripts/build.sh. Never claim 0.8 ship without gate green. Plane close with commit SHA."

store "pattern-plane-dag" \
  "Plane is SoT. scripts/plane-dag.sh ready/claim/done. Two-label rule (workstream+finding-type). 0.8.x ship track; 0.7.x locked. Task weft-plane-dag. Claim before code; close with shipped/commits/tests/build."

store "pattern-viewspec-flywheel" \
  "ViewSpec fixtures in config/views/. Flywheel anchors .metaharness/eval/. Evaluate-only by default; promote needs confirm+keys. Never silent ViewSpec/promote-gate changes. WEFT-727."

store "pattern-data-surface-governance" \
  "ADR-097: all WeftOS data surfaces (fs, sqlite/agentdb, sensors, mesh nodes, substrate, chain, BVH, ECC, View tables) under same MH governance: inventory, ACL, receipt for policy, chain audit for state. No surface is 'shadow' ungoverned."

store "pattern-ruv-worldgraph" \
  "rUv WorldGraph ADR-139 = typed twin+provenance. OccWorld=predictive (LeWM analogue). Map WorldGraph→Graph Views; constraint fusion→live attach; MetaHarness evolves policies as churn."

echo "Done. Manifest: .metaharness/patterns-manifest.md"
echo "Verify: npx ruflo@3.32.38 memory search --query 'MetaHarness Graph Views' --namespace patterns"
