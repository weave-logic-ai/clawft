import { create } from "zustand";

export interface CanvasElementData {
  id: string;
  type: string;
  [key: string]: unknown;
}

/** Maximum number of undo history entries to retain. */
const MAX_HISTORY_DEPTH = 50;

/** Serializable snapshot of the elements map for history. */
type ElementsSnapshot = Array<[string, CanvasElementData]>;

interface CanvasStore {
  elements: Map<string, CanvasElementData>;

  // ── History (undo/redo) ──────────────────────────────────────
  past: ElementsSnapshot[];
  future: ElementsSnapshot[];
  canUndo: boolean;
  canRedo: boolean;

  // ── Mutations (auto-push history) ────────────────────────────
  addElement: (id: string, element: CanvasElementData) => void;
  updateElement: (id: string, element: CanvasElementData) => void;
  removeElement: (id: string) => void;
  reset: () => void;

  // ── History actions ──────────────────────────────────────────
  undo: () => void;
  redo: () => void;
}

/** Convert a Map to a serializable snapshot. */
function snapshotOf(map: Map<string, CanvasElementData>): ElementsSnapshot {
  return Array.from(map.entries());
}

/** Restore a Map from a snapshot. */
function restoreFrom(snapshot: ElementsSnapshot): Map<string, CanvasElementData> {
  return new Map(snapshot.map(([k, v]) => [k, { ...v }]));
}

export const useCanvasStore = create<CanvasStore>((set, get) => ({
  elements: new Map(),
  past: [],
  future: [],
  canUndo: false,
  canRedo: false,

  addElement: (id, element) => {
    const state = get();
    const snapshot = snapshotOf(state.elements);
    const next = new Map(state.elements);
    next.set(id, { ...element, id });

    const newPast = [...state.past, snapshot].slice(-MAX_HISTORY_DEPTH);
    set({
      elements: next,
      past: newPast,
      future: [],
      canUndo: newPast.length > 0,
      canRedo: false,
    });
  },

  updateElement: (id, element) => {
    const state = get();
    if (!state.elements.has(id)) return;

    const snapshot = snapshotOf(state.elements);
    const next = new Map(state.elements);
    next.set(id, { ...element, id });

    const newPast = [...state.past, snapshot].slice(-MAX_HISTORY_DEPTH);
    set({
      elements: next,
      past: newPast,
      future: [],
      canUndo: newPast.length > 0,
      canRedo: false,
    });
  },

  removeElement: (id) => {
    const state = get();
    if (!state.elements.has(id)) return;

    const snapshot = snapshotOf(state.elements);
    const next = new Map(state.elements);
    next.delete(id);

    const newPast = [...state.past, snapshot].slice(-MAX_HISTORY_DEPTH);
    set({
      elements: next,
      past: newPast,
      future: [],
      canUndo: newPast.length > 0,
      canRedo: false,
    });
  },

  reset: () =>
    set({
      elements: new Map(),
      past: [],
      future: [],
      canUndo: false,
      canRedo: false,
    }),

  undo: () => {
    const state = get();
    if (state.past.length === 0) return;

    const previousSnapshot = state.past[state.past.length - 1];
    const currentSnapshot = snapshotOf(state.elements);
    const newPast = state.past.slice(0, -1);
    const newFuture = [...state.future, currentSnapshot].slice(
      -MAX_HISTORY_DEPTH,
    );

    set({
      elements: restoreFrom(previousSnapshot),
      past: newPast,
      future: newFuture,
      canUndo: newPast.length > 0,
      canRedo: newFuture.length > 0,
    });
  },

  redo: () => {
    const state = get();
    if (state.future.length === 0) return;

    const nextSnapshot = state.future[state.future.length - 1];
    const currentSnapshot = snapshotOf(state.elements);
    const newFuture = state.future.slice(0, -1);
    const newPast = [...state.past, currentSnapshot].slice(
      -MAX_HISTORY_DEPTH,
    );

    set({
      elements: restoreFrom(nextSnapshot),
      past: newPast,
      future: newFuture,
      canUndo: newPast.length > 0,
      canRedo: newFuture.length > 0,
    });
  },
}));
