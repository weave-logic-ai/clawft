// clawft-wasm main-thread Web Worker client (WEFT-400)
//
// Thin promise facade over postMessage. Mirrors the wasm-bindgen surface
// (init / send_message / stream_chat / set_env / …) so harness and UI code
// can swap main-thread imports for a worker-backed client with minimal churn.

import {
  PROTOCOL_VERSION,
  RequestType,
  ResponseType,
  makeRequest,
  validateResponse,
} from "./protocol.js";

/**
 * @typedef {object} StreamHandlers
 * @property {(chunk: string) => void} [onChunk]
 */

/**
 * Main-thread client that owns a module Worker running clawft-wasm.
 */
export class ClawftWorkerClient {
  /**
   * @param {object} [options]
   * @param {string|URL} [options.workerUrl] - URL of worker.js (default: same dir).
   * @param {number} [options.timeoutMs] - Per-request timeout (0 = none). Default 0.
   */
  constructor(options = {}) {
    this._workerUrl =
      options.workerUrl ??
      new URL("./worker.js", import.meta.url);
    this._timeoutMs = options.timeoutMs ?? 0;
    /** @type {Worker | null} */
    this._worker = null;
    /** @type {Map<string|number, { resolve: Function, reject: Function, onChunk?: Function, timer?: ReturnType<typeof setTimeout> }>} */
    this._pending = new Map();
    this._nextId = 1;
    this._ready = false;
    /** @type {Promise<void> | null} */
    this._readyPromise = null;
    /** @type {((v?: void) => void) | null} */
    this._readyResolve = null;
    this._terminated = false;
    this.protocolVersion = PROTOCOL_VERSION;
  }

  /**
   * Start the worker and wait until WASM is loaded (`worker_ready` or `load`).
   * Idempotent.
   * @returns {Promise<void>}
   */
  async start() {
    if (this._terminated) {
      throw new Error("ClawftWorkerClient has been terminated");
    }
    if (this._worker && this._ready) return;

    if (!this._worker) {
      this._readyPromise = new Promise((resolve, reject) => {
        this._readyResolve = resolve;
        this._readyReject = reject;
        try {
          this._worker = new Worker(this._workerUrl, { type: "module" });
        } catch (err) {
          reject(err);
          return;
        }

        this._worker.onmessage = (event) => this._onMessage(event.data);
        this._worker.onerror = (event) => {
          const msg = event.message || "worker error";
          for (const [, p] of this._pending) {
            if (p.timer) clearTimeout(p.timer);
            p.reject(new Error(msg));
          }
          this._pending.clear();
          reject(new Error(msg));
        };
      });
    }

    // Explicit load: succeeds whether or not the worker already emitted
    // worker_ready from its eager import. Surfaces import failures cleanly.
    try {
      await this._request(RequestType.LOAD);
      this._ready = true;
      if (this._readyResolve) {
        this._readyResolve();
        this._readyResolve = null;
      }
    } catch (err) {
      if (this._readyReject) {
        this._readyReject(err);
        this._readyReject = null;
      }
      throw err;
    }
  }

  /**
   * Initialize the agent inside the worker.
   * @param {string} configJson
   * @param {string|null|undefined} [envJson]
   * @returns {Promise<void>}
   */
  async init(configJson, envJson) {
    await this.start();
    const fields = { configJson };
    if (envJson !== undefined) fields.envJson = envJson;
    await this._request(RequestType.INIT, fields);
  }

  /**
   * @param {string} text
   * @returns {Promise<string>}
   */
  async sendMessage(text) {
    await this.start();
    const res = await this._request(RequestType.SEND, { text });
    return /** @type {string} */ (res);
  }

  /**
   * Stream a message; onChunk is called for each partial token.
   * @param {string} text
   * @param {(chunk: string) => void} [onChunk]
   * @returns {Promise<string>} full response text
   */
  async streamChat(text, onChunk) {
    await this.start();
    const res = await this._request(RequestType.STREAM, { text }, { onChunk });
    return /** @type {string} */ (res);
  }

  /**
   * @param {string} key
   * @param {string} value
   * @returns {Promise<void>}
   */
  async setEnv(key, value) {
    await this.start();
    await this._request(RequestType.SET_ENV, { key, value });
  }

  /**
   * @param {string} key
   * @returns {Promise<string|null>}
   */
  async getEnv(key) {
    await this.start();
    return /** @type {string|null} */ (
      await this._request(RequestType.GET_ENV, { key })
    );
  }

  /** @returns {Promise<string>} */
  async bootInfo() {
    await this.start();
    return /** @type {string} */ (await this._request(RequestType.BOOT_INFO));
  }

  /**
   * @param {string} filesJson
   * @returns {Promise<string>}
   */
  async analyzeFiles(filesJson) {
    await this.start();
    return /** @type {string} */ (
      await this._request(RequestType.ANALYZE_FILES, { filesJson })
    );
  }

  /** @returns {Promise<string>} */
  async toolList() {
    await this.start();
    return /** @type {string} */ (await this._request(RequestType.TOOL_LIST));
  }

  /**
   * @param {string} slug
   * @returns {Promise<string>}
   */
  async toolSchema(slug) {
    await this.start();
    return /** @type {string} */ (
      await this._request(RequestType.TOOL_SCHEMA, { key: slug })
    );
  }

  /** @returns {Promise<{ protocolVersion: number, wasmLoaded: boolean, initialized: boolean }>} */
  async ping() {
    await this.start();
    // PONG carries fields on the envelope, not under result — handle specially.
    const id = this._nextId++;
    const msg = makeRequest(id, RequestType.PING);
    return new Promise((resolve, reject) => {
      this._register(id, {
        resolve: (envelope) => {
          resolve({
            protocolVersion: envelope.protocolVersion ?? PROTOCOL_VERSION,
            wasmLoaded: !!envelope.wasmLoaded,
            initialized: !!envelope.initialized,
          });
        },
        reject,
        raw: true,
      });
      this._worker.postMessage(msg);
    });
  }

  /** Terminate the worker and reject in-flight requests. */
  terminate() {
    this._terminated = true;
    for (const [, p] of this._pending) {
      if (p.timer) clearTimeout(p.timer);
      p.reject(new Error("worker terminated"));
    }
    this._pending.clear();
    if (this._worker) {
      this._worker.terminate();
      this._worker = null;
    }
    this._ready = false;
  }

  /**
   * @param {string} type
   * @param {Record<string, unknown>} [fields]
   * @param {{ onChunk?: (c: string) => void }} [handlers]
   * @returns {Promise<unknown>}
   */
  _request(type, fields = {}, handlers = {}) {
    if (!this._worker) {
      return Promise.reject(new Error("worker not started"));
    }
    const id = this._nextId++;
    const msg = makeRequest(id, type, fields);
    return new Promise((resolve, reject) => {
      this._register(id, {
        resolve: (result) => resolve(result),
        reject,
        onChunk: handlers.onChunk,
      });
      this._worker.postMessage(msg);
    });
  }

  /**
   * @param {string|number} id
   * @param {{ resolve: Function, reject: Function, onChunk?: Function, raw?: boolean }} handlers
   */
  _register(id, handlers) {
    /** @type {{ resolve: Function, reject: Function, onChunk?: Function, raw?: boolean, timer?: ReturnType<typeof setTimeout> }} */
    const entry = { ...handlers };
    if (this._timeoutMs > 0) {
      entry.timer = setTimeout(() => {
        this._pending.delete(id);
        handlers.reject(new Error(`worker request ${id} timed out after ${this._timeoutMs}ms`));
      }, this._timeoutMs);
    }
    this._pending.set(id, entry);
  }

  /**
   * @param {unknown} data
   */
  _onMessage(data) {
    const checked = validateResponse(data);
    if (!checked.ok) {
      console.warn("[clawft-worker-client] invalid response:", checked.error, data);
      return;
    }
    const msg = checked.value;

    if (msg.type === ResponseType.WORKER_READY) {
      this._ready = true;
      if (this._readyResolve) {
        this._readyResolve();
        this._readyResolve = null;
      }
      return;
    }

    if (msg.id === undefined || msg.id === null) {
      console.warn("[clawft-worker-client] response missing id:", msg);
      return;
    }

    const pending = this._pending.get(msg.id);
    if (!pending) {
      // Late / duplicate — ignore.
      return;
    }

    if (msg.type === ResponseType.CHUNK) {
      if (typeof pending.onChunk === "function") {
        pending.onChunk(msg.text ?? "");
      }
      return; // keep pending until OK / ERROR
    }

    if (pending.timer) clearTimeout(pending.timer);
    this._pending.delete(msg.id);

    if (msg.type === ResponseType.ERROR) {
      pending.reject(new Error(msg.error || "worker error"));
      return;
    }

    if (msg.type === ResponseType.PONG || pending.raw) {
      pending.resolve(msg);
      return;
    }

    if (msg.type === ResponseType.OK) {
      // Prefer result; fall back to text for send/stream convenience.
      const value = msg.result !== undefined ? msg.result : msg.text;
      pending.resolve(value);
      return;
    }

    pending.reject(new Error(`unexpected response type: ${msg.type}`));
  }
}

export { PROTOCOL_VERSION, RequestType, ResponseType };
