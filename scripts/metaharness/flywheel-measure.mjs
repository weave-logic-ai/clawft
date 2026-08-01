#!/usr/bin/env node
/**
 * WeftOS adapter for @metaharness/flywheel (ADR-096 / WEFT-730).
 *
 * Modes:
 *   measure  — evaluate gen-0 policy against live scores; write signed receipt (no promote loop)
 *   smoke    — one-generation flywheel with identity proposer (exercises engine; rarely promotes)
 *   status   — summarize receipts + latest score/genome
 *
 * Does NOT run Darwin. Does NOT mint .harness/. Does NOT touch crates/.
 * Big write-path changes require human approval (see .metaharness/flywheel/GAPS.md).
 */
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  appendFileSync,
  writeFileSync,
  readdirSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  makeSigner,
  meetsPromotionRule,
  gateFingerprint,
  runFlywheelGenerations,
  verifyReceipt,
} from "@metaharness/flywheel";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "../..");
const MH = join(ROOT, ".metaharness");
const FW = join(MH, "flywheel");
const RECEIPTS = join(FW, "receipts");
const POLICY_PATH = join(FW, "policy-root.json");

function sh(cmd, args) {
  const r = spawnSync(cmd, args, {
    cwd: ROOT,
    encoding: "utf8",
    env: process.env,
  });
  return r;
}

function loadPolicyRoot() {
  const doc = JSON.parse(readFileSync(POLICY_PATH, "utf8"));
  return doc.policy || doc;
}

function refreshScores() {
  sh("bash", [join(ROOT, "scripts/metaharness/score.sh")]);
  sh("bash", [join(ROOT, "scripts/metaharness/weftos-score.sh")]);
}

function readJson(path, fallback = null) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return fallback;
  }
}

/** Project WeftOS readiness onto flywheel Score axes. */
function evaluatePolicy(policy) {
  const found = readJson(join(MH, "weftos-score-latest.json"), {});
  const genome = readJson(join(MH, "genome-latest.json"), {});
  const scorecard = readJson(join(MH, "score-latest.json"), {});

  const foundation = Number(found.weftosFoundationScore ?? 0);
  const minFoundation = Number(policy.min_foundation ?? 80);
  const members = Number(found.components?.workspaceMembers ?? 0);
  const minMembers = Number(policy.min_workspace_members ?? 20);
  const dist = Number(found.components?.distTargetCount ?? 0);
  const minDist = Number(policy.min_dist_targets ?? 5);
  const domains = Number(found.components?.domainsHit ?? 0);
  const minDomains = Number(policy.min_domains_hit ?? 4);

  const requireGenome = (policy.require_genome_ready ?? "true") === "true";
  const publish = Number(genome.publish_readiness ?? 0);
  const testConf = Number(genome.test_confidence ?? 0);
  const genomeOk =
    !requireGenome ||
    genome.degraded === true ||
    (publish >= 0.8 && testConf >= 0.8);

  // Run view validate as commit signal
  const views = sh("bash", [
    join(ROOT, "scripts/metaharness/validate-views.sh"),
  ]);
  const viewsOk = views.status === 0;

  // Ecosystem alignment (not ADR-041 vanity) — pathfinding + cross-compat
  const alignment = {
    grokRufloPath:
      existsSync(join(ROOT, ".grok/rules/ruflo-grok.md")) &&
      existsSync(join(ROOT, "scripts/grok-team-bus.mjs")),
    dualHostPatterns: (() => {
      if (
        !existsSync(join(MH, "patterns-manifest.md")) ||
        !existsSync(join(ROOT, "scripts/metaharness/seed-patterns.sh"))
      )
        return false;
      // Gen-1 WIRE: manifest must list real pattern-* keys (agent-visible surface)
      const body = readFileSync(join(MH, "patterns-manifest.md"), "utf8");
      const keys = (body.match(/`pattern-[a-z0-9-]+`/gi) || []).length;
      return keys >= 5;
    })(),
    worldgraphCrosswalk: existsSync(
      join(ROOT, "docs/research/ruv-worldgraph-vs-weftos.md"),
    ),
    ecosystemSynergyDoc: existsSync(
      join(ROOT, "docs/research/ruv-ecosystem-synergy-flywheel.md"),
    ),
    rufloPinDoc: (() => {
      try {
        const pkg = JSON.parse(
          readFileSync(join(ROOT, "package.json"), "utf8"),
        );
        return Boolean(pkg.weftos?.rufloPin && pkg.weftos?.rufloPinNote);
      } catch {
        return false;
      }
    })(),
    metaharnessFoundationAdr: existsSync(
      join(ROOT, "docs/adr/adr-096-metaharness-foundation.md"),
    ),
    // Cognitum: product gate dep (tilezero) + synergy doc coverage
    cognitumGateDep: (() => {
      try {
        const cargo = readFileSync(join(ROOT, "Cargo.toml"), "utf8");
        return /cognitum-gate-tilezero/.test(cargo);
      } catch {
        return false;
      }
    })(),
    cognitumInSynergyDoc: (() => {
      try {
        const t = readFileSync(
          join(ROOT, "docs/research/ruv-ecosystem-synergy-flywheel.md"),
          "utf8",
        );
        return /Cognitum/i.test(t) && /tilezero|MaaS|Seed/i.test(t);
      } catch {
        return false;
      }
    })(),
  };

  const requireAlign = {
    grok: (policy.require_grok_ruflo_path ?? "true") === "true",
    patterns: (policy.require_dual_host_patterns ?? "true") === "true",
    world: (policy.require_worldgraph_crosswalk ?? "true") === "true",
    pin: (policy.require_ruflo_pin_doc ?? "true") === "true",
    cognitum: (policy.require_cognitum_gate_dep ?? "true") === "true",
  };

  const alignmentChecks = [
    !requireAlign.grok || alignment.grokRufloPath,
    !requireAlign.patterns || alignment.dualHostPatterns,
    !requireAlign.world || alignment.worldgraphCrosswalk,
    !requireAlign.pin || alignment.rufloPinDoc,
    !requireAlign.cognitum ||
      (alignment.cognitumGateDep && alignment.cognitumInSynergyDoc),
    alignment.ecosystemSynergyDoc,
    alignment.metaharnessFoundationAdr,
  ];

  const checks = [
    foundation >= minFoundation,
    members >= minMembers,
    dist >= minDist,
    domains >= minDomains,
    genomeOk,
    viewsOk,
    ...alignmentChecks,
  ];
  const passed = checks.filter(Boolean).length;
  const primary = foundation / 100;
  // noopRate: fraction of failed readiness checks (lower better)
  const noopRate = 1 - passed / checks.length;
  const costPerWin = foundation > 0 ? 1 / foundation : 999;
  const alignFailed = alignmentChecks.some((c) => !c);
  const regressed =
    foundation < minFoundation ||
    members < minMembers ||
    !viewsOk ||
    alignFailed ||
    (requireGenome && !genomeOk && !genome.degraded);

  return {
    score: {
      primary,
      noopRate,
      costPerWin,
      regressed,
    },
    evidence: {
      foundation,
      members,
      dist,
      domains,
      genomeOk,
      viewsOk,
      publish_readiness: publish,
      test_confidence: testConf,
      scorecardArchetype: scorecard.archetype ?? null,
      scorecardHarnessFit: scorecard.harnessFit ?? null,
      alignment,
      note: "SEE → WIRE → BUILD → UPSTREAM · rUv+Cognitum+Grok+Ruflo · not ADR-041 hacks",
      flywheel_string: policy.flywheel_string || "SEE → WIRE → BUILD → UPSTREAM",
    },
  };
}

function ensureDirs() {
  mkdirSync(RECEIPTS, { recursive: true });
}

function appendReceipt(kind, body) {
  ensureDirs();
  const line = JSON.stringify({ kind, ...body }) + "\n";
  const day = new Date().toISOString().slice(0, 10);
  const path = join(RECEIPTS, `${day}.jsonl`);
  appendFileSync(path, line);
  const latest = join(RECEIPTS, "latest.json");
  writeFileSync(latest, JSON.stringify({ kind, ...body }, null, 2) + "\n");
  return { path, latest };
}

async function measure() {
  refreshScores();
  const policy = loadPolicyRoot();
  const { score, evidence } = evaluatePolicy(policy);
  const signer = makeSigner();
  const gateFp = gateFingerprint(meetsPromotionRule);
  const payload = {
    mode: "measure",
    schema: 1,
    createdAt: new Date().toISOString(),
    policy,
    score,
    evidence,
    gateFingerprint: gateFp,
    dataSource: "LIVE",
  };
  const receipt = signer.sign(payload);
  const ok = verifyReceipt(receipt);
  const paths = appendReceipt("measure", {
    createdAt: payload.createdAt,
    score,
    evidence,
    gateFingerprint: gateFp,
    receipt,
    verifyReceipt: ok,
  });
  console.log(
    JSON.stringify(
      {
        ok: !score.regressed && score.primary >= 0.8,
        score,
        evidence,
        gateFingerprint: gateFp,
        receiptVerify: ok,
        wrote: paths,
      },
      null,
      2,
    ),
  );
  process.exit(score.regressed || score.primary < 0.8 ? 1 : 0);
}

async function smoke() {
  refreshScores();
  const rootPolicy = loadPolicyRoot();
  const signer = makeSigner();
  const gateFp = gateFingerprint(meetsPromotionRule);

  // Identity proposer: returns same lever value → no real lift (engine smoke).
  const proposer = async (base, target) => base.policy[target] ?? "";

  const evaluator = async (policy, suite) => {
    // suite.id distinguishes holdout vs anchor; same evaluation for smoke
    void suite;
    const { score } = evaluatePolicy(policy);
    return score;
  };

  const result = await runFlywheelGenerations({
    rootPolicy,
    proposer,
    evaluator,
    promotionRule: meetsPromotionRule,
    holdout: { id: "weftos-holdout", items: ["foundation", "platform", "views"] },
    anchor: {
      id: "weftos-anchor",
      items: ["adr090", "no-silent-promote", "gate"],
    },
    mutationTargets: ["viewspec_eval", "promote_mode"],
    maxGenerations: 1,
    signer,
    dataSource: "LIVE",
    now: (g) => `gen-${g}`,
    rootId: "weftos-policy-root",
  });

  const paths = appendReceipt("smoke", {
    createdAt: new Date().toISOString(),
    gateFingerprint: gateFp,
    generationsRun: result.generationsRun,
    milestoneReached: result.milestoneReached,
    liftCurve: result.liftCurve,
    promotions: result.promotions.map((p) => ({
      id: p.id,
      generation: p.generation,
      verdict: p.verdict,
      failureReasons: p.failureReasons,
    })),
    finalPolicy: result.finalPolicy,
    replayBundleRoot: result.replayBundle.root_id,
  });

  // Persist full replay for audit (large)
  writeFileSync(
    join(FW, "smoke-replay-latest.json"),
    JSON.stringify(result.replayBundle, null, 2) + "\n",
  );

  console.log(
    JSON.stringify(
      {
        ok: true,
        generationsRun: result.generationsRun,
        milestoneReached: result.milestoneReached,
        liftCurve: result.liftCurve,
        promotionVerdicts: result.promotions.map((p) => p.verdict),
        wrote: paths,
        replay: join(FW, "smoke-replay-latest.json"),
      },
      null,
      2,
    ),
  );
}

function status() {
  ensureDirs();
  const found = readJson(join(MH, "weftos-score-latest.json"));
  const genome = readJson(join(MH, "genome-latest.json"));
  const scorecard = readJson(join(MH, "score-latest.json"));
  const latest = readJson(join(RECEIPTS, "latest.json"));
  let receiptFiles = [];
  if (existsSync(RECEIPTS)) {
    receiptFiles = readdirSync(RECEIPTS).filter((f) => f.endsWith(".jsonl"));
  }
  console.log(
    JSON.stringify(
      {
        foundation: found?.weftosFoundationScore ?? null,
        platformSurface: found?.componentScores?.platformSurface ?? null,
        workspaceMembers: found?.components?.workspaceMembers ?? null,
        genome: genome
          ? {
              repo_type: genome.repo_type,
              publish_readiness: genome.publish_readiness,
              test_confidence: genome.test_confidence,
              risk_score: genome.risk_score,
            }
          : null,
        scorecard: scorecard
          ? {
              archetype: scorecard.archetype,
              harnessFit: scorecard.harnessFit,
              taskCoverage: scorecard.taskCoverage,
              memoryUsefulness: scorecard.memoryUsefulness,
            }
          : null,
        latestReceipt: latest
          ? {
              kind: latest.kind,
              createdAt: latest.createdAt,
              score: latest.score,
              evidence: latest.evidence,
            }
          : null,
        receiptDays: receiptFiles,
        gaps: join(FW, "GAPS.md"),
        note: "Darwin evolve / mint .harness / production keys require approval — see GAPS.md",
      },
      null,
      2,
    ),
  );
}

const mode = process.argv[2] || "status";
if (mode === "measure") {
  await measure();
} else if (mode === "smoke") {
  await smoke();
} else if (mode === "status") {
  status();
} else {
  console.error("Usage: flywheel-measure.mjs [measure|smoke|status]");
  process.exit(2);
}
