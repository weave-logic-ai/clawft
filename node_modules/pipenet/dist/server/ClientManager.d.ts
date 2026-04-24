import type { TunnelServer } from './TunnelServer.js';
import { Client } from './Client.js';
export interface ClientManagerOptions {
    maxTcpSockets?: number;
    onTunnelClosed?: (tunnel: TunnelInfo) => void;
    onTunnelCreated?: (tunnel: TunnelInfo) => void;
    tunnelServer?: TunnelServer;
}
export interface NewClientInfo {
    domain: string;
    id: string;
    maxConnCount?: number;
    port: number;
    url: string;
}
export interface TunnelInfo {
    domain: string;
    id: string;
    url: string;
}
export declare class ClientManager {
    stats: {
        tunnels: number;
    };
    private clients;
    private clientTunnelInfo;
    private log;
    private opt;
    constructor(opt?: ClientManagerOptions);
    getClient(id: string): Client | undefined;
    hasClient(id: string): boolean;
    newClient(requestedId: string | undefined, url: string, domain: string): Promise<NewClientInfo>;
    removeClient(id: string): void;
}
//# sourceMappingURL=ClientManager.d.ts.map