import { UUID } from "crypto";
import { ULID } from "./types.js";
/**
 * Convert a ULID to a UUID
 * @param ulid The ULID to convert
 * @returns A UUID string
 */
export declare function ulidToUUID(ulid: ULID): UUID;
/**
 * Convert a UUID to a ULID
 * @param uuid The UUID to convert
 * @returns A ULID string
 */
export declare function uuidToULID(uuid: string): ULID;
