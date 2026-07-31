// WEFT-284 — ThreadDock wire contract for the VSCode panel bridge.
//
// Mirrors `clawft_canon::thread_dock` (`ui://thread-dock`) so the
// extension host can reason about parallel agent columns without
// pulling the wasm surface. Pure functions — unit-testable without a
// VSCode host.
//
// Design (session-7 multi-agent parallel output):
//   • Column-per-agent — never interleaved in a single scroll.
//   • One focus lane; non-focused columns keep streaming with presence
//     pips (phase + token rate).
//   • Agent columns must not raise modals.

/** Ontology IRI for the ThreadDock composition primitive. */
export const THREAD_DOCK_IDENTITY = "ui://thread-dock" as const;

/** Wire message types (webview ↔ host). */
export const THREAD_DOCK_UPDATE_TYPE = "thread-dock-update" as const;
export const THREAD_DOCK_FOCUS_TYPE = "thread-dock-focus" as const;

/** Live phase for an agent column's presence pip. */
export type ThreadPhase =
    | "idle"
    | "thinking"
    | "streaming"
    | "awaiting-input"
    | "done"
    | "error";

export const THREAD_PHASES: readonly ThreadPhase[] = Object.freeze([
    "idle",
    "thinking",
    "streaming",
    "awaiting-input",
    "done",
    "error",
]);

/** One parallel agent column — ontology-addressable, streamable. */
export interface AgentThread {
    /** Stable thread id (substrate path segment or swarm agent id). */
    id: string;
    /** Human / voice label ("coder", "planner", "supervisor"). */
    label: string;
    /** Presence phase for the header pip. */
    phase: ThreadPhase;
    /** Monotonic output lines (never interleaved with other threads). */
    lines: string[];
    /** Approximate tokens/sec (0 = unknown). */
    tokenRate: number;
    /** Active-radar variant-id for the last pulse that fed this column. */
    variantId: string | number;
}

/** Caller-owned dock state — focus lane + columns. */
export interface ThreadDockState {
    threads: AgentThread[];
    /** Index of the focus lane (keyboard / composer target). */
    focused: number;
}

export function emptyThreadDock(): ThreadDockState {
    return { threads: [], focused: 0 };
}

/** True when the dock shows parallel columns (≥2 streams). */
export function isParallel(state: ThreadDockState): boolean {
    return state.threads.length >= 2;
}

/** Parse a wire phase label; accepts `generating` → streaming. */
export function parseThreadPhase(raw: unknown): ThreadPhase | null {
    if (typeof raw !== "string") return null;
    const s = raw.trim().toLowerCase();
    if (s === "generating") return "streaming";
    if (s === "awaiting") return "awaiting-input";
    if ((THREAD_PHASES as readonly string[]).includes(s)) {
        return s as ThreadPhase;
    }
    return null;
}

export function isActivePhase(phase: ThreadPhase): boolean {
    return phase === "thinking" || phase === "streaming" || phase === "awaiting-input";
}

export function createAgentThread(
    id: string,
    label: string,
    opts: Partial<Omit<AgentThread, "id" | "label">> = {},
): AgentThread {
    return {
        id,
        label,
        phase: opts.phase ?? "idle",
        lines: opts.lines ? [...opts.lines] : [],
        tokenRate: Math.max(0, opts.tokenRate ?? 0),
        variantId: opts.variantId ?? 0,
    };
}

/** Push a thread; returns its index. */
export function pushThread(state: ThreadDockState, thread: AgentThread): number {
    state.threads.push({
        ...thread,
        lines: [...thread.lines],
    });
    const idx = state.threads.length - 1;
    if (state.threads.length === 1) {
        state.focused = 0;
    }
    return idx;
}

/** Upsert by id — replace if present, else push. Returns index. */
export function upsertThread(state: ThreadDockState, thread: AgentThread): number {
    const idx = state.threads.findIndex((t) => t.id === thread.id);
    if (idx >= 0) {
        state.threads[idx] = { ...thread, lines: [...thread.lines] };
        return idx;
    }
    return pushThread(state, thread);
}

export function focusThread(state: ThreadDockState, idx: number): boolean {
    if (idx < 0 || idx >= state.threads.length) return false;
    state.focused = idx;
    return true;
}

export function focusThreadById(state: ThreadDockState, id: string): boolean {
    const idx = state.threads.findIndex((t) => t.id === id);
    if (idx < 0) return false;
    state.focused = idx;
    return true;
}

/**
 * Replace a thread's line buffer from an accumulated stream draft
 * (WEFT-253 style: full text, not delta).
 */
export function setStreamText(
    state: ThreadDockState,
    id: string,
    text: string,
    phase?: ThreadPhase,
): boolean {
    const t = state.threads.find((x) => x.id === id);
    if (!t) return false;
    t.lines = text === "" ? [] : text.split("\n");
    if (phase !== undefined) {
        t.phase = phase;
    } else {
        t.phase = text === "" ? "thinking" : "streaming";
    }
    return true;
}

/** Append one line to a thread by id. */
export function appendLine(state: ThreadDockState, id: string, line: string): boolean {
    const t = state.threads.find((x) => x.id === id);
    if (!t) return false;
    t.lines.push(line);
    return true;
}

export function setPhase(state: ThreadDockState, id: string, phase: ThreadPhase): boolean {
    const t = state.threads.find((x) => x.id === id);
    if (!t) return false;
    t.phase = phase;
    return true;
}

export function removeThread(state: ThreadDockState, id: string): boolean {
    const idx = state.threads.findIndex((t) => t.id === id);
    if (idx < 0) return false;
    state.threads.splice(idx, 1);
    if (state.threads.length === 0) {
        state.focused = 0;
    } else if (state.focused > idx) {
        state.focused -= 1;
    } else if (state.focused >= state.threads.length) {
        state.focused = state.threads.length - 1;
    }
    return true;
}

export function clearThreads(state: ThreadDockState): void {
    state.threads = [];
    state.focused = 0;
}

/**
 * Structural invariant: no line from thread A may appear in thread B's
 * buffer after independent feeds. Used by smoke tests for ≥2 streams.
 */
export function streamsAreIsolated(state: ThreadDockState): boolean {
    for (let i = 0; i < state.threads.length; i++) {
        const a = state.threads[i];
        for (let j = 0; j < state.threads.length; j++) {
            if (i === j) continue;
            const b = state.threads[j];
            // Soft check: lines that uniquely tag their owner must not
            // leak. Callers that want a hard check should pass tagged
            // lines (e.g. "coder: …" / "reviewer: …").
            for (const line of a.lines) {
                if (line.includes(`[${b.id}]`)) return false;
            }
        }
    }
    return true;
}

/** Validate a partial wire payload into an AgentThread, or null. */
export function parseAgentThread(raw: unknown): AgentThread | null {
    if (!raw || typeof raw !== "object") return null;
    const o = raw as Record<string, unknown>;
    if (typeof o.id !== "string" || o.id.length === 0) return null;
    if (typeof o.label !== "string") return null;
    const phase = parseThreadPhase(o.phase) ?? "idle";
    const lines = Array.isArray(o.lines)
        ? o.lines.filter((x): x is string => typeof x === "string")
        : [];
    const tokenRate =
        typeof o.tokenRate === "number"
            ? Math.max(0, o.tokenRate)
            : typeof o.token_rate === "number"
              ? Math.max(0, o.token_rate)
              : 0;
    const variantId =
        typeof o.variantId === "string" || typeof o.variantId === "number"
            ? o.variantId
            : typeof o["variant-id"] === "string" || typeof o["variant-id"] === "number"
              ? (o["variant-id"] as string | number)
              : 0;
    return {
        id: o.id,
        label: o.label,
        phase,
        lines,
        tokenRate,
        variantId,
    };
}
