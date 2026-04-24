/**
 * Simple API key authentication for proxy
 */
export declare class AuthManager {
    private validKeys;
    constructor(apiKeys?: string[]);
    authenticate(headers: Record<string, string | string[] | undefined>): boolean;
    private extractApiKey;
    addKey(key: string): void;
    removeKey(key: string): void;
    hasKeys(): boolean;
}
//# sourceMappingURL=auth.d.ts.map