import net from 'net';
import request from 'supertest';
import { describe, expect, it, vi } from 'vitest';
import { WebSocket, WebSocketServer } from 'ws';

import type { RequestInfo, TunnelInfo } from './server.js';

import { createServer } from './server.js';

describe('Server', () => {
  it('server starts and stops', async () => {
    const server = createServer();
    await new Promise<void>(resolve => server.listen(resolve));
    await new Promise<void>(resolve => server.close(() => resolve()));
  });

  it('should redirect root requests to landing page', async () => {
    const server = createServer();
    const res = await request(server).get('/');
    expect(res.headers.location).toBe('https://pipenet.dev/');
  });

  it('should support custom base domains', async () => {
    const server = createServer({
      domains: ['domain.example.com'],
    });

    const res = await request(server).get('/');
    expect(res.headers.location).toBe('https://pipenet.dev/');
  });

  it('reject long domain name requests', async () => {
    const server = createServer();
    const res = await request(server).get('/thisdomainisoutsidethesizeofwhatweallowwhichissixtythreecharacters');
    expect(res.body.message).toBe('Invalid subdomain. Subdomains must be lowercase and between 4 and 63 alphanumeric characters.');
  });

  it('should upgrade websocket requests', async () => {
    const hostname = 'websocket-test';
    const server = createServer({
      domains: ['example.com'],
    });
    await new Promise<void>(resolve => server.listen(resolve));

    const res = await request(server).get('/websocket-test');
    const localTunnelPort = res.body.port;

    const wss = await new Promise<WebSocketServer>((resolve) => {
      const wsServer = new WebSocketServer({ port: 0 }, () => {
        resolve(wsServer);
      });
    });

    const websocketServerPort = (wss.address() as net.AddressInfo).port;

    const ltSocket = net.createConnection({ port: localTunnelPort });
    const wsSocket = net.createConnection({ port: websocketServerPort });

    // Wait for both sockets to connect
    await Promise.all([
      new Promise<void>(resolve => ltSocket.once('connect', resolve)),
      new Promise<void>(resolve => wsSocket.once('connect', resolve)),
    ]);

    ltSocket.pipe(wsSocket).pipe(ltSocket);

    wss.once('connection', (ws) => {
      ws.once('message', (message) => {
        ws.send(message);
      });
    });

    const ws = new WebSocket('http://localhost:' + (server.address() as net.AddressInfo).port, {
      headers: {
        host: hostname + '.example.com',
      }
    });

    ws.on('open', () => {
      ws.send('something');
    });

    await new Promise<void>((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error('WebSocket message timeout')), 10000);
      ws.once('message', (msg) => {
        clearTimeout(timeout);
        expect(msg.toString()).toBe('something');
        resolve();
      });
      ws.once('error', (err) => {
        clearTimeout(timeout);
        reject(err);
      });
    });

    ws.close();
    ltSocket.destroy();
    wsSocket.destroy();
    wss.close();
    await new Promise<void>(resolve => server.close(() => resolve()));
  });

  it('should support the /api/tunnels/:id/status endpoint', async () => {
    const server = createServer();
    await new Promise<void>(resolve => server.listen(resolve));

    // no such tunnel yet
    const res = await request(server).get('/api/tunnels/foobar-test/status');
    expect(res.statusCode).toBe(404);

    // request a new client called foobar-test
    await request(server).get('/foobar-test');

    {
      const res = await request(server).get('/api/tunnels/foobar-test/status');
      expect(res.statusCode).toBe(200);
      expect(res.body).toEqual({
        connectedSockets: 0,
      });
    }

    await new Promise<void>(resolve => server.close(() => resolve()));
  });

  it('should include CORS headers in responses', async () => {
    const server = createServer();
    const res = await request(server).get('/api/status');

    expect(res.headers['access-control-allow-origin']).toBe('*');
    expect(res.headers['access-control-allow-methods']).toBe('GET, POST, PUT, DELETE, OPTIONS');
    expect(res.headers['access-control-allow-headers']).toBe('Content-Type, Authorization');
  });

  it('should handle OPTIONS preflight requests', async () => {
    const server = createServer();
    const res = await request(server).options('/api/status');

    expect(res.statusCode).toBe(204);
    expect(res.headers['access-control-allow-origin']).toBe('*');
  });

  describe('hooks', () => {
    it('should call onTunnelCreated when a tunnel is created', async () => {
      const onTunnelCreated = vi.fn<(tunnel: TunnelInfo) => void>();
      const server = createServer({ onTunnelCreated });
      await new Promise<void>(resolve => server.listen(resolve));

      await request(server).get('/test-tunnel');

      expect(onTunnelCreated).toHaveBeenCalledTimes(1);
      expect(onTunnelCreated).toHaveBeenCalledWith(
        expect.objectContaining({
          domain: expect.any(String),
          id: 'test-tunnel',
          url: expect.stringContaining('test-tunnel'),
        })
      );

      await new Promise<void>(resolve => server.close(() => resolve()));
    });

    it('should include domain in tunnel hooks', async () => {
      const onTunnelCreated = vi.fn<(tunnel: TunnelInfo) => void>();
      const server = createServer({
        domains: ['tunnel.example.com'],
        onTunnelCreated,
      });
      await new Promise<void>(resolve => server.listen(resolve));

      await request(server)
        .get('/domain-test')
        .set('Host', 'tunnel.example.com');

      expect(onTunnelCreated).toHaveBeenCalledTimes(1);
      expect(onTunnelCreated).toHaveBeenCalledWith({
        domain: 'tunnel.example.com',
        id: 'domain-test',
        url: 'http://domain-test.tunnel.example.com',
      });

      await new Promise<void>(resolve => server.close(() => resolve()));
    });

    it('should call onTunnelClosed when a tunnel is closed', async () => {
      const onTunnelCreated = vi.fn<(tunnel: TunnelInfo) => void>();
      const onTunnelClosed = vi.fn<(tunnel: TunnelInfo) => void>();
      const server = createServer({ onTunnelClosed, onTunnelCreated });
      await new Promise<void>(resolve => server.listen(resolve));

      const res = await request(server).get('/close-test');
      const localTunnelPort = res.body.port;

      // Connect a socket to activate the tunnel
      const socket = net.createConnection({ port: localTunnelPort });
      await new Promise<void>(resolve => socket.once('connect', resolve));

      expect(onTunnelCreated).toHaveBeenCalledTimes(1);
      expect(onTunnelClosed).not.toHaveBeenCalled();

      // Close the socket to trigger tunnel close
      socket.destroy();

      // Wait for the close event to propagate (Client has a 1s grace timeout after offline)
      await new Promise(resolve => setTimeout(resolve, 1200));

      expect(onTunnelClosed).toHaveBeenCalledTimes(1);
      expect(onTunnelClosed).toHaveBeenCalledWith(
        expect.objectContaining({
          domain: expect.any(String),
          id: 'close-test',
          url: expect.stringContaining('close-test'),
        })
      );

      await new Promise<void>(resolve => server.close(() => resolve()));
    }, 5000);

    it('should call onRequest when a request is proxied', async () => {
      const onRequest = vi.fn<(request: RequestInfo) => void>();
      const server = createServer({
        domains: ['example.com'],
        onRequest,
      });
      await new Promise<void>(resolve => server.listen(resolve));

      // Create a tunnel
      const res = await request(server).get('/request-test');
      const localTunnelPort = res.body.port;

      // Create a simple echo server
      const echoServer = net.createServer((socket) => {
        socket.on('data', () => {
          const response = 'HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK';
          socket.write(response);
        });
      });
      await new Promise<void>(resolve => echoServer.listen(0, resolve));
      const echoPort = (echoServer.address() as net.AddressInfo).port;

      // Connect tunnel socket to echo server
      const ltSocket = net.createConnection({ port: localTunnelPort });
      const echoSocket = net.createConnection({ port: echoPort });
      await Promise.all([
        new Promise<void>(resolve => ltSocket.once('connect', resolve)),
        new Promise<void>(resolve => echoSocket.once('connect', resolve)),
      ]);
      ltSocket.pipe(echoSocket).pipe(ltSocket);

      // Make a request through the tunnel
      await request(server)
        .get('/some/path')
        .set('Host', 'request-test.example.com');

      expect(onRequest).toHaveBeenCalledTimes(1);
      expect(onRequest).toHaveBeenCalledWith({
        headers: expect.objectContaining({
          host: 'request-test.example.com',
        }),
        method: 'GET',
        path: '/some/path',
        remoteAddress: expect.any(String),
        tunnelId: 'request-test',
      });

      ltSocket.destroy();
      echoSocket.destroy();
      echoServer.close();
      await new Promise<void>(resolve => server.close(() => resolve()));
    });
  });
});
