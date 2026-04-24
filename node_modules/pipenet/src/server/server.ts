import debug from 'debug';
import http from 'http';
import { hri } from 'human-readable-ids';
import Koa from 'koa';
import Router from 'koa-router';
import tldjs from 'tldjs';

import { ClientManager } from './ClientManager.js';
import { TunnelServer } from './TunnelServer.js';

const log = debug('pipenet:server');

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

export function createServer(opt: ServerOptions = {}): PipenetServer {
  const validHosts = opt.domains && opt.domains.length > 0 ? opt.domains : undefined;
  const myTldjs = tldjs.fromUserSettings({ validHosts });
  const landingPage = opt.landing || 'https://pipenet.dev/';

  function GetClientIdFromHostname(hostname: string): null | string {
    return myTldjs.getSubdomain(hostname);
  }

  // Create shared tunnel server if tunnelPort is specified
  const tunnelServer = opt.tunnelPort ? new TunnelServer() : undefined;

  const manager = new ClientManager({
    ...opt,
    tunnelServer,
  });

  const schema = opt.secure ? 'https' : 'http';
  const tunnelPort = opt.tunnelPort;

  const app = new Koa();
  const router = new Router();

  // CORS middleware
  app.use(async (ctx, next) => {
    ctx.set('Access-Control-Allow-Origin', '*');
    ctx.set('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS');
    ctx.set('Access-Control-Allow-Headers', 'Content-Type, Authorization');

    if (ctx.method === 'OPTIONS') {
      ctx.status = 204;
      return;
    }

    await next();
  });

  router.get('/api/status', async (ctx) => {
    const stats = manager.stats;
    ctx.body = {
      mem: process.memoryUsage(),
      tunnels: stats.tunnels,
    };
  });

  router.get('/api/tunnels/:id/status', async (ctx) => {
    const clientId = ctx.params.id;
    const client = manager.getClient(clientId);
    if (!client) {
      ctx.throw(404);
      return;
    }

    const stats = client.stats();
    ctx.body = {
      connectedSockets: stats.connectedSockets,
    };
  });

  app.use(router.routes());
  app.use(router.allowedMethods());

  // Helper to extract domain from host (strips port if present)
  const getDomain = (host: string) => {
    // Remove port if present (e.g., "localhost:3000" -> "localhost")
    return host.split(':')[0];
  };

  // Helper to build the tunnel URL
  const buildUrl = (id: string, host: string) => {
    return schema + '://' + id + '.' + host;
  };

  // Helper to build response with tunnel port if configured
  const buildResponse = (info: { domain: string; id: string; maxConnCount?: number; port: number; url: string }) => {
    const response: Record<string, unknown> = { ...info };
    // If using shared tunnel server, override port and add sharedTunnel flag
    if (tunnelPort) {
      response.port = tunnelPort;
      response.sharedTunnel = true;
    }
    return response;
  };

  app.use(async (ctx, next) => {
    const path = ctx.request.path;

    if (path !== '/') {
      await next();
      return;
    }

    const isNewClientRequest = ctx.query['new'] !== undefined;
    if (isNewClientRequest) {
      const reqId = hri.random();
      const domain = getDomain(ctx.request.host);
      const url = buildUrl(reqId, ctx.request.host);
      log('making new client with id %s', reqId);
      const info = await manager.newClient(reqId, url, domain);

      ctx.body = buildResponse(info);
      return;
    }

    ctx.redirect(landingPage);
  });

  app.use(async (ctx, next) => {
    const parts = ctx.request.path.split('/');

    if (parts.length !== 2) {
      await next();
      return;
    }

    const reqId = parts[1];

    if (!/^(?:[a-z0-9][a-z0-9-]{4,63}[a-z0-9]|[a-z0-9]{4,63})$/.test(reqId)) {
      const msg = 'Invalid subdomain. Subdomains must be lowercase and between 4 and 63 alphanumeric characters.';
      ctx.status = 403;
      ctx.body = { message: msg };
      return;
    }

    const domain = getDomain(ctx.request.host);
    const url = buildUrl(reqId, ctx.request.host);
    log('making new client with id %s', reqId);
    const info = await manager.newClient(reqId, url, domain);

    ctx.body = buildResponse(info);
  });

  const server: PipenetServer = http.createServer();
  server.tunnelServer = tunnelServer;

  const appCallback = app.callback();

  server.on('request', (req, res) => {
    const hostname = req.headers.host;
    if (!hostname) {
      res.statusCode = 400;
      res.end('Host header is required');
      return;
    }

    const clientId = GetClientIdFromHostname(hostname);
    if (!clientId) {
      appCallback(req, res);
      return;
    }

    const client = manager.getClient(clientId);
    if (!client) {
      res.statusCode = 404;
      res.end('404');
      return;
    }

    // Call the onRequest hook
    if (opt.onRequest) {
      opt.onRequest({
        headers: req.headers,
        method: req.method || 'GET',
        path: req.url || '/',
        remoteAddress: req.socket.remoteAddress,
        tunnelId: clientId,
      });
    }

    client.handleRequest(req, res);
  });

  server.on('upgrade', (req, socket) => {
    const hostname = req.headers.host;
    if (!hostname) {
      socket.destroy();
      return;
    }

    const clientId = GetClientIdFromHostname(hostname);
    if (!clientId) {
      socket.destroy();
      return;
    }

    const client = manager.getClient(clientId);
    if (!client) {
      socket.destroy();
      return;
    }

    client.handleUpgrade(req, socket);
  });

  return server;
}
