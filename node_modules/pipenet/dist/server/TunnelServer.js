import debug from 'debug';
import net from 'net';
const log = debug('pipenet:tunnel-server');
/**
 * A single TCP server that handles all tunnel connections.
 * Clients send their ID as the first message, and the server routes
 * the connection to the appropriate handler.
 */
export class TunnelServer {
    handlers;
    server;
    constructor() {
        this.handlers = new Map();
        this.server = net.createServer((socket) => this.onConnection(socket));
        this.server.on('error', (err) => {
            if (err.code === 'ECONNRESET' || err.code === 'ETIMEDOUT')
                return;
            console.error('TunnelServer error:', err);
        });
    }
    close() {
        this.server.close();
    }
    listen(port, address) {
        return new Promise((resolve) => {
            if (address) {
                this.server.listen(port, address, () => {
                    log('tunnel server listening on %s:%d', address, port);
                    resolve();
                });
            }
            else {
                this.server.listen(port, () => {
                    log('tunnel server listening on port %d', port);
                    resolve();
                });
            }
        });
    }
    registerHandler(clientId, handler) {
        log('registering handler for client: %s', clientId);
        this.handlers.set(clientId, handler);
    }
    unregisterHandler(clientId) {
        log('unregistering handler for client: %s', clientId);
        this.handlers.delete(clientId);
    }
    onConnection(socket) {
        log('new connection from %s:%s', socket.remoteAddress, socket.remotePort);
        // Set a timeout for receiving the client ID
        const timeout = setTimeout(() => {
            log('timeout waiting for client ID');
            socket.destroy();
        }, 5000);
        // Buffer for accumulating data until we get a newline
        let buffer = '';
        const onData = (data) => {
            buffer += data.toString();
            const newlineIndex = buffer.indexOf('\n');
            if (newlineIndex === -1) {
                // Haven't received a complete client ID yet
                if (buffer.length > 100) {
                    // Protect against buffer overflow
                    log('client ID too long');
                    clearTimeout(timeout);
                    socket.destroy();
                }
                return;
            }
            // Got the client ID
            clearTimeout(timeout);
            socket.removeListener('data', onData);
            const clientId = buffer.substring(0, newlineIndex).trim();
            const remaining = buffer.substring(newlineIndex + 1);
            log('received client ID: %s', clientId);
            const handler = this.handlers.get(clientId);
            if (!handler) {
                log('no handler for client: %s', clientId);
                socket.destroy();
                return;
            }
            // If there's remaining data after the client ID, unshift it back
            if (remaining.length > 0) {
                socket.unshift(Buffer.from(remaining));
            }
            // Pass the socket to the handler
            handler(socket);
        };
        socket.on('data', onData);
        socket.once('error', () => {
            clearTimeout(timeout);
            socket.destroy();
        });
    }
}
//# sourceMappingURL=TunnelServer.js.map