/**
 * Unit tests for WEFT-231 partial-transcript / TTS highlight helpers.
 *
 *   node --experimental-strip-types --test clawft-ui/src/lib/voice-highlight.test.ts
 */

import { describe, it } from "node:test";
import { strict as assert } from "node:assert";
import {
  mergeTranscriptDisplay,
  parsePartialTranscriptPayload,
  tokenizeWords,
  wordIndexAtChar,
  wordIndexAtElapsed,
  VOICE_PARTIAL_TOPIC,
} from "./voice-highlight.ts";

describe("tokenizeWords", () => {
  it("returns empty for blank input", () => {
    assert.deepEqual(tokenizeWords(""), []);
    assert.deepEqual(tokenizeWords("   "), []);
  });

  it("preserves character offsets", () => {
    const spans = tokenizeWords("Hello world");
    assert.equal(spans.length, 2);
    assert.equal(spans[0].word, "Hello");
    assert.equal(spans[0].start, 0);
    assert.equal(spans[0].end, 5);
    assert.equal(spans[0].index, 0);
    assert.equal(spans[1].word, "world");
    assert.equal(spans[1].start, 6);
    assert.equal(spans[1].end, 11);
    assert.equal(spans[1].index, 1);
  });
});

describe("wordIndexAtChar", () => {
  const spans = tokenizeWords("one two three");

  it("maps mid-word char to that word", () => {
    assert.equal(wordIndexAtChar(spans, 0), 0);
    assert.equal(wordIndexAtChar(spans, 4), 1); // 't' of two
    assert.equal(wordIndexAtChar(spans, 8), 2);
  });

  it("returns -1 for empty / negative", () => {
    assert.equal(wordIndexAtChar([], 0), -1);
    assert.equal(wordIndexAtChar(spans, -1), -1);
  });
});

describe("wordIndexAtElapsed", () => {
  const timings = [
    { word: "a", startMs: 0, endMs: 100 },
    { word: "b", startMs: 100, endMs: 200 },
    { word: "c", startMs: 200, endMs: 300 },
  ];

  it("returns null when no timings (graceful no-op)", () => {
    assert.equal(wordIndexAtElapsed([], 50), null);
  });

  it("selects the active word by elapsed ms", () => {
    assert.equal(wordIndexAtElapsed(timings, 0), 0);
    assert.equal(wordIndexAtElapsed(timings, 150), 1);
    assert.equal(wordIndexAtElapsed(timings, 250), 2);
    assert.equal(wordIndexAtElapsed(timings, 999), 2);
  });
});

describe("mergeTranscriptDisplay", () => {
  it("joins final + interim with a space when needed", () => {
    assert.equal(mergeTranscriptDisplay("hello", "world"), "hello world");
    assert.equal(mergeTranscriptDisplay("hello ", "world"), "hello world");
    assert.equal(mergeTranscriptDisplay("", "partial"), "partial");
    assert.equal(mergeTranscriptDisplay("final", ""), "final");
  });
});

describe("parsePartialTranscriptPayload", () => {
  it("parses voice:partial shape", () => {
    assert.deepEqual(parsePartialTranscriptPayload({ text: "hi", isFinal: false }), {
      text: "hi",
      isFinal: false,
    });
  });

  it("parses voice:status-style isPartial / transcript", () => {
    assert.deepEqual(
      parsePartialTranscriptPayload({ transcript: "yo", isPartial: true }),
      { text: "yo", isFinal: false },
    );
  });

  it("returns null for garbage", () => {
    assert.equal(parsePartialTranscriptPayload(null), null);
    assert.equal(parsePartialTranscriptPayload({}), null);
    assert.equal(parsePartialTranscriptPayload("x"), null);
  });
});

describe("VOICE_PARTIAL_TOPIC", () => {
  it("is the dedicated WS topic", () => {
    assert.equal(VOICE_PARTIAL_TOPIC, "voice:partial");
  });
});
