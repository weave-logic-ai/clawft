// Minimal UDS client for the WeftOS kernel daemon.
//
// Wire format (mirrors `clawft_rpc::protocol::{Request, Response}`):
//   { "method": "kernel.status", "params": null, "id": "<uuid>" }\n
//   { "ok": true, "result": { ... }, "id": "<uuid>" }\n
//
// One connection per request keeps the M0 code trivial. M1 can migrate
// to a long-lived multiplexed connection if we need it.
//
// Socket resolution mirrors `clawft_rpc::protocol::socket_path()`:
//   1. $WEFTOS_RUNTIME_DIR/kernel.sock
//   2. nearest ancestor `.weftos/runtime/kernel.sock`
//   3. ~/.clawft/kernel.sock
// See `crates/clawft-rpc/src/protocol.rs` for canonical behaviour.

import * as net from "node:net";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

export interface RpcRequest {
    method: string;
    params?: unknown;
    id?: string;
}

export interface RpcResponse {
    ok: boolean;
    result?: unknown;
    error?: string;
    id?: string;
}

const SOCKET_NAME = "kernel.sock";

export function resolveSocketPath(cwd?: string): string {
    const override = process.env.WEFTOS_RUNTIME_DIR;
    if (override && override.length > 0) {
        return path.join(override, SOCKET_NAME);
    }

    // Walk up from cwd looking for a `.weftos/` directory.
    if (cwd) {
        let dir = path.resolve(cwd);
        while (true) {
            const candidate = path.join(dir, ".weftos");
            try {
                if (fs.statSync(candidate).isDirectory()) {
                    return path.join(candidate, "runtime", SOCKET_NAME);
                }
            } catch {
                // not found, keep walking
            }
            const parent = path.dirname(dir);
            if (parent === dir) {
                break;
            }
            dir = parent;
        }
    }

    // Global fallback.
    return path.join(os.homedir() || "/tmp", ".clawft", SOCKET_NAME);
}

export class RpcError extends Error {
    constructor(
        message: string,
        public readonly code: "ENOENT" | "ECONNREFUSED" | "EPROTO" | "EOTHER",
    ) {
        super(message);
        this.name = "RpcError";
    }
}

export function rpcCall(
    socketPath: string,
    req: RpcRequest,
    timeoutMs = 3000,
): Promise<RpcResponse> {
    return new Promise((resolve, reject) => {
        let settled = false;
        const done = (fn: () => void): void => {
            if (!settled) {
                settled = true;
                fn();
            }
        };

        const payload: RpcRequest = {
            method: req.method,
            params: req.params ?? null,
            id: req.id,
        };

        let buffer = "";
        const socket = net.createConnection(socketPath);

        const timer = setTimeout(() => {
            done(() => {
                socket.destroy();
                reject(new RpcError(`rpc timeout after ${timeoutMs}ms`, "EOTHER"));
            });
        }, timeoutMs);

        socket.on("connect", () => {
            socket.write(JSON.stringify(payload) + "\n");
        });

        socket.on("data", (chunk: Buffer) => {
            buffer += chunk.toString("utf8");
            const nl = buffer.indexOf("\n");
            if (nl >= 0) {
                const line = buffer.slice(0, nl);
                try {
                    const parsed = JSON.parse(line) as RpcResponse;
                    done(() => {
                        clearTimeout(timer);
                        socket.end();
                        resolve(parsed);
                    });
                } catch (err) {
                    done(() => {
                        clearTimeout(timer);
                        socket.destroy();
                        reject(new RpcError(`invalid json from daemon: ${String(err)}`, "EPROTO"));
                    });
                }
            }
        });

        socket.on("error", (err: NodeJS.ErrnoException) => {
            done(() => {
                clearTimeout(timer);
                const code: RpcError["code"] =
                    err.code === "ENOENT"
                        ? "ENOENT"
                        : err.code === "ECONNREFUSED"
                          ? "ECONNREFUSED"
                          : "EOTHER";
                const hint =
                    code === "ENOENT" || code === "ECONNREFUSED"
                        ? " (daemon not running? start with `weaver kernel start`)"
                        : "";
                reject(new RpcError(`${err.message}${hint}`, code));
            });
        });

        socket.on("close", () => {
            done(() => {
                clearTimeout(timer);
                reject(new RpcError("daemon closed connection before reply", "EPROTO"));
            });
        });
    });
}
