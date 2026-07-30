// WEFT-495 / ADR-071 — panel auth unit tests (no VSCode host required for
// the pure module; mocha loads the compiled `out/panelAuth.js`).
//
// Critical case: denied-by-identity when a multi-user panel's scopes do
// not cover the method's required CapScope.

import * as assert from "node:assert";
// eslint-disable-next-line @typescript-eslint/no-require-imports
const { suite, test } = require("mocha");

// Compiled by `npm run compile` (pretest). Path is relative to out-test/suite/.
import {
    authorizePanelRpc,
    isMultiUserMode,
    issuePanelSession,
    requiredScope,
    scopesAllowMethod,
    scopesToAuth,
    type PanelSession,
} from "../../out/panelAuth";

suite("weft-panel: panelAuth (WEFT-495)", () => {
    test("isMultiUserMode defaults off", () => {
        assert.strictEqual(isMultiUserMode({}), false);
        assert.strictEqual(isMultiUserMode({ WEFTOS_MULTI_USER: "" }), false);
        assert.strictEqual(isMultiUserMode({ WEFTOS_MULTI_USER: "0" }), false);
    });

    test("isMultiUserMode honours env and override", () => {
        assert.strictEqual(isMultiUserMode({ WEFTOS_MULTI_USER: "1" }), true);
        assert.strictEqual(isMultiUserMode({ WEFTOS_MULTI_USER: "true" }), true);
        assert.strictEqual(isMultiUserMode({ WEFTOS_MULTI_USER: "YES" }), true);
        assert.strictEqual(isMultiUserMode({}, true), true);
        assert.strictEqual(
            isMultiUserMode({ WEFTOS_MULTI_USER: "1" }, false),
            false,
            "explicit override false wins over env",
        );
    });

    test("requiredScope classifies known verbs", () => {
        assert.strictEqual(requiredScope("kernel.status"), "read");
        assert.strictEqual(requiredScope("agent.chat"), "chat");
        assert.strictEqual(requiredScope("agent.chat_stream"), "chat");
        assert.strictEqual(requiredScope("terminal.spawn"), "write");
        assert.strictEqual(requiredScope("control.set_enabled"), "write");
        assert.strictEqual(requiredScope("kernel.kill-process"), "admin");
        assert.strictEqual(requiredScope("zzz.unknown"), "read");
    });

    test("default panel session is viewer (read+chat)", () => {
        const s = issuePanelSession();
        assert.ok(s.panelId.length > 0);
        assert.ok(s.token.length > 0);
        assert.deepStrictEqual(s.scopes, ["read", "chat"]);
        assert.strictEqual(scopesToAuth(s.scopes), "read,chat");
        assert.ok(scopesAllowMethod(s.scopes, "kernel.status"));
        assert.ok(scopesAllowMethod(s.scopes, "agent.chat"));
        assert.ok(!scopesAllowMethod(s.scopes, "terminal.spawn"));
        assert.ok(!scopesAllowMethod(s.scopes, "kernel.kill-process"));
    });

    test("denied-by-identity: viewer panel cannot call Write method", () => {
        const session: PanelSession = issuePanelSession(["read", "chat"]);
        const decision = authorizePanelRpc(session, "terminal.spawn");
        assert.strictEqual(decision.ok, false);
        if (!decision.ok) {
            assert.strictEqual(decision.reason, "scope");
            assert.match(decision.error, /permission denied/);
            assert.match(decision.error, /lack 'write'/);
            assert.match(decision.error, /terminal\.spawn/);
        }
    });

    test("denied-by-identity: missing session under multi-user", () => {
        const decision = authorizePanelRpc(undefined, "kernel.status");
        assert.strictEqual(decision.ok, false);
        if (!decision.ok) {
            assert.strictEqual(decision.reason, "no_session");
            assert.match(decision.error, /panel identity required/);
        }
    });

    test("denied-by-identity: expired session", () => {
        const session = issuePanelSession(["read", "chat"], 1);
        // Force past expiry.
        session.expiresAt = Date.now() - 1;
        const decision = authorizePanelRpc(session, "kernel.status");
        assert.strictEqual(decision.ok, false);
        if (!decision.ok) {
            assert.strictEqual(decision.reason, "expired");
        }
    });

    test("authorize allows read+chat for viewer and attaches wire auth", () => {
        const session = issuePanelSession();
        const status = authorizePanelRpc(session, "kernel.status");
        assert.strictEqual(status.ok, true);
        if (status.ok) {
            assert.strictEqual(status.auth, "read,chat");
        }
        const chat = authorizePanelRpc(session, "agent.chat");
        assert.strictEqual(chat.ok, true);
        if (chat.ok) {
            assert.strictEqual(chat.auth, "read,chat");
        }
    });

    test("admin scope implies all methods", () => {
        const session = issuePanelSession(["admin"]);
        assert.ok(scopesAllowMethod(session.scopes, "kernel.kill-process"));
        assert.ok(scopesAllowMethod(session.scopes, "terminal.spawn"));
        const decision = authorizePanelRpc(session, "kernel.kill-process");
        assert.strictEqual(decision.ok, true);
        if (decision.ok) {
            assert.strictEqual(decision.auth, "admin");
        }
    });

    test("write scope does not imply admin", () => {
        const session = issuePanelSession(["write"]);
        assert.ok(scopesAllowMethod(session.scopes, "terminal.spawn"));
        assert.ok(!scopesAllowMethod(session.scopes, "kernel.kill-process"));
        const denied = authorizePanelRpc(session, "kernel.kill-process");
        assert.strictEqual(denied.ok, false);
        if (!denied.ok) {
            assert.strictEqual(denied.reason, "scope");
        }
    });
});
