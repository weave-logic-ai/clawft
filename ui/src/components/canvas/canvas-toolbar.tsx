import { useEffect, useCallback } from "react";
import { useCanvasStore } from "../../stores/canvas-store";
import { Button } from "../ui/button";

/**
 * Canvas toolbar with Undo/Redo controls.
 * Registers keyboard shortcuts: Ctrl+Z (undo), Ctrl+Shift+Z (redo).
 */
export function CanvasToolbar() {
  const undo = useCanvasStore((s) => s.undo);
  const redo = useCanvasStore((s) => s.redo);
  const canUndo = useCanvasStore((s) => s.canUndo);
  const canRedo = useCanvasStore((s) => s.canRedo);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      const isCtrlOrCmd = e.ctrlKey || e.metaKey;
      if (!isCtrlOrCmd) return;

      if (e.key === "z" && !e.shiftKey) {
        e.preventDefault();
        undo();
      } else if (e.key === "z" && e.shiftKey) {
        e.preventDefault();
        redo();
      } else if (e.key === "y") {
        // Ctrl+Y as alternative redo
        e.preventDefault();
        redo();
      }
    },
    [undo, redo],
  );

  useEffect(() => {
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  return (
    <div className="flex items-center gap-1">
      <Button
        variant="ghost"
        size="sm"
        disabled={!canUndo}
        onClick={undo}
        title="Undo (Ctrl+Z)"
        aria-label="Undo"
      >
        <UndoIcon />
        <span className="ml-1 hidden sm:inline">Undo</span>
      </Button>
      <Button
        variant="ghost"
        size="sm"
        disabled={!canRedo}
        onClick={redo}
        title="Redo (Ctrl+Shift+Z)"
        aria-label="Redo"
      >
        <RedoIcon />
        <span className="ml-1 hidden sm:inline">Redo</span>
      </Button>
    </div>
  );
}

/** Simple undo arrow icon (no dependency on lucide). */
function UndoIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      className="h-4 w-4"
    >
      <path d="M3 7v6h6" />
      <path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13" />
    </svg>
  );
}

/** Simple redo arrow icon (no dependency on lucide). */
function RedoIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      className="h-4 w-4"
    >
      <path d="M21 7v6h-6" />
      <path d="M3 17a9 9 0 0 1 9-9 9 9 0 0 1 6 2.3L21 13" />
    </svg>
  );
}
