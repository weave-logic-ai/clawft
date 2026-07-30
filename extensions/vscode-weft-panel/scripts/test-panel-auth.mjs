#!/usr/bin/env node
// Lightweight node runner for WEFT-495 panelAuth unit tests.
// Does not need a VSCode host. Expects `npm run compile` first.
//
// Usage (from extensions/vscode-weft-panel):
//   npm run compile && node scripts/test-panel-auth.mjs

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import path from "node:path";

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const {
    authorizePanelRpc,
    isMultiUserMode,
    issuePanelSession,
    requiredScope,
    scopesAllowMethod,
    scopesToAuth,
} = require(path.join(root, "out", "panelAuth.js"));

let failed = 0;
function check(name, fn) {
    try {
        fn();
        console.log(`  ok  ${name}`);
    } catch (err) {
        failed += 1;
        console.error(`  FAIL ${name}`);
        console.error(err);
    }
}

console.log("panelAuth (WEFT-495)");

check("multi-user defaults off", () => {
    assert.equal(isMultiUserMode({}), false);
});

check("multi-user env on", () => {
    assert.equal(isMultiUserMode({ WEFTOS_MULTI_USER: "1" }), true);
});

check("requiredScope", () => {
    assert.equal(requiredScope("kernel.status"), "read");
    assert.equal(requiredScope("agent.chat"), "chat");
    assert.equal(requiredScope("terminal.spawn"), "write");
    assert.equal(requiredScope("kernel.kill-process"), "admin");
});

check("default viewer scopes", () => {
    const s = issuePanelSession();
    assert.deepEqual(s.scopes, ["read", "chat"]);
    assert.equal(scopesToAuth(s.scopes), "read,chat");
    assert.equal(scopesAllowMethod(s.scopes, "agent.chat"), true);
    assert.equal(scopesAllowMethod(s.scopes, "terminal.spawn"), false);
});

// ── Critical acceptance criterion: denied-by-identity ──────────────
check("denied-by-identity: viewer cannot terminal.spawn", () => {
    const session = issuePanelSession(["read", "chat"]);
    const decision = authorizePanelRpc(session, "terminal.spawn");
    assert.equal(decision.ok, false);
    assert.equal(decision.reason, "scope");
    assert.match(decision.error, /lack 'write'/);
    assert.match(decision.error, /terminal\.spawn/);
});

check("denied-by-identity: no session", () => {
    const decision = authorizePanelRpc(undefined, "kernel.status");
    assert.equal(decision.ok, false);
    assert.equal(decision.reason, "no_session");
});

check("denied-by-identity: expired", () => {
    const session = issuePanelSession(["read"], 1);
    session.expiresAt = Date.now() - 1;
    const decision = authorizePanelRpc(session, "kernel.status");
    assert.equal(decision.ok, false);
    assert.equal(decision.reason, "expired");
});

check("authorize allows viewer read+chat", () => {
    const session = issuePanelSession();
    const d = authorizePanelRpc(session, "kernel.status");
    assert.equal(d.ok, true);
    assert.equal(d.auth, "read,chat");
});

check("admin implies all", () => {
    const session = issuePanelSession(["admin"]);
    const d = authorizePanelRpc(session, "kernel.kill-process");
    assert.equal(d.ok, true);
    assert.equal(d.auth, "admin");
});

if (failed > 0) {
    console.error(`\n${failed} failed`);
    process.exit(1);
}
console.log("\nall passed");
