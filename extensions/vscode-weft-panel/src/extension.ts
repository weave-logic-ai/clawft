// WeftOS dev-panel extension.
//
// M0: static HTML + two sample RPC buttons. M1 (this file): replaces
// the static UI with the egui surface compiled to wasm, loaded into
// the webview, and wired to the daemon through a typed postMessage
// bridge.
//
// Scope references:
//   .planning/symposiums/compositional-ui/session-7-dev-panel-embedding.md
//   .planning/symposiums/compositional-ui/adrs/adr-011-dev-panel-embedding-hybrid.md
//
// Out of scope for M1: WSP-0.1 verbs (raw kernel.* RPC only),
// voice/capture sidecar (M2), workspace editor topics (M2),
// active-radar typed return channel (M2).

import * as vscode from "vscode";
import { randomUUID } from "node:crypto";
import { resolveSocketPath, rpcCall, RpcError } from "./rpc";

const VIEW_TYPE = "weft.panel";

// Allowed RPC methods the extension will proxy from the webview. M1
// expands on M0 to cover the four poll methods the wasm `Live` uses.
const ALLOWED_METHODS = new Set<string>([
    "kernel.status",
    "kernel.ps",
    "kernel.services",
    "kernel.logs",
]);

interface WasmRpcRequest {
    type: "rpc-request";
    id: number;
    method: string;
    params?: unknown;
}

interface WebviewReadyMessage {
    type: "ready";
}

type WebviewInbound = WasmRpcRequest | WebviewReadyMessage;

export function activate(context: vscode.ExtensionContext): void {
    const openCmd = vscode.commands.registerCommand("weft.openPanel", () => {
        createOrShowPanel(context);
    });
    context.subscriptions.push(openCmd);

    const serializer: vscode.WebviewPanelSerializer = {
        async deserializeWebviewPanel(panel: vscode.WebviewPanel): Promise<void> {
            wirePanel(context, panel);
        },
    };
    context.subscriptions.push(
        vscode.window.registerWebviewPanelSerializer(VIEW_TYPE, serializer),
    );
}

export function deactivate(): void {}

let currentPanel: vscode.WebviewPanel | undefined;

function createOrShowPanel(context: vscode.ExtensionContext): void {
    if (currentPanel) {
        currentPanel.reveal(vscode.ViewColumn.Active);
        return;
    }

    const panel = vscode.window.createWebviewPanel(
        VIEW_TYPE,
        "WeftOS Panel",
        vscode.ViewColumn.Active,
        {
            enableScripts: true,
            retainContextWhenHidden: true,
            localResourceRoots: [
                vscode.Uri.joinPath(context.extensionUri, "media"),
                vscode.Uri.joinPath(context.extensionUri, "webview"),
            ],
        },
    );
    wirePanel(context, panel);
}

function wirePanel(context: vscode.ExtensionContext, panel: vscode.WebviewPanel): void {
    currentPanel = panel;
    panel.webview.options = {
        enableScripts: true,
        localResourceRoots: [
            vscode.Uri.joinPath(context.extensionUri, "media"),
            vscode.Uri.joinPath(context.extensionUri, "webview"),
        ],
    };
    panel.webview.html = renderHtml(context, panel.webview);

    const cwd = getWorkspaceCwd();
    const socketPath = resolveSocketPath(cwd);

    panel.webview.onDidReceiveMessage(
        async (raw: unknown) => {
            const msg = raw as WebviewInbound | null;
            if (!msg || typeof msg !== "object") {
                return;
            }
            if (msg.type === "ready") {
                void panel.webview.postMessage({ type: "hello", socketPath });
                return;
            }
            if (msg.type === "rpc-request") {
                await handleRpc(panel, socketPath, msg);
            }
        },
        undefined,
        context.subscriptions,
    );

    panel.onDidDispose(
        () => {
            if (currentPanel === panel) {
                currentPanel = undefined;
            }
        },
        null,
        context.subscriptions,
    );
}

async function handleRpc(
    panel: vscode.WebviewPanel,
    socketPath: string,
    req: WasmRpcRequest,
): Promise<void> {
    if (!ALLOWED_METHODS.has(req.method)) {
        void panel.webview.postMessage({
            type: "rpc-response",
            id: req.id,
            ok: false,
            error: `method not allowed: ${req.method}`,
        });
        return;
    }

    try {
        const resp = await rpcCall(socketPath, {
            method: req.method,
            params: req.params ?? null,
            id: randomUUID(),
        });
        void panel.webview.postMessage({
            type: "rpc-response",
            id: req.id,
            ok: resp.ok,
            result: resp.result ?? null,
            error: resp.error,
        });
    } catch (err) {
        const message = err instanceof RpcError ? err.message : String(err);
        void panel.webview.postMessage({
            type: "rpc-response",
            id: req.id,
            ok: false,
            error: message,
        });
    }
}

function getWorkspaceCwd(): string | undefined {
    const folder = vscode.workspace.workspaceFolders?.[0];
    return folder?.uri.fsPath;
}

function renderHtml(context: vscode.ExtensionContext, webview: vscode.Webview): string {
    const nonce = makeNonce();
    const wasmRoot = vscode.Uri.joinPath(context.extensionUri, "webview", "wasm");
    const jsUri = webview.asWebviewUri(vscode.Uri.joinPath(wasmRoot, "clawft_gui_egui.js"));
    const wasmUri = webview.asWebviewUri(
        vscode.Uri.joinPath(wasmRoot, "clawft_gui_egui_bg.wasm"),
    );

    // CSP note: `wasm-unsafe-eval` is required to instantiate WebAssembly
    // modules inside a webview. `script-src` allows the nonce'd bootstrap
    // plus the wasm-bindgen JS glue from `localResourceRoots`.
    const csp = [
        "default-src 'none'",
        `script-src 'nonce-${nonce}' ${webview.cspSource} 'wasm-unsafe-eval'`,
        `style-src 'nonce-${nonce}' ${webview.cspSource} 'unsafe-inline'`,
        `img-src ${webview.cspSource} data: blob:`,
        `font-src ${webview.cspSource}`,
        `connect-src ${webview.cspSource} blob:`,
        "worker-src blob:",
    ].join("; ");

    const wasmNotFoundHint = wasmUri.toString();

    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta http-equiv="Content-Security-Policy" content="${csp}" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>WeftOS Panel</title>
  <style nonce="${nonce}">
    html, body { margin: 0; padding: 0; height: 100%; background: #04040a;
      color: #e0dee8; font-family: system-ui, sans-serif; overflow: hidden; }
    #weft-root { position: fixed; inset: 0; display: flex; flex-direction: column; }
    #weft-canvas { flex: 1 1 auto; width: 100%; display: block; outline: none; }
    #weft-fallback { padding: 24px; font-size: 13px; color: #aaa; display: none; }
    #weft-fallback code { background: #1a1a24; padding: 2px 6px; border-radius: 3px;
      color: #c4a25c; }
    body.loading #weft-splash { position: fixed; inset: 0; display: grid;
      place-items: center; font-size: 13px; color: #7a7a86;
      background: #04040a; z-index: 10; }
    body:not(.loading) #weft-splash { display: none; }
  </style>
</head>
<body class="loading">
  <div id="weft-splash">loading egui shell…</div>
  <div id="weft-root">
    <canvas id="weft-canvas" tabindex="0"></canvas>
    <div id="weft-fallback">
      <p><strong>Failed to load the wasm bundle.</strong></p>
      <p>Run <code>extensions/vscode-weft-panel/scripts/build-wasm.sh</code>
         from the repo root and reload the panel.</p>
      <p>Expected at <code>${wasmNotFoundHint}</code>.</p>
    </div>
  </div>
  <script type="module" nonce="${nonce}">
    import init, { weft_start } from "${jsUri.toString()}";

    const vscode = acquireVsCodeApi();

    // Bridge exposed to the wasm module. clawft_gui_egui::live::wasm_live
    // looks up window.__weftPostToHost and calls it with a JSON value.
    // Request shape:  { type: "rpc-request", id, method, params }
    // Response shape: { type: "rpc-response", id, ok, result?, error? }
    window.__weftPostToHost = (payload) => {
      try {
        vscode.postMessage(payload);
      } catch (e) {
        console.error("postMessage failed", e);
      }
    };

    // VSCode delivers extension→webview messages via window 'message'
    // events on the default message channel. The wasm side's listener
    // reads ev.data directly; nothing extra needed here.

    try {
      await init({ module_or_path: "${wasmUri.toString()}" });
      await weft_start("weft-canvas");
      document.body.classList.remove("loading");
      vscode.postMessage({ type: "ready" });
    } catch (e) {
      console.error("wasm boot failed", e);
      document.body.classList.remove("loading");
      document.getElementById("weft-fallback").style.display = "block";
      document.getElementById("weft-canvas").style.display = "none";
    }
  </script>
</body>
</html>`;
}

function makeNonce(): string {
    const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let out = "";
    for (let i = 0; i < 32; i += 1) {
        out += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return out;
}
