// clawft-wasm Web Worker message protocol (WEFT-400)
//
// Shared constants and lightweight validators used by worker.js (worker
// entry) and worker-client.js (main-thread facade). Keep this free of
// DOM / Worker APIs so protocol shape can be unit-tested in Node.

/** Protocol version — bump when request/response shapes break. */
export const PROTOCOL_VERSION = 1;

/**
 * Host → Worker request `type` values.
 * @readonly
 * @enum {string}
 */
export const RequestType = Object.freeze({
  /** Health check; worker replies with `pong`. */
  PING: "ping",
  /** Load the WASM module (idempotent). Optional; first init loads if needed. */
  LOAD: "load",
  /** Initialize the agent: { configJson, envJson? }. */
  INIT: "init",
  /** Send a chat message: { text }. */
  SEND: "send",
  /** Stream a chat message: { text }; worker emits `chunk` then `ok`. */
  STREAM: "stream",
  /** set_env(key, value). */
  SET_ENV: "set_env",
  /** get_env(key). */
  GET_ENV: "get_env",
  /** boot_info() → JSON string. */
  BOOT_INFO: "boot_info",
  /** analyze_files(filesJson). */
  ANALYZE_FILES: "analyze_files",
  /** tool_list(). */
  TOOL_LIST: "tool_list",
  /** tool_schema(slug). */
  TOOL_SCHEMA: "tool_schema",
});

/**
 * Worker → Host response `type` values.
 * @readonly
 * @enum {string}
 */
export const ResponseType = Object.freeze({
  /** Reply to ping. */
  PONG: "pong",
  /** Unsolicited: WASM binary loaded and ready for init (no request id). */
  WORKER_READY: "worker_ready",
  /** Successful completion of a request. */
  OK: "ok",
  /** Streaming partial token / chunk for STREAM requests. */
  CHUNK: "chunk",
  /** Failed request. */
  ERROR: "error",
});

/** All known request types (for validation). */
export const REQUEST_TYPES = Object.freeze(Object.values(RequestType));

/** All known response types (for validation). */
export const RESPONSE_TYPES = Object.freeze(Object.values(ResponseType));

/**
 * @typedef {object} WorkerRequest
 * @property {string|number} id - Correlation id for matching replies.
 * @property {string} type - One of {@link RequestType}.
 * @property {string} [configJson] - INIT: config JSON string.
 * @property {string|null} [envJson] - INIT: optional env seed JSON.
 * @property {string} [text] - SEND / STREAM: user message.
 * @property {string} [key] - SET_ENV / GET_ENV / TOOL_SCHEMA.
 * @property {string} [value] - SET_ENV.
 * @property {string} [filesJson] - ANALYZE_FILES.
 */

/**
 * @typedef {object} WorkerResponse
 * @property {string|number} [id] - Correlation id (omitted for unsolicited).
 * @property {string} type - One of {@link ResponseType}.
 * @property {*} [result] - OK payload (string, object, null).
 * @property {string} [text] - CHUNK: partial text; also legacy alias.
 * @property {string} [error] - ERROR message.
 * @property {number} [protocolVersion] - Present on WORKER_READY / PONG.
 */

/**
 * Validate a host→worker request envelope.
 * @param {unknown} msg
 * @returns {{ ok: true, value: WorkerRequest } | { ok: false, error: string }}
 */
export function validateRequest(msg) {
  if (msg === null || typeof msg !== "object" || Array.isArray(msg)) {
    return { ok: false, error: "request must be a plain object" };
  }
  const m = /** @type {Record<string, unknown>} */ (msg);
  if (m.id === undefined || m.id === null || m.id === "") {
    return { ok: false, error: "request.id is required" };
  }
  if (typeof m.id !== "string" && typeof m.id !== "number") {
    return { ok: false, error: "request.id must be string or number" };
  }
  if (typeof m.type !== "string" || !REQUEST_TYPES.includes(m.type)) {
    return {
      ok: false,
      error: `request.type must be one of: ${REQUEST_TYPES.join(", ")}`,
    };
  }
  switch (m.type) {
    case RequestType.INIT:
      if (typeof m.configJson !== "string" || m.configJson.length === 0) {
        return { ok: false, error: "init requires non-empty configJson string" };
      }
      if (
        m.envJson !== undefined &&
        m.envJson !== null &&
        typeof m.envJson !== "string"
      ) {
        return { ok: false, error: "init envJson must be string, null, or omitted" };
      }
      break;
    case RequestType.SEND:
    case RequestType.STREAM:
      if (typeof m.text !== "string") {
        return { ok: false, error: `${m.type} requires text string` };
      }
      break;
    case RequestType.SET_ENV:
      if (typeof m.key !== "string" || typeof m.value !== "string") {
        return { ok: false, error: "set_env requires key and value strings" };
      }
      break;
    case RequestType.GET_ENV:
    case RequestType.TOOL_SCHEMA:
      if (typeof m.key !== "string") {
        return { ok: false, error: `${m.type} requires key string` };
      }
      break;
    case RequestType.ANALYZE_FILES:
      if (typeof m.filesJson !== "string") {
        return { ok: false, error: "analyze_files requires filesJson string" };
      }
      break;
    default:
      break;
  }
  return { ok: true, value: /** @type {WorkerRequest} */ (m) };
}

/**
 * Validate a worker→host response envelope.
 * @param {unknown} msg
 * @returns {{ ok: true, value: WorkerResponse } | { ok: false, error: string }}
 */
export function validateResponse(msg) {
  if (msg === null || typeof msg !== "object" || Array.isArray(msg)) {
    return { ok: false, error: "response must be a plain object" };
  }
  const m = /** @type {Record<string, unknown>} */ (msg);
  if (typeof m.type !== "string" || !RESPONSE_TYPES.includes(m.type)) {
    return {
      ok: false,
      error: `response.type must be one of: ${RESPONSE_TYPES.join(", ")}`,
    };
  }
  if (m.type === ResponseType.WORKER_READY) {
    // Unsolicited; id optional.
    return { ok: true, value: /** @type {WorkerResponse} */ (m) };
  }
  if (m.id === undefined || m.id === null || m.id === "") {
    return { ok: false, error: "response.id is required (except worker_ready)" };
  }
  if (m.type === ResponseType.ERROR && typeof m.error !== "string") {
    return { ok: false, error: "error response requires error string" };
  }
  if (m.type === ResponseType.CHUNK && typeof m.text !== "string") {
    return { ok: false, error: "chunk response requires text string" };
  }
  return { ok: true, value: /** @type {WorkerResponse} */ (m) };
}

/**
 * Build a typed request envelope.
 * @param {string|number} id
 * @param {string} type
 * @param {Record<string, unknown>} [fields]
 * @returns {WorkerRequest}
 */
export function makeRequest(id, type, fields = {}) {
  return /** @type {WorkerRequest} */ ({ id, type, ...fields });
}

/**
 * Build a typed response envelope.
 * @param {string|number|undefined} id
 * @param {string} type
 * @param {Record<string, unknown>} [fields]
 * @returns {WorkerResponse}
 */
export function makeResponse(id, type, fields = {}) {
  const base = id === undefined ? { type } : { id, type };
  return /** @type {WorkerResponse} */ ({ ...base, ...fields });
}
