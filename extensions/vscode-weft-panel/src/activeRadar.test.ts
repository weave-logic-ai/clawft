// Unit tests for WEFT-283 typed active-radar return schema + variant-id echo.
//
// Runnable with `npx --yes tsx --test src/activeRadar.test.ts` from this
// package directory (no VSCode host required).

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
    RADAR_RETURN_ACK_TYPE,
    RADAR_RETURN_TYPE,
    RPC_RESPONSE_TYPE,
    VARIANT_ID_KEY,
    buildRadarReturn,
    buildRadarReturnAck,
    buildRpcResponse,
    extractVariantId,
    isRadarReturn,
    isRadarReturnType,
    parseRadarReturn,
    withVariantIdEcho,
} from "./activeRadar";

describe("extractVariantId", () => {
    it("reads wire kebab-case variant-id", () => {
        assert.equal(extractVariantId({ "variant-id": "v42" }), "v42");
        assert.equal(extractVariantId({ "variant-id": 7 }), 7);
    });

    it("accepts camelCase and snake_case aliases", () => {
        assert.equal(extractVariantId({ variantId: "pulse-1" }), "pulse-1");
        assert.equal(extractVariantId({ variant_id: 99 }), 99);
    });

    it("returns undefined for missing / empty / bad types", () => {
        assert.equal(extractVariantId(null), undefined);
        assert.equal(extractVariantId({}), undefined);
        assert.equal(extractVariantId({ "variant-id": "" }), undefined);
        assert.equal(extractVariantId({ "variant-id": true }), undefined);
        assert.equal(extractVariantId({ "variant-id": NaN }), undefined);
    });
});

describe("withVariantIdEcho", () => {
    it("omits the key when variant-id is absent (plain RPC compat)", () => {
        const out = withVariantIdEcho({ type: "rpc-response", id: 1, ok: true }, undefined);
        assert.equal(VARIANT_ID_KEY in out, false);
        assert.deepEqual(out, { type: "rpc-response", id: 1, ok: true });
    });

    it("stamps variant-id when present", () => {
        const out = withVariantIdEcho({ type: "rpc-response", id: 1, ok: true }, "v9");
        assert.equal(out[VARIANT_ID_KEY], "v9");
    });
});

describe("isRadarReturnType", () => {
    it("accepts the five canonical return-types", () => {
        for (const t of ["topology", "doppler", "range", "bearing", "composite"]) {
            assert.equal(isRadarReturnType(t), true, t);
        }
    });

    it("rejects unknown", () => {
        assert.equal(isRadarReturnType("click"), false);
        assert.equal(isRadarReturnType(1), false);
    });
});

describe("parseRadarReturn / isRadarReturn", () => {
    it("accepts a well-formed bearing return with variant-id", () => {
        const raw = {
            type: "radar-return",
            "variant-id": "v1",
            kind: "explicit",
            "return-type": "bearing",
            bearing: {
                surface_id: "s1",
                path: "/children/0",
                affordance: "submit",
                actor: "human",
            },
            seq: 3,
        };
        const parsed = parseRadarReturn(raw);
        assert.ok(parsed);
        assert.equal(parsed!.type, RADAR_RETURN_TYPE);
        assert.equal(parsed![VARIANT_ID_KEY], "v1");
        assert.equal(parsed!.kind, "explicit");
        assert.equal(parsed!["return-type"], "bearing");
        assert.equal(parsed!.seq, 3);
        assert.equal(parsed!.bearing?.affordance, "submit");
        assert.equal(isRadarReturn(raw), true);
    });

    it("accepts numeric variant-id and camelCase returnType alias", () => {
        const parsed = parseRadarReturn({
            type: "radar-return",
            variantId: 42,
            kind: "implicit",
            returnType: "topology",
            topology: {
                surface_id: "s",
                path: "/",
                dwell_ms: 120,
                order_index: 0,
            },
        });
        assert.ok(parsed);
        assert.equal(parsed![VARIANT_ID_KEY], 42);
        assert.equal(parsed!["return-type"], "topology");
    });

    it("rejects missing variant-id", () => {
        assert.equal(
            parseRadarReturn({
                type: "radar-return",
                kind: "explicit",
                "return-type": "bearing",
            }),
            null,
        );
        assert.equal(isRadarReturn({ type: "radar-return", kind: "x", "return-type": "bearing" }), false);
    });

    it("rejects wrong type discriminator", () => {
        assert.equal(
            parseRadarReturn({
                type: "rpc-request",
                "variant-id": "v1",
                kind: "explicit",
                "return-type": "bearing",
            }),
            null,
        );
    });

    it("rejects invalid return-type", () => {
        assert.equal(
            parseRadarReturn({
                type: "radar-return",
                "variant-id": "v1",
                kind: "explicit",
                "return-type": "click",
            }),
            null,
        );
    });

    it("rejects empty kind", () => {
        assert.equal(
            parseRadarReturn({
                type: "radar-return",
                "variant-id": "v1",
                kind: "",
                "return-type": "range",
            }),
            null,
        );
    });
});

describe("buildRadarReturn", () => {
    it("builds a wire-shaped envelope with variant-id", () => {
        const msg = buildRadarReturn({
            variantId: "pulse-7",
            kind: "explicit",
            returnType: "composite",
            topology: {
                surface_id: "s",
                path: "/0",
                dwell_ms: 10,
                order_index: 1,
            },
            range: { surface_id: "s", path: "/0", latency_ms: 45 },
        });
        assert.equal(msg.type, RADAR_RETURN_TYPE);
        assert.equal(msg[VARIANT_ID_KEY], "pulse-7");
        assert.equal(msg["return-type"], "composite");
        assert.ok(msg.topology);
        assert.ok(msg.range);
    });

    it("throws without variant-id", () => {
        assert.throws(
            () =>
                buildRadarReturn({
                    variantId: "",
                    kind: "explicit",
                    returnType: "bearing",
                }),
            /variant-id/,
        );
    });
});

describe("buildRadarReturnAck", () => {
    it("echoes variant-id on success", () => {
        const ack = buildRadarReturnAck({ "variant-id": "v88", seq: 2 }, true);
        assert.equal(ack.type, RADAR_RETURN_ACK_TYPE);
        assert.equal(ack[VARIANT_ID_KEY], "v88");
        assert.equal(ack.ok, true);
        assert.equal(ack.seq, 2);
        assert.equal(ack.error, undefined);
    });

    it("echoes variant-id on failure with error", () => {
        const ack = buildRadarReturnAck({ "variant-id": 3 }, false, "malformed body");
        assert.equal(ack[VARIANT_ID_KEY], 3);
        assert.equal(ack.ok, false);
        assert.equal(ack.error, "malformed body");
    });
});

describe("buildRpcResponse (variant-id echo on RPC path)", () => {
    it("plain success has no variant-id field", () => {
        const resp = buildRpcResponse({ id: 1, ok: true, result: { a: 1 } });
        assert.equal(resp.type, RPC_RESPONSE_TYPE);
        assert.equal(VARIANT_ID_KEY in resp, false);
        assert.deepEqual(resp.result, { a: 1 });
    });

    it("echoes variant-id when request carried one", () => {
        const resp = buildRpcResponse({
            id: 9,
            ok: false,
            error: "method not allowed: evil",
            variantId: "v-echo",
        });
        assert.equal(resp[VARIANT_ID_KEY], "v-echo");
        assert.equal(resp.ok, false);
        assert.equal(resp.id, 9);
    });

    it("forwards error_kind with variant-id", () => {
        const resp = buildRpcResponse({
            id: 2,
            ok: false,
            error: "timeout",
            error_kind: "timeout",
            variantId: 100,
        });
        assert.equal(resp.error_kind, "timeout");
        assert.equal(resp[VARIANT_ID_KEY], 100);
    });
});

describe("backward compat: plain RPC shapes still parse as non-radar", () => {
    it("rpc-request without variant-id is not a radar-return", () => {
        assert.equal(
            isRadarReturn({ type: "rpc-request", id: 1, method: "kernel.status" }),
            false,
        );
        assert.equal(extractVariantId({ type: "rpc-request", id: 1, method: "kernel.status" }), undefined);
    });

    it("rpc-request with optional variant-id extracts cleanly", () => {
        assert.equal(
            extractVariantId({
                type: "rpc-request",
                id: 1,
                method: "agent.chat",
                "variant-id": "pulse-chat",
            }),
            "pulse-chat",
        );
    });
});
