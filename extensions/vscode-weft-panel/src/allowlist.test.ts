// Unit tests for webview RPC allowlist / denylist (WEFT-496).
//
// Runnable with `npx --yes tsx --test src/allowlist.test.ts` from this
// package directory (no VSCode host required).

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
    STATIC_ALLOWED_METHODS,
    WEBVIEW_DENIED_METHODS,
    isMethodAllowed,
    mergeAllowlist,
} from "./allowlist";

describe("WEBVIEW_DENIED_METHODS", () => {
    it("always blocks substrate.publish", () => {
        assert.equal(WEBVIEW_DENIED_METHODS.has("substrate.publish"), true);
    });

    it("does not block substrate.read/list/subscribe", () => {
        for (const m of ["substrate.read", "substrate.list", "substrate.subscribe"]) {
            assert.equal(WEBVIEW_DENIED_METHODS.has(m), false, m);
        }
    });
});

describe("STATIC_ALLOWED_METHODS", () => {
    it("includes agent.chat and agent.chat_stream", () => {
        assert.equal(STATIC_ALLOWED_METHODS.has("agent.chat"), true);
        assert.equal(STATIC_ALLOWED_METHODS.has("agent.chat_stream"), true);
    });

    it("never seeds substrate.publish", () => {
        assert.equal(STATIC_ALLOWED_METHODS.has("substrate.publish"), false);
    });
});

describe("isMethodAllowed", () => {
    it("allows static seed methods", () => {
        assert.equal(isMethodAllowed("substrate.read", STATIC_ALLOWED_METHODS), true);
        assert.equal(isMethodAllowed("agent.chat", STATIC_ALLOWED_METHODS), true);
    });

    it("denies substrate.publish even if present in allowed", () => {
        const poisoned = new Set(STATIC_ALLOWED_METHODS);
        poisoned.add("substrate.publish");
        assert.equal(isMethodAllowed("substrate.publish", poisoned), false);
    });

    it("denies unknown methods", () => {
        assert.equal(isMethodAllowed("evil.method", STATIC_ALLOWED_METHODS), false);
    });
});

describe("mergeAllowlist", () => {
    it("unions daemon methods into the static seed", () => {
        const merged = mergeAllowlist(STATIC_ALLOWED_METHODS, ["mcp.list", "foo.bar"]);
        assert.equal(merged.has("agent.chat"), true);
        assert.equal(merged.has("mcp.list"), true);
        assert.equal(merged.has("foo.bar"), true);
    });

    it("strips substrate.publish even when daemon advertises it (WEFT-250 ∪ WEFT-496)", () => {
        const merged = mergeAllowlist(STATIC_ALLOWED_METHODS, [
            "substrate.publish",
            "substrate.read",
            "mcp.add",
        ]);
        assert.equal(merged.has("substrate.publish"), false);
        assert.equal(merged.has("substrate.read"), true);
        assert.equal(merged.has("mcp.add"), true);
        assert.equal(isMethodAllowed("substrate.publish", merged), false);
    });

    it("strips denied methods if they leak into the static seed", () => {
        const leaky = new Set(STATIC_ALLOWED_METHODS);
        leaky.add("substrate.publish");
        const merged = mergeAllowlist(leaky, []);
        assert.equal(merged.has("substrate.publish"), false);
    });
});
