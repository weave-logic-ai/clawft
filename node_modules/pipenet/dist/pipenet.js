import { Tunnel } from './Tunnel.js';
export { HeaderHostTransformer } from './HeaderHostTransformer.js';
export { Tunnel } from './Tunnel.js';
export { TunnelCluster } from './TunnelCluster.js';
function pipenet(arg1, arg2, arg3) {
    const options = typeof arg1 === 'object' ? arg1 : { ...arg2, port: arg1 };
    const callback = typeof arg1 === 'object' ? arg2 : arg3;
    const client = new Tunnel(options);
    if (callback) {
        client.open((err) => (err ? callback(err) : callback(null, client)));
        return client;
    }
    return new Promise((resolve, reject) => client.open((err) => (err ? reject(err) : resolve(client))));
}
export { pipenet };
//# sourceMappingURL=pipenet.js.map