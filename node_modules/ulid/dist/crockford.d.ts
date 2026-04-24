export declare function crockfordEncode(input: Uint8Array): string;
export declare function crockfordDecode(input: string): Uint8Array;
/**
 * Fix a ULID's Base32 encoding -
 * i and l (case-insensitive) will be treated as 1 and o (case-insensitive) will be treated as 0.
 * hyphens are ignored during decoding.
 * @param id The ULID
 * @returns The cleaned up ULID
 */
export declare function fixULIDBase32(id: string): string;
export declare function incrementBase32(str: string): string;
