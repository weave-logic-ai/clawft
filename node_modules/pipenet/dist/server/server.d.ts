import http from 'http';
import { TunnelServer } from './TunnelServer.js';
export interface PipenetServer extends http.Server {
    tunnelServer?: TunnelServer;
}
export interface RequestInfo {
    headers: Record<string, string | string[] | undefined>;
    method: string;
    path: string;
    remoteAddress?: string;
    tunnelId: string;
}
export interface ServerHooks {
    /**
     * Called when a request is proxied through a tunnel
     */
    onRequest?: (request: RequestInfo) => void;
    /**
     * Called when a tunnel is closed
     */
    onTunnelClosed?: (tunnel: TunnelInfo) => void;
    /**
     * Called when a new tunnel is created
     */
    onTunnelCreated?: (tunnel: TunnelInfo) => void;
}
export interface ServerOptions extends ServerHooks {
    domains?: string[];
    landing?: string;
    maxTcpSockets?: number;
    secure?: boolean;
    tunnelPort?: number;
}
export interface TunnelInfo {
    domain: string;
    id: string;
    url: string;
}
export declare function createServer(opt?: ServerOptions): PipenetServer;
//# sourceMappingURL=server.d.ts.map