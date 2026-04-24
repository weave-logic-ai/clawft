#!/usr/bin/env node
// These values should NEVER change. The values are precisely for
// generating ULIDs.
const ENCODING = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"; // Crockford's Base32
const ENCODING_LEN = 32; // from ENCODING.length;
const RANDOM_LEN = 16;
const TIME_LEN = 10;
const TIME_MAX = 281474976710655; // from Math.pow(2, 48) - 1;

var ULIDErrorCode;
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
})(ULIDErrorCode || (ULIDErrorCode = {}));
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
function incrementBase32(str) {
    let done = undefined, index = str.length, char, charIndex, output = str;
    const maxCharIndex = ENCODING_LEN - 1;
    while (!done && index-- >= 0) {
        char = output[index];
        charIndex = ENCODING.indexOf(char);
        if (charIndex === -1) {
            throw new ULIDError(ULIDErrorCode.Base32IncorrectEncoding, "Incorrectly encoded string");
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
    throw new ULIDError(ULIDErrorCode.Base32IncorrectEncoding, "Failed incrementing string");
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
    throw new ULIDError(ULIDErrorCode.PRNGDetectFailure, "Failed to find a reliable PRNG");
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
        throw new ULIDError(ULIDErrorCode.EncodeTimeValueMalformed, `Time must be a number: ${now}`);
    }
    else if (now > TIME_MAX) {
        throw new ULIDError(ULIDErrorCode.EncodeTimeSizeExceeded, `Cannot encode a time larger than ${TIME_MAX}: ${now}`);
    }
    else if (now < 0) {
        throw new ULIDError(ULIDErrorCode.EncodeTimeNegative, `Time must be positive: ${now}`);
    }
    else if (Number.isInteger(now) === false) {
        throw new ULIDError(ULIDErrorCode.EncodeTimeValueMalformed, `Time must be an integer: ${now}`);
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
 * Create a ULID factory to generate monotonically-increasing
 *  ULIDs
 * @param prng The PRNG to use
 * @returns A ulid factory
 * @example
 *  const ulid = monotonicFactory();
 *  ulid(); // "01HNZXD07M5CEN5XA66EMZSRZW"
 */
function monotonicFactory(prng) {
    const currentPRNG = detectPRNG();
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

function parseArgs(args) {
    const output = {};
    while (args.length > 0) {
        const arg = args.shift();
        if (/^\-\-/.test(arg)) {
            if (/=/.test(arg)) {
                const [key, value] = arg.split("=");
                output[key.substring(2)] = value;
            }
            else {
                const value = args.shift();
                if (/^-/.test(value)) {
                    args.unshift(value);
                }
                else if (!value) {
                    output[arg.substring(2)] = true;
                }
                else {
                    output[arg.substring(2)] = value;
                }
            }
        }
    }
    return output;
}
const argv = parseArgs(process.argv.slice(2));
const count = /^\d+/.test(argv["count"]) ? parseInt(argv["count"], 10) : 1;
const factory = monotonicFactory();
for (let i = 0; i < count; i += 1) {
    console.log(factory());
}
