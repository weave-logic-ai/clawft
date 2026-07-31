// clawft-wasm Web Worker harness UI (WEFT-400)
//
// Same UX as main.js, but the agent runs inside a module Worker via
// ClawftWorkerClient. Long LLM calls no longer freeze the chat UI.

import { ClawftWorkerClient } from "./worker-client.js";

// -- DOM elements ----------------------------------------------------------

const configTextarea = document.getElementById("config-textarea");
const initBtn = document.getElementById("init-btn");
const streamToggle = document.getElementById("stream-toggle");
const chatDisplay = document.getElementById("chat-display");
const msgInput = document.getElementById("msg-input");
const sendBtn = document.getElementById("send-btn");
const statusIndicator = document.getElementById("status-indicator");
const statusText = document.getElementById("status-text");

// -- State -----------------------------------------------------------------

/** @type {ClawftWorkerClient | null} */
let client = null;
let initialized = false;
let useStream = false;
let envKeys = {};

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
  return body;
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

function obfuscate(key) {
  if (!key || key.length < 12) return "****";
  return key.slice(0, 6) + "****" + key.slice(-4);
}

// -- Environment key injection (parity with main.js) -----------------------

async function loadEnvKeys() {
  try {
    const resp = await fetch("./.env-keys.json");
    if (!resp.ok) return;
    const keys = await resp.json();
    if (!keys || typeof keys !== "object" || Object.keys(keys).length === 0) return;

    envKeys = keys;

    let config;
    try {
      config = JSON.parse(configTextarea.value);
    } catch {
      return;
    }

    if (!config.providers) config.providers = {};

    let injected = 0;
    for (const [provider, realKey] of Object.entries(keys)) {
      if (!realKey) continue;
      if (!config.providers[provider]) config.providers[provider] = {};
      config.providers[provider].apiKey = obfuscate(realKey);
      injected++;
    }

    if (injected > 0) {
      configTextarea.value = JSON.stringify(config, null, 2);
      appendMessage(
        "system",
        `${injected} API key(s) loaded from environment (obfuscated in display)`,
      );
    }
  } catch {
    // No .env-keys.json — use textarea defaults.
  }
}

function buildRealConfig() {
  const config = JSON.parse(configTextarea.value);

  if (config.providers && Object.keys(envKeys).length > 0) {
    for (const [provider, realKey] of Object.entries(envKeys)) {
      if (!realKey || !config.providers[provider]) continue;
      const displayKey = config.providers[provider].apiKey || "";
      if (
        displayKey === obfuscate(realKey) ||
        displayKey.includes("****") ||
        displayKey.startsWith("YOUR_")
      ) {
        config.providers[provider].apiKey = realKey;
      }
    }
  }

  if (
    window.location.hostname === "localhost" ||
    window.location.hostname === "127.0.0.1"
  ) {
    const proxyBase = "http://" + window.location.host + "/proxy";
    if (config.providers) {
      for (const [, cfg] of Object.entries(config.providers)) {
        if (cfg && typeof cfg === "object" && !cfg.corsProxy) {
          cfg.corsProxy = proxyBase;
        }
      }
    }
  }

  return JSON.stringify(config);
}

// -- Worker boot -----------------------------------------------------------

async function bootWorker() {
  setStatus("loading", "starting web worker…");
  const bootStart = performance.now();

  try {
    client = new ClawftWorkerClient();
    await client.start();

    const ping = await client.ping();
    const bootMs = (performance.now() - bootStart).toFixed(1);
    console.log(`[clawft-worker] ready in ${bootMs}ms`, ping);

    setStatus("", "worker ready, wasm loaded — initialize to chat");
    appendMessage(
      "system",
      `Web Worker ready in ${bootMs}ms (protocol v${ping.protocolVersion}, wasmLoaded=${ping.wasmLoaded})`,
    );
  } catch (err) {
    const bootMs = (performance.now() - bootStart).toFixed(1);
    console.error(`[clawft-worker] boot failed after ${bootMs}ms:`, err);
    setStatus("error", "worker failed to start");
    appendMessage("error", `Worker boot failed: ${err.message || err}`);
    initBtn.disabled = true;
  }
}

// -- Initialize ------------------------------------------------------------

async function handleInit() {
  if (!client) {
    appendMessage("error", "Worker not ready yet.");
    return;
  }

  try {
    JSON.parse(configTextarea.value.trim());
  } catch (parseErr) {
    appendMessage("error", `Invalid JSON: ${parseErr.message}`);
    return;
  }

  const configJson = buildRealConfig();

  try {
    const dbg = JSON.parse(configJson);
    if (dbg.providers) {
      for (const [name, cfg] of Object.entries(dbg.providers)) {
        const hasKey = !!(cfg.apiKey && cfg.apiKey.length > 5);
        console.log(
          `[clawft-worker] provider ${name}: key=${hasKey}, corsProxy=${cfg.corsProxy || "none"}`,
        );
      }
    }
  } catch {
    /* ignore */
  }

  initBtn.disabled = true;
  setStatus("loading", "initializing in worker…");
  appendMessage("system", "Initializing clawft-wasm inside Web Worker…");

  const initStart = performance.now();

  try {
    await client.init(configJson);

    const initMs = (performance.now() - initStart).toFixed(1);
    console.log(`[clawft-worker] init: ${initMs}ms`);

    initialized = true;
    setStatus("ready", "initialized (worker)");
    appendMessage(
      "system",
      `Initialized in worker in ${initMs}ms. Main thread stays free during LLM calls.`,
    );
    enableChat(true);
    configTextarea.disabled = true;
  } catch (err) {
    const initMs = (performance.now() - initStart).toFixed(1);
    console.error(`[clawft-worker] init failed after ${initMs}ms:`, err);
    setStatus("error", "initialization failed");
    appendMessage("error", `Init failed: ${err.message || err}`);
    initBtn.disabled = false;
  }
}

// -- Send ------------------------------------------------------------------

async function handleSend() {
  if (!initialized || !client) return;

  const text = msgInput.value.trim();
  if (!text) return;

  msgInput.value = "";
  appendMessage("user", text);
  sendBtn.disabled = true;
  msgInput.disabled = true;

  const sendStart = performance.now();

  try {
    let response;
    if (useStream) {
      const body = appendMessage("assistant", "");
      response = await client.streamChat(text, (chunk) => {
        body.textContent += chunk;
        chatDisplay.scrollTop = chatDisplay.scrollHeight;
      });
      // Ensure final text is exact (in case chunks were partial rebuilds).
      body.textContent = response;
    } else {
      response = await client.sendMessage(text);
      appendMessage("assistant", response);
    }

    const sendMs = (performance.now() - sendStart).toFixed(1);
    console.log(`[clawft-worker] send (${useStream ? "stream" : "send"}): ${sendMs}ms`);
  } catch (err) {
    const sendMs = (performance.now() - sendStart).toFixed(1);
    console.error(`[clawft-worker] send failed after ${sendMs}ms:`, err);
    appendMessage("error", `Error: ${err.message || err}`);
  } finally {
    enableChat(true);
  }
}

// -- Event wiring ----------------------------------------------------------

initBtn.addEventListener("click", handleInit);
sendBtn.addEventListener("click", handleSend);

streamToggle.addEventListener("click", () => {
  useStream = !useStream;
  streamToggle.classList.toggle("active", useStream);
  streamToggle.textContent = useStream ? "stream: on" : "stream: off";
});

msgInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    handleSend();
  }
});

// -- Boot ------------------------------------------------------------------

loadEnvKeys().then(() => bootWorker());
