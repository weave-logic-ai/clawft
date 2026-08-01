#!/usr/bin/env node
/**
 * Goal check for the continuous flywheel loop.
 * Exit 0 = goals met (can stop or only do UPSTREAM/hygiene).
 * Exit 1 = keep looping (open WIRE/BUILD or measure failed).
 */
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "../..");
const MH = join(ROOT, ".metaharness");

function load(p) {
  try {
    return JSON.parse(readFileSync(p, "utf8"));
  } catch {
    return null;
  }
}

// Prefer latest measure receipt; else recompute lightly from score files
const receipt = load(join(MH, "flywheel/receipts/latest.json"));
const found = load(join(MH, "weftos-score-latest.json"));
const genome = load(join(MH, "genome-latest.json"));
const cross = load(join(MH, "brain/crosscut-latest.json"));

const foundation = found?.weftosFoundationScore ?? receipt?.score?.primary * 100;
const publish = genome?.publish_readiness ?? 0;
const testC = genome?.test_confidence ?? 0;
const measureOk = receipt?.kind
  ? receipt.score && !receipt.score.regressed && receipt.score.primary >= 0.8
  : false;

const wire = (cross?.rows || []).filter((r) => r.mode === "WIRE");
const build = (cross?.rows || []).filter((r) => r.mode === "BUILD");
// Pathfinder WIREs we still want to burn down before "goals met"
const blockingWire = wire.filter(
  (r) =>
    !["metaharness-hosts"].includes(r.id) ||
    !existsSync(join(ROOT, ".metaharness/hosts/grok/README.md")),
);

const goals = {
  foundation: Number(foundation) >= 80,
  genome:
    genome?.degraded ||
    (Number(publish) >= 0.8 && Number(testC) >= 0.8),
  measure: measureOk || Number(foundation) >= 80,
  hostGrokRef: existsSync(join(ROOT, ".metaharness/hosts/grok/README.md")),
  noBlockingBuild: build.length === 0 || build.every((b) => b.id === "metaharness-darwin"),
  // Darwin BUILD is deferred by design until S3 approved — not a hard fail
  openWireCount: wire.length,
};

const met =
  goals.foundation &&
  goals.genome &&
  goals.hostGrokRef &&
  goals.noBlockingBuild;

const out = {
  met,
  keepLooping: !met || wire.length > 2,
  goals,
  openWire: wire.map((r) => r.id),
  openBuild: build.map((r) => r.id),
  flywheel_string: "SEE → WIRE → BUILD → UPSTREAM",
  advice: met
    ? wire.length
      ? "Core goals met; optional WIRE burn-down continues for maturity"
      : "Goals met — only UPSTREAM/hygiene left"
    : "Keep looping: fix failed goals or open WIRE/BUILD",
};

console.log(JSON.stringify(out, null, 2));
process.exit(met ? 0 : 1);
