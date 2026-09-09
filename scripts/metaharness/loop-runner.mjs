#!/usr/bin/env node
/**
 * Continuous flywheel runner.
 *
 *   node scripts/metaharness/loop-runner.mjs
 *     → run until goal-check exits 0 (goals met), max 20 gens
 *
 *   node scripts/metaharness/loop-runner.mjs --forever --interval 10
 *     → keep looping forever every 10s (hygiene / maturity)
 *
 *   node scripts/metaharness/loop-runner.mjs --max 5
 *     → at most 5 full loops then exit
 *
 * Each generation: brain index → crosscut → darwin dry → measure → goal-check
 * String: SEE → WIRE → BUILD → UPSTREAM
 */
import { spawnSync } from "node:child_process";
import { appendFileSync, mkdirSync, writeFileSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "../..");
const LOG_DIR = join(ROOT, ".metaharness", "runs");
const forever = process.argv.includes("--forever");
const maxArg = process.argv.indexOf("--max");
const maxGens = maxArg >= 0 ? Number(process.argv[maxArg + 1]) || 20 : forever ? Infinity : 20;
const intArg = process.argv.indexOf("--interval");
const intervalSec = intArg >= 0 ? Number(process.argv[intArg + 1]) || 120 : forever ? 10 : 5;

function run(label, cmd, args) {
  console.log(`\n── ${label} ──`);
  const r = spawnSync(cmd, args, {
    cwd: ROOT,
    encoding: "utf8",
    env: process.env,
    maxBuffer: 20 * 1024 * 1024,
  });
  if (r.stdout) process.stdout.write(r.stdout);
  if (r.stderr) process.stderr.write(r.stderr);
  return r.status ?? 1;
}

function logLine(obj) {
  mkdirSync(LOG_DIR, { recursive: true });
  const path = join(LOG_DIR, "loop-runner.jsonl");
  appendFileSync(path, JSON.stringify({ t: new Date().toISOString(), ...obj }) + "\n");
}

console.log(
  JSON.stringify(
    {
      flywheel_string: "SEE → WIRE → BUILD → UPSTREAM",
      forever,
      maxGens: maxGens === Infinity ? "∞" : maxGens,
      intervalSec,
    },
    null,
    2,
  ),
);

let gen = 0;
while (gen < maxGens) {
  gen += 1;
  console.log(`\n========== GENERATION ${gen} ==========`);
  const t0 = Date.now();

  let code = run("brain:index", "node", [
    join(ROOT, "scripts/metaharness/weftos-brain.mjs"),
    "index",
  ]);
  if (code !== 0) {
    logLine({ gen, step: "index", code });
    if (!forever) process.exit(code);
  }

  code = run("crosscut", "node", [join(ROOT, "scripts/metaharness/crosscut.mjs")]);
  if (code !== 0) {
    logLine({ gen, step: "crosscut", code });
    if (!forever) process.exit(code);
  }

  code = run("darwin-dry", "node", [
    join(ROOT, "scripts/metaharness/darwin-loop.mjs"),
  ]);

  code = run("measure", "node", [
    join(ROOT, "scripts/metaharness/flywheel-measure.mjs"),
    "measure",
  ]);

  const goal = spawnSync(
    "node",
    [join(ROOT, "scripts/metaharness/goal-check.mjs")],
    { cwd: ROOT, encoding: "utf8" },
  );
  if (goal.stdout) process.stdout.write(goal.stdout);
  const goalOk = goal.status === 0;
  let goalJson = {};
  try {
    goalJson = JSON.parse(goal.stdout || "{}");
  } catch {
    /* ignore */
  }

  logLine({
    gen,
    ms: Date.now() - t0,
    goalOk,
    openWire: goalJson.openWire,
    openBuild: goalJson.openBuild,
    advice: goalJson.advice,
  });

  writeFileSync(
    join(LOG_DIR, "loop-status.json"),
    JSON.stringify(
      {
        gen,
        lastAt: new Date().toISOString(),
        goalOk,
        forever,
        goal: goalJson,
      },
      null,
      2,
    ) + "\n",
  );

  if (goalOk && !forever) {
    console.log("\n✓ Goals met — stopping (use --forever to keep hygiene loops).");
    process.exit(0);
  }

  if (gen < maxGens) {
    console.log(`\n… sleep ${intervalSec}s before next generation`);
    spawnSync("sleep", [String(intervalSec)]);
  }
}

console.log(`\nHit max generations (${maxGens}).`);
process.exit(forever ? 0 : 1);
