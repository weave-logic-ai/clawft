// Webview-side client. Uses acquireVsCodeApi() once and only once.
// All daemon I/O goes through the extension host via postMessage.
//
// Return-signal log is a local placeholder for the active-radar topic
// described in .planning/symposiums/compositional-ui/foundations.md.
// Future WSP-0.1 iterations will push these events onto the substrate.

(function () {
  const vscode = acquireVsCodeApi();

  const statusLine = document.getElementById("statusLine");
  const output = document.getElementById("output");
  const radarLog = document.getElementById("radarLog");
  const btnStatus = document.getElementById("btnStatus");
  const btnPs = document.getElementById("btnPs");

  const pending = new Map(); // id -> { method, startedAt }
  const radar = []; // most-recent-first, max 5

  function uuid() {
    if (crypto && typeof crypto.randomUUID === "function") {
      return crypto.randomUUID();
    }
    return "xxxxxxxxxxxx".replace(/x/g, () => Math.floor(Math.random() * 16).toString(16));
  }

  function setStatus(text, cls) {
    statusLine.textContent = text;
    statusLine.className = "status" + (cls ? " " + cls : "");
  }

  function recordReturn(method, ok, latencyMs, error) {
    radar.unshift({
      ts: new Date().toISOString().split("T")[1].replace("Z", ""),
      method,
      ok,
      latencyMs,
      error,
    });
    while (radar.length > 5) radar.pop();
    radarLog.innerHTML = "";
    for (const entry of radar) {
      const li = document.createElement("li");
      const ts = document.createElement("span");
      ts.className = "ts";
      ts.textContent = entry.ts;
      const method = document.createElement("span");
      method.className = "method";
      method.textContent = entry.method;
      const outcome = document.createElement("span");
      outcome.className = "outcome " + (entry.ok ? "ok" : "err");
      outcome.textContent = entry.ok ? "ok" : "err: " + (entry.error || "unknown");
      const latency = document.createElement("span");
      latency.className = "latency";
      latency.textContent = entry.latencyMs + "ms";
      li.append(ts, method, outcome, latency);
      radarLog.appendChild(li);
    }
  }

  function send(method) {
    const id = uuid();
    pending.set(id, { method, startedAt: performance.now() });
    btnStatus.disabled = true;
    btnPs.disabled = true;
    output.classList.remove("err");
    output.textContent = "(awaiting " + method + " ...)";
    vscode.postMessage({ kind: "rpc", id, method });
  }

  btnStatus.addEventListener("click", () => send("kernel.status"));
  btnPs.addEventListener("click", () => send("kernel.ps"));

  window.addEventListener("message", (event) => {
    const msg = event.data;
    if (!msg || typeof msg !== "object") return;

    if (msg.kind === "hello") {
      setStatus("socket: " + msg.socketPath + " (idle)", null);
      return;
    }

    if (msg.kind === "rpc-result") {
      const tracked = pending.get(msg.id);
      pending.delete(msg.id);
      const latency = tracked
        ? Math.round(performance.now() - tracked.startedAt)
        : 0;
      btnStatus.disabled = false;
      btnPs.disabled = false;

      if (msg.ok) {
        output.classList.remove("err");
        output.textContent = JSON.stringify(msg.result, null, 2);
        setStatus("connected (" + msg.method + " ok in " + latency + "ms)", "ok");
        recordReturn(msg.method, true, latency);
      } else {
        output.classList.add("err");
        output.textContent = msg.error || "(unknown error)";
        setStatus("disconnected: " + msg.method + " failed", "err");
        recordReturn(msg.method, false, latency, msg.error);
      }
    }
  });

  vscode.postMessage({ kind: "ready" });
})();
