import axios from 'axios';
import debug from 'debug';
import { EventEmitter } from 'events';
import { TunnelCluster } from './TunnelCluster.js';
const log = debug('pipenet:client');
export class Tunnel extends EventEmitter {
    cachedUrl;
    clientId;
    closed;
    opts;
    tunnelCluster;
    url;
    constructor(opts = {}) {
        super();
        this.opts = opts;
        this.closed = false;
        if (!this.opts.host) {
            this.opts.host = 'https://pipenet.dev';
        }
    }
    close() {
        this.closed = true;
        this.emit('close');
    }
    open(cb) {
        this._init((err, info) => {
            if (err) {
                cb(err);
                return;
            }
            this.clientId = info.name;
            this.url = info.url;
            if (info.cachedUrl) {
                this.cachedUrl = info.cachedUrl;
            }
            this._establish(info);
            cb();
        });
    }
    _establish(info) {
        this.setMaxListeners(info.maxConn + (EventEmitter.defaultMaxListeners || 10));
        this.tunnelCluster = new TunnelCluster(info);
        this.tunnelCluster.once('open', () => {
            this.emit('url', info.url);
        });
        this.tunnelCluster.on('error', (err) => {
            log('got socket error', err.message);
            this.emit('error', err);
        });
        let tunnelCount = 0;
        this.tunnelCluster.on('open', (tunnel) => {
            tunnelCount++;
            log('tunnel open [total: %d]', tunnelCount);
            const closeHandler = () => {
                tunnel.destroy();
            };
            if (this.closed) {
                closeHandler();
                return;
            }
            this.once('close', closeHandler);
            tunnel.once('close', () => {
                this.removeListener('close', closeHandler);
            });
        });
        this.tunnelCluster.on('dead', () => {
            tunnelCount--;
            log('tunnel dead [total: %d]', tunnelCount);
            if (this.closed) {
                return;
            }
            this.tunnelCluster.open();
        });
        this.tunnelCluster.on('request', (req) => {
            this.emit('request', req);
        });
        for (let count = 0; count < info.maxConn; ++count) {
            this.tunnelCluster.open();
        }
    }
    _getInfo(body) {
        const { cachedUrl, id, ip, maxConnCount, port, sharedTunnel, url } = body;
        const { host, localHost, port: localPort } = this.opts;
        const { allowInvalidCert, localCa, localCert, localHttps, localKey } = this.opts;
        return {
            allowInvalidCert,
            cachedUrl,
            localCa,
            localCert,
            localHost,
            localHttps,
            localKey,
            localPort,
            maxConn: maxConnCount || 1,
            name: id,
            remoteHost: new URL(host).hostname,
            remoteIp: ip,
            remotePort: port,
            sharedTunnel,
            url,
        };
    }
    _init(cb) {
        const opt = this.opts;
        const getInfo = this._getInfo.bind(this);
        const params = {
            headers: opt.headers || {},
            responseType: 'json',
        };
        const baseUri = `${opt.host}/`;
        const assignedDomain = opt.subdomain;
        const uri = baseUri + (assignedDomain || '?new');
        const getUrl = () => {
            axios
                .get(uri, params)
                .then((res) => {
                const body = res.data;
                log('got tunnel information', res.data);
                if (res.status !== 200) {
                    const err = new Error(body?.message || 'pipenet server returned an error, please try again');
                    return cb(err);
                }
                cb(null, getInfo(body));
            })
                .catch((err) => {
                log(`tunnel server offline: ${err.message}, retry 1s`);
                setTimeout(getUrl, 1000);
            });
        };
        getUrl();
    }
}
//# sourceMappingURL=Tunnel.js.map