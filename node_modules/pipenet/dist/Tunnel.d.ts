import { EventEmitter } from 'events';
import { TunnelCluster } from './TunnelCluster.js';
export interface TunnelOptions {
    allowInvalidCert?: boolean;
    headers?: Record<string, string>;
    host?: string;
    localCa?: string;
    localCert?: string;
    localHost?: string;
    localHttps?: boolean;
    localKey?: string;
    port?: number;
    subdomain?: string;
}
export declare class Tunnel extends EventEmitter {
    cachedUrl?: string;
    clientId?: string;
    closed: boolean;
    opts: TunnelOptions;
    tunnelCluster?: TunnelCluster;
    url?: string;
    constructor(opts?: TunnelOptions);
    close(): void;
    open(cb: (err?: Error) => void): void;
    private _establish;
    private _getInfo;
    private _init;
}
//# sourceMappingURL=Tunnel.d.ts.map