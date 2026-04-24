export type Options = {
    hashLength?: number | undefined;
    timeCost?: number | undefined;
    memoryCost?: number | undefined;
    parallelism?: number | undefined;
    type?: 0 | 1 | 2 | undefined;
    version?: number | undefined;
    salt?: Buffer | undefined;
    associatedData?: Buffer | undefined;
    secret?: Buffer | undefined;
};
export const argon2d: 0;
export const argon2i: 1;
export const argon2id: 2;
/**
 * Hashes a password with Argon2, producing a raw hash
 *
 * @overload
 * @param {Buffer | string} password The plaintext password to be hashed
 * @param {Options & { raw: true }} options The parameters for Argon2
 * @returns {Promise<Buffer>} The raw hash generated from `password`
 */
export function hash(password: Buffer | string, options: Options & {
    raw: true;
}): Promise<Buffer>;
/**
 * Hashes a password with Argon2, producing an encoded hash
 *
 * @overload
 * @param {Buffer | string} password The plaintext password to be hashed
 * @param {Options & { raw?: boolean }} [options] The parameters for Argon2
 * @returns {Promise<string>} The encoded hash generated from `password`
 */
export function hash(password: Buffer | string, options?: (Options & {
    raw?: boolean;
}) | undefined): Promise<string>;
/**
 * @param {string} digest The digest to be checked
 * @param {Object} [options] The current parameters for Argon2
 * @param {number} [options.timeCost=3]
 * @param {number} [options.memoryCost=65536]
 * @param {number} [options.parallelism=4]
 * @param {number} [options.version=0x13]
 * @returns {boolean} `true` if the digest parameters do not match the parameters in `options`, otherwise `false`
 */
export function needsRehash(digest: string, options?: {
    timeCost?: number | undefined;
    memoryCost?: number | undefined;
    parallelism?: number | undefined;
    version?: number | undefined;
} | undefined): boolean;
/**
 * @param {string} digest The digest to be checked
 * @param {Buffer | string} password The plaintext password to be verified
 * @param {Object} [options] The current parameters for Argon2
 * @param {Buffer} [options.secret]
 * @returns {Promise<boolean>} `true` if the digest parameters matches the hash generated from `password`, otherwise `false`
 */
export function verify(digest: string, password: Buffer | string, options?: {
    secret?: Buffer | undefined;
} | undefined): Promise<boolean>;
//# sourceMappingURL=argon2.d.cts.map