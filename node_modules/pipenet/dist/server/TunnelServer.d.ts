import net from 'net';
export type SocketHandler = (socket: net.Socket) => void;
/**
 * A single TCP server that handles all tunnel connections.
 * Clients send their ID as the first message, and the server routes
 * the connection to the appropriate handler.
 */
export declare class TunnelServer {
    private handlers;
    private server;
    constructor();
    close(): void;
    listen(port: number, address?: string): Promise<void>;
    registerHandler(clientId: string, handler: SocketHandler): void;
    unregisterHandler(clientId: string): void;
    private onConnection;
}
//# sourceMappingURL=TunnelServer.d.ts.map