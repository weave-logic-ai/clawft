#!/usr/bin/env node
/**
 * WeftOS process-compose: up + enforce desired.yaml.
 *   node scripts/pc-enforce.mjs status|up|enforce --agent lead --reason "…"
 */
import { spawnSync } from "node:child_process";
import { mkdirSync, readFileSync } from "node:fs";
import { createConnection } from "node:net";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const REPO = dirname(dirname(fileURLToPath(import.meta.url)));
const YAML = join(REPO, "process-compose.yaml");
const DESIRED = join(REPO, "compose", "desired.yaml");
const DIR = join(REPO, ".grok", "compose");
const PC_PORT = 18090;

function die(m) {
  console.error(m);
  process.exit(2);
}
function parseArgs(argv) {
  const out = { _: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith("--")) {
      const n = argv[i + 1];
      if (!n || n.startsWith("--")) out[a.slice(2)] = true;
      else {
        out[a.slice(2)] = n;
        i++;
      }
    } else out._.push(a);
  }
  return out;
}
function portOpen(port) {
  return new Promise((resolve) => {
    const s = createConnection({ host: "127.0.0.1", port });
    const done = (ok) => {
      try { s.destroy(); } catch { /* */ }
      resolve(ok);
    };
    s.setTimeout(400);
    s.on("connect", () => done(true));
    s.on("timeout", () => done(false));
    s.on("error", () => done(false));
  });
}
function pgrep(match) {
  const r = spawnSync("pgrep", ["-fl", match], { encoding: "utf8" });
  if (r.status !== 0) return [];
  return (r.stdout || "")
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l && !l.includes("pgrep") && !l.includes(`${match}/`));
}
function parseDesired() {
  const out = {};
  let cur = null;
  for (const raw of readFileSync(DESIRED, "utf8").split("\n")) {
    const line = raw.replace(/#.*$/, "");
    const m = line.match(/^  ([A-Za-z0-9_-]+):\s*$/);
    if (m) {
      cur = m[1];
      out[cur] = { name: cur };
      continue;
    }
    const kv = line.match(/^    ([a-z_]+):\s*(.+)\s*$/);
    if (!cur || !kv) continue;
    let v = kv[2].trim();
    if (v === "true") v = true;
    else if (v === "false") v = false;
    else if (/^\d+$/.test(v)) v = Number(v);
    out[cur][kv[1]] = v;
  }
  return out;
}
function pc(args) {
  return spawnSync("process-compose", ["-p", String(PC_PORT), "--address", "127.0.0.1", ...args], {
    encoding: "utf8",
    cwd: REPO,
    timeout: 45000,
  });
}
async function ensurePc() {
  mkdirSync(join(DIR, "logs"), { recursive: true });
  if (await portOpen(PC_PORT)) return;
  const r = spawnSync(
    "process-compose",
    [
      "up", "-D", "-t=false", "--keep-project", "-f", YAML, "-p", String(PC_PORT),
      "--address", "127.0.0.1", "-L", join(DIR, "process-compose.log"),
    ],
    { encoding: "utf8", cwd: REPO, timeout: 45000 },
  );
  if (r.status !== 0) die(`up failed: ${r.stderr || r.stdout}`);
  for (let i = 0; i < 25; i++) {
    if (await portOpen(PC_PORT)) return;
    spawnSync("sleep", ["0.2"]);
  }
  die("PC :18090 did not come up");
}
async function live(p) {
  if (p.port && (await portOpen(p.port))) return "port";
  if (p.match && pgrep(p.match).length) return "match";
  return null;
}

async function main() {
  const a = parseArgs(process.argv.slice(2));
  const cmd = a._[0] || "status";
  const desired = parseDesired();
  if (cmd === "status") {
    const rows = [];
    for (const [name, p] of Object.entries(desired)) {
      rows.push({ name, desired: p.desired, live: (await live(p)) || "down" });
    }
    console.log(JSON.stringify({ pc: await portOpen(PC_PORT), processes: rows }, null, 2));
    return;
  }
  if (!a.agent || !a.reason) die("up|enforce needs --agent --reason");
  await ensurePc();
  if (cmd === "up") {
    console.log(JSON.stringify({ up: true, mcp: "http://127.0.0.1:18091/sse" }));
    return;
  }
  if (cmd !== "enforce") die(`unknown ${cmd}`);
  const actions = [];
  for (const [name, p] of Object.entries(desired)) {
    if (p.desired !== "up") {
      actions.push({ name, op: "leave-down" });
      continue;
    }
    if (await live(p)) {
      actions.push({ name, op: "adopted" });
      continue;
    }
    const r = pc(["process", "start", name]);
    const err = `${r.stderr || ""}${r.stdout || ""}`;
    if (r.status === 0 || /already running/i.test(err)) actions.push({ name, op: "started" });
    else die(`start ${name}: ${err}`);
  }
  console.log(JSON.stringify({ enforce: actions }, null, 2));
}
main().catch((e) => {
  console.error(e);
  process.exit(1);
});
