import type { Duplex } from 'stream';
import { EventEmitter } from 'events';
import http from 'http';
import type { TunnelAgent } from './TunnelAgent.js';
export interface ClientOptions {
    agent: TunnelAgent;
    id?: string;
}
export declare class Client extends EventEmitter {
    id?: string;
    private agent;
    private graceTimeout;
    private log;
    constructor(options: ClientOptions);
    close(): void;
    handleRequest(req: http.IncomingMessage, res: http.ServerResponse): void;
    handleUpgrade(req: http.IncomingMessage, socket: Duplex): void;
    stats(): import("./TunnelAgent.js").TunnelAgentStats;
}
//# sourceMappingURL=Client.d.ts.map