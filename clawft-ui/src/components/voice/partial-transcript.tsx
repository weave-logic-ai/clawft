/**
 * WEFT-231: streaming partial-transcription display.
 *
 * Final (committed) STT text is solid; live interim text is muted + italic
 * so the user sees STT progress without mistaking it for a finalized turn.
 */

import { mergeTranscriptDisplay } from "../../lib/voice-highlight";
import { cn } from "../../lib/utils";

export interface PartialTranscriptViewProps {
  /** Committed / final STT text for the current utterance. */
  finalText: string;
  /** Live interim (non-final) STT hypothesis. */
  interimText?: string;
  className?: string;
  /** Optional label above the transcript. */
  label?: string;
  /**
   * `overlay` = talk-mode dark backdrop (white text).
   * `panel` = light/dark themed card on the Voice page.
   */
  variant?: "overlay" | "panel";
}

export function PartialTranscriptView({
  finalText,
  interimText = "",
  className,
  label = "You said",
  variant = "overlay",
}: PartialTranscriptViewProps) {
  const display = mergeTranscriptDisplay(finalText, interimText);
  if (!display) return null;

  const hasInterim = interimText.trim().length > 0;
  const isPanel = variant === "panel";

  return (
    <div
      className={cn(
        "mb-4 max-w-lg rounded-lg px-6 py-3",
        isPanel ? "bg-transparent" : "bg-white/10",
        className,
      )}
      data-testid="partial-transcript"
      data-has-interim={hasInterim ? "true" : "false"}
      aria-live="polite"
      aria-atomic="false"
    >
      <p
        className={cn(
          "text-xs font-medium uppercase mb-1",
          isPanel
            ? "text-gray-500 dark:text-gray-400"
            : "text-gray-400",
        )}
      >
        {label}
        {hasInterim && (
          <span className="ml-2 normal-case tracking-normal text-gray-500">
            · listening
          </span>
        )}
      </p>
      <p
        className={cn(
          "text-sm",
          isPanel
            ? "text-gray-900 dark:text-gray-100"
            : "text-white",
        )}
      >
        {finalText.trim() && (
          <span className="tts-final-text">{finalText.trimEnd()}</span>
        )}
        {hasInterim && (
          <>
            {finalText.trim() && !/\s$/.test(finalText) ? " " : null}
            <span
              className={cn(
                "partial-transcript-interim italic",
                isPanel
                  ? "text-gray-500 dark:text-gray-400"
                  : "text-white/60",
              )}
              data-testid="partial-transcript-interim"
            >
              {interimText.trimStart()}
            </span>
          </>
        )}
      </p>
    </div>
  );
}
