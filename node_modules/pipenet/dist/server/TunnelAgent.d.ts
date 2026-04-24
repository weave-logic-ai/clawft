import { Agent, ClientRequestArgs } from 'http';
import { Duplex } from 'stream';
import type { TunnelServer } from './TunnelServer.js';
export interface TunnelAgentListenInfo {
    port: number;
}
export interface TunnelAgentOptions {
    clientId?: string;
    maxTcpSockets?: number;
    tunnelServer?: TunnelServer;
}
export interface TunnelAgentStats {
    connectedSockets: number;
}
type CreateConnectionCallback = (err: Error | null, socket: Duplex) => void;
export declare class TunnelAgent extends Agent {
    started: boolean;
    private availableSockets;
    private clientId?;
    private closed;
    private connectedSockets;
    private log;
    private maxTcpSockets;
    private server?;
    private tunnelServer?;
    private waitingCreateConn;
    constructor(options?: TunnelAgentOptions);
    createConnection(options: ClientRequestArgs, cb?: CreateConnectionCallback): Duplex | null | undefined;
    destroy(): void;
    listen(): Promise<TunnelAgentListenInfo>;
    stats(): TunnelAgentStats;
    private _onClose;
    private _onConnection;
}
export {};
//# sourceMappingURL=TunnelAgent.d.ts.map