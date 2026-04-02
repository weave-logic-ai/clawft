/**
 * RVF Binary Reader for browser.
 *
 * Reads `.rvf` files produced by build-kb and extracts segments.
 * Each segment has a 64-byte header followed by a payload padded to
 * a 64-byte boundary.
 *
 * Wire format (little-endian):
 *   [0..4]   magic      u32   0x53465652 ("RVFS" LE)
 *   [4]      version    u8    1
 *   [5]      seg_type   u8    (0x01=Vec, 0x07=Meta, ...)
 *   [6..8]   flags      u16
 *   [8..16]  segment_id u64
 *   [16..24] payload_length u64
 *   [24..32] timestamp_ns   u64
 *   [32]     checksum_algo  u8
 *   [33]     compression    u8
 *   [34..36] reserved_0     u16
 *   [36..40] reserved_1     u32
 *   [40..56] content_hash   16 bytes
 *   [56..60] uncompressed_len u32
 *   [60..64] alignment_pad  u32
 */

// Matches SEGMENT_MAGIC from rvf-types: 0x5256_4653
// Written to file as to_le_bytes() => [0x53, 0x46, 0x56, 0x52]
// Read back with getUint32(offset, true) => 0x52564653
const SEGMENT_MAGIC = 0x52564653;
const SEGMENT_HEADER_SIZE = 64;

// Segment type discriminants (from rvf-types SegmentType enum)
const SEG_TYPE_VEC = 0x01;
const SEG_TYPE_META = 0x07;

export interface RvfSegment {
  segType: number;
  segmentId: number;
  flags: number;
  payload: Uint8Array;
}

export interface KBManifest {
  agent_id: string;
  namespace: string;
  segment_count: number;
  dimension: number;
  embedder_name: string;
  created_at: string;
  version: number;
}

export interface KBEntry {
  id: string;
  text: string;
  embedding: Float32Array;
  metadata: Record<string, unknown>;
  tags: string[];
  namespace: string;
  dimension: number;
  embedder_name: string;
}

/**
 * Parse all RVF segments from a binary buffer.
 *
 * Iterates through concatenated segments, reading the 64-byte header
 * to find each payload, then advancing by header + payload + alignment
 * padding.
 */
export function readRvfSegments(buffer: ArrayBuffer): RvfSegment[] {
  const view = new DataView(buffer);
  const segments: RvfSegment[] = [];
  let offset = 0;

  while (offset + SEGMENT_HEADER_SIZE <= buffer.byteLength) {
    const magic = view.getUint32(offset, true);
    if (magic !== SEGMENT_MAGIC) {
      // Not a valid segment header -- stop parsing.
      break;
    }

    const segType = view.getUint8(offset + 5);
    const flags = view.getUint16(offset + 6, true);
    // Read segment_id as two 32-bit halves (avoids BigInt for portability).
    const segIdLo = view.getUint32(offset + 8, true);
    const segIdHi = view.getUint32(offset + 12, true);
    const segmentId = segIdLo + segIdHi * 0x100000000;

    // payload_length as two 32-bit halves
    const payloadLenLo = view.getUint32(offset + 16, true);
    const payloadLenHi = view.getUint32(offset + 20, true);
    const payloadLength = payloadLenLo + payloadLenHi * 0x100000000;

    const alignmentPad = view.getUint32(offset + 60, true);

    const payloadStart = offset + SEGMENT_HEADER_SIZE;
    const payload = new Uint8Array(buffer, payloadStart, payloadLength);

    segments.push({ segType, segmentId, flags, payload });

    // Advance: header + payload + padding to next 64-byte boundary
    offset += SEGMENT_HEADER_SIZE + payloadLength + alignmentPad;
  }

  return segments;
}

/**
 * Parse a complete knowledge base from an RVF buffer.
 *
 * Requires the `cbor-x` package (or any CBOR decoder that exposes
 * `decode(Uint8Array)`). Pass the decode function to avoid a hard
 * dependency.
 *
 * Usage:
 * ```ts
 * import { decode } from 'cbor-x';
 * const { manifest, entries } = parseKnowledgeBase(buffer, decode);
 * ```
 */
export function parseKnowledgeBase(
  buffer: ArrayBuffer,
  cborDecode: (data: Uint8Array) => unknown,
): { manifest: KBManifest; entries: KBEntry[] } {
  const segments = readRvfSegments(buffer);

  let manifest: KBManifest | null = null;
  const entries: KBEntry[] = [];

  for (const seg of segments) {
    const decoded = cborDecode(seg.payload) as Record<string, unknown>;

    if (seg.segType === SEG_TYPE_META && seg.segmentId === 0) {
      // Manifest segment
      manifest = decoded as unknown as KBManifest;
    } else if (seg.segType === SEG_TYPE_VEC) {
      // Vec segment -- document chunk
      const rawEmbedding = decoded.embedding;
      let embedding: Float32Array;
      if (rawEmbedding instanceof Uint8Array || rawEmbedding instanceof ArrayBuffer) {
        // Raw bytes: reinterpret as f32
        const buf = rawEmbedding instanceof ArrayBuffer ? rawEmbedding : rawEmbedding.buffer;
        embedding = new Float32Array(buf);
      } else if (Array.isArray(rawEmbedding)) {
        embedding = new Float32Array(rawEmbedding as number[]);
      } else {
        embedding = new Float32Array(0);
      }

      entries.push({
        id: decoded.id as string,
        text: decoded.text as string,
        embedding,
        metadata: decoded.metadata as Record<string, unknown>,
        tags: decoded.tags as string[],
        namespace: decoded.namespace as string,
        dimension: decoded.dimension as number,
        embedder_name: decoded.embedder_name as string,
      });
    }
  }

  if (!manifest) {
    throw new Error("No manifest segment (Meta/0x07, id=0) found in RVF file");
  }

  return { manifest, entries };
}
