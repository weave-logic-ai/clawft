// clawft-wasm Browser Test Harness
//
// Loads the WASM module built by wasm-pack and wires the UI controls
// to the exported init(), send_message(), and set_env() functions.

// -- DOM elements ----------------------------------------------------------

const configTextarea = document.getElementById("config-textarea");
const initBtn = document.getElementById("init-btn");
const chatDisplay = document.getElementById("chat-display");
const msgInput = document.getElementById("msg-input");
const sendBtn = document.getElementById("send-btn");
const statusIndicator = document.getElementById("status-indicator");
const statusText = document.getElementById("status-text");

// -- State -----------------------------------------------------------------

let wasmModule = null;
let initialized = false;

// -- Helpers ---------------------------------------------------------------

function timestamp() {
  return new Date().toISOString().slice(11, 23);
}

function appendMessage(role, text) {
  const div = document.createElement("div");
  div.className = `msg ${role}`;

  const ts = document.createElement("div");
  ts.className = "ts";
  ts.textContent = timestamp();

  const body = document.createElement("div");
  body.textContent = text;

  div.appendChild(ts);
  div.appendChild(body);
  chatDisplay.appendChild(div);
  chatDisplay.scrollTop = chatDisplay.scrollHeight;
}

function setStatus(state, message) {
  statusIndicator.className = state;
  statusText.textContent = message;
}

function enableChat(enabled) {
  msgInput.disabled = !enabled;
  sendBtn.disabled = !enabled;
  if (enabled) {
    msgInput.focus();
  }
}

// -- WASM loading ----------------------------------------------------------

async function loadWasm() {
  setStatus("loading", "loading wasm module...");

  const loadStart = performance.now();

  try {
    // wasm-pack outputs to pkg/ by default. Adjust the path if your
    // build places it elsewhere.
    wasmModule = await import("../pkg/clawft_wasm.js");

    // wasm-bindgen generates an `default` (or __wbg_init) initializer
    // for the bg.wasm. Some wasm-pack targets export it as default.
    if (typeof wasmModule.default === "function") {
      await wasmModule.default();
    }

    const loadMs = (performance.now() - loadStart).toFixed(1);
    console.log(`[clawft] wasm-load: ${loadMs}ms`);
    setStatus("", "wasm loaded, ready to initialize");
    appendMessage("system", `WASM module loaded in ${loadMs}ms`);
  } catch (err) {
    const loadMs = (performance.now() - loadStart).toFixed(1);
    console.error(`[clawft] wasm-load failed after ${loadMs}ms:`, err);
    setStatus("error", "failed to load wasm module");
    appendMessage("error", `Failed to load WASM: ${err.message || err}`);
    initBtn.disabled = true;
  }
}

// -- Initialize handler ----------------------------------------------------

async function handleInit() {
  if (!wasmModule) {
    appendMessage("error", "WASM module not loaded yet.");
    return;
  }

  const configJson = configTextarea.value.trim();

  // Validate JSON locally before sending to WASM.
  try {
    JSON.parse(configJson);
  } catch (parseErr) {
    appendMessage("error", `Invalid JSON: ${parseErr.message}`);
    return;
  }

  initBtn.disabled = true;
  setStatus("loading", "initializing...");
  appendMessage("system", "Initializing clawft-wasm...");

  const initStart = performance.now();

  try {
    await wasmModule.init(configJson);

    const initMs = (performance.now() - initStart).toFixed(1);
    console.log(`[clawft] init: ${initMs}ms`);

    initialized = true;
    setStatus("ready", "initialized");
    appendMessage("system", `Initialized in ${initMs}ms. Ready to chat.`);
    enableChat(true);
    configTextarea.disabled = true;
  } catch (err) {
    const initMs = (performance.now() - initStart).toFixed(1);
    console.error(`[clawft] init failed after ${initMs}ms:`, err);
    setStatus("error", "initialization failed");
    appendMessage("error", `Init failed: ${err.message || err}`);
    initBtn.disabled = false;
  }
}

// -- Send handler ----------------------------------------------------------

async function handleSend() {
  if (!initialized || !wasmModule) return;

  const text = msgInput.value.trim();
  if (!text) return;

  msgInput.value = "";
  appendMessage("user", text);
  sendBtn.disabled = true;
  msgInput.disabled = true;

  const sendStart = performance.now();

  try {
    const response = await wasmModule.send_message(text);

    const sendMs = (performance.now() - sendStart).toFixed(1);
    console.log(`[clawft] send_message: ${sendMs}ms`);

    appendMessage("assistant", response);
  } catch (err) {
    const sendMs = (performance.now() - sendStart).toFixed(1);
    console.error(`[clawft] send_message failed after ${sendMs}ms:`, err);
    appendMessage("error", `Error: ${err.message || err}`);
  } finally {
    enableChat(true);
  }
}

// -- Event wiring ----------------------------------------------------------

initBtn.addEventListener("click", handleInit);
sendBtn.addEventListener("click", handleSend);

msgInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    handleSend();
  }
});

// -- Boot ------------------------------------------------------------------

loadWasm();
