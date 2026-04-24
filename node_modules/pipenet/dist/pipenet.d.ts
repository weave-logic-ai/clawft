import { Tunnel, TunnelOptions } from './Tunnel.js';
export { HeaderHostTransformer } from './HeaderHostTransformer.js';
export type { TunnelOptions } from './Tunnel.js';
export { Tunnel } from './Tunnel.js';
export type { TunnelClusterOptions, TunnelRequest } from './TunnelCluster.js';
export { TunnelCluster } from './TunnelCluster.js';
export type TunnelCallback = (err: Error | null, tunnel?: Tunnel) => void;
type OptionsWithPort = {
    port: number;
} & TunnelOptions;
declare function pipenet(port: number): Promise<Tunnel>;
declare function pipenet(opts: OptionsWithPort): Promise<Tunnel>;
declare function pipenet(port: number, opts: TunnelOptions): Promise<Tunnel>;
declare function pipenet(opts: OptionsWithPort, callback: TunnelCallback): Tunnel;
declare function pipenet(port: number, opts: TunnelOptions, callback: TunnelCallback): Tunnel;
export { pipenet };
//# sourceMappingURL=pipenet.d.ts.map