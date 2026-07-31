/**
 * WEFT-231: TTS word-highlighting surface.
 *
 * When `activeWordIndex` is a non-negative number, the matching word span
 * receives `.tts-word--active` (and prior words `.tts-word--spoken`).
 * When timings / boundary events are unavailable (`activeWordIndex == null`
 * or `< 0`), the component is a graceful no-op — plain text, no highlight
 * classes beyond the inert `.tts-word` wrappers (still useful for CSS hooks).
 */

import { useMemo, type ReactNode } from "react";
import { tokenizeWords } from "../../lib/voice-highlight";
import { cn } from "../../lib/utils";

export interface HighlightedSpeechProps {
  text: string;
  /**
   * Zero-based index of the currently spoken word.
   * `null` / negative → no active highlight (graceful no-op).
   */
  activeWordIndex?: number | null;
  className?: string;
  label?: string;
  /**
   * `overlay` = talk-mode dark backdrop (white text).
   * `panel` = light/dark themed card on the Voice page.
   */
  variant?: "overlay" | "panel";
}

export function HighlightedSpeech({
  text,
  activeWordIndex = null,
  className,
  label = "Assistant",
  variant = "overlay",
}: HighlightedSpeechProps) {
  const spans = useMemo(() => tokenizeWords(text), [text]);
  const highlight =
    activeWordIndex !== null &&
    activeWordIndex !== undefined &&
    activeWordIndex >= 0;
  const isPanel = variant === "panel";

  if (!text) return null;

  const shellClass = cn(
    "mb-8 max-w-lg rounded-lg px-6 py-3",
    isPanel ? "bg-transparent" : "bg-blue-500/10",
    className,
  );
  const labelClass = cn(
    "text-xs font-medium uppercase mb-1",
    isPanel
      ? "text-blue-600 dark:text-blue-300"
      : "text-blue-300",
  );
  const bodyClass = cn(
    "text-sm leading-relaxed",
    isPanel
      ? "text-gray-900 dark:text-gray-100"
      : "text-white",
  );

  // No word tokens (e.g. punctuation-only) — render plain.
  if (spans.length === 0) {
    return (
      <div
        className={shellClass}
        data-testid="highlighted-speech"
        data-highlight="false"
      >
        {label && <p className={labelClass}>{label}</p>}
        <p className={bodyClass}>{text}</p>
      </div>
    );
  }

  // Rebuild text with per-word spans, preserving whitespace between tokens.
  const nodes: ReactNode[] = [];
  let cursor = 0;
  for (const span of spans) {
    if (span.start > cursor) {
      nodes.push(
        <span key={`ws-${cursor}`} className="tts-ws">
          {text.slice(cursor, span.start)}
        </span>,
      );
    }
    const isActive = highlight && span.index === activeWordIndex;
    const isSpoken = highlight && span.index < (activeWordIndex as number);
    nodes.push(
      <span
        key={`w-${span.index}`}
        className={cn(
          "tts-word",
          isActive && "tts-word--active",
          isSpoken && "tts-word--spoken",
        )}
        data-word-index={span.index}
        data-active={isActive ? "true" : "false"}
      >
        {span.word}
      </span>,
    );
    cursor = span.end;
  }
  if (cursor < text.length) {
    nodes.push(
      <span key={`ws-tail`} className="tts-ws">
        {text.slice(cursor)}
      </span>,
    );
  }

  return (
    <div
      className={shellClass}
      data-testid="highlighted-speech"
      data-highlight={highlight ? "true" : "false"}
      data-active-word={highlight ? String(activeWordIndex) : undefined}
      aria-live="polite"
    >
      {label && <p className={labelClass}>{label}</p>}
      <p className={bodyClass}>{nodes}</p>
    </div>
  );
}
