// WEFT-283 / ADR-007 — Typed active-radar return schema for the panel bridge.
//
// Foundations frames display as an emitted pulse and user interaction as
// the return echo. Every return observation MUST carry the `variant-id`
// of the pulse that produced it so ECC can attribute echoes without
// shared state (ADR-006 primitive-head + ADR-007 return schema).
//
// This module is the TypeScript contract for the VSCode postMessage
// channel between webview (wasm / bootstrap) and extension host:
//
//   • `radar-return`        webview → host  (typed observation envelope)
//   • `radar-return-ack`    host → webview  (variant-id echo + ok)
//   • optional `variant-id` on plain `rpc-request` / `rpc-response`
//     (echo path for invoke-under-pulse; plain RPC remains valid)
//
// Pure functions — unit-testable without a VSCode host.
// Full `ux/returns` substrate publish lands with M2 radar loop wiring;
// this ticket only ships the typed envelope + echo discipline.

/** Wire message type discriminators. */
export const RADAR_RETURN_TYPE = "radar-return" as const;
export const RADAR_RETURN_ACK_TYPE = "radar-return-ack" as const;
export const RPC_REQUEST_TYPE = "rpc-request" as const;
export const RPC_RESPONSE_TYPE = "rpc-response" as const;

/** Wire key for pulse attribution (protocol-spec kebab-case). */
export const VARIANT_ID_KEY = "variant-id" as const;

/**
 * Observation kind set (ADR-007 / session-6 §7). Open — new kinds
 * may arrive as ontology IRIs; known literals are documented.
 */
export type RadarObservationKind =
    | "explicit"
    | "implicit"
    | "ambient"
    | "topology"
    | "doppler"
    | "range"
    | "bearing"
    | string;

/**
 * Primary return-type discriminator for the envelope body.
 * The four active-radar quantities plus a composite multi-body form.
 */
export type RadarReturnType =
    | "topology"
    | "doppler"
    | "range"
    | "bearing"
    | "composite";

export const RADAR_RETURN_TYPES: readonly RadarReturnType[] = Object.freeze([
    "topology",
    "doppler",
    "range",
    "bearing",
    "composite",
]);

/** Opaque pulse id — protocol CDDL is `tstr`; Rust canon uses `u64`. Accept both. */
export type VariantId = string | number;

export interface TopologyBody {
    surface_id: string;
    path: string;
    dwell_ms: number;
    order_index: number;
}

export interface DopplerBody {
    surface_id: string;
    path: string;
    velocity: number;
    direction: "forward" | "backward";
}

export interface RangeBody {
    surface_id: string;
    path: string;
    latency_ms: number;
}

export interface BearingBody {
    surface_id: string;
    path: string;
    /** Affordance IRI / name — not an index (ADR-007). */
    affordance: string;
    actor: string;
}

/**
 * Typed active-radar return observation (webview → host).
 *
 * Wire shape (JSON postMessage):
 * ```json
 * {
 *   "type": "radar-return",
 *   "variant-id": "v1",
 *   "kind": "explicit",
 *   "return-type": "bearing",
 *   "bearing": { "surface_id": "...", "path": "/0", "affordance": "submit", "actor": "human" }
 * }
 * ```
 */
export interface RadarReturnMessage {
    type: typeof RADAR_RETURN_TYPE;
    [VARIANT_ID_KEY]: VariantId;
    kind: RadarObservationKind;
    "return-type": RadarReturnType;
    seq?: number;
    surface_id?: string;
    path?: string;
    topology?: TopologyBody;
    doppler?: DopplerBody | null;
    range?: RangeBody | null;
    bearing?: BearingBody | null;
    consent_id?: string | null;
    /** Open-kind extension payload. */
    body?: unknown;
    ts_ms?: number;
}

/**
 * Host acknowledgement with mandatory variant-id echo (ADR-007 discipline).
 */
export interface RadarReturnAck {
    type: typeof RADAR_RETURN_ACK_TYPE;
    [VARIANT_ID_KEY]: VariantId;
    ok: boolean;
    seq?: number;
    error?: string;
}

/** Optional pulse attribution on plain RPC request. */
export interface RpcRequestWithVariant {
    type: typeof RPC_REQUEST_TYPE;
    id: number;
    method: string;
    params?: unknown;
    [VARIANT_ID_KEY]?: VariantId;
}

/** RPC response that may carry an echoed variant-id. */
export interface RpcResponseWithVariant {
    type: typeof RPC_RESPONSE_TYPE;
    id: number;
    ok: boolean;
    result?: unknown;
    error?: string;
    error_kind?: string;
    [VARIANT_ID_KEY]?: VariantId;
}

export type VariantIdCarrier = {
    [VARIANT_ID_KEY]?: VariantId;
    /** CamelCase alias accepted on parse for convenience. */
    variantId?: VariantId;
    variant_id?: VariantId;
};

/**
 * Extract a variant-id from a message-like object.
 * Accepts wire `variant-id`, camelCase `variantId`, or snake `variant_id`.
 * Returns `undefined` when absent / empty / non-scalar.
 */
export function extractVariantId(msg: unknown): VariantId | undefined {
    if (!msg || typeof msg !== "object") {
        return undefined;
    }
    const o = msg as VariantIdCarrier;
    const raw = o[VARIANT_ID_KEY] ?? o.variantId ?? o.variant_id;
    if (typeof raw === "string") {
        return raw.length > 0 ? raw : undefined;
    }
    if (typeof raw === "number" && Number.isFinite(raw)) {
        return raw;
    }
    return undefined;
}

/**
 * Attach `variant-id` to a response-like object when present.
 * Omits the key when undefined so plain RPC stays wire-compatible.
 */
export function withVariantIdEcho<T extends object>(
    msg: T,
    variantId: VariantId | undefined,
): T & { [VARIANT_ID_KEY]?: VariantId } {
    if (variantId === undefined) {
        return msg as T & { [VARIANT_ID_KEY]?: VariantId };
    }
    return { ...msg, [VARIANT_ID_KEY]: variantId };
}

export function isRadarReturnType(v: unknown): v is RadarReturnType {
    return (
        typeof v === "string" &&
        (RADAR_RETURN_TYPES as readonly string[]).includes(v)
    );
}

/**
 * Type guard: raw postMessage payload is a well-formed radar-return.
 * Requires `type`, non-empty `variant-id`, `kind`, and valid `return-type`.
 */
export function isRadarReturn(raw: unknown): raw is RadarReturnMessage {
    return parseRadarReturn(raw) !== null;
}

/**
 * Parse and validate a radar-return envelope. Returns null if malformed.
 * Does not throw — host inbound path drops bad shapes silently (same
 * posture as non-object webview messages).
 */
export function parseRadarReturn(raw: unknown): RadarReturnMessage | null {
    if (!raw || typeof raw !== "object") {
        return null;
    }
    const o = raw as Record<string, unknown>;
    if (o.type !== RADAR_RETURN_TYPE) {
        return null;
    }
    const variantId = extractVariantId(o);
    if (variantId === undefined) {
        return null;
    }
    if (typeof o.kind !== "string" || o.kind.length === 0) {
        return null;
    }
    if (!isRadarReturnType(o["return-type"])) {
        // Also accept camelCase alias from TS callers.
        if (!isRadarReturnType(o.returnType)) {
            return null;
        }
    }
    const returnType = (isRadarReturnType(o["return-type"])
        ? o["return-type"]
        : o.returnType) as RadarReturnType;

    const msg: RadarReturnMessage = {
        type: RADAR_RETURN_TYPE,
        [VARIANT_ID_KEY]: variantId,
        kind: o.kind,
        "return-type": returnType,
    };

    if (typeof o.seq === "number" && Number.isFinite(o.seq)) {
        msg.seq = o.seq;
    }
    if (typeof o.surface_id === "string") {
        msg.surface_id = o.surface_id;
    }
    if (typeof o.path === "string") {
        msg.path = o.path;
    }
    if (o.topology && typeof o.topology === "object") {
        msg.topology = o.topology as TopologyBody;
    }
    if (o.doppler !== undefined) {
        msg.doppler = o.doppler as DopplerBody | null;
    }
    if (o.range !== undefined) {
        msg.range = o.range as RangeBody | null;
    }
    if (o.bearing !== undefined) {
        msg.bearing = o.bearing as BearingBody | null;
    }
    if (o.consent_id !== undefined) {
        msg.consent_id =
            o.consent_id === null || typeof o.consent_id === "string"
                ? (o.consent_id as string | null)
                : undefined;
    }
    if ("body" in o) {
        msg.body = o.body;
    }
    if (typeof o.ts_ms === "number" && Number.isFinite(o.ts_ms)) {
        msg.ts_ms = o.ts_ms;
    }

    return msg;
}

/**
 * Build a host → webview ack that always echoes the observation's variant-id.
 */
export function buildRadarReturnAck(
    ret: Pick<RadarReturnMessage, typeof VARIANT_ID_KEY | "seq">,
    ok: boolean,
    error?: string,
): RadarReturnAck {
    const ack: RadarReturnAck = {
        type: RADAR_RETURN_ACK_TYPE,
        [VARIANT_ID_KEY]: ret[VARIANT_ID_KEY],
        ok,
    };
    if (ret.seq !== undefined) {
        ack.seq = ret.seq;
    }
    if (error !== undefined) {
        ack.error = error;
    }
    return ack;
}

/**
 * Build a typed radar-return message (webview helper).
 * Throws if variant-id is missing or return-type is invalid.
 */
export function buildRadarReturn(input: {
    variantId: VariantId;
    kind: RadarObservationKind;
    returnType: RadarReturnType;
    seq?: number;
    surface_id?: string;
    path?: string;
    topology?: TopologyBody;
    doppler?: DopplerBody | null;
    range?: RangeBody | null;
    bearing?: BearingBody | null;
    consent_id?: string | null;
    body?: unknown;
    ts_ms?: number;
}): RadarReturnMessage {
    if (
        input.variantId === undefined ||
        input.variantId === null ||
        (typeof input.variantId === "string" && input.variantId.length === 0)
    ) {
        throw new Error("radar-return requires a non-empty variant-id");
    }
    if (!isRadarReturnType(input.returnType)) {
        throw new Error(`invalid return-type: ${String(input.returnType)}`);
    }
    if (typeof input.kind !== "string" || input.kind.length === 0) {
        throw new Error("radar-return requires a non-empty kind");
    }
    const msg: RadarReturnMessage = {
        type: RADAR_RETURN_TYPE,
        [VARIANT_ID_KEY]: input.variantId,
        kind: input.kind,
        "return-type": input.returnType,
    };
    if (input.seq !== undefined) msg.seq = input.seq;
    if (input.surface_id !== undefined) msg.surface_id = input.surface_id;
    if (input.path !== undefined) msg.path = input.path;
    if (input.topology !== undefined) msg.topology = input.topology;
    if (input.doppler !== undefined) msg.doppler = input.doppler;
    if (input.range !== undefined) msg.range = input.range;
    if (input.bearing !== undefined) msg.bearing = input.bearing;
    if (input.consent_id !== undefined) msg.consent_id = input.consent_id;
    if (input.body !== undefined) msg.body = input.body;
    if (input.ts_ms !== undefined) msg.ts_ms = input.ts_ms;
    return msg;
}

/**
 * Build an rpc-response payload, echoing variant-id when the matching
 * request carried one. Keeps plain responses free of the field otherwise.
 */
export function buildRpcResponse(args: {
    id: number;
    ok: boolean;
    result?: unknown;
    error?: string;
    error_kind?: string;
    variantId?: VariantId;
}): RpcResponseWithVariant {
    const base: RpcResponseWithVariant = {
        type: RPC_RESPONSE_TYPE,
        id: args.id,
        ok: args.ok,
    };
    if (args.result !== undefined) base.result = args.result;
    if (args.error !== undefined) base.error = args.error;
    if (args.error_kind !== undefined) base.error_kind = args.error_kind;
    return withVariantIdEcho(base, args.variantId);
}
