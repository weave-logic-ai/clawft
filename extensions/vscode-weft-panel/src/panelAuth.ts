// WEFT-495 / ADR-071 — per-panel token + capability model for the webview proxy.
//
// The webview never holds a token. The extension host mints a PanelSession
// when multi-user mode is on, attaches `auth` on the UDS wire (WEFT-479
// literal scopes), and refuses methods the session scopes do not cover
// (denied-by-identity) before the daemon sees them.
//
// Single-user mode (default) leaves the 0.7 allowlist-only posture alone.

import { randomBytes, randomUUID } from "node:crypto";

/** Four-class capability scopes — wire-compatible with WEFT-479. */
export type CapScope = "read" | "chat" | "write" | "admin";

export const ALL_SCOPES: readonly CapScope[] = [
    "read",
    "chat",
    "write",
    "admin",
];

/** Default multi-user panel: viewer / concierge (matches daemon anonymous baseline). */
export const DEFAULT_PANEL_SCOPES: readonly CapScope[] = ["read", "chat"];

/**
 * Per-panel identity issued by the extension host.
 * `token` is host-private — never postMessage'd into the webview.
 */
export interface PanelSession {
    panelId: string;
    /** Opaque secret; used for host-side identity checks only. */
    token: string;
    scopes: CapScope[];
    issuedAt: number;
    /** Unix-ms expiry; null = until panel dispose. */
    expiresAt: number | null;
}

export type AuthorizeResult =
    | { ok: true; auth: string }
    | { ok: false; error: string; reason: "no_session" | "expired" | "scope" };

/**
 * Multi-user mode detection.
 *
 * Sources (first true wins):
 *   1. Explicit override argument (tests / callers that already read config).
 *   2. `WEFTOS_MULTI_USER` env ∈ {1, true, yes, on} (case-insensitive).
 *   3. Otherwise false (single-user default).
 *
 * VSCode setting `weft.multiUser` is read by the extension and passed in as
 * the override so this module stays free of the `vscode` import (unit-testable
 * under plain mocha/node without a host).
 */
export function isMultiUserMode(
    env: NodeJS.ProcessEnv = process.env,
    override?: boolean,
): boolean {
    if (typeof override === "boolean") {
        return override;
    }
    const raw = (env.WEFTOS_MULTI_USER ?? "").trim().toLowerCase();
    return raw === "1" || raw === "true" || raw === "yes" || raw === "on";
}

/**
 * Capability required by a JSON-RPC method, mirroring
 * `clawft_weave::capability::required_capability` for the methods the
 * panel is allowed to name. Unknown methods default to `read` (same as
 * the daemon table) so a typo surfaces as allowlist denial, not scope
 * denial, when multi-user is on.
 */
export function requiredScope(method: string): CapScope {
    switch (method) {
        // Admin — destructive daemon verbs.
        case "kernel.shutdown":
        case "kernel.kill-process":
        case "kernel.restart-service":
        case "cluster.join":
        case "cluster.leave":
        case "chain.checkpoint":
            return "admin";

        // Write — state-mutating verbs the panel may still allowlist.
        case "agent.register":
        case "agent.spawn":
        case "agent.stop":
        case "agent.restart":
        case "agent.send":
        case "agent.proposal.accept":
        case "agent.proposal.discard":
        case "node.register":
        case "memory.delete":
        case "substrate.publish":
        case "substrate.canonical_publish_payload":
        case "substrate.notify":
        case "control.set_enabled":
        case "terminal.spawn":
        case "terminal.write":
        case "terminal.resize":
        case "terminal.close":
        case "chain.append":
        case "ipc.publish":
        case "mcp.add":
        case "mcp.remove":
        case "mcp.reload":
        case "cron.add":
        case "cron.remove":
        case "service.start":
        case "service.stop":
        case "service.restart":
            return "write";

        // Chat — LLM / concierge verbs.
        case "agent.chat":
        case "agent.chat_stream":
        case "agent.chat.cancel":
        case "agent.chat.end":
        case "agent.turn.record":
        case "llm.prompt":
            return "chat";

        // Read — inspection + default for unknown.
        default:
            return "read";
    }
}

/** True when `scopes` cover the capability needed by `method`. */
export function scopesAllowMethod(
    scopes: readonly CapScope[],
    method: string,
): boolean {
    const needed = requiredScope(method);
    if (scopes.includes("admin")) {
        // Admin implies every class (mirrors CallerCapabilities::from_scopes).
        return true;
    }
    return scopes.includes(needed);
}

/**
 * Encode scopes as the WEFT-479 literal `auth` wire value.
 * Admin collapses to the single token `"admin"` (implies the rest on
 * the daemon side).
 */
export function scopesToAuth(scopes: readonly CapScope[]): string {
    if (scopes.includes("admin")) {
        return "admin";
    }
    // Stable order for tests / logs.
    const ordered = ALL_SCOPES.filter((s) => scopes.includes(s));
    return ordered.join(",");
}

/**
 * Mint a new panel session.
 *
 * @param scopes  Capability set; defaults to viewer `{read, chat}`.
 * @param ttlMs   Optional lifetime; null/omit = until panel dispose.
 */
export function issuePanelSession(
    scopes: readonly CapScope[] = DEFAULT_PANEL_SCOPES,
    ttlMs: number | null = null,
): PanelSession {
    const now = Date.now();
    const unique = new Set<CapScope>(scopes);
    // Authenticated callers always get at least read (daemon posture).
    unique.add("read");
    return {
        panelId: randomUUID(),
        token: randomBytes(32).toString("base64url"),
        scopes: ALL_SCOPES.filter((s) => unique.has(s)),
        issuedAt: now,
        expiresAt: ttlMs != null && ttlMs > 0 ? now + ttlMs : null,
    };
}

export function isSessionExpired(
    session: PanelSession,
    nowMs: number = Date.now(),
): boolean {
    return session.expiresAt != null && nowMs >= session.expiresAt;
}

/**
 * Authorize a proxied RPC under multi-user mode.
 *
 * Returns the wire `auth` string on success, or a deny reason.
 * Callers in single-user mode should not invoke this — they keep the
 * pre-WEFT-495 allowlist-only path.
 */
export function authorizePanelRpc(
    session: PanelSession | undefined,
    method: string,
    nowMs: number = Date.now(),
): AuthorizeResult {
    if (!session) {
        return {
            ok: false,
            error: "permission denied: panel identity required (multi-user mode)",
            reason: "no_session",
        };
    }
    if (isSessionExpired(session, nowMs)) {
        return {
            ok: false,
            error: "permission denied: panel session expired",
            reason: "expired",
        };
    }
    if (!scopesAllowMethod(session.scopes, method)) {
        const needed = requiredScope(method);
        return {
            ok: false,
            error:
                `permission denied: panel scopes [${session.scopes.join(",")}] ` +
                `lack '${needed}' required by '${method}'`,
            reason: "scope",
        };
    }
    return { ok: true, auth: scopesToAuth(session.scopes) };
}
