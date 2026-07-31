// Protocol unit tests for WEFT-400 (Node, no browser/DOM).
//
// Run: node --test crates/clawft-wasm/www/protocol.test.mjs
//
// These lock the request/response envelope contract shared by worker.js
// and worker-client.js so harness and UI consumers stay compatible.

import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  PROTOCOL_VERSION,
  RequestType,
  ResponseType,
  REQUEST_TYPES,
  RESPONSE_TYPES,
  validateRequest,
  validateResponse,
  makeRequest,
  makeResponse,
} from "./protocol.js";

describe("protocol constants", () => {
  it("exposes a positive protocol version", () => {
    assert.equal(typeof PROTOCOL_VERSION, "number");
    assert.ok(PROTOCOL_VERSION >= 1);
  });

  it("lists all request types", () => {
    for (const t of [
      "ping",
      "load",
      "init",
      "send",
      "stream",
      "set_env",
      "get_env",
      "boot_info",
      "analyze_files",
      "tool_list",
      "tool_schema",
    ]) {
      assert.ok(REQUEST_TYPES.includes(t), `missing request type ${t}`);
    }
  });

  it("lists all response types", () => {
    for (const t of ["pong", "worker_ready", "ok", "chunk", "error"]) {
      assert.ok(RESPONSE_TYPES.includes(t), `missing response type ${t}`);
    }
  });
});

describe("validateRequest", () => {
  it("rejects non-objects", () => {
    assert.equal(validateRequest(null).ok, false);
    assert.equal(validateRequest("init").ok, false);
    assert.equal(validateRequest([]).ok, false);
  });

  it("rejects missing id or type", () => {
    assert.equal(validateRequest({ type: "ping" }).ok, false);
    assert.equal(validateRequest({ id: 1 }).ok, false);
    assert.equal(validateRequest({ id: 1, type: "nope" }).ok, false);
  });

  it("accepts ping / load / boot_info with id only", () => {
    for (const type of [
      RequestType.PING,
      RequestType.LOAD,
      RequestType.BOOT_INFO,
      RequestType.TOOL_LIST,
    ]) {
      const r = validateRequest({ id: 1, type });
      assert.equal(r.ok, true, type);
    }
  });

  it("requires configJson for init", () => {
    assert.equal(validateRequest({ id: 1, type: RequestType.INIT }).ok, false);
    assert.equal(
      validateRequest({ id: 1, type: RequestType.INIT, configJson: "" }).ok,
      false,
    );
    const ok = validateRequest({
      id: "a",
      type: RequestType.INIT,
      configJson: "{}",
    });
    assert.equal(ok.ok, true);
    const withEnv = validateRequest({
      id: "a",
      type: RequestType.INIT,
      configJson: "{}",
      envJson: null,
    });
    assert.equal(withEnv.ok, true);
  });

  it("requires text for send and stream", () => {
    assert.equal(validateRequest({ id: 1, type: RequestType.SEND }).ok, false);
    assert.equal(
      validateRequest({ id: 1, type: RequestType.SEND, text: "hi" }).ok,
      true,
    );
    assert.equal(
      validateRequest({ id: 1, type: RequestType.STREAM, text: "hi" }).ok,
      true,
    );
  });

  it("requires key/value for set_env and key for get_env", () => {
    assert.equal(
      validateRequest({ id: 1, type: RequestType.SET_ENV, key: "K" }).ok,
      false,
    );
    assert.equal(
      validateRequest({
        id: 1,
        type: RequestType.SET_ENV,
        key: "K",
        value: "V",
      }).ok,
      true,
    );
    assert.equal(
      validateRequest({ id: 1, type: RequestType.GET_ENV, key: "K" }).ok,
      true,
    );
  });

  it("requires filesJson for analyze_files", () => {
    assert.equal(
      validateRequest({ id: 1, type: RequestType.ANALYZE_FILES }).ok,
      false,
    );
    assert.equal(
      validateRequest({
        id: 1,
        type: RequestType.ANALYZE_FILES,
        filesJson: "[]",
      }).ok,
      true,
    );
  });

  it("requires key for tool_schema", () => {
    assert.equal(
      validateRequest({ id: 1, type: RequestType.TOOL_SCHEMA }).ok,
      false,
    );
    assert.equal(
      validateRequest({
        id: 1,
        type: RequestType.TOOL_SCHEMA,
        key: "read_file",
      }).ok,
      true,
    );
  });
});

describe("validateResponse", () => {
  it("accepts unsolicited worker_ready without id", () => {
    const r = validateResponse({
      type: ResponseType.WORKER_READY,
      protocolVersion: 1,
    });
    assert.equal(r.ok, true);
  });

  it("requires id for ok/error/chunk/pong", () => {
    assert.equal(validateResponse({ type: ResponseType.OK }).ok, false);
    assert.equal(
      validateResponse({ type: ResponseType.OK, id: 1, result: "x" }).ok,
      true,
    );
    assert.equal(
      validateResponse({ type: ResponseType.ERROR, id: 1 }).ok,
      false,
    );
    assert.equal(
      validateResponse({
        type: ResponseType.ERROR,
        id: 1,
        error: "nope",
      }).ok,
      true,
    );
    assert.equal(
      validateResponse({ type: ResponseType.CHUNK, id: 1 }).ok,
      false,
    );
    assert.equal(
      validateResponse({ type: ResponseType.CHUNK, id: 1, text: "hi" }).ok,
      true,
    );
    assert.equal(
      validateResponse({ type: ResponseType.PONG, id: 1 }).ok,
      true,
    );
  });

  it("rejects unknown types", () => {
    assert.equal(validateResponse({ id: 1, type: "init_ok" }).ok, false);
  });
});

describe("makeRequest / makeResponse", () => {
  it("builds envelopes that pass validation", () => {
    const req = makeRequest(42, RequestType.SEND, { text: "hello" });
    const vr = validateRequest(req);
    assert.equal(vr.ok, true);
    assert.equal(vr.value.id, 42);
    assert.equal(vr.value.text, "hello");

    const res = makeResponse(42, ResponseType.OK, { result: "world", text: "world" });
    const vs = validateResponse(res);
    assert.equal(vs.ok, true);
    assert.equal(vs.value.result, "world");
  });

  it("allows worker_ready without id", () => {
    const res = makeResponse(undefined, ResponseType.WORKER_READY, {
      protocolVersion: PROTOCOL_VERSION,
    });
    assert.equal("id" in res, false);
    assert.equal(validateResponse(res).ok, true);
  });
});

describe("round-trip shapes (BW6-compatible intent)", () => {
  it("init → ok mirrors main-thread init contract", () => {
    const req = makeRequest("1", RequestType.INIT, {
      configJson: JSON.stringify({ providers: {} }),
      envJson: null,
    });
    assert.equal(validateRequest(req).ok, true);

    const res = makeResponse("1", ResponseType.OK, {
      result: { initialized: true },
    });
    assert.equal(validateResponse(res).ok, true);
  });

  it("send → ok carries text result", () => {
    const req = makeRequest(2, RequestType.SEND, { text: "ping" });
    assert.equal(validateRequest(req).ok, true);
    const res = makeResponse(2, ResponseType.OK, {
      result: "pong from agent",
      text: "pong from agent",
    });
    assert.equal(validateResponse(res).ok, true);
  });

  it("stream emits chunk then ok", () => {
    const chunks = [
      makeResponse(3, ResponseType.CHUNK, { text: "Hel" }),
      makeResponse(3, ResponseType.CHUNK, { text: "lo" }),
      makeResponse(3, ResponseType.OK, { result: "Hello", text: "Hello" }),
    ];
    for (const c of chunks) {
      assert.equal(validateResponse(c).ok, true, JSON.stringify(c));
    }
  });

  it("error path for not initialized", () => {
    const res = makeResponse(9, ResponseType.ERROR, {
      error: "not initialized — call init first",
    });
    assert.equal(validateResponse(res).ok, true);
  });
});
