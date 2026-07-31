// Unit tests for WSP-0.1 verb adapter (WEFT-285).
//
// Runnable with:
//   npx --yes tsx --test src/wsp.test.ts
// from extensions/vscode-weft-panel (no VSCode host required).

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
    AFFORDANCE_RPC_MAP,
    RESOURCE_RPC_MAP,
    WSP_VERBS,
    WSP_VERB_SET,
    WSP_WIRE_VERSION,
    createWspSession,
    extractSession,
    isWspVerb,
    lookupAffordance,
    lookupResourceBinding,
    publicWspResult,
    translateWsp,
    verbMappingTable,
    wrapWspRpcResult,
} from "./wsp";

describe("WSP-0.1 verb catalog", () => {
    it("freezes exactly 17 verbs (ADR-005 hard cap)", () => {
        assert.equal(WSP_VERBS.length, 17);
        assert.equal(WSP_VERB_SET.size, 17);
    });

    it("includes session, surface, subscribe, invoke, gate, ontology", () => {
        for (const v of [
            "session.initialize",
            "session.shutdown",
            "surface.compose",
            "surface.get",
            "subscribe",
            "invoke",
            "gate.check",
            "ontology.describe",
            "cancel",
        ]) {
            assert.equal(isWspVerb(v), true, v);
        }
    });

    it("rejects raw daemon RPC names as WSP verbs", () => {
        assert.equal(isWspVerb("kernel.status"), false);
        assert.equal(isWspVerb("agent.chat"), false);
        assert.equal(isWspVerb("substrate.publish"), false);
    });

    it("verbMappingTable covers every verb", () => {
        const rows = verbMappingTable();
        assert.equal(rows.length, 17);
        const verbs = new Set(rows.map((r) => r.verb));
        for (const v of WSP_VERBS) {
            assert.equal(verbs.has(v), true, `missing row for ${v}`);
        }
    });
});

describe("resource → RPC map", () => {
    it("maps kernel status surface (migrated common op)", () => {
        const b = lookupResourceBinding("resource://kernel/status");
        assert.ok(b);
        assert.equal(b!.method, "kernel.status");
        assert.equal(b!.scope, "read");
    });

    it("is case-insensitive on URI", () => {
        const b = lookupResourceBinding("Resource://Kernel/Status");
        assert.ok(b);
        assert.equal(b!.method, "kernel.status");
    });

    it("covers panel read surfaces used by chips / explorer", () => {
        const expected = [
            "kernel.status",
            "kernel.ps",
            "kernel.services",
            "kernel.logs",
            "cluster.status",
            "chain.status",
            "control.list",
            "cron.list",
            "llm.models",
        ];
        const methods = Object.values(RESOURCE_RPC_MAP).map((r) => r.method);
        for (const m of expected) {
            assert.equal(methods.includes(m), true, m);
        }
    });

    it("never maps a resource to substrate.publish", () => {
        for (const [uri, b] of Object.entries(RESOURCE_RPC_MAP)) {
            assert.notEqual(b.method, "substrate.publish", uri);
        }
    });
});

describe("affordance → RPC map", () => {
    it("maps chat and terminal affordances", () => {
        assert.equal(lookupAffordance("chat")?.method, "agent.chat");
        assert.equal(
            lookupAffordance("chat_stream")?.method,
            "agent.chat_stream",
        );
        assert.equal(
            lookupAffordance("terminal_write")?.method,
            "terminal.write",
        );
    });

    it("never includes substrate.publish", () => {
        for (const [name, b] of Object.entries(AFFORDANCE_RPC_MAP)) {
            assert.notEqual(b.method, "substrate.publish", name);
        }
    });
});

describe("session.initialize / shutdown", () => {
    it("creates session and returns server-caps", () => {
        const action = translateWsp(undefined, "session.initialize", {
            "wsp-version": WSP_WIRE_VERSION,
            persona: "persona://dev-panel",
            locale: "en-US",
            "supported-encodings": ["json"],
        });
        assert.equal(action.kind, "local");
        if (action.kind !== "local") return;
        const pub = publicWspResult(action.result) as Record<string, unknown>;
        assert.equal(pub["wsp-version"], WSP_WIRE_VERSION);
        assert.equal(typeof pub["session-id"], "string");
        assert.ok(Array.isArray(pub.verbs));
        assert.equal((pub.verbs as string[]).length, 17);
        assert.ok(
            (pub["capability-flags"] as string[]).includes("raw-rpc-compat"),
        );
        const session = extractSession(action.result);
        assert.ok(session);
        assert.equal(session!.closed, false);
        assert.equal(session!.persona, "persona://dev-panel");
    });

    it("rejects unsupported wire version", () => {
        const action = translateWsp(undefined, "session.initialize", {
            "wsp-version": 99,
        });
        assert.equal(action.kind, "error");
        if (action.kind === "error") {
            assert.equal(action.code, -32002);
        }
    });

    it("requires session for non-initialize verbs", () => {
        const action = translateWsp(undefined, "subscribe", {
            resource_uri: "resource://kernel/status",
        });
        assert.equal(action.kind, "error");
        if (action.kind === "error") {
            assert.equal(action.code, -32031);
        }
    });

    it("shutdown drains and marks closed", () => {
        const session = createWspSession();
        session.subscriptions.set("sub-1", {
            subscription_id: "sub-1",
            resource_uri: "resource://kernel/status",
            created_at: Date.now(),
        });
        const action = translateWsp(session, "session.shutdown", {});
        assert.equal(action.kind, "local");
        assert.equal(session.closed, true);
        assert.equal(session.subscriptions.size, 0);
    });
});

describe("subscribe → RPC mapping (migrated kernel status surface)", () => {
    it("maps resource://kernel/status to kernel.status RPC", () => {
        const session = createWspSession();
        const action = translateWsp(session, "subscribe", {
            resource_uri: "resource://kernel/status",
        });
        assert.equal(action.kind, "rpc");
        if (action.kind !== "rpc") return;
        assert.equal(action.method, "kernel.status");
        assert.ok(action.wrapResult);
        assert.equal(action.wrapResult!.type, "subscription");
        assert.equal(session.subscriptions.size, 1);
    });

    it("maps substrate topics to substrate.subscribe", () => {
        const session = createWspSession();
        const action = translateWsp(session, "subscribe", {
            resource_uri: "substrate://_derived/chat/demo/stream",
        });
        assert.equal(action.kind, "rpc");
        if (action.kind !== "rpc") return;
        assert.equal(action.method, "substrate.subscribe");
    });

    it("maps substrate read mode", () => {
        const session = createWspSession();
        const action = translateWsp(session, "subscribe", {
            resource_uri: "substrate://foo/bar",
            filter: { mode: "read" },
        });
        assert.equal(action.kind, "rpc");
        if (action.kind !== "rpc") return;
        assert.equal(action.method, "substrate.read");
    });

    it("errors on unknown topic", () => {
        const session = createWspSession();
        const action = translateWsp(session, "subscribe", {
            resource_uri: "bogus://nope",
        });
        assert.equal(action.kind, "error");
        if (action.kind === "error") {
            assert.equal(action.code, -32016);
        }
    });

    it("unsubscribe is local ok", () => {
        const session = createWspSession();
        const sub = translateWsp(session, "subscribe", {
            resource_uri: "resource://kernel/ps",
        });
        assert.equal(sub.kind, "rpc");
        const subId = [...session.subscriptions.keys()][0];
        const action = translateWsp(session, "unsubscribe", {
            subscription_id: subId,
        });
        assert.equal(action.kind, "local");
        assert.equal(session.subscriptions.size, 0);
    });
});

describe("invoke → RPC mapping", () => {
    it("maps chat affordance to agent.chat", () => {
        const session = createWspSession();
        const action = translateWsp(session, "invoke", {
            surface_id: "s1",
            path: "/chat",
            affordance: "chat",
            args: { message: "hello" },
            actor: "actor://user",
        });
        assert.equal(action.kind, "rpc");
        if (action.kind !== "rpc") return;
        assert.equal(action.method, "agent.chat");
        assert.deepEqual(action.params, { message: "hello" });
    });

    it("rejects unknown affordance", () => {
        const session = createWspSession();
        const action = translateWsp(session, "invoke", {
            affordance: "explode_everything",
            args: {},
        });
        assert.equal(action.kind, "error");
        if (action.kind === "error") {
            assert.equal(action.code, -32021);
        }
    });
});

describe("surfaces", () => {
    it("compose → get → update → dispose", () => {
        const session = createWspSession();
        const c = translateWsp(session, "surface.compose", {
            root: { type: "ui://chip", label: "Kernel" },
        });
        assert.equal(c.kind, "local");
        if (c.kind !== "local") return;
        const { surface_id, surface_version } = c.result as {
            surface_id: string;
            surface_version: number;
        };
        assert.equal(surface_version, 1);

        const g = translateWsp(session, "surface.get", { surface_id });
        assert.equal(g.kind, "local");

        const u = translateWsp(session, "surface.update", {
            surface_id,
            base_version: 1,
            ops: [{ op: "replace", path: "/label", value: "OK" }],
        });
        assert.equal(u.kind, "local");
        if (u.kind === "local") {
            assert.equal((u.result as { new_version: number }).new_version, 2);
        }

        const mismatch = translateWsp(session, "surface.update", {
            surface_id,
            base_version: 1,
            ops: [],
        });
        assert.equal(mismatch.kind, "error");

        const d = translateWsp(session, "surface.dispose", { surface_id });
        assert.equal(d.kind, "local");
        assert.equal(session.surfaces.size, 0);
    });
});

describe("gate.check / mutate / consent / ontology / cancel", () => {
    it("denies substrate.publish via gate.check", () => {
        const session = createWspSession();
        const action = translateWsp(session, "gate.check", {
            action: "substrate.publish",
            subject: "webview",
            context: {},
        });
        assert.equal(action.kind, "local");
        if (action.kind === "local") {
            const r = action.result as { decision: { deny: unknown } };
            assert.ok(r.decision.deny);
        }
    });

    it("allows known resource actions", () => {
        const session = createWspSession();
        const action = translateWsp(session, "gate.check", {
            action: "resource://kernel/status",
            subject: "webview",
        });
        assert.equal(action.kind, "local");
        if (action.kind === "local") {
            assert.equal(
                (action.result as { decision: string }).decision,
                "allow",
            );
        }
    });

    it("freezes consent/safety/brand mutate axes", () => {
        const session = createWspSession();
        const c = translateWsp(session, "surface.compose", { root: {} });
        assert.equal(c.kind, "local");
        const surface_id = (c as { result: { surface_id: string } }).result
            .surface_id;
        for (const axis of ["consent", "safety", "brand"]) {
            const action = translateWsp(session, "mutate", {
                surface_id,
                axis,
                new_variant: "x",
            });
            assert.equal(action.kind, "error", axis);
            if (action.kind === "error") {
                assert.equal(action.code, -32022);
            }
        }
    });

    it("consent.request composes a modal surface", () => {
        const session = createWspSession();
        const action = translateWsp(session, "consent.request", {
            scope: "scope://mic",
            purpose: "voice input",
            duration: "session",
        });
        assert.equal(action.kind, "local");
        if (action.kind === "local") {
            const r = action.result as {
                consent_id: string;
                surface_id: string;
            };
            assert.ok(r.consent_id);
            assert.ok(session.surfaces.has(r.surface_id));
        }
    });

    it("ontology.describe dumps verb catalog", () => {
        const session = createWspSession();
        const action = translateWsp(session, "ontology.describe", {
            target: "wsp://verbs",
        });
        assert.equal(action.kind, "local");
        if (action.kind === "local") {
            const r = action.result as { verbs: string[] };
            assert.equal(r.verbs.length, 17);
        }
    });

    it("ontology.describe on resource binding", () => {
        const session = createWspSession();
        const action = translateWsp(session, "ontology.describe", {
            target: "resource://kernel/status",
        });
        assert.equal(action.kind, "local");
        if (action.kind === "local") {
            assert.equal(
                (action.result as { rpc_method: string }).rpc_method,
                "kernel.status",
            );
        }
    });

    it("cancel is idempotent", () => {
        const session = createWspSession();
        const a = translateWsp(session, "cancel", { id: "already-done" });
        assert.equal(a.kind, "local");
        if (a.kind === "local") {
            assert.equal((a.result as { ok: boolean }).ok, true);
        }
    });
});

describe("wrapWspRpcResult / publicWspResult", () => {
    it("wraps subscription snapshot", () => {
        const wrapped = wrapWspRpcResult(
            {
                type: "subscription",
                subscription_id: "sub-1",
                resource_uri: "resource://kernel/status",
            },
            { uptime: 42 },
            true,
        ) as {
            subscription_id: string;
            snapshot: { uptime: number };
        };
        assert.equal(wrapped.subscription_id, "sub-1");
        assert.equal(wrapped.snapshot.uptime, 42);
    });

    it("strips __session from public result", () => {
        const session = createWspSession();
        const pub = publicWspResult({
            "session-id": session.session_id,
            __session: session,
        }) as Record<string, unknown>;
        assert.equal(pub["session-id"], session.session_id);
        assert.equal("__session" in pub, false);
    });
});

describe("unknown verb", () => {
    it("returns MethodNotFound", () => {
        const action = translateWsp(createWspSession(), "kernel.status", {});
        assert.equal(action.kind, "error");
        if (action.kind === "error") {
            assert.equal(action.code, -32601);
        }
    });
});
