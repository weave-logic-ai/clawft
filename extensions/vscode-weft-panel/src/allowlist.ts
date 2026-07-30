// Webview RPC allowlist / denylist helpers (WEFT-496 / ADR-072).
//
// Pure functions so the gating rules are unit-testable without a
// VSCode host. The extension host (`extension.ts`) owns the live
// Sets and the UDS introspection refresh; this module owns the
// *semantics*: what the panel may proxy, and what it must never
// proxy even if the daemon advertises the method.

/**
 * Methods the webview proxy will **never** forward, even when
 * `daemon.list_methods` (WEFT-250) returns them.
 *
 * Rationale (ADR-072): the panel may invoke intentional high-level
 * mutators (`agent.chat`, `terminal.*`, `control.set_enabled`, …).
 * It must not hold a raw substrate write pen. Daemon-mediated
 * substrate writes (conversation sink, chat stream frames, soul
 * journal, routing log) go through grant-gated paths under
 * `substrate/_derived/…` inside the daemon process — never via a
 * webview-issued `substrate.publish`.
 */
export const WEBVIEW_DENIED_METHODS: ReadonlySet<string> = new Set([
    "substrate.publish",
]);

/**
 * Documented minimum surface the panel relies on when the daemon
 * does not answer introspection. Kept here so tests and the host
 * share one source of truth for the static seed.
 *
 * History is documented on the `STATIC_ALLOWED_METHODS` binding in
 * `extension.ts` (which re-exports this set).
 */
export const STATIC_ALLOWED_METHODS: ReadonlySet<string> = new Set([
    "kernel.status",
    "kernel.ps",
    "kernel.services",
    "kernel.logs",
    "kernel.kill-process",
    "kernel.restart-service",
    "service.start",
    "service.stop",
    "service.restart",
    "cluster.status",
    "cluster.nodes",
    "chain.status",
    "chain.tail",
    "sensor.mic.status",
    "cron.list",
    "cron.add",
    "cron.remove",
    "ecc.status",
    "substrate.read",
    "substrate.subscribe",
    "substrate.list",
    "control.set_enabled",
    "control.list",
    "llm.prompt",
    // WEFT-256: model/provider enumeration for the chat chip strip.
    "llm.models",
    "agent.chat",
    "agent.chat_stream",
    // WEFT-331: human decision for interactive gate Defer.
    "agent.chat.defer_decide",
    "terminal.spawn",
    "terminal.write",
    "terminal.resize",
    "terminal.close",
]);

/**
 * True when the webview proxy may forward `method`.
 *
 * Order of checks:
 * 1. Hard denylist wins (never proxy raw substrate writes).
 * 2. Runtime allowlist must contain the method.
 */
export function isMethodAllowed(
    method: string,
    allowed: ReadonlySet<string>,
    denied: ReadonlySet<string> = WEBVIEW_DENIED_METHODS,
): boolean {
    if (denied.has(method)) return false;
    return allowed.has(method);
}

/**
 * Rebuild the runtime allowlist from the static seed plus any
 * methods the daemon advertised, then subtract the denylist.
 *
 * WEFT-250 unions daemon methods into the runtime set so the panel
 * tracks new verbs without a hand-edit; WEFT-496 ensures that union
 * never re-opens methods the webview must not call.
 */
export function mergeAllowlist(
    staticSeed: ReadonlySet<string>,
    daemonMethods: readonly string[],
    denied: ReadonlySet<string> = WEBVIEW_DENIED_METHODS,
): Set<string> {
    const out = new Set<string>();
    for (const m of staticSeed) {
        if (!denied.has(m)) out.add(m);
    }
    for (const m of daemonMethods) {
        if (typeof m === "string" && m.length > 0 && !denied.has(m)) {
            out.add(m);
        }
    }
    return out;
}
