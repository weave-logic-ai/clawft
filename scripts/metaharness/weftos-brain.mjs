#!/usr/bin/env node
/**
 * WeftOS brain — ruvbrain-shaped local corpus for capability crawl.
 *
 * Commands:
 *   index   — rebuild .metaharness/brain/index.jsonl from repo sources
 *   search  — query the index (BM25-ish token overlap)
 *   stats   — document counts by kind
 *
 * Removable: never required by `weft` runtime (ADR-150).
 * Doctrine: SEE → WIRE → BUILD → UPSTREAM (.metaharness/flywheel/STRING.md)
 */
import {
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  readdirSync,
  statSync,
} from "node:fs";
import { join, relative, resolve, dirname, basename } from "node:path";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "../..");
const BRAIN = join(ROOT, ".metaharness", "brain");
const INDEX = join(BRAIN, "index.jsonl");
const META = join(BRAIN, "meta.json");

const STOP = new Set(
  "the a an and or for with this that from into are was were be to of in on at by as is it its our your their not no yes".split(
    " ",
  ),
);

function tokenize(text) {
  return String(text || "")
    .toLowerCase()
    .replace(/[^a-z0-9+#./_-]+/g, " ")
    .split(/\s+/)
    .filter((w) => w.length >= 3 && !STOP.has(w));
}

function walk(dir, pred, out = []) {
  if (!existsSync(dir)) return out;
  for (const name of readdirSync(dir)) {
    if (name === "node_modules" || name === "target" || name === ".git") continue;
    const p = join(dir, name);
    let st;
    try {
      st = statSync(p);
    } catch {
      continue;
    }
    if (st.isDirectory()) walk(p, pred, out);
    else if (pred(p, name)) out.push(p);
  }
  return out;
}

function readLimited(path, max = 120_000) {
  try {
    const buf = readFileSync(path);
    return buf.slice(0, max).toString("utf8");
  } catch {
    return "";
  }
}

function firstHeading(text) {
  const m = text.match(/^#\s+(.+)$/m);
  return m ? m[1].trim() : null;
}

function docId(rel) {
  return createHash("sha1").update(rel).digest("hex").slice(0, 16);
}

function makeDoc(absPath, kind, extra = {}) {
  const rel = relative(ROOT, absPath).replace(/\\/g, "/");
  const text = readLimited(absPath);
  if (!text.trim()) return null;
  const title = extra.title || firstHeading(text) || basename(absPath);
  const body = text.slice(0, 40_000);
  const tokens = tokenize(`${title}\n${body}\n${extra.tags || ""}`);
  return {
    id: docId(rel),
    path: rel,
    kind,
    title,
    tags: extra.tags || "",
    tokenCount: tokens.length,
    // store unique tokens for search (cap)
    tokens: [...new Set(tokens)].slice(0, 800),
    excerpt: body.replace(/\s+/g, " ").trim().slice(0, 400),
  };
}

function collectDocs() {
  const docs = [];

  // ADRs
  for (const p of walk(join(ROOT, "docs/adr"), (path, n) =>
    /^adr-.*\.md$/i.test(n),
  )) {
    const d = makeDoc(p, "adr");
    if (d) docs.push(d);
  }

  // Research
  for (const p of walk(join(ROOT, "docs/research"), (path, n) =>
    n.endsWith(".md"),
  )) {
    const d = makeDoc(p, "research");
    if (d) docs.push(d);
  }

  // Feature gates / design surfaces
  for (const rel of [
    "docs/weftos/FEATURE_GATES.md",
    "docs/DESIGN.md",
    "CONTRIBUTING.md",
    "CHANGELOG.md",
    "README.md",
  ]) {
    const p = join(ROOT, rel);
    if (existsSync(p)) {
      const d = makeDoc(p, "docs");
      if (d) docs.push(d);
    }
  }

  // Crates: Cargo.toml description + optional README
  const cratesDir = join(ROOT, "crates");
  if (existsSync(cratesDir)) {
    for (const name of readdirSync(cratesDir)) {
      const cargo = join(cratesDir, name, "Cargo.toml");
      if (!existsSync(cargo)) continue;
      const raw = readLimited(cargo, 20_000);
      const desc =
        (raw.match(/description\s*=\s*"([^"]+)"/) || [])[1] || name;
      const readme = join(cratesDir, name, "README.md");
      const text = existsSync(readme)
        ? `${desc}\n${readLimited(readme, 30_000)}`
        : `${name}\n${desc}\n${raw}`;
      const rel = relative(ROOT, cargo).replace(/\\/g, "/");
      const tokens = tokenize(`${name} ${desc} ${text}`);
      docs.push({
        id: docId(rel),
        path: rel,
        kind: "crate",
        title: name,
        tags: "crate rust workspace",
        tokenCount: tokens.length,
        tokens: [...new Set(tokens)].slice(0, 400),
        excerpt: desc.slice(0, 400),
      });
    }
  }

  // MetaHarness + Grok surface
  for (const p of walk(join(ROOT, ".metaharness"), (path, n) =>
    n.endsWith(".md") || n.endsWith(".yaml") || n.endsWith(".yml") || n.endsWith(".json"),
  )) {
    if (p.includes("/brain/") || p.includes("/receipts/") || p.includes("latest.json"))
      continue;
    const d = makeDoc(p, "metaharness");
    if (d) docs.push(d);
  }
  for (const p of walk(join(ROOT, ".grok"), (path, n) => n.endsWith(".md"))) {
    const d = makeDoc(p, "grok");
    if (d) docs.push(d);
  }

  // ViewSpecs
  for (const p of walk(join(ROOT, "config/views"), (path, n) =>
    n.endsWith(".yaml") || n.endsWith(".yml") || n.endsWith(".md"),
  )) {
    const d = makeDoc(p, "viewspec");
    if (d) docs.push(d);
  }

  return docs;
}

function buildIndex() {
  mkdirSync(BRAIN, { recursive: true });
  const docs = collectDocs();
  // DF for BM25
  const df = new Map();
  for (const d of docs) {
    for (const t of d.tokens) df.set(t, (df.get(t) || 0) + 1);
  }
  const N = docs.length || 1;
  const lines = docs.map((d) => JSON.stringify(d)).join("\n") + "\n";
  writeFileSync(INDEX, lines);
  const byKind = {};
  for (const d of docs) byKind[d.kind] = (byKind[d.kind] || 0) + 1;
  const meta = {
    schema: 1,
    kind: "weftos-brain-meta",
    generatedAt: new Date().toISOString(),
    docCount: docs.length,
    byKind,
    flywheel_string: "SEE → WIRE → BUILD → UPSTREAM",
    indexPath: relative(ROOT, INDEX),
  };
  writeFileSync(META, JSON.stringify(meta, null, 2) + "\n");
  // compact df for search
  writeFileSync(
    join(BRAIN, "df.json"),
    JSON.stringify(Object.fromEntries(df)) + "\n",
  );
  return meta;
}

function loadIndex() {
  if (!existsSync(INDEX)) return { docs: [], df: new Map(), N: 0 };
  const docs = readFileSync(INDEX, "utf8")
    .split("\n")
    .filter(Boolean)
    .map((l) => JSON.parse(l));
  let df = new Map();
  if (existsSync(join(BRAIN, "df.json"))) {
    df = new Map(Object.entries(JSON.parse(readFileSync(join(BRAIN, "df.json"), "utf8"))));
  }
  return { docs, df, N: docs.length };
}

function search(query, k = 10) {
  const { docs, df, N } = loadIndex();
  if (!docs.length) {
    return { error: "empty-index", hint: "node scripts/metaharness/weftos-brain.mjs index" };
  }
  const q = tokenize(query);
  if (!q.length) return { query, hits: [] };

  const avgdl = docs.reduce((s, d) => s + d.tokenCount, 0) / N;
  const k1 = 1.2;
  const b = 0.75;
  const hits = [];
  for (const d of docs) {
    const tfmap = new Map();
    for (const t of d.tokens) tfmap.set(t, (tfmap.get(t) || 0) + 1);
    let score = 0;
    for (const term of q) {
      const tf = tfmap.get(term) || 0;
      if (!tf) continue;
      const n = df.get(term) || 0;
      const idf = Math.log(1 + (N - n + 0.5) / (n + 0.5));
      const denom = tf + k1 * (1 - b + b * (d.tokenCount / (avgdl || 1)));
      score += idf * ((tf * (k1 + 1)) / denom);
    }
    if (score > 0) {
      hits.push({
        score: Math.round(score * 1000) / 1000,
        id: d.id,
        kind: d.kind,
        path: d.path,
        title: d.title,
        excerpt: d.excerpt,
      });
    }
  }
  hits.sort((a, b) => b.score - a.score);
  return { query, hits: hits.slice(0, k), corpus: N };
}

function stats() {
  if (!existsSync(META)) {
    console.log(JSON.stringify({ error: "no-index", hint: "run index" }));
    return;
  }
  console.log(readFileSync(META, "utf8"));
}

const cmd = process.argv[2] || "stats";
if (cmd === "index") {
  const meta = buildIndex();
  console.log(JSON.stringify(meta, null, 2));
} else if (cmd === "search") {
  const q = process.argv.slice(3).join(" ") || "metaharness flywheel";
  const k = Number(process.env.WEFTOS_BRAIN_K || 10);
  console.log(JSON.stringify(search(q, k), null, 2));
} else if (cmd === "stats") {
  stats();
} else {
  console.error("Usage: weftos-brain.mjs [index|search|stats] [query...]");
  process.exit(2);
}
