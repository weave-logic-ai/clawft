// VSCode panel end-to-end smoke (WEFT-486 / M6-B + WEFT-558 chip DOM).
//
// Boots a headless VSCode host with the WeftOS Dev Panel installed,
// then exercises the extension's command surface and webview shell.
//
// Scope:
//   - Activation succeeds; the package's `weft.openPanel` command is
//     registered.
//   - `executeCommand("weft.openPanel")` resolves without throwing
//     (panel construction completes; the webview HTML is assigned).
//   - WEFT-558: chip strip exposes >=1 chip with a stable identifier
//     via the DOM-side a11y overlay (`data-weft-chip` /
//     `data-chip-id="kernel"`). Tray chips paint inside the egui
//     canvas, so the harness uses a test-only mock inject
//     (`weft._test.chipStripSnapshot`) that writes known chips into
//     `#weft-chip-a11y` and parses them back from `webview.html`.
//
// The Mocha suite is loaded by `suite/index.ts`; the host is launched
// by `runTest.ts` (under `xvfb-run` in CI).

import * as assert from "node:assert";
import * as vscode from "vscode";
// eslint-disable-next-line @typescript-eslint/no-require-imports
const { suite, test } = require("mocha");

suite("weft-panel: smoke", () => {
    test("extension is present", () => {
        const ext = vscode.extensions.getExtension(
            "weavelogic.vscode-weft-panel",
        );
        assert.ok(ext, "weavelogic.vscode-weft-panel extension not found");
    });

    test("extension activates", async () => {
        const ext = vscode.extensions.getExtension(
            "weavelogic.vscode-weft-panel",
        );
        assert.ok(ext, "extension not present");
        await ext!.activate();
        assert.strictEqual(ext!.isActive, true, "extension failed to activate");
    });

    test("weft.openPanel command is registered", async () => {
        const cmds = await vscode.commands.getCommands(true);
        assert.ok(
            cmds.includes("weft.openPanel"),
            "weft.openPanel not registered — package.json contributes drift?",
        );
    });

    test("weft.openPanel resolves without throwing", async () => {
        // Panel construction creates a webview, assigns html, wires the
        // postMessage bridge, and starts the daemon-allowlist refresh
        // (which fails-open against an offline daemon under test).
        // This proves the host wiring is intact even without a daemon.
        // `executeCommand` returns a `Thenable`, not a `Promise`. Wrap
        // in a thunk so `assert.doesNotReject`'s overloads pick the
        // Promise<unknown> path.
        await assert.doesNotReject(
            async () => {
                await vscode.commands.executeCommand("weft.openPanel");
            },
            "weft.openPanel threw",
        );
    });

    // ------------------------------------------------------------------
    // WEFT-558: chip-icon DOM assertion (followup to WEFT-486 scaffold).
    // ------------------------------------------------------------------
    test("chip strip exposes >=1 chip element with stable id", async () => {
        // Ensure the panel is open so currentPanel is set.
        await vscode.commands.executeCommand("weft.openPanel");

        const snap = (await vscode.commands.executeCommand(
            "weft._test.chipStripSnapshot",
        )) as { chips: Array<{ id: string; label: string; tone: string }>; source: string } | undefined;

        assert.ok(snap, "chipStripSnapshot returned nothing");
        assert.strictEqual(snap!.source, "html");
        assert.ok(
            Array.isArray(snap!.chips) && snap!.chips.length >= 1,
            `expected >=1 chip, got ${JSON.stringify(snap!.chips)}`,
        );
        // Stable identifier — matches tray ChipId::Kernel / MOCK_E2E_CHIPS.
        const kernel = snap!.chips.find((c) => c.id === "kernel");
        assert.ok(
            kernel,
            `expected data-chip-id="kernel" among ${snap!.chips.map((c) => c.id).join(",")}`,
        );
        assert.strictEqual(kernel!.label, "Kernel");
    });
});
