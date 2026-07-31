/**
 * WEFT-231: partial-transcript streaming helpers + TTS word-highlight math.
 *
 * Pure functions — no DOM / Web Speech — so they unit-test under `node --test`.
 */

// ── Word tokenization (for highlight surfaces) ──────────────────

/** A word span within a TTS utterance, with character offsets. */
export interface WordSpan {
  /** The word token (no trailing space). */
  word: string;
  /** Inclusive start character index in the source text. */
  start: number;
  /** Exclusive end character index. */
  end: number;
  /** Zero-based word index. */
  index: number;
}

/**
 * Optional absolute timings for a word (ms from audio start).
 * When absent, the UI falls back to Web Speech `boundary` events or a no-op.
 */
export interface WordTiming {
  word: string;
  startMs: number;
  endMs: number;
}

/** Callback payload for a spoken-word boundary. */
export interface WordBoundaryInfo {
  /** Zero-based word index in {@link tokenizeWords} order, or -1 if unknown. */
  wordIndex: number;
  /** Character offset into the utterance (SpeechSynthesis / timing source). */
  charIndex: number;
  /** Boundary name when known (`word` | `sentence`); empty otherwise. */
  name: string;
  /** The word token if resolvable. */
  word: string;
}

/**
 * Split text into word spans (whitespace-delimited, offsets preserved).
 *
 * Empty / whitespace-only input → `[]`.
 */
export function tokenizeWords(text: string): WordSpan[] {
  const spans: WordSpan[] = [];
  if (!text) return spans;

  const re = /\S+/g;
  let match: RegExpExecArray | null;
  let index = 0;
  while ((match = re.exec(text)) !== null) {
    spans.push({
      word: match[0],
      start: match.index,
      end: match.index + match[0].length,
      index,
    });
    index += 1;
  }
  return spans;
}

/**
 * Map a character index (e.g. SpeechSynthesisUtterance boundary) to a word index.
 * Returns -1 when out of range or spans are empty.
 */
export function wordIndexAtChar(spans: WordSpan[], charIndex: number): number {
  if (!spans.length || charIndex < 0) return -1;
  for (let i = spans.length - 1; i >= 0; i--) {
    if (charIndex >= spans[i].start) return spans[i].index;
  }
  return -1;
}

/**
 * Map elapsed playback ms to a word index using absolute timings.
 * Returns `null` when timings are empty (graceful no-op for the highlight surface).
 * Returns -1 when elapsed is before the first word; last index after the last end.
 */
export function wordIndexAtElapsed(
  timings: WordTiming[],
  elapsedMs: number,
): number | null {
  if (!timings.length) return null;
  if (elapsedMs < timings[0].startMs) return -1;
  for (let i = timings.length - 1; i >= 0; i--) {
    if (elapsedMs >= timings[i].startMs) return i;
  }
  return -1;
}

// ── Partial transcript merge / parse ────────────────────────────

/** Display merge of committed final + live interim STT text. */
export function mergeTranscriptDisplay(
  finalText: string,
  interimText: string,
): string {
  const f = finalText.trimEnd();
  const i = interimText.trimStart();
  if (!f) return i;
  if (!i) return f;
  // Prefer a single space between final and interim when neither provides it.
  const needsSpace = !/\s$/.test(f) && !/^\s/.test(i);
  return needsSpace ? `${f} ${i}` : `${f}${i}`;
}

/**
 * Parse a WS / REST partial-transcript payload.
 *
 * Accepts shapes used by `voice:partial` and the optional fields on
 * `voice:status`:
 * - `{ text, isFinal? }`
 * - `{ transcript, isPartial? }`  (isPartial=true ⇒ interim)
 */
export function parsePartialTranscriptPayload(
  data: unknown,
): { text: string; isFinal: boolean } | null {
  if (!data || typeof data !== "object") return null;
  const obj = data as Record<string, unknown>;

  const textRaw =
    typeof obj.text === "string"
      ? obj.text
      : typeof obj.transcript === "string"
        ? obj.transcript
        : null;
  if (textRaw === null) return null;

  let isFinal = true;
  if (typeof obj.isFinal === "boolean") {
    isFinal = obj.isFinal;
  } else if (typeof obj.is_final === "boolean") {
    isFinal = obj.is_final;
  } else if (typeof obj.isPartial === "boolean") {
    isFinal = !obj.isPartial;
  } else if (typeof obj.is_partial === "boolean") {
    isFinal = !obj.is_partial;
  }

  return { text: textRaw, isFinal };
}

/** Topic name for dedicated partial-transcript WS fan-out (WEFT-231). */
export const VOICE_PARTIAL_TOPIC = "voice:partial";
