// M0 WeftOS dev-panel extension.
//
// Scope (do not expand without reading
//   .planning/symposiums/compositional-ui/session-7-dev-panel-embedding.md):
//   - register `weft.openPanel`
//   - open a sovereign-posture WebviewPanel in the editor area
//   - retainContextWhenHidden so tab switches do not reboot the view
//   - WebviewPanelSerializer so window reloads restore an empty panel
//   - proxy `kernel.status` and `kernel.ps` to the daemon UDS
//   - record return events locally to prove the active-radar channel exists
//
// Out of scope for M0: WSP-0.1 (protocol spec lives with RQ6),
// egui-wasm rendering (M1), voice/capture sidecar (M2), workspace topics (M2).

import * as vscode from "vscode";
import { randomUUID } from "node:crypto";
import { resolveSocketPath, rpcCall, RpcError } from "./rpc";

const VIEW_TYPE = "weft.panel";
const ALLOWED_METHODS = new Set<string>(["kernel.status", "kernel.ps"]);

interface WebviewRpcRequest {
    kind: "rpc";
    id: string;
    method: string;
}

interface WebviewReadyMessage {
    kind: "ready";
}

type WebviewInbound = WebviewRpcRequest | WebviewReadyMessage;

export function activate(context: vscode.ExtensionContext): void {
    const openCmd = vscode.commands.registerCommand("weft.openPanel", () => {
        createOrShowPanel(context);
    });
    context.subscriptions.push(openCmd);

    // Survives VSCode window reloads. M0 restores the shell only —
    // state is not rehydrated; the user re-clicks to re-fetch.
    const serializer: vscode.WebviewPanelSerializer = {
        async deserializeWebviewPanel(panel: vscode.WebviewPanel): Promise<void> {
            wirePanel(context, panel);
        },
    };
    context.subscriptions.push(
        vscode.window.registerWebviewPanelSerializer(VIEW_TYPE, serializer),
    );
}

export function deactivate(): void {
    // Nothing durable to tear down in M0.
}

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
            localResourceRoots: [vscode.Uri.joinPath(context.extensionUri, "media")],
        },
    );
    wirePanel(context, panel);
}

function wirePanel(context: vscode.ExtensionContext, panel: vscode.WebviewPanel): void {
    currentPanel = panel;
    panel.webview.options = {
        enableScripts: true,
        localResourceRoots: [vscode.Uri.joinPath(context.extensionUri, "media")],
    };
    panel.webview.html = renderHtml(context, panel.webview);

    const cwd = getWorkspaceCwd();
    const socketPath = resolveSocketPath(cwd);

    panel.webview.onDidReceiveMessage(
        async (raw: unknown) => {
            const msg = raw as WebviewInbound;
            if (!msg || typeof msg !== "object") {
                return;
            }
            if (msg.kind === "ready") {
                void panel.webview.postMessage({
                    kind: "hello",
                    socketPath,
                });
                return;
            }
            if (msg.kind === "rpc") {
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
    req: WebviewRpcRequest,
): Promise<void> {
    if (!ALLOWED_METHODS.has(req.method)) {
        void panel.webview.postMessage({
            kind: "rpc-result",
            id: req.id,
            method: req.method,
            ok: false,
            error: `method not allowed in M0: ${req.method}`,
        });
        return;
    }

    try {
        const resp = await rpcCall(socketPath, {
            method: req.method,
            params: null,
            id: randomUUID(),
        });
        void panel.webview.postMessage({
            kind: "rpc-result",
            id: req.id,
            method: req.method,
            ok: resp.ok,
            result: resp.result ?? null,
            error: resp.error,
        });
    } catch (err) {
        const message = err instanceof RpcError ? err.message : String(err);
        void panel.webview.postMessage({
            kind: "rpc-result",
            id: req.id,
            method: req.method,
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
    const mediaRoot = vscode.Uri.joinPath(context.extensionUri, "media");
    const cssUri = webview.asWebviewUri(vscode.Uri.joinPath(mediaRoot, "main.css"));
    const jsUri = webview.asWebviewUri(vscode.Uri.joinPath(mediaRoot, "main.js"));
    const csp = [
        "default-src 'none'",
        `style-src ${webview.cspSource}`,
        `script-src 'nonce-${nonce}'`,
        `img-src ${webview.cspSource}`,
        "connect-src 'none'",
    ].join("; ");

    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta http-equiv="Content-Security-Policy" content="${csp}" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <link rel="stylesheet" href="${cssUri}" />
  <title>WeftOS Panel</title>
</head>
<body>
  <header class="bar">
    <h1>WeftOS Panel <span class="tag">M0</span></h1>
    <div class="status" id="statusLine">initialising...</div>
  </header>
  <main>
    <section class="controls">
      <button id="btnStatus" type="button">Fetch kernel.status</button>
      <button id="btnPs" type="button">Fetch kernel.ps</button>
    </section>
    <section class="output">
      <h2>Response</h2>
      <pre id="output">(no request yet)</pre>
    </section>
    <section class="radar">
      <h2>Active-radar return log (last 5)</h2>
      <ol id="radarLog"></ol>
    </section>
  </main>
  <script nonce="${nonce}" src="${jsUri}"></script>
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
