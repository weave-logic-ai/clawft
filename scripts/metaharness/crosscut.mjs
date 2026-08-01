#!/usr/bin/env node
/**
 * Capability crosscut: traverse catalog → search WeftOS brain → classify
 * SEE | WIRE | BUILD | UPSTREAM for the Darwin flywheel loop.
 *
 * Usage:
 *   node scripts/metaharness/crosscut.mjs           # write latest JSON + md summary
 *   node scripts/metaharness/crosscut.mjs --json    # stdout only
 *
 * Requires: weftos-brain index (auto-rebuilds if missing).
 */
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "../..");
const BRAIN = join(ROOT, ".metaharness", "brain");
const OUT_JSON = join(BRAIN, "crosscut-latest.json");
const OUT_MD = join(ROOT, "docs/research/crosscut-latest.md");

/** Ecosystem + WeftOS-native nodes for traverse/compare. */
const NODES = [
  {
    id: "ruflo",
    domain: "orchestrator",
    ruv: "Ruflo/claude-flow swarm, MCP, memory, team bus, hooks",
    query: "ruflo swarm mcp team bus memory hooks orchestration",
    weftHints: [".grok/rules/ruflo-grok.md", "scripts/grok-team-bus.mjs", "package.json"],
  },
  {
    id: "agentdb",
    domain: "memory",
    ruv: "AgentDB RVF, HNSW, patterns, ReasoningBank, causal graph",
    query: "agentdb rvf hnsw pattern reasoningbank causal memory",
    weftHints: [".metaharness/patterns-manifest.md", "scripts/metaharness/seed-patterns.sh"],
  },
  {
    id: "agenticow",
    domain: "memory",
    ruv: "COW vector branching (git for agent memory)",
    query: "cow-memory agenticow branch checkpoint promote",
    weftHints: ["crates/clawft-cow-memory"],
  },
  {
    id: "metaharness-read",
    domain: "harness",
    ruv: "score + genome + oia-audit + mcp-scan (ADR-150 READ)",
    query: "metaharness score genome flywheel foundation",
    weftHints: ["scripts/metaharness/score.sh", ".metaharness"],
  },
  {
    id: "metaharness-flywheel",
    domain: "harness",
    ruv: "@metaharness/flywheel promote receipts lineage",
    query: "flywheel promote receipt meetspromotionrule SEE WIRE",
    weftHints: ["scripts/metaharness/flywheel-measure.mjs", ".metaharness/flywheel"],
  },
  {
    id: "metaharness-darwin",
    domain: "harness",
    ruv: "@metaharness/darwin evolve harness freeze model",
    query: "darwin evolve variants mutator harness",
    weftHints: [".metaharness/flywheel/GAPS.md"],
  },
  {
    id: "metaharness-hosts",
    domain: "host",
    ruv: "9 host adapters; no host-grok upstream",
    query: "host-grok grok build ruflo executor host adapter",
    weftHints: [".grok", "docs/adr/adr-075-grok-weftos-mcp-client-bridge.md"],
  },
  {
    id: "metaharness-router",
    domain: "routing",
    ruv: "@metaharness/router cost-optimal model routing ADR-148",
    query: "router cost optimal model routing savings",
    weftHints: [],
  },
  {
    id: "ruvector",
    domain: "substrate",
    ruv: "RuVector HNSW, RVF, cluster, raft, replication",
    query: "ruvector hnsw rvf cluster raft vector",
    weftHints: ["Cargo.toml"],
  },
  {
    id: "cognitum-gate-tilezero",
    domain: "governance",
    ruv: "Permit/Defer/Deny gate + receipts",
    query: "cognitum-gate-tilezero tilezero permit defer deny receipt",
    weftHints: ["Cargo.toml", "docs/weftos/FEATURE_GATES.md"],
  },
  {
    id: "cognitum-maas",
    domain: "cloud",
    ruv: "MaaS/Fugu metered completions, cog_ keys, approval pods",
    query: "cognitum fugu maas cog_ api metered completions",
    weftHints: ["docs/research/ruv-ecosystem-synergy-flywheel.md"],
  },
  {
    id: "cognitum-seed",
    domain: "edge",
    ruv: "Seed device vector store + cogs (Pi/edge apps)",
    query: "cognitum seed edge cog vector store pair",
    weftHints: ["crates/clawft-edge-pad", "crates/clawft-android-edge"],
  },
  {
    id: "worldgraph",
    domain: "twin",
    ruv: "WorldGraph typed petgraph twin + provenance",
    query: "worldgraph graph views bvh provenance fusion twin",
    weftHints: [
      "docs/research/ruv-worldgraph-vs-weftos.md",
      "docs/research/graph-views.md",
      "config/views",
    ],
  },
  {
    id: "quic-mesh",
    domain: "network",
    ruv: "midstream/agentic-flow QUIC; ruflo federation",
    query: "quic mesh federation transport",
    weftHints: ["docs/weftos/FEATURE_GATES.md"],
  },
  {
    id: "ruvllm-sona",
    domain: "learning",
    ruv: "ruvllm SONA MicroLoRA micro-loop under flywheel",
    query: "ruvllm sona microlora sona adapt",
    weftHints: [],
  },
  {
    id: "weftos-kernel-ecc",
    domain: "weftos",
    ruv: "— (WeftOS-native)",
    query: "ecc causal graph hnsw cognitive tick kernel",
    weftHints: ["docs/weftos/FEATURE_GATES.md", "crates/clawft-kernel"],
    weftosOnly: true,
  },
  {
    id: "weftos-exochain",
    domain: "weftos",
    ruv: "witness-like chains (related: RVF witness)",
    query: "exochain dual sign audit governance chain",
    weftHints: ["docs/weftos/FEATURE_GATES.md"],
    weftosOnly: true,
  },
  {
    id: "weftos-bvh-spatial",
    domain: "weftos",
    ruv: "spatial twin contrast WorldGraph",
    query: "bvh spatial VectorRef phase dual-index",
    weftHints: ["crates/clawft-bvh", "docs/adr"],
    weftosOnly: true,
  },
  {
    id: "weftos-lewm",
    domain: "weftos",
    ruv: "OccWorld-class predictive contrast",
    query: "lewm worldmodel ecc decoupling ADR-090",
    weftHints: ["docs/adr/adr-090-lewm-ecc-decoupling-invariant.md"],
    weftosOnly: true,
  },
  {
    id: "weftos-voice",
    domain: "weftos",
    ruv: "—",
    query: "voice aec tts onnx whisper talk-mode",
    weftHints: ["crates/clawft-voice-aec"],
    weftosOnly: true,
  },
  {
    id: "weftos-splat-edge",
    domain: "weftos",
    ruv: "edge capture / sensing contrast",
    query: "splat android edge pad capture world model",
    weftHints: ["crates/clawft-splatd", "crates/clawft-android-edge"],
    weftosOnly: true,
  },
];

function ensureIndex() {
  const index = join(BRAIN, "index.jsonl");
  if (!existsSync(index)) {
    const r = spawnSync(
      "node",
      [join(ROOT, "scripts/metaharness/weftos-brain.mjs"), "index"],
      { cwd: ROOT, encoding: "utf8" },
    );
    if (r.status !== 0) throw new Error(r.stderr || "brain index failed");
  }
}

function brainSearch(query, k = 5) {
  const r = spawnSync(
    "node",
    [join(ROOT, "scripts/metaharness/weftos-brain.mjs"), "search", query],
    {
      cwd: ROOT,
      encoding: "utf8",
      env: { ...process.env, WEFTOS_BRAIN_K: String(k) },
    },
  );
  try {
    return JSON.parse(r.stdout);
  } catch {
    return { hits: [] };
  }
}

function pathExists(rel) {
  if (!rel) return false;
  return existsSync(join(ROOT, rel));
}

function classify(node, hits) {
  const hintHits = (node.weftHints || []).filter(pathExists);
  const top = hits.hits?.[0];
  const score = top?.score ?? 0;
  const hasStrongHit = score >= 1.5 || (hits.hits || []).length >= 3;
  const hasAnyHit = (hits.hits || []).length > 0;

  let mode;
  let next;

  if (node.weftosOnly) {
    if (hintHits.length && hasStrongHit) {
      mode = "SEE";
      next = "Index/doctor already ok; keep brain index fresh so agents find it";
    } else if (hintHits.length) {
      mode = "WIRE";
      next = `Surface ${hintHits[0]} via brain/tasks/MCP so agents discover it`;
    } else if (hasAnyHit) {
      mode = "WIRE";
      next = "Hits exist but thin — add task card or pattern";
    } else {
      mode = "BUILD";
      next = "No corpus hit — confirm feature or document";
    }
  } else if (hintHits.length && hasStrongHit) {
    mode = "SEE";
    next = "Present on both sides; ensure dual-host/MCP path uses it";
  } else if (hintHits.length || hasStrongHit) {
    mode = "WIRE";
    next = hintHits.length
      ? `Wire existing ${hintHits[0]} into agent path / flywheel measure`
      : "Brain found related docs; add explicit package pin or task";
  } else if (hasAnyHit) {
    mode = "WIRE";
    next = "Weak presence — document mapping in synergy research";
  } else if (node.id.includes("upstream") || node.id === "metaharness-hosts" || node.id === "metaharness-router" || node.id === "cognitum-maas" || node.id === "ruvllm-sona") {
    mode = "UPSTREAM";
    next = "Primarily lives in rUv/Cognitum — contribute reference or consume when ready";
  } else {
    mode = "BUILD";
    next = "No WeftOS hit — build adapter/provider or accept out-of-scope";
  }

  // Heuristic overrides for known pathfinder cases
  if (node.id === "metaharness-hosts") {
    if (pathExists(".metaharness/hosts/grok/README.md")) {
      mode = "SEE";
      next =
        "Grok host reference present — agents can SEE pathfinder; UPSTREAM host-grok package still open (S1 publish)";
    } else if (pathExists(".grok/rules/ruflo-grok.md")) {
      mode = "WIRE";
      next =
        "Grok overlay exists; add .metaharness/hosts/grok reference for agents/upstream";
    } else {
      mode = "BUILD";
      next = "Missing Grok host overlay";
    }
  }
  if (node.id === "metaharness-darwin") {
    if (pathExists("scripts/metaharness/darwin-loop.mjs")) {
      mode = "SEE";
      next =
        "Dry loop present (darwin-loop.mjs); full @metaharness/darwin evolve is optional S3 with --confirm";
    } else {
      mode = "BUILD";
      next = "Add darwin-loop.mjs dry wrapper";
    }
  }
  if (node.id === "cognitum-gate-tilezero" && pathExists("Cargo.toml")) {
    const cargo = readFileSync(join(ROOT, "Cargo.toml"), "utf8");
    if (/cognitum-gate-tilezero/.test(cargo)) {
      if (pathExists(".metaharness/tasks/tilezero-smoke.md")) {
        mode = "SEE";
        next =
          "Dep + agent task present — C3 full cargo CI smoke still optional maturity";
      } else {
        mode = "WIRE";
        next = "Dep present — add tilezero-smoke task / CI (C3)";
      }
    }
  }
  if (node.id === "metaharness-router") {
    mode = "UPSTREAM";
    next = "Optional consume @metaharness/router + savings (S2)";
  }
  if (node.id === "ruvllm-sona") {
    mode = "UPSTREAM";
    next =
      "Optional micro-loop (ADR-234); not required for WeftOS product — document only (no BUILD until product need)";
  }

  return {
    id: node.id,
    domain: node.domain,
    ruv: node.ruv,
    mode,
    next,
    weftHintsPresent: hintHits,
    topHits: (hits.hits || []).slice(0, 3).map((h) => ({
      path: h.path,
      title: h.title,
      score: h.score,
      kind: h.kind,
    })),
    brainScore: score,
  };
}

function main() {
  ensureIndex();
  const rows = [];
  for (const node of NODES) {
    const hits = brainSearch(node.query, 8);
    rows.push(classify(node, hits));
  }

  const counts = { SEE: 0, WIRE: 0, BUILD: 0, UPSTREAM: 0 };
  for (const r of rows) counts[r.mode] = (counts[r.mode] || 0) + 1;

  const report = {
    schema: 1,
    kind: "weftos-crosscut",
    flywheel_string: "SEE → WIRE → BUILD → UPSTREAM",
    darwin_loop: "traverse_compare_classify_mutate_measure_promote",
    generatedAt: new Date().toISOString(),
    counts,
    rows,
    topWire: rows.filter((r) => r.mode === "WIRE").slice(0, 10),
    topBuild: rows.filter((r) => r.mode === "BUILD").slice(0, 5),
    topUpstream: rows.filter((r) => r.mode === "UPSTREAM").slice(0, 5),
    suggestedDarwinGen1: {
      lever: "prefer_intervention",
      targetMode: "WIRE",
      focus: rows.find((r) => r.mode === "WIRE")?.id || "agentdb",
      note: "Mutate harness docs/tasks only — one WIRE node per generation",
    },
  };

  mkdirSync(BRAIN, { recursive: true });
  writeFileSync(OUT_JSON, JSON.stringify(report, null, 2) + "\n");

  const md = [
    "# Capability crosscut (auto)",
    "",
    `Generated: ${report.generatedAt}`,
    "",
    "**String:** `SEE → WIRE → BUILD → UPSTREAM`",
    "",
    "## Counts",
    "",
    `| Mode | N |`,
    `|------|---|`,
    ...Object.entries(counts).map(([k, v]) => `| ${k} | ${v} |`),
    "",
    "## Full table",
    "",
    "| Node | Domain | Mode | Next | Brain score |",
    "|------|--------|------|------|-------------|",
    ...rows.map(
      (r) =>
        `| \`${r.id}\` | ${r.domain} | **${r.mode}** | ${r.next.replace(/\|/g, "/")} | ${r.brainScore} |`,
    ),
    "",
    "## Suggested Darwin gen-1",
    "",
    "```json",
    JSON.stringify(report.suggestedDarwinGen1, null, 2),
    "```",
    "",
    "Machine JSON: `.metaharness/brain/crosscut-latest.json`",
    "",
  ].join("\n");

  mkdirSync(join(ROOT, "docs/research"), { recursive: true });
  writeFileSync(OUT_MD, md);

  if (process.argv.includes("--json")) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    console.log(
      JSON.stringify(
        {
          ok: true,
          counts,
          outJson: OUT_JSON,
          outMd: OUT_MD,
          suggestedDarwinGen1: report.suggestedDarwinGen1,
        },
        null,
        2,
      ),
    );
  }
}

main();
