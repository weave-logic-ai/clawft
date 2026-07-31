// clawft-wasm Web Worker entry (WEFT-400)
//
// Runs the browser WASM agent off the main thread so long LLM calls and
// tool work do not block the UI. Hosts speak the protocol defined in
// protocol.js via postMessage.
//
// Load as a module worker:
//   new Worker(new URL("./worker.js", import.meta.url), { type: "module" })
//
// Requires crates/clawft-wasm/www/pkg/ from `scripts/build.sh browser`.

import {
  PROTOCOL_VERSION,
  RequestType,
  ResponseType,
  validateRequest,
  makeResponse,
} from "./protocol.js";

/** @type {typeof import("./pkg/clawft_wasm.js") | null} */
let wasmModule = null;
let wasmLoaded = false;
let initialized = false;

/**
 * Post a protocol response to the host.
 * @param {import("./protocol.js").WorkerResponse} msg
 */
function reply(msg) {
  self.postMessage(msg);
}

/**
 * Ensure the WASM package is imported and its bg.wasm is initialized.
 */
async function ensureWasmLoaded() {
  if (wasmLoaded && wasmModule) return;
  wasmModule = await import("./pkg/clawft_wasm.js");
  if (typeof wasmModule.default === "function") {
    await wasmModule.default();
  }
  wasmLoaded = true;
}

/**
 * Normalize a thrown value to an error string.
 * @param {unknown} err
 */
function errString(err) {
  if (err == null) return "unknown error";
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message || String(err);
  // wasm-bindgen often rejects with a plain string JsValue
  try {
    return String(err);
  } catch {
    return "unknown error";
  }
}

/**
 * Handle one validated request.
 * @param {import("./protocol.js").WorkerRequest} req
 */
async function handleRequest(req) {
  const { id, type } = req;

  switch (type) {
    case RequestType.PING: {
      reply(
        makeResponse(id, ResponseType.PONG, {
          protocolVersion: PROTOCOL_VERSION,
          wasmLoaded,
          initialized,
        }),
      );
      return;
    }

    case RequestType.LOAD: {
      await ensureWasmLoaded();
      reply(
        makeResponse(id, ResponseType.OK, {
          result: { wasmLoaded: true, protocolVersion: PROTOCOL_VERSION },
        }),
      );
      return;
    }

    case RequestType.INIT: {
      await ensureWasmLoaded();
      const env =
        req.envJson === undefined || req.envJson === null
          ? undefined
          : req.envJson;
      await wasmModule.init(req.configJson, env);
      initialized = true;
      reply(makeResponse(id, ResponseType.OK, { result: { initialized: true } }));
      return;
    }

    case RequestType.SEND: {
      if (!initialized || !wasmModule) {
        reply(
          makeResponse(id, ResponseType.ERROR, {
            error: "not initialized — call init first",
          }),
        );
        return;
      }
      const text = await wasmModule.send_message(req.text);
      reply(makeResponse(id, ResponseType.OK, { result: text, text }));
      return;
    }

    case RequestType.STREAM: {
      if (!initialized || !wasmModule) {
        reply(
          makeResponse(id, ResponseType.ERROR, {
            error: "not initialized — call init first",
          }),
        );
        return;
      }
      // Bridge stream_chat's JS callback into postMessage CHUNK events.
      const onChunk = (chunk) => {
        const piece = typeof chunk === "string" ? chunk : String(chunk ?? "");
        reply(makeResponse(id, ResponseType.CHUNK, { text: piece }));
      };
      const full = await wasmModule.stream_chat(req.text, onChunk);
      reply(makeResponse(id, ResponseType.OK, { result: full, text: full }));
      return;
    }

    case RequestType.SET_ENV: {
      if (!initialized || !wasmModule) {
        reply(
          makeResponse(id, ResponseType.ERROR, {
            error: "not initialized — call init first",
          }),
        );
        return;
      }
      wasmModule.set_env(req.key, req.value);
      reply(makeResponse(id, ResponseType.OK, { result: null }));
      return;
    }

    case RequestType.GET_ENV: {
      if (!initialized || !wasmModule) {
        reply(
          makeResponse(id, ResponseType.ERROR, {
            error: "not initialized — call init first",
          }),
        );
        return;
      }
      const value = wasmModule.get_env(req.key);
      reply(
        makeResponse(id, ResponseType.OK, {
          result: value === undefined ? null : value,
        }),
      );
      return;
    }

    case RequestType.BOOT_INFO: {
      await ensureWasmLoaded();
      const json = wasmModule.boot_info();
      reply(makeResponse(id, ResponseType.OK, { result: json }));
      return;
    }

    case RequestType.ANALYZE_FILES: {
      await ensureWasmLoaded();
      const json = wasmModule.analyze_files(req.filesJson);
      reply(makeResponse(id, ResponseType.OK, { result: json }));
      return;
    }

    case RequestType.TOOL_LIST: {
      if (!initialized || !wasmModule) {
        reply(
          makeResponse(id, ResponseType.ERROR, {
            error: "not initialized — call init first",
          }),
        );
        return;
      }
      const json = wasmModule.tool_list();
      reply(makeResponse(id, ResponseType.OK, { result: json }));
      return;
    }

    case RequestType.TOOL_SCHEMA: {
      if (!initialized || !wasmModule) {
        reply(
          makeResponse(id, ResponseType.ERROR, {
            error: "not initialized — call init first",
          }),
        );
        return;
      }
      const json = wasmModule.tool_schema(req.key);
      reply(makeResponse(id, ResponseType.OK, { result: json }));
      return;
    }

    default: {
      reply(
        makeResponse(id, ResponseType.ERROR, {
          error: `unsupported request type: ${type}`,
        }),
      );
    }
  }
}

self.onmessage = async (event) => {
  const raw = event.data;
  const checked = validateRequest(raw);
  if (!checked.ok) {
    const id =
      raw && typeof raw === "object" && "id" in raw ? raw.id : "unknown";
    reply(
      makeResponse(id, ResponseType.ERROR, {
        error: `invalid request: ${checked.error}`,
      }),
    );
    return;
  }

  try {
    await handleRequest(checked.value);
  } catch (err) {
    reply(
      makeResponse(checked.value.id, ResponseType.ERROR, {
        error: errString(err),
      }),
    );
  }
};

// Eager-load WASM so the host can show "worker ready" without a round-trip.
// Failures are reported as an error-shaped unsolicited message is awkward;
// host will see load/init fail with a clear error instead.
ensureWasmLoaded()
  .then(() => {
    reply(
      makeResponse(undefined, ResponseType.WORKER_READY, {
        protocolVersion: PROTOCOL_VERSION,
      }),
    );
  })
  .catch((err) => {
    // No id — host listens for worker_ready; if never arrives, load() fails.
    console.error("[clawft-worker] WASM load failed:", errString(err));
  });
