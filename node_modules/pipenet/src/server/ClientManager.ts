import debug from 'debug';
import { hri } from 'human-readable-ids';

import type { TunnelServer } from './TunnelServer.js';

import { Client } from './Client.js';
import { TunnelAgent } from './TunnelAgent.js';

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

export class ClientManager {
  public stats: { tunnels: number };
  private clients: Map<string, Client>;
  private clientTunnelInfo: Map<string, { domain: string; url: string }>;
  private log: debug.Debugger;
  private opt: ClientManagerOptions;

  constructor(opt: ClientManagerOptions = {}) {
    this.opt = opt;
    this.clients = new Map();
    this.clientTunnelInfo = new Map();
    this.stats = { tunnels: 0 };
    this.log = debug('lt:ClientManager');
  }

  getClient(id: string): Client | undefined {
    return this.clients.get(id);
  }

  hasClient(id: string): boolean {
    return this.clients.has(id);
  }

  async newClient(requestedId: string | undefined, url: string, domain: string): Promise<NewClientInfo> {
    let id: string;
    if (requestedId && !this.clients.has(requestedId)) {
      id = requestedId;
    } else {
      id = hri.random();
    }

    const maxSockets = this.opt.maxTcpSockets;
    const agent = new TunnelAgent({
      clientId: id,
      maxTcpSockets: 10,
      tunnelServer: this.opt.tunnelServer,
    });

    const client = new Client({ agent, id });

    this.clients.set(id, client);
    this.clientTunnelInfo.set(id, { domain, url });

    client.once('close', () => {
      this.removeClient(id);
    });

    try {
      const info = await agent.listen();
      ++this.stats.tunnels;

      // Call the onTunnelCreated hook
      if (this.opt.onTunnelCreated) {
        this.opt.onTunnelCreated({ domain, id, url });
      }

      return {
        domain,
        id: id,
        maxConnCount: maxSockets,
        port: info.port,
        url,
      };
    } catch (err) {
      this.removeClient(id);
      throw err;
    }
  }

  removeClient(id: string): void {
    this.log('removing client: %s', id);
    const client = this.clients.get(id);
    if (!client) return;

    const tunnelInfo = this.clientTunnelInfo.get(id);
    const domain = tunnelInfo?.domain || '';
    const url = tunnelInfo?.url || '';

    --this.stats.tunnels;
    this.clients.delete(id);
    this.clientTunnelInfo.delete(id);
    client.close();

    // Call the onTunnelClosed hook
    if (this.opt.onTunnelClosed) {
      this.opt.onTunnelClosed({ domain, id, url });
    }
  }
}
