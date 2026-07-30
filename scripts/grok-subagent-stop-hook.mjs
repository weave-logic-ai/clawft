#!/usr/bin/env node
/**
 * Grok SubagentStop → team bus on-stop (ADR-320).
 * Reads hook JSON from stdin (Grok / Claude compatible fields).
 * Fail-open: always exit 0.
 */
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const projectRoot = process.env.CLAUDE_PROJECT_DIR
  || process.env.GROK_WORKSPACE_ROOT
  || process.cwd();

const bus = path.join(projectRoot, 'scripts', 'grok-team-bus.mjs');

function readStdin() {
  return new Promise((resolve) => {
    let data = '';
    const t = setTimeout(() => resolve(data), 300);
    process.stdin.setEncoding('utf8');
    process.stdin.on('data', (c) => { data += c; });
    process.stdin.on('end', () => { clearTimeout(t); resolve(data); });
    process.stdin.on('error', () => { clearTimeout(t); resolve(data); });
  });
}

function activeTeam() {
  const teamsDir = path.join(projectRoot, '.claude-flow', 'teams');
  if (!fs.existsSync(teamsDir)) return null;
  const names = fs.readdirSync(teamsDir).filter((n) => {
    try {
      const t = JSON.parse(fs.readFileSync(path.join(teamsDir, n, 'team.json'), 'utf8'));
      return t && t.status === 'active';
    } catch {
      return false;
    }
  });
  return names[0] || null;
}

const raw = await readStdin();
let input = {};
try {
  input = raw.trim() ? JSON.parse(raw) : {};
} catch {
  input = {};
}

const agent =
  process.env.SUBAGENT_NAME
  || input.subagentName
  || input.agentName
  || input.agent
  || input.description
  || input.toolInput?.description
  || 'unknown';

const team =
  process.env.TEAM_NAME
  || input.teamName
  || input.team
  || activeTeam();

if (team && fs.existsSync(bus)) {
  try {
    spawnSync(process.execPath, [bus, 'on-stop', '--team', String(team), '--agent', String(agent).replace(/\s+/g, '-').slice(0, 64)], {
      cwd: projectRoot,
      timeout: 4000,
      stdio: 'ignore',
    });
  } catch {
    /* fail-open */
  }
}

process.exit(0);
