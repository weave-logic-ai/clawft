import { useState, useCallback, useRef, useEffect } from "react";
import type { CanvasElementData } from "../../stores/canvas-store";
import { Badge } from "../ui/badge";

interface CodeElementProps {
  element: CanvasElementData;
}

/**
 * Simple code display for the basic "code" canvas element type.
 * Shows syntax-highlighted code with an optional language badge.
 */
export function CodeElement({ element }: CodeElementProps) {
  const code = (element.code as string) ?? "";
  const language = (element.language as string) ?? "";

  return (
    <div className="relative">
      {language && (
        <Badge
          variant="outline"
          className="absolute right-2 top-2 text-xs"
        >
          {language}
        </Badge>
      )}
      <pre className="overflow-auto rounded-md bg-gray-100 p-4 text-sm text-gray-800 dark:bg-gray-900 dark:text-gray-300">
        <code>{code}</code>
      </pre>
    </div>
  );
}

interface CodeEditorElementProps {
  element: CanvasElementData;
  onInteraction: (interaction: {
    type: string;
    element_id: string;
    [key: string]: unknown;
  }) => void;
}

/**
 * Enhanced code element for the "code_editor" canvas element type.
 * Supports line numbers, optional editing via a textarea overlay,
 * and CSS-based syntax highlighting using pre/code blocks.
 */
export function CodeEditorElement({
  element,
  onInteraction,
}: CodeEditorElementProps) {
  const initialCode = (element.code as string) ?? "";
  const language = (element.language as string) ?? "";
  const editable = (element.editable as boolean) ?? false;
  const showLineNumbers = (element.lineNumbers as boolean) ?? true;

  const [code, setCode] = useState(initialCode);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const preRef = useRef<HTMLPreElement>(null);

  const lines = code.split("\n");

  // Sync scroll between textarea and pre
  const handleScroll = useCallback(() => {
    if (textareaRef.current && preRef.current) {
      preRef.current.scrollTop = textareaRef.current.scrollTop;
      preRef.current.scrollLeft = textareaRef.current.scrollLeft;
    }
  }, []);

  // Handle tab key for indentation
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Tab") {
        e.preventDefault();
        const textarea = e.currentTarget;
        const start = textarea.selectionStart;
        const end = textarea.selectionEnd;
        const newValue =
          code.substring(0, start) + "  " + code.substring(end);
        setCode(newValue);
        // Restore cursor position after React re-render
        requestAnimationFrame(() => {
          textarea.selectionStart = start + 2;
          textarea.selectionEnd = start + 2;
        });
      }

      // Ctrl/Cmd+Enter to submit
      if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
        onInteraction({
          type: "code_submit",
          element_id: element.id,
          code,
          language,
        });
      }
    },
    [code, element.id, language, onInteraction],
  );

  // Keep textarea sized properly
  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
      textareaRef.current.style.height =
        textareaRef.current.scrollHeight + "px";
    }
  }, [code]);

  const gutterWidth = showLineNumbers
    ? `${String(lines.length).length * 0.7 + 1.2}rem`
    : "0";

  return (
    <div className="relative rounded-md border border-gray-200 dark:border-gray-700 overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between bg-gray-50 px-3 py-1.5 dark:bg-gray-800">
        <div className="flex items-center gap-2">
          {language && (
            <Badge variant="outline" className="text-xs">
              {language}
            </Badge>
          )}
          {editable && (
            <Badge
              variant="secondary"
              className="text-xs"
            >
              editable
            </Badge>
          )}
        </div>
        {editable && (
          <button
            type="button"
            className="text-xs text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300"
            onClick={() =>
              onInteraction({
                type: "code_submit",
                element_id: element.id,
                code,
                language,
              })
            }
          >
            Submit (Ctrl+Enter)
          </button>
        )}
      </div>

      {/* Code area */}
      <div className="relative overflow-auto bg-gray-100 dark:bg-gray-900">
        <div className="flex">
          {/* Line number gutter */}
          {showLineNumbers && (
            <div
              className="shrink-0 select-none border-r border-gray-200 bg-gray-50 py-3 text-right dark:border-gray-700 dark:bg-gray-800/50"
              style={{ width: gutterWidth }}
            >
              {lines.map((_, i) => (
                <div
                  key={i}
                  className="px-2 text-xs leading-5 text-gray-400 dark:text-gray-600"
                >
                  {i + 1}
                </div>
              ))}
            </div>
          )}

          {/* Code content */}
          <div className="relative flex-1 min-w-0">
            <pre
              ref={preRef}
              className="overflow-auto p-3 text-sm leading-5 text-gray-800 dark:text-gray-300"
              aria-hidden={editable ? "true" : undefined}
            >
              <code>{code}</code>
            </pre>

            {/* Editable textarea overlay */}
            {editable && (
              <textarea
                ref={textareaRef}
                value={code}
                onChange={(e) => setCode(e.target.value)}
                onScroll={handleScroll}
                onKeyDown={handleKeyDown}
                spellCheck={false}
                className="absolute inset-0 w-full h-full resize-none bg-transparent p-3 font-mono text-sm leading-5 text-transparent caret-gray-800 outline-none dark:caret-gray-200"
                style={{
                  // Transparent text so the pre underneath shows through
                  // with styling, but the caret is visible
                  caretColor: "inherit",
                }}
              />
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
