// WEFT-284 — unit tests for ThreadDock wire helpers (no VSCode host).
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
    THREAD_DOCK_IDENTITY,
    appendLine,
    clearThreads,
    createAgentThread,
    emptyThreadDock,
    focusThread,
    focusThreadById,
    isActivePhase,
    isParallel,
    parseAgentThread,
    parseThreadPhase,
    pushThread,
    removeThread,
    setStreamText,
    streamsAreIsolated,
    upsertThread,
} from "./threadDock.ts";

describe("WEFT-284 ThreadDock", () => {
    it("identity is stable", () => {
        assert.equal(THREAD_DOCK_IDENTITY, "ui://thread-dock");
    });

    it("isParallel requires ≥2 columns", () => {
        const s = emptyThreadDock();
        assert.equal(isParallel(s), false);
        pushThread(s, createAgentThread("a", "coder"));
        assert.equal(isParallel(s), false);
        pushThread(s, createAgentThread("b", "reviewer"));
        assert.equal(isParallel(s), true);
        assert.equal(s.threads.length, 2);
    });

    it("smoke: ≥2 parallel streams stay isolated", () => {
        const s = emptyThreadDock();
        pushThread(
            s,
            createAgentThread("swarm/coder", "coder", {
                phase: "streaming",
                variantId: 11,
            }),
        );
        pushThread(
            s,
            createAgentThread("swarm/reviewer", "reviewer", {
                phase: "streaming",
                variantId: 12,
            }),
        );
        for (let i = 0; i < 5; i++) {
            appendLine(s, "swarm/coder", `[swarm/coder] token ${i}`);
            appendLine(s, "swarm/reviewer", `[swarm/reviewer] token ${i}`);
        }
        assert.equal(isParallel(s), true);
        assert.equal(s.threads[0].lines.length, 5);
        assert.equal(s.threads[1].lines.length, 5);
        assert.equal(streamsAreIsolated(s), true);
        // No interleaving: each buffer only has its own tags.
        assert.ok(s.threads[0].lines.every((l) => l.includes("[swarm/coder]")));
        assert.ok(s.threads[1].lines.every((l) => l.includes("[swarm/reviewer]")));
        assert.ok(!s.threads[0].lines.some((l) => l.includes("[swarm/reviewer]")));
    });

    it("focus lane switches by index and id", () => {
        const s = emptyThreadDock();
        pushThread(s, createAgentThread("a", "coder"));
        pushThread(s, createAgentThread("b", "reviewer"));
        assert.equal(s.focused, 0);
        assert.equal(focusThread(s, 1), true);
        assert.equal(s.threads[s.focused].id, "b");
        assert.equal(focusThreadById(s, "a"), true);
        assert.equal(s.threads[s.focused].label, "coder");
        assert.equal(focusThreadById(s, "missing"), false);
        assert.equal(focusThread(s, 99), false);
    });

    it("setStreamText replaces with accumulated draft", () => {
        const s = emptyThreadDock();
        pushThread(s, createAgentThread("a", "coder"));
        assert.equal(setStreamText(s, "a", "hello\nworld\n"), true);
        assert.deepEqual(s.threads[0].lines, ["hello", "world", ""]);
        assert.equal(s.threads[0].phase, "streaming");
        assert.equal(setStreamText(s, "missing", "x"), false);
    });

    it("upsert replaces same id; remove adjusts focus", () => {
        const s = emptyThreadDock();
        pushThread(s, createAgentThread("a", "coder"));
        pushThread(s, createAgentThread("b", "reviewer"));
        pushThread(s, createAgentThread("c", "tester"));
        upsertThread(
            s,
            createAgentThread("a", "coder-v2", { phase: "done", lines: ["done"] }),
        );
        assert.equal(s.threads.length, 3);
        assert.equal(s.threads[0].label, "coder-v2");
        assert.equal(s.threads[0].phase, "done");
        focusThread(s, 2);
        assert.equal(removeThread(s, "c"), true);
        assert.equal(s.threads.length, 2);
        assert.equal(s.focused, 1);
        clearThreads(s);
        assert.equal(s.threads.length, 0);
        assert.equal(isParallel(s), false);
    });

    it("parseThreadPhase accepts generating alias", () => {
        assert.equal(parseThreadPhase("generating"), "streaming");
        assert.equal(parseThreadPhase("awaiting"), "awaiting-input");
        assert.equal(parseThreadPhase("thinking"), "thinking");
        assert.equal(parseThreadPhase("nope"), null);
        assert.equal(isActivePhase("streaming"), true);
        assert.equal(isActivePhase("done"), false);
    });

    it("parseAgentThread accepts kebab and camel wire keys", () => {
        const t = parseAgentThread({
            id: "x",
            label: "coder",
            phase: "generating",
            lines: ["a", "b"],
            token_rate: 12,
            "variant-id": "pulse-9",
        });
        assert.ok(t);
        assert.equal(t!.phase, "streaming");
        assert.equal(t!.tokenRate, 12);
        assert.equal(t!.variantId, "pulse-9");
        assert.deepEqual(t!.lines, ["a", "b"]);
        assert.equal(parseAgentThread({ label: "no-id" }), null);
        assert.equal(parseAgentThread(null), null);
    });
});
