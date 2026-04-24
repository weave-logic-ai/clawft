import debug from 'debug';
import { EventEmitter } from 'events';
import http from 'http';
import pump from 'pump';
export class Client extends EventEmitter {
    id;
    agent;
    graceTimeout;
    log;
    constructor(options) {
        super();
        this.agent = options.agent;
        this.id = options.id;
        this.log = debug(`lt:Client[${this.id}]`);
        this.graceTimeout = setTimeout(() => {
            this.close();
        }, 1000);
        this.graceTimeout.unref();
        this.agent.on('online', () => {
            this.log('client online %s', this.id);
            clearTimeout(this.graceTimeout);
        });
        this.agent.on('offline', () => {
            this.log('client offline %s', this.id);
            clearTimeout(this.graceTimeout);
            this.graceTimeout = setTimeout(() => {
                this.close();
            }, 1000);
            this.graceTimeout.unref();
        });
        this.agent.once('error', () => {
            this.close();
        });
    }
    close() {
        clearTimeout(this.graceTimeout);
        this.agent.destroy();
        this.emit('close');
    }
    handleRequest(req, res) {
        this.log('> %s', req.url);
        const opt = {
            agent: this.agent,
            headers: req.headers,
            method: req.method,
            path: req.url,
        };
        const clientReq = http.request(opt, (clientRes) => {
            this.log('< %s', req.url);
            res.writeHead(clientRes.statusCode, clientRes.headers);
            pump(clientRes, res);
        });
        clientReq.once('error', () => {
            // TODO: if headers not sent - respond with gateway unavailable
        });
        pump(req, clientReq);
    }
    handleUpgrade(req, socket) {
        this.log('> [up] %s', req.url);
        socket.once('error', (err) => {
            if (err.code === 'ECONNRESET' || err.code === 'ETIMEDOUT')
                return;
            console.error(err);
        });
        this.agent.createConnection({}, (err, conn) => {
            this.log('< [up] %s', req.url);
            if (err || !conn) {
                socket.end();
                return;
            }
            if (!socket.readable || !socket.writable) {
                conn.destroy();
                socket.end();
                return;
            }
            const arr = [`${req.method} ${req.url} HTTP/${req.httpVersion}`];
            for (let i = 0; i < req.rawHeaders.length - 1; i += 2) {
                arr.push(`${req.rawHeaders[i]}: ${req.rawHeaders[i + 1]}`);
            }
            arr.push('');
            arr.push('');
            pump(conn, socket);
            pump(socket, conn);
            conn.write(arr.join('\r\n'));
        });
    }
    stats() {
        return this.agent.stats();
    }
}
//# sourceMappingURL=Client.js.map