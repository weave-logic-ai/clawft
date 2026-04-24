'use strict';

// These values should NEVER change. The values are precisely for
// generating ULIDs.
const B32_CHARACTERS = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const ENCODING = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"; // Crockford's Base32
const ENCODING_LEN = 32; // from ENCODING.length;
const MAX_ULID = "7ZZZZZZZZZZZZZZZZZZZZZZZZZ";
const MIN_ULID = "00000000000000000000000000";
const RANDOM_LEN = 16;
const TIME_LEN = 10;
const TIME_MAX = 281474976710655; // from Math.pow(2, 48) - 1;
const ULID_REGEX = /^[0-7][0-9a-hjkmnp-tv-zA-HJKMNP-TV-Z]{25}$/;
const UUID_REGEX = /^[0-9a-fA-F]{8}-(?:[0-9a-fA-F]{4}-){3}[0-9a-fA-F]{12}$/;

exports.ULIDErrorCode = void 0;
(function (ULIDErrorCode) {
    ULIDErrorCode["Base32IncorrectEncoding"] = "B32_ENC_INVALID";
    ULIDErrorCode["DecodeTimeInvalidCharacter"] = "DEC_TIME_CHAR";
    ULIDErrorCode["DecodeTimeValueMalformed"] = "DEC_TIME_MALFORMED";
    ULIDErrorCode["EncodeTimeNegative"] = "ENC_TIME_NEG";
    ULIDErrorCode["EncodeTimeSizeExceeded"] = "ENC_TIME_SIZE_EXCEED";
    ULIDErrorCode["EncodeTimeValueMalformed"] = "ENC_TIME_MALFORMED";
    ULIDErrorCode["PRNGDetectFailure"] = "PRNG_DETECT";
    ULIDErrorCode["ULIDInvalid"] = "ULID_INVALID";
    ULIDErrorCode["Unexpected"] = "UNEXPECTED";
    ULIDErrorCode["UUIDInvalid"] = "UUID_INVALID";
})(exports.ULIDErrorCode || (exports.ULIDErrorCode = {}));
class ULIDError extends Error {
    constructor(errorCode, message) {
        super(`${message} (${errorCode})`);
        this.name = "ULIDError";
        this.code = errorCode;
    }
}

function randomChar(prng) {
    // Currently PRNGs generate fractions from 0 to _less than_ 1, so no "%" is necessary.
    // However, just in case a future PRNG can generate 1,
    // we are applying "% ENCODING LEN" to wrap back to the first character
    const randomPosition = Math.floor(prng() * ENCODING_LEN) % ENCODING_LEN;
    return ENCODING.charAt(randomPosition);
}
function replaceCharAt(str, index, char) {
    if (index > str.length - 1) {
        return str;
    }
    return str.substr(0, index) + char + str.substr(index + 1);
}

// Code from https://github.com/devbanana/crockford-base32/blob/develop/src/index.ts
function crockfordEncode(input) {
    const output = [];
    let bitsRead = 0;
    let buffer = 0;
    const reversedInput = new Uint8Array(input.slice().reverse());
    for (const byte of reversedInput) {
        buffer |= byte << bitsRead;
        bitsRead += 8;
        while (bitsRead >= 5) {
            output.unshift(buffer & 0x1f);
            buffer >>>= 5;
            bitsRead -= 5;
        }
    }
    if (bitsRead > 0) {
        output.unshift(buffer & 0x1f);
    }
    return output.map(byte => B32_CHARACTERS.charAt(byte)).join("");
}
function crockfordDecode(input) {
    const sanitizedInput = input.toUpperCase().split("").reverse().join("");
    const output = [];
    let bitsRead = 0;
    let buffer = 0;
    for (const character of sanitizedInput) {
        const byte = B32_CHARACTERS.indexOf(character);
        if (byte === -1) {
            throw new Error(`Invalid base 32 character found in string: ${character}`);
        }
        buffer |= byte << bitsRead;
        bitsRead += 5;
        while (bitsRead >= 8) {
            output.unshift(buffer & 0xff);
            buffer >>>= 8;
            bitsRead -= 8;
        }
    }
    if (bitsRead >= 5 || buffer > 0) {
        output.unshift(buffer & 0xff);
    }
    return new Uint8Array(output);
}
/**
 * Fix a ULID's Base32 encoding -
 * i and l (case-insensitive) will be treated as 1 and o (case-insensitive) will be treated as 0.
 * hyphens are ignored during decoding.
 * @param id The ULID
 * @returns The cleaned up ULID
 */
function fixULIDBase32(id) {
    return id.replace(/i/gi, "1").replace(/l/gi, "1").replace(/o/gi, "0").replace(/-/g, "");
}
function incrementBase32(str) {
    let done = undefined, index = str.length, char, charIndex, output = str;
    const maxCharIndex = ENCODING_LEN - 1;
    while (!done && index-- >= 0) {
        char = output[index];
        charIndex = ENCODING.indexOf(char);
        if (charIndex === -1) {
            throw new ULIDError(exports.ULIDErrorCode.Base32IncorrectEncoding, "Incorrectly encoded string");
        }
        if (charIndex === maxCharIndex) {
            output = replaceCharAt(output, index, ENCODING[0]);
            continue;
        }
        done = replaceCharAt(output, index, ENCODING[charIndex + 1]);
    }
    if (typeof done === "string") {
        return done;
    }
    throw new ULIDError(exports.ULIDErrorCode.Base32IncorrectEncoding, "Failed incrementing string");
}

/**
 * Decode time from a ULID
 * @param id The ULID
 * @returns The decoded timestamp
 */
function decodeTime(id) {
    if (id.length !== TIME_LEN + RANDOM_LEN) {
        throw new ULIDError(exports.ULIDErrorCode.DecodeTimeValueMalformed, "Malformed ULID");
    }
    const time = id
        .substr(0, TIME_LEN)
        .toUpperCase()
        .split("")
        .reverse()
        .reduce((carry, char, index) => {
        const encodingIndex = ENCODING.indexOf(char);
        if (encodingIndex === -1) {
            throw new ULIDError(exports.ULIDErrorCode.DecodeTimeInvalidCharacter, `Time decode error: Invalid character: ${char}`);
        }
        return (carry += encodingIndex * Math.pow(ENCODING_LEN, index));
    }, 0);
    if (time > TIME_MAX) {
        throw new ULIDError(exports.ULIDErrorCode.DecodeTimeValueMalformed, `Malformed ULID: timestamp too large: ${time}`);
    }
    return time;
}
/**
 * Detect the best PRNG (pseudo-random number generator)
 * @param root The root to check from (global/window)
 * @returns The PRNG function
 */
function detectPRNG(root) {
    const rootLookup = detectRoot();
    const globalCrypto = (rootLookup && (rootLookup.crypto || rootLookup.msCrypto)) ||
        (null);
    if (typeof globalCrypto?.getRandomValues === "function") {
        return () => {
            const buffer = new Uint8Array(1);
            globalCrypto.getRandomValues(buffer);
            return buffer[0] / 256;
        };
    }
    else if (typeof globalCrypto?.randomBytes === "function") {
        return () => globalCrypto.randomBytes(1).readUInt8() / 256;
    }
    else ;
    throw new ULIDError(exports.ULIDErrorCode.PRNGDetectFailure, "Failed to find a reliable PRNG");
}
function detectRoot() {
    if (inWebWorker())
        return self;
    if (typeof window !== "undefined") {
        return window;
    }
    if (typeof global !== "undefined") {
        return global;
    }
    if (typeof globalThis !== "undefined") {
        return globalThis;
    }
    return null;
}
function encodeRandom(len, prng) {
    let str = "";
    for (; len > 0; len--) {
        str = randomChar(prng) + str;
    }
    return str;
}
/**
 * Encode the time portion of a ULID
 * @param now The current timestamp
 * @param len Length to generate
 * @returns The encoded time
 */
function encodeTime(now, len = TIME_LEN) {
    if (isNaN(now)) {
        throw new ULIDError(exports.ULIDErrorCode.EncodeTimeValueMalformed, `Time must be a number: ${now}`);
    }
    else if (now > TIME_MAX) {
        throw new ULIDError(exports.ULIDErrorCode.EncodeTimeSizeExceeded, `Cannot encode a time larger than ${TIME_MAX}: ${now}`);
    }
    else if (now < 0) {
        throw new ULIDError(exports.ULIDErrorCode.EncodeTimeNegative, `Time must be positive: ${now}`);
    }
    else if (Number.isInteger(now) === false) {
        throw new ULIDError(exports.ULIDErrorCode.EncodeTimeValueMalformed, `Time must be an integer: ${now}`);
    }
    let mod, str = "";
    for (let currentLen = len; currentLen > 0; currentLen--) {
        mod = now % ENCODING_LEN;
        str = ENCODING.charAt(mod) + str;
        now = (now - mod) / ENCODING_LEN;
    }
    return str;
}
function inWebWorker() {
    // @ts-ignore
    return typeof WorkerGlobalScope !== "undefined" && self instanceof WorkerGlobalScope;
}
/**
 * Check if a ULID is valid
 * @param id The ULID to test
 * @returns True if valid, false otherwise
 * @example
 *   isValid("01HNZX8JGFACFA36RBXDHEQN6E"); // true
 *   isValid(""); // false
 */
function isValid(id) {
    return (typeof id === "string" &&
        id.length === TIME_LEN + RANDOM_LEN &&
        id
            .toUpperCase()
            .split("")
            .every(char => ENCODING.indexOf(char) !== -1));
}
/**
 * Create a ULID factory to generate monotonically-increasing
 *  ULIDs
 * @param prng The PRNG to use
 * @returns A ulid factory
 * @example
 *  const ulid = monotonicFactory();
 *  ulid(); // "01HNZXD07M5CEN5XA66EMZSRZW"
 */
function monotonicFactory(prng) {
    const currentPRNG = prng || detectPRNG();
    let lastTime = 0, lastRandom;
    return function _ulid(seedTime) {
        const seed = !seedTime || isNaN(seedTime) ? Date.now() : seedTime;
        if (seed <= lastTime) {
            const incrementedRandom = (lastRandom = incrementBase32(lastRandom));
            return encodeTime(lastTime, TIME_LEN) + incrementedRandom;
        }
        lastTime = seed;
        const newRandom = (lastRandom = encodeRandom(RANDOM_LEN, currentPRNG));
        return encodeTime(seed, TIME_LEN) + newRandom;
    };
}
/**
 * Generate a ULID
 * @param seedTime Optional time seed
 * @param prng Optional PRNG function
 * @returns A ULID string
 * @example
 *  ulid(); // "01HNZXD07M5CEN5XA66EMZSRZW"
 */
function ulid(seedTime, prng) {
    const currentPRNG = prng || detectPRNG();
    const seed = !seedTime || isNaN(seedTime) ? Date.now() : seedTime;
    return encodeTime(seed, TIME_LEN) + encodeRandom(RANDOM_LEN, currentPRNG);
}

/**
 * Convert a ULID to a UUID
 * @param ulid The ULID to convert
 * @returns A UUID string
 */
function ulidToUUID(ulid) {
    const isValid = ULID_REGEX.test(ulid);
    if (!isValid) {
        throw new ULIDError(exports.ULIDErrorCode.ULIDInvalid, `Invalid ULID: ${ulid}`);
    }
    const uint8Array = crockfordDecode(ulid);
    let uuid = Array.from(uint8Array)
        .map(byte => byte.toString(16).padStart(2, "0"))
        .join("");
    uuid =
        uuid.substring(0, 8) +
            "-" +
            uuid.substring(8, 12) +
            "-" +
            uuid.substring(12, 16) +
            "-" +
            uuid.substring(16, 20) +
            "-" +
            uuid.substring(20);
    return uuid.toUpperCase();
}
/**
 * Convert a UUID to a ULID
 * @param uuid The UUID to convert
 * @returns A ULID string
 */
function uuidToULID(uuid) {
    const isValid = UUID_REGEX.test(uuid);
    if (!isValid) {
        throw new ULIDError(exports.ULIDErrorCode.UUIDInvalid, `Invalid UUID: ${uuid}`);
    }
    const bytes = uuid.replace(/-/g, "").match(/.{1,2}/g);
    if (!bytes) {
        throw new ULIDError(exports.ULIDErrorCode.Unexpected, `Failed parsing UUID bytes: ${uuid}`);
    }
    const uint8Array = new Uint8Array(bytes.map(byte => parseInt(byte, 16)));
    return crockfordEncode(uint8Array);
}

exports.MAX_ULID = MAX_ULID;
exports.MIN_ULID = MIN_ULID;
exports.TIME_LEN = TIME_LEN;
exports.TIME_MAX = TIME_MAX;
exports.ULIDError = ULIDError;
exports.decodeTime = decodeTime;
exports.encodeTime = encodeTime;
exports.fixULIDBase32 = fixULIDBase32;
exports.incrementBase32 = incrementBase32;
exports.isValid = isValid;
exports.monotonicFactory = monotonicFactory;
exports.ulid = ulid;
exports.ulidToUUID = ulidToUUID;
exports.uuidToULID = uuidToULID;
