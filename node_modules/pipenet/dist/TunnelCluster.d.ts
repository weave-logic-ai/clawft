import { EventEmitter } from 'events';
export interface TunnelClusterOptions {
    allowInvalidCert?: boolean;
    cachedUrl?: string;
    localCa?: string;
    localCert?: string;
    localHost?: string;
    localHttps?: boolean;
    localKey?: string;
    localPort?: number;
    maxConn?: number;
    name?: string;
    remoteHost?: string;
    remoteIp?: string;
    remotePort?: number;
    sharedTunnel?: boolean;
    url?: string;
}
export interface TunnelRequest {
    method: string;
    path: string;
}
export declare class TunnelCluster extends EventEmitter {
    private opts;
    constructor(opts?: TunnelClusterOptions);
    open(): void;
}
//# sourceMappingURL=TunnelCluster.d.ts.map