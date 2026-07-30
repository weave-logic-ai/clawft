// Headless VSCode E2E test entry point (WEFT-486 / WEFT-558).
//
// Downloads a pinned VSCode build, launches it with the
// `vscode-weft-panel` extension installed, and runs the Mocha suite at
// `out/test/suite/index.js`. Designed to run under `xvfb-run` in CI.
//
// WEFT-558 seeds `WEFT_PANEL_E2E=1` so the panel HTML includes the
// mock chip a11y strip on first paint; the suite also re-injects via
// `weft._test.chipStripSnapshot` for a deterministic assertion.
import * as os from "node:os";
import * as path from "node:path";
import { runTests } from "@vscode/test-electron";

async function main(): Promise<void> {
    try {
        // Extension under test = this extension. Lets the harness find
        // package.json and load `out/extension.js`.
        // __dirname at runtime: .../extensions/vscode-weft-panel/out-test
        const extensionDevelopmentPath = path.resolve(__dirname, "..");
        // Compiled Mocha entry — emitted by `tsc -p test/` into out-test/suite/.
        const extensionTestsPath = path.resolve(__dirname, "./suite/index");

        // Keep user-data / extensions dirs short: defaultCachePath under a
        // deep worktree exceeds the ~103-char Unix domain socket limit
        // (`…/user-data/1.13-main.sock` → EINVAL). tmpdir is always short.
        const shortProfile = path.join(os.tmpdir(), "weft-vsc-e2e");

        // Empty workspace — no folder open. The panel doesn't need a
        // workspace to render, only to resolve the daemon socket. Tests
        // that need a workspace can pass `--folder-uri` later.
        await runTests({
            extensionDevelopmentPath,
            extensionTestsPath,
            launchArgs: [
                "--disable-extensions",
                "--disable-workspace-trust",
                `--user-data-dir=${path.join(shortProfile, "ud")}`,
                `--extensions-dir=${path.join(shortProfile, "ex")}`,
            ],
            // Propagate into the extension host so renderHtml can seed
            // the mock chip strip (WEFT-558). Harmless if ignored.
            extensionTestsEnv: {
                ...process.env,
                WEFT_PANEL_E2E: "1",
            },
        });
    } catch (err) {
        console.error("Failed to run tests:", err);
        process.exit(1);
    }
}

void main();
