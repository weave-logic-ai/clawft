#!/usr/bin/env node
/**
 * Darwin-shaped flywheel generation (dry by default).
 *
 * Loop: load crosscut → pick one WIRE/BUILD node → propose harness mutation
 * (docs/tasks only) → optional apply with --confirm → measure.
 *
 * Does NOT run @metaharness/darwin source mutators unless --confirm.
 * Never touches crates/ or ADR-090 surfaces.
 *
 * Usage:
 *   node scripts/metaharness/darwin-loop.mjs           # dry plan
 *   node scripts/metaharness/darwin-loop.mjs --confirm # write proposal under .metaharness/variants/
 *   node scripts/metaharness/darwin-loop.mjs --measure # run flywheel measure after plan
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
const CROSSCUT = join(ROOT, ".metaharness", "brain", "crosscut-latest.json");
const VARIANTS = join(ROOT, ".metaharness", "variants");

function run(cmd, args) {
  return spawnSync(cmd, args, { cwd: ROOT, encoding: "utf8" });
}

function ensureCrosscut() {
  if (!existsSync(CROSSCUT)) {
    const r = run("node", [join(ROOT, "scripts/metaharness/crosscut.mjs")]);
    if (r.status !== 0) {
      console.error(r.stderr || r.stdout);
      process.exit(1);
    }
  }
}

function main() {
  const confirm = process.argv.includes("--confirm");
  const measure = process.argv.includes("--measure");
  ensureCrosscut();
  const report = JSON.parse(readFileSync(CROSSCUT, "utf8"));

  const wire = report.rows.filter((r) => r.mode === "WIRE");
  const build = report.rows.filter((r) => r.mode === "BUILD");
  const pick = wire[0] || build[0] || report.rows[0];

  const generation = {
    schema: 1,
    kind: "weftos-darwin-generation-plan",
    flywheel_string: "SEE → WIRE → BUILD → UPSTREAM",
    createdAt: new Date().toISOString(),
    dryRun: !confirm,
    focus: pick,
    rationale: `Prefer WIRE over BUILD. First open WIRE node: ${pick?.id}`,
    mutations: [
      {
        surface: "metaharness_task_or_pattern",
        action: "document_or_wire",
        target: pick?.id,
        next: pick?.next,
        allowedPaths: [
          ".metaharness/tasks/",
          ".metaharness/commands/",
          ".metaharness/patterns-manifest.md",
          "docs/research/",
          ".grok/",
        ],
        forbidden: ["crates/", "docs/adr/adr-090*", "scripts/build.sh gate phases"],
      },
    ],
    measureCommand: "node scripts/metaharness/flywheel-measure.mjs measure",
  };

  console.log(JSON.stringify(generation, null, 2));

  if (confirm) {
    mkdirSync(VARIANTS, { recursive: true });
    const stamp = new Date().toISOString().replace(/[:.]/g, "-");
    const path = join(VARIANTS, `gen-${stamp}.json`);
    writeFileSync(path, JSON.stringify(generation, null, 2) + "\n");
    // Human-readable proposal note (not a code mutator)
    const note = join(VARIANTS, `gen-${stamp}-PROPOSAL.md`);
    writeFileSync(
      note,
      [
        `# Darwin generation proposal`,
        ``,
        `**Mode focus:** ${pick?.mode} \`${pick?.id}\``,
        ``,
        pick?.next || "",
        ``,
        `String: SEE → WIRE → BUILD → UPSTREAM`,
        ``,
        `Apply manually in allowed paths only, then:`,
        ``,
        `\`\`\`bash`,
        `node scripts/metaharness/flywheel-measure.mjs measure`,
        `node scripts/metaharness/crosscut.mjs`,
        `\`\`\``,
        ``,
      ].join("\n"),
    );
    console.error(`Wrote ${path}`);
    console.error(`Wrote ${note}`);
  }

  if (measure) {
    const m = run("node", [
      join(ROOT, "scripts/metaharness/flywheel-measure.mjs"),
      "measure",
    ]);
    process.stdout.write(m.stdout || "");
    if (m.status !== 0) process.exit(m.status || 1);
  }
}

main();
