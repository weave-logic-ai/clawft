// WEFT-285 — WSP-0.1 verb adapter for the VSCode / Cursor panel.
//
// The panel historically spoke only raw daemon JSON-RPC (`kernel.*`,
// `agent.chat`, …). WSP-0.1 (protocol-spec.md / ADR-005) defines a
// 17-verb surface protocol. This module:
//
//   1. Freezes the WSP-0.1 verb catalog (hard cap of 17).
//   2. Maps common panel operations (status, substrate, chat, terminal,
//      control, services, …) onto existing daemon RPC methods.
//   3. Hosts local session / surface / subscription state so verbs that
//      are purely protocol-shaped (session.*, surface.*, cancel) work
//      without a full WSP kernel.
//
// Pure TypeScript — no `vscode` import — so unit tests run under
// plain node:test / tsx without a host.
//
// Spec refs:
//   .planning/symposiums/compositional-ui/protocol-spec.md  §5
//   .planning/symposiums/compositional-ui/adrs/adr-005-wsp-verb-set.md

import { randomUUID } from "node:crypto";

// ── Wire version ───────────────────────────────────────────────────

/** Document identifier (WSP-0.1). Wire discriminator is integer 1. */
export const WSP_DOC_VERSION = "0.1";

/** On-the-wire `wsp_version` / `wsp-version` integer (protocol-spec §1.1). */
export const WSP_WIRE_VERSION = 1;

// ── Verb catalog (ADR-005 — hard cap of 17) ────────────────────────

/**
 * The 17 WSP-0.1 verbs. New behaviour arrives via ontology URIs, not
 * new verbs. Keep this list in lockstep with ADR-005 / protocol-spec §5.
 */
export const WSP_VERBS = [
    // Session (2)
    "session.initialize",
    "session.shutdown",
    // Surfaces (4)
    "surface.compose",
    "surface.get",
    "surface.update",
    "surface.dispose",
    // Substrate subscriptions (2)
    "subscribe",
    "unsubscribe",
    // Observation / return-signal (2)
    "observe",
    "observe.close",
    // Affordance invocation (2)
    "invoke",
    "mutate",
    // Governance + consent (3)
    "gate.check",
    "consent.request",
    "consent.revoke",
    // Introspection (2)
    "ontology.describe",
    "cancel",
] as const;

export type WspVerb = (typeof WSP_VERBS)[number];

export const WSP_VERB_SET: ReadonlySet<string> = new Set(WSP_VERBS);

/** Notifications the server may emit (derived; not in the verb budget). */
export const WSP_NOTIFICATIONS = [
    "substrate.update",
    "observation.update",
    "surface.invalidate",
    "surface.update",
] as const;

export function isWspVerb(method: string): method is WspVerb {
    return WSP_VERB_SET.has(method);
}

// ── Resource URI → daemon RPC (common panel operations) ────────────

/**
 * Ontology / resource URIs the panel treats as one-shot snapshots
 * backed by an existing allowlisted daemon RPC.
 *
 * Used by `subscribe` (initial snapshot) and by `ontology.describe`
 * when the target is a known resource URI.
 */
export interface ResourceRpcBinding {
    /** Daemon JSON-RPC method (must be on the panel allowlist). */
    method: string;
    /**
     * Optional fixed params. When omitted, `subscribe` / callers may
     * pass through a `filter` object as params.
     */
    params?: unknown;
    /** Human label for ontology.describe. */
    label: string;
    /** Cap scope class (mirrors panelAuth). Informational for docs. */
    scope: "read" | "chat" | "write" | "admin";
}

/**
 * Canonical resource URI map for the panel's common operations.
 * Keys are lowercased for lookup; callers should pass the canonical form.
 */
export const RESOURCE_RPC_MAP: Readonly<Record<string, ResourceRpcBinding>> = {
    "resource://kernel/status": {
        method: "kernel.status",
        params: null,
        label: "Kernel status",
        scope: "read",
    },
    "resource://kernel/ps": {
        method: "kernel.ps",
        params: null,
        label: "Kernel process list",
        scope: "read",
    },
    "resource://kernel/services": {
        method: "kernel.services",
        params: null,
        label: "Kernel services",
        scope: "read",
    },
    "resource://kernel/logs": {
        method: "kernel.logs",
        params: null,
        label: "Kernel logs",
        scope: "read",
    },
    "resource://cluster/status": {
        method: "cluster.status",
        params: null,
        label: "Cluster status",
        scope: "read",
    },
    "resource://cluster/nodes": {
        method: "cluster.nodes",
        params: null,
        label: "Cluster nodes",
        scope: "read",
    },
    "resource://chain/status": {
        method: "chain.status",
        params: null,
        label: "Chain status",
        scope: "read",
    },
    "resource://chain/tail": {
        method: "chain.tail",
        params: null,
        label: "Chain tail",
        scope: "read",
    },
    "resource://control/list": {
        method: "control.list",
        params: null,
        label: "Control plane list",
        scope: "read",
    },
    "resource://cron/list": {
        method: "cron.list",
        params: null,
        label: "Cron jobs",
        scope: "read",
    },
    "resource://ecc/status": {
        method: "ecc.status",
        params: null,
        label: "ECC status",
        scope: "read",
    },
    "resource://llm/models": {
        method: "llm.models",
        params: null,
        label: "LLM models",
        scope: "read",
    },
    "resource://sensor/mic/status": {
        method: "sensor.mic.status",
        params: null,
        label: "Microphone sensor status",
        scope: "read",
    },
};

/** Normalize resource URI for map lookup (trim + lower-case scheme/host). */
export function normalizeResourceUri(uri: string): string {
    return uri.trim().toLowerCase();
}

export function lookupResourceBinding(
    uri: string,
): ResourceRpcBinding | undefined {
    return RESOURCE_RPC_MAP[normalizeResourceUri(uri)];
}

// ── Affordance → daemon RPC (invoke) ───────────────────────────────

/**
 * Known affordance names the panel maps onto intentional high-level
 * mutators. Args from `invoke` become the RPC params (with light
 * reshaping where noted).
 */
export interface AffordanceRpcBinding {
    method: string;
    scope: "read" | "chat" | "write" | "admin";
    /**
     * Optional params reshape. Default: pass `args` through as-is
     * (or `null` when args is undefined).
     */
    mapParams?: (args: unknown) => unknown;
}

export const AFFORDANCE_RPC_MAP: Readonly<Record<string, AffordanceRpcBinding>> =
    {
        // Concierge / chat surface
        chat: { method: "agent.chat", scope: "chat" },
        chat_stream: { method: "agent.chat_stream", scope: "chat" },
        chat_defer_decide: {
            method: "agent.chat.defer_decide",
            scope: "chat",
        },
        llm_prompt: { method: "llm.prompt", scope: "chat" },
        // Terminal ForeignSurface
        terminal_spawn: { method: "terminal.spawn", scope: "write" },
        terminal_write: { method: "terminal.write", scope: "write" },
        terminal_resize: { method: "terminal.resize", scope: "write" },
        terminal_close: { method: "terminal.close", scope: "write" },
        // Services panel lifecycle
        service_start: { method: "service.start", scope: "write" },
        service_stop: { method: "service.stop", scope: "write" },
        service_restart: { method: "service.restart", scope: "write" },
        // Control plane
        control_set_enabled: {
            method: "control.set_enabled",
            scope: "write",
        },
        // Scheduler
        cron_add: { method: "cron.add", scope: "write" },
        cron_remove: { method: "cron.remove", scope: "write" },
        // Kernel admin
        kill_process: { method: "kernel.kill-process", scope: "admin" },
        restart_service: {
            method: "kernel.restart-service",
            scope: "admin",
        },
        // Substrate reads (never publish — denylist stays absolute)
        substrate_read: { method: "substrate.read", scope: "read" },
        substrate_list: { method: "substrate.list", scope: "read" },
    };

export function lookupAffordance(
    name: string,
): AffordanceRpcBinding | undefined {
    return AFFORDANCE_RPC_MAP[name];
}

// ── Translate result types ─────────────────────────────────────────

/** Call an allowlisted daemon RPC with these method/params. */
export interface WspRpcAction {
    kind: "rpc";
    method: string;
    params: unknown;
    /**
     * When set, the host SHOULD wrap the RPC result into this shape
     * before posting the WSP response (e.g. subscription open).
     */
    wrapResult?: WspResultWrapper;
}

/** Fully resolved locally — no UDS hop. */
export interface WspLocalAction {
    kind: "local";
    result: unknown;
}

/** Structured WSP-style failure (protocol-spec §15 style codes). */
export interface WspErrorAction {
    kind: "error";
    /** JSON-RPC / WSP error code. */
    code: number;
    message: string;
    data?: unknown;
}

export type WspAction = WspRpcAction | WspLocalAction | WspErrorAction;

export type WspResultWrapper =
    | { type: "subscription"; subscription_id: string; resource_uri: string }
    | { type: "observation"; observation_id: string; surface_id: string }
    | { type: "identity" };

// ── Session state ──────────────────────────────────────────────────

export interface WspSurfaceRecord {
    surface_id: string;
    surface_version: number;
    /** Opaque surface-spec (composer payload). */
    spec: unknown;
    created_at: number;
}

export interface WspSubscriptionRecord {
    subscription_id: string;
    resource_uri: string;
    /** Daemon method used for snapshot refresh, when mapped. */
    rpc_method?: string;
    filter?: unknown;
    created_at: number;
}

export interface WspObservationRecord {
    observation_id: string;
    surface_id: string;
    channels: string[];
    created_at: number;
}

export interface WspConsentRecord {
    consent_id: string;
    scope: string;
    purpose: string;
    duration: string;
    created_at: number;
    revoked: boolean;
}

/**
 * Host-side WSP session for one webview panel.
 * Created by `session.initialize`, drained by `session.shutdown`.
 */
export interface WspSession {
    session_id: string;
    wsp_version: number;
    persona?: string;
    locale?: string;
    client_caps: Record<string, unknown>;
    /** Active goal; adhoc-scratch when client omits initial-goal. */
    goal_id: string;
    surfaces: Map<string, WspSurfaceRecord>;
    subscriptions: Map<string, WspSubscriptionRecord>;
    observations: Map<string, WspObservationRecord>;
    consents: Map<string, WspConsentRecord>;
    /** In-flight request ids eligible for cancel. */
    pending_ids: Set<string>;
    created_at: number;
    closed: boolean;
}

export interface WspClientCaps {
    "wsp-version"?: number;
    wsp_version?: number;
    persona?: string;
    locale?: string;
    "supported-encodings"?: string[];
    "supported-primitives"?: string[];
    "supported-wrappers"?: string[];
    "supported-topics"?: string[];
    "a11y-hints"?: Record<string, unknown>;
    "initial-goal"?: string;
    initial_goal?: string;
    reattach?: string;
    [key: string]: unknown;
}

/** Capability flags the panel host advertises today. */
export const PANEL_CAPABILITY_FLAGS: readonly string[] = [
    "wsp.panel-adapter",
    "wsp.resource-rpc-map",
    "wsp.affordance-rpc-map",
    "wsp.local-surfaces",
    "raw-rpc-compat",
];

/** Primitives the panel adapter claims (subset; full canon is egui-side). */
export const PANEL_SUPPORTED_PRIMITIVES: readonly string[] = [
    "ui://chip",
    "ui://pressable",
    "ui://field",
    "ui://toggle",
    "ui://stack",
    "ui://sheet",
    "ui://gauge",
];

export function createWspSession(clientCaps: WspClientCaps = {}): WspSession {
    const version =
        clientCaps["wsp-version"] ?? clientCaps.wsp_version ?? WSP_WIRE_VERSION;
    const goal =
        (typeof clientCaps["initial-goal"] === "string" &&
            clientCaps["initial-goal"]) ||
        (typeof clientCaps.initial_goal === "string" &&
            clientCaps.initial_goal) ||
        "goal://adhoc-scratch";
    return {
        session_id: randomUUID(),
        wsp_version: typeof version === "number" ? version : WSP_WIRE_VERSION,
        persona:
            typeof clientCaps.persona === "string"
                ? clientCaps.persona
                : undefined,
        locale:
            typeof clientCaps.locale === "string"
                ? clientCaps.locale
                : undefined,
        client_caps: { ...clientCaps },
        goal_id: goal,
        surfaces: new Map(),
        subscriptions: new Map(),
        observations: new Map(),
        consents: new Map(),
        pending_ids: new Set(),
        created_at: Date.now(),
        closed: false,
    };
}

export function serverCapsFor(session: WspSession): Record<string, unknown> {
    return {
        "wsp-version": session.wsp_version,
        encoding: "json",
        "supported-primitives": [...PANEL_SUPPORTED_PRIMITIVES],
        "supported-wrappers": ["ui://foreign-surface"],
        "supported-topics": [
            "resource://",
            "service://",
            ...Object.keys(RESOURCE_RPC_MAP),
        ],
        "capability-flags": [...PANEL_CAPABILITY_FLAGS],
        "session-id": session.session_id,
        "goal-id": session.goal_id,
        verbs: [...WSP_VERBS],
        resources: Object.keys(RESOURCE_RPC_MAP),
        affordances: Object.keys(AFFORDANCE_RPC_MAP),
    };
}

// ── Error codes (protocol-spec §15 subset) ─────────────────────────

export const WSP_ERR = {
    MethodNotFound: -32601,
    InvalidParams: -32602,
    Internal: -32603,
    VersionUnsupported: -32002,
    Malformed: -32010,
    GoalMissing: -32011,
    GatingDenied: -32013,
    SurfaceUnknown: -32014,
    VersionMismatch: -32015,
    TopicUnknown: -32016,
    AffordanceUnknown: -32021,
    AxisFrozen: -32022,
    AxisUnknown: -32023,
    ConsentRequired: -32020,
    SessionClosed: -32030,
    SessionRequired: -32031,
} as const;

// ── Verb translation ───────────────────────────────────────────────

function asRecord(v: unknown): Record<string, unknown> {
    if (v && typeof v === "object" && !Array.isArray(v)) {
        return v as Record<string, unknown>;
    }
    return {};
}

function strField(
    obj: Record<string, unknown>,
    ...keys: string[]
): string | undefined {
    for (const k of keys) {
        const v = obj[k];
        if (typeof v === "string" && v.length > 0) return v;
    }
    return undefined;
}

function numField(
    obj: Record<string, unknown>,
    ...keys: string[]
): number | undefined {
    for (const k of keys) {
        const v = obj[k];
        if (typeof v === "number" && Number.isFinite(v)) return v;
    }
    return undefined;
}

/**
 * Translate a WSP-0.1 verb + params into a local result or daemon RPC.
 *
 * @param session  Active session, or `undefined` before initialize
 *                 (only `session.initialize` is legal without one).
 * @param method   Verb name (must be in WSP_VERBS).
 * @param params   Verb params object.
 *
 * Mutates `session` for local state verbs (compose, subscribe, …).
 * The host is responsible for actually performing any returned `rpc`
 * action and for posting the WSP response envelope.
 */
export function translateWsp(
    session: WspSession | undefined,
    method: string,
    params: unknown,
): WspAction {
    if (!isWspVerb(method)) {
        return {
            kind: "error",
            code: WSP_ERR.MethodNotFound,
            message: `unknown WSP verb: ${method}`,
        };
    }

    // session.initialize is the only verb legal without a live session.
    if (method === "session.initialize") {
        return translateInitialize(params);
    }

    if (!session || session.closed) {
        return {
            kind: "error",
            code: WSP_ERR.SessionRequired,
            message: "WSP session required — call session.initialize first",
        };
    }

    switch (method) {
        case "session.shutdown":
            return translateShutdown(session);
        case "surface.compose":
            return translateSurfaceCompose(session, params);
        case "surface.get":
            return translateSurfaceGet(session, params);
        case "surface.update":
            return translateSurfaceUpdate(session, params);
        case "surface.dispose":
            return translateSurfaceDispose(session, params);
        case "subscribe":
            return translateSubscribe(session, params);
        case "unsubscribe":
            return translateUnsubscribe(session, params);
        case "observe":
            return translateObserve(session, params);
        case "observe.close":
            return translateObserveClose(session, params);
        case "invoke":
            return translateInvoke(session, params);
        case "mutate":
            return translateMutate(session, params);
        case "gate.check":
            return translateGateCheck(params);
        case "consent.request":
            return translateConsentRequest(session, params);
        case "consent.revoke":
            return translateConsentRevoke(session, params);
        case "ontology.describe":
            return translateOntologyDescribe(session, params);
        case "cancel":
            return translateCancel(session, params);
        default:
            return {
                kind: "error",
                code: WSP_ERR.MethodNotFound,
                message: `unhandled WSP verb: ${method}`,
            };
    }
}

function translateInitialize(params: unknown): WspAction {
    const caps = asRecord(params) as WspClientCaps;
    const version = caps["wsp-version"] ?? caps.wsp_version ?? WSP_WIRE_VERSION;
    if (typeof version === "number" && version !== WSP_WIRE_VERSION) {
        // Panel adapter only speaks wire version 1 today.
        if (version !== 1) {
            return {
                kind: "error",
                code: WSP_ERR.VersionUnsupported,
                message: `unsupported wsp-version ${version}; panel speaks ${WSP_WIRE_VERSION}`,
                data: { supported: [WSP_WIRE_VERSION] },
            };
        }
    }
    // Caller (host) owns storing the session; we return caps + a fresh
    // session payload so the host can attach it without re-parsing.
    const session = createWspSession(caps);
    return {
        kind: "local",
        result: {
            ...serverCapsFor(session),
            // Host-only: attach the full session object under a
            // non-wire key so extension.ts can stash it. Stripped
            // before postMessage when present.
            __session: session,
        },
    };
}

function translateShutdown(session: WspSession): WspAction {
    session.closed = true;
    session.subscriptions.clear();
    session.observations.clear();
    session.surfaces.clear();
    session.pending_ids.clear();
    return { kind: "local", result: { ok: true } };
}

function translateSurfaceCompose(
    session: WspSession,
    params: unknown,
): WspAction {
    const p = asRecord(params);
    const hinted = strField(p, "surface_id", "surface-id", "id");
    const surface_id = hinted ?? randomUUID();
    if (session.surfaces.has(surface_id)) {
        return {
            kind: "error",
            code: WSP_ERR.Malformed,
            message: `surface already exists: ${surface_id}`,
        };
    }
    const rec: WspSurfaceRecord = {
        surface_id,
        surface_version: 1,
        spec: params ?? {},
        created_at: Date.now(),
    };
    session.surfaces.set(surface_id, rec);
    return {
        kind: "local",
        result: { surface_id, surface_version: 1 },
    };
}

function translateSurfaceGet(session: WspSession, params: unknown): WspAction {
    const p = asRecord(params);
    const surface_id = strField(p, "surface_id", "surface-id");
    if (!surface_id) {
        return {
            kind: "error",
            code: WSP_ERR.InvalidParams,
            message: "surface.get requires surface_id",
        };
    }
    const rec = session.surfaces.get(surface_id);
    if (!rec) {
        return {
            kind: "error",
            code: WSP_ERR.SurfaceUnknown,
            message: `unknown surface: ${surface_id}`,
        };
    }
    return {
        kind: "local",
        result: {
            surface_id: rec.surface_id,
            surface_version: rec.surface_version,
            spec: rec.spec,
        },
    };
}

function translateSurfaceUpdate(
    session: WspSession,
    params: unknown,
): WspAction {
    const p = asRecord(params);
    const surface_id = strField(p, "surface_id", "surface-id");
    if (!surface_id) {
        return {
            kind: "error",
            code: WSP_ERR.InvalidParams,
            message: "surface.update requires surface_id",
        };
    }
    const rec = session.surfaces.get(surface_id);
    if (!rec) {
        return {
            kind: "error",
            code: WSP_ERR.SurfaceUnknown,
            message: `unknown surface: ${surface_id}`,
        };
    }
    const base = numField(p, "base_version", "base-version");
    if (base !== undefined && base !== rec.surface_version) {
        return {
            kind: "error",
            code: WSP_ERR.VersionMismatch,
            message: `base_version ${base} != current ${rec.surface_version}`,
            data: { current: rec.surface_version },
        };
    }
    // Panel adapter stores ops opaquely; full JSON-Pointer apply is
    // renderer-side. Bump version and retain latest params as spec.
    rec.surface_version += 1;
    rec.spec = params;
    return { kind: "local", result: { new_version: rec.surface_version } };
}

function translateSurfaceDispose(
    session: WspSession,
    params: unknown,
): WspAction {
    const p = asRecord(params);
    const surface_id = strField(p, "surface_id", "surface-id");
    if (!surface_id) {
        return {
            kind: "error",
            code: WSP_ERR.InvalidParams,
            message: "surface.dispose requires surface_id",
        };
    }
    session.surfaces.delete(surface_id);
    // Drain observations rooted on this surface.
    for (const [id, obs] of session.observations) {
        if (obs.surface_id === surface_id) session.observations.delete(id);
    }
    return { kind: "local", result: { ok: true } };
}

/**
 * `subscribe` maps known resource:// URIs onto a snapshot RPC, and
 * maps `substrate://…` / raw substrate topics onto `substrate.subscribe`
 * (or `substrate.read` when filter.mode === "read").
 */
function translateSubscribe(session: WspSession, params: unknown): WspAction {
    const p = asRecord(params);
    const resource_uri = strField(
        p,
        "resource_uri",
        "resource-uri",
        "uri",
        "topic",
    );
    if (!resource_uri) {
        return {
            kind: "error",
            code: WSP_ERR.InvalidParams,
            message: "subscribe requires resource_uri",
        };
    }

    const subscription_id = randomUUID();
    const filter = p.filter;

    // 1) Known panel resource → snapshot RPC (common status surfaces).
    const binding = lookupResourceBinding(resource_uri);
    if (binding) {
        const rec: WspSubscriptionRecord = {
            subscription_id,
            resource_uri,
            rpc_method: binding.method,
            filter,
            created_at: Date.now(),
        };
        session.subscriptions.set(subscription_id, rec);
        return {
            kind: "rpc",
            method: binding.method,
            params: binding.params ?? filter ?? null,
            wrapResult: {
                type: "subscription",
                subscription_id,
                resource_uri,
            },
        };
    }

    // 2) Substrate-style URIs → substrate.subscribe / substrate.read.
    const lower = resource_uri.toLowerCase();
    if (
        lower.startsWith("substrate://") ||
        lower.startsWith("substrate/") ||
        lower.startsWith("topic://") ||
        lower.includes("/_derived/")
    ) {
        const mode =
            typeof filter === "object" &&
            filter &&
            (filter as { mode?: string }).mode === "read"
                ? "read"
                : "subscribe";
        const method =
            mode === "read" ? "substrate.read" : "substrate.subscribe";
        const rpcParams =
            mode === "read"
                ? { path: resource_uri.replace(/^substrate:\/\//, "") }
                : {
                      topic: resource_uri,
                      ...(filter && typeof filter === "object"
                          ? (filter as object)
                          : {}),
                  };
        const rec: WspSubscriptionRecord = {
            subscription_id,
            resource_uri,
            rpc_method: method,
            filter,
            created_at: Date.now(),
        };
        session.subscriptions.set(subscription_id, rec);
        return {
            kind: "rpc",
            method,
            params: rpcParams,
            wrapResult: {
                type: "subscription",
                subscription_id,
                resource_uri,
            },
        };
    }

    // 3) Generic substrate.list when URI is a list prefix.
    if (lower === "substrate://*" || lower === "resource://*") {
        const rec: WspSubscriptionRecord = {
            subscription_id,
            resource_uri,
            rpc_method: "substrate.list",
            filter,
            created_at: Date.now(),
        };
        session.subscriptions.set(subscription_id, rec);
        return {
            kind: "rpc",
            method: "substrate.list",
            params: filter ?? null,
            wrapResult: {
                type: "subscription",
                subscription_id,
                resource_uri,
            },
        };
    }

    return {
        kind: "error",
        code: WSP_ERR.TopicUnknown,
        message: `no RPC binding for resource_uri: ${resource_uri}`,
        data: {
            known_resources: Object.keys(RESOURCE_RPC_MAP),
            hint: "use resource://kernel/status or substrate://… topics",
        },
    };
}

function translateUnsubscribe(session: WspSession, params: unknown): WspAction {
    const p = asRecord(params);
    const subscription_id = strField(
        p,
        "subscription_id",
        "subscription-id",
    );
    if (!subscription_id) {
        return {
            kind: "error",
            code: WSP_ERR.InvalidParams,
            message: "unsubscribe requires subscription_id",
        };
    }
    session.subscriptions.delete(subscription_id);
    return { kind: "local", result: { ok: true } };
}

function translateObserve(session: WspSession, params: unknown): WspAction {
    const p = asRecord(params);
    const surface_id = strField(p, "surface_id", "surface-id");
    if (!surface_id) {
        return {
            kind: "error",
            code: WSP_ERR.InvalidParams,
            message: "observe requires surface_id",
        };
    }
    // Observation streams are host-local until active-radar is wired
    // (M2). Opening still returns an observation_id so renderers can
    // complete the handshake.
    const channels = Array.isArray(p.channels)
        ? (p.channels as unknown[]).filter((c): c is string => typeof c === "string")
        : [];
    const observation_id = randomUUID();
    session.observations.set(observation_id, {
        observation_id,
        surface_id,
        channels,
        created_at: Date.now(),
    });
    return {
        kind: "local",
        result: { observation_id, surface_id, channels },
    };
}

function translateObserveClose(
    session: WspSession,
    params: unknown,
): WspAction {
    const p = asRecord(params);
    const observation_id = strField(
        p,
        "observation_id",
        "observation-id",
    );
    if (!observation_id) {
        return {
            kind: "error",
            code: WSP_ERR.InvalidParams,
            message: "observe.close requires observation_id",
        };
    }
    session.observations.delete(observation_id);
    return { kind: "local", result: { ok: true } };
}

function translateInvoke(session: WspSession, params: unknown): WspAction {
    void session; // goal_id available for future double-gate
    const p = asRecord(params);
    const affordance = strField(p, "affordance", "name");
    if (!affordance) {
        return {
            kind: "error",
            code: WSP_ERR.InvalidParams,
            message: "invoke requires affordance",
        };
    }
    const binding = lookupAffordance(affordance);
    if (!binding) {
        return {
            kind: "error",
            code: WSP_ERR.AffordanceUnknown,
            message: `unknown affordance: ${affordance}`,
            data: { known: Object.keys(AFFORDANCE_RPC_MAP) },
        };
    }
    // Never allow an invoke path to smuggle substrate.publish.
    if (binding.method === "substrate.publish") {
        return {
            kind: "error",
            code: WSP_ERR.GatingDenied,
            message: "affordance denied: substrate.publish is webview-denied",
        };
    }
    const args = "args" in p ? p.args : p.params ?? null;
    const rpcParams = binding.mapParams ? binding.mapParams(args) : args ?? null;
    return {
        kind: "rpc",
        method: binding.method,
        params: rpcParams,
    };
}

function translateMutate(session: WspSession, params: unknown): WspAction {
    const p = asRecord(params);
    const surface_id = strField(p, "surface_id", "surface-id");
    const axis = strField(p, "axis", "variant_axis", "variant-axis");
    if (!surface_id || !axis) {
        return {
            kind: "error",
            code: WSP_ERR.InvalidParams,
            message: "mutate requires surface_id and axis",
        };
    }
    // Safety / consent / brand axes are frozen (protocol-spec §5.5.2).
    const frozen = new Set(["consent", "safety", "brand"]);
    if (frozen.has(axis.toLowerCase())) {
        return {
            kind: "error",
            code: WSP_ERR.AxisFrozen,
            message: `axis frozen by governance: ${axis}`,
        };
    }
    const rec = session.surfaces.get(surface_id);
    if (!rec) {
        return {
            kind: "error",
            code: WSP_ERR.SurfaceUnknown,
            message: `unknown surface: ${surface_id}`,
        };
    }
    const new_variant = strField(p, "new_variant", "new-variant", "variant_id");
    rec.surface_version += 1;
    return {
        kind: "local",
        result: {
            ok: true,
            surface_id,
            axis,
            new_variant: new_variant ?? null,
            surface_version: rec.surface_version,
        },
    };
}

/**
 * Pre-flight honesty check. Panel adapter answers from the static
 * resource + affordance maps; real governance is daemon-side on the
 * subsequent invoke/RPC.
 */
function translateGateCheck(params: unknown): WspAction {
    const p = asRecord(params);
    const action = strField(p, "action") ?? "";
    // Allow known affordances and resource reads; deny publish.
    if (action === "substrate.publish" || action.includes("publish")) {
        return {
            kind: "local",
            result: {
                decision: {
                    deny: {
                        reason: "webview must not hold a raw substrate write pen",
                        "policy-uri": "policy://weft/webview-denied",
                    },
                },
            },
        };
    }
    if (lookupAffordance(action) || lookupResourceBinding(action)) {
        return { kind: "local", result: { decision: "allow" } };
    }
    // Unknown actions → allow with note (daemon is authority on RPC).
    return {
        kind: "local",
        result: {
            decision: "allow",
            note: "panel adapter has no binding; daemon will enforce",
        },
    };
}

function translateConsentRequest(
    session: WspSession,
    params: unknown,
): WspAction {
    const p = asRecord(params);
    const scope = strField(p, "scope") ?? "scope://unspecified";
    const purpose = strField(p, "purpose") ?? "";
    const duration = strField(p, "duration") ?? "session";
    const consent_id = randomUUID();
    // Side-effect surface for the consent flow (protocol-spec §5.6.2).
    const surface_id = randomUUID();
    session.consents.set(consent_id, {
        consent_id,
        scope,
        purpose,
        duration,
        created_at: Date.now(),
        revoked: false,
    });
    session.surfaces.set(surface_id, {
        surface_id,
        surface_version: 1,
        spec: {
            type: "ui://modal",
            modality: "modal",
            kind: "consent",
            consent_id,
            scope,
            purpose,
        },
        created_at: Date.now(),
    });
    return {
        kind: "local",
        result: { consent_id, surface_id },
    };
}

function translateConsentRevoke(
    session: WspSession,
    params: unknown,
): WspAction {
    const p = asRecord(params);
    const consent_id = strField(p, "consent_id", "consent-id");
    if (!consent_id) {
        return {
            kind: "error",
            code: WSP_ERR.InvalidParams,
            message: "consent.revoke requires consent_id",
        };
    }
    const rec = session.consents.get(consent_id);
    if (rec) rec.revoked = true;
    return { kind: "local", result: { ok: true } };
}

function translateOntologyDescribe(
    session: WspSession,
    params: unknown,
): WspAction {
    const p = asRecord(params);
    const target = strField(p, "target", "uri", "surface_id", "surface-id");
    if (!target) {
        return {
            kind: "error",
            code: WSP_ERR.InvalidParams,
            message: "ontology.describe requires target",
        };
    }

    // Surface id?
    const surface = session.surfaces.get(target);
    if (surface) {
        return {
            kind: "local",
            result: {
                iri: `surface://${surface.surface_id}`,
                "state-schema": "schema://wsp/surface-spec",
                affordances: Object.keys(AFFORDANCE_RPC_MAP).map((name) => ({
                    name,
                    verb: "invoke",
                })),
                "confidence-model": { source: "deterministic" },
                "mutation-axes": [],
                surface_version: surface.surface_version,
            },
        };
    }

    // Known resource URI?
    const binding = lookupResourceBinding(target);
    if (binding) {
        return {
            kind: "local",
            result: {
                iri: normalizeResourceUri(target),
                label: binding.label,
                "state-schema": `schema://rpc/${binding.method}`,
                rpc_method: binding.method,
                scope: binding.scope,
                affordances: [
                    { name: "read", verb: "subscribe" },
                    { name: "refresh", verb: "subscribe" },
                ],
                "confidence-model": { source: "deterministic" },
                "mutation-axes": [],
            },
        };
    }

    // Affordance name?
    const aff = lookupAffordance(target);
    if (aff) {
        return {
            kind: "local",
            result: {
                iri: `affordance://${target}`,
                rpc_method: aff.method,
                scope: aff.scope,
                affordances: [{ name: target, verb: "invoke" }],
                "confidence-model": { source: "deterministic" },
                "mutation-axes": [],
            },
        };
    }

    // Catalog dump for well-known meta targets.
    if (
        target === "wsp://verbs" ||
        target === "ontology://wsp/verbs" ||
        target === "ui://*"
    ) {
        return {
            kind: "local",
            result: {
                iri: "ontology://wsp/verbs",
                verbs: [...WSP_VERBS],
                resources: Object.keys(RESOURCE_RPC_MAP),
                affordances: Object.keys(AFFORDANCE_RPC_MAP),
                "capability-flags": [...PANEL_CAPABILITY_FLAGS],
            },
        };
    }

    return {
        kind: "error",
        code: WSP_ERR.TopicUnknown,
        message: `ontology.describe: unknown target ${target}`,
    };
}

function translateCancel(session: WspSession, params: unknown): WspAction {
    const p = asRecord(params);
    const id = strField(p, "id", "request_id", "request-id");
    const subscription_id = strField(
        p,
        "subscription_id",
        "subscription-id",
    );
    if (subscription_id) {
        session.subscriptions.delete(subscription_id);
    }
    if (id) {
        session.pending_ids.delete(id);
    }
    // Spec: tolerate cancel of already-complete requests.
    return { kind: "local", result: { ok: true } };
}

// ── Result wrapping (after RPC returns) ────────────────────────────

/**
 * Apply a `wrapResult` directive to a successful daemon RPC payload
 * so the WSP response matches protocol shapes (subscription open, …).
 */
export function wrapWspRpcResult(
    wrap: WspResultWrapper | undefined,
    rpcResult: unknown,
    rpcOk: boolean,
): unknown {
    if (!wrap || !rpcOk) return rpcResult;
    switch (wrap.type) {
        case "subscription":
            return {
                subscription_id: wrap.subscription_id,
                resource_uri: wrap.resource_uri,
                // Initial snapshot from the mapped RPC (panel convenience).
                snapshot: rpcResult ?? null,
            };
        case "observation":
            return {
                observation_id: wrap.observation_id,
                surface_id: wrap.surface_id,
                snapshot: rpcResult ?? null,
            };
        case "identity":
            return rpcResult;
        default:
            return rpcResult;
    }
}

/**
 * Strip host-only keys (e.g. `__session`) before posting to the webview.
 */
export function publicWspResult(result: unknown): unknown {
    if (!result || typeof result !== "object" || Array.isArray(result)) {
        return result;
    }
    const { __session: _drop, ...rest } = result as Record<string, unknown> & {
        __session?: WspSession;
    };
    void _drop;
    return rest;
}

/**
 * Extract the host session object from a `session.initialize` local result.
 */
export function extractSession(result: unknown): WspSession | undefined {
    if (!result || typeof result !== "object") return undefined;
    const s = (result as { __session?: WspSession }).__session;
    return s;
}

// ── Docs helper: verb → RPC table ──────────────────────────────────

export interface WspVerbDocRow {
    verb: WspVerb;
    direction: "request";
    mapping: string;
    notes: string;
}

/** Documentation rows for SMOKE / result reports. */
export function verbMappingTable(): WspVerbDocRow[] {
    return [
        {
            verb: "session.initialize",
            direction: "request",
            mapping: "local (panel session + server-caps)",
            notes: "Optional health via kernel.status is host policy",
        },
        {
            verb: "session.shutdown",
            direction: "request",
            mapping: "local (drain session state)",
            notes: "Does not stop the daemon",
        },
        {
            verb: "surface.compose",
            direction: "request",
            mapping: "local surface registry",
            notes: "surface_version starts at 1",
        },
        {
            verb: "surface.get",
            direction: "request",
            mapping: "local surface registry",
            notes: "Cold-start snapshot",
        },
        {
            verb: "surface.update",
            direction: "request",
            mapping: "local surface registry",
            notes: "Version bump; ops stored opaque",
        },
        {
            verb: "surface.dispose",
            direction: "request",
            mapping: "local surface registry",
            notes: "Drains observations on surface",
        },
        {
            verb: "subscribe",
            direction: "request",
            mapping: "RESOURCE_RPC_MAP or substrate.subscribe/read/list",
            notes: "Returns subscription_id + snapshot",
        },
        {
            verb: "unsubscribe",
            direction: "request",
            mapping: "local subscription map",
            notes: "Daemon stream cancel is best-effort",
        },
        {
            verb: "observe",
            direction: "request",
            mapping: "local observation registry",
            notes: "Active-radar stream = M2",
        },
        {
            verb: "observe.close",
            direction: "request",
            mapping: "local observation registry",
            notes: "",
        },
        {
            verb: "invoke",
            direction: "request",
            mapping: "AFFORDANCE_RPC_MAP → daemon RPC",
            notes: "chat, terminal_*, service_*, control_*, …",
        },
        {
            verb: "mutate",
            direction: "request",
            mapping: "local (frozen axes denied)",
            notes: "consent/safety/brand → AxisFrozen",
        },
        {
            verb: "gate.check",
            direction: "request",
            mapping: "local map + denylist",
            notes: "substrate.publish always denied",
        },
        {
            verb: "consent.request",
            direction: "request",
            mapping: "local consent + modal surface",
            notes: "WEFT-282: scope://mic|camera grants host capture bridge",
        },
        {
            verb: "consent.revoke",
            direction: "request",
            mapping: "local consent map",
            notes: "",
        },
        {
            verb: "ontology.describe",
            direction: "request",
            mapping: "local catalogs / surface registry",
            notes: "target=wsp://verbs dumps verb set",
        },
        {
            verb: "cancel",
            direction: "request",
            mapping: "local pending + subscription maps",
            notes: "Idempotent ok",
        },
    ];
}
