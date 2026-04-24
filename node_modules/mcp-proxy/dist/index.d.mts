import http, { IncomingMessage } from "http";
import { EventStore } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import { JSONRPCMessage, ServerCapabilities } from "@modelcontextprotocol/sdk/types.js";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { SSEClientTransportOptions } from "@modelcontextprotocol/sdk/client/sse.js";
import { StreamableHTTPClientTransportOptions } from "@modelcontextprotocol/sdk/client/streamableHttp.js";
import { Transport } from "@modelcontextprotocol/sdk/shared/transport.js";

//#region src/authentication.d.ts
interface AuthConfig {
  apiKey?: string;
  oauth?: {
    error?: string;
    error_description?: string;
    error_uri?: string;
    protectedResource?: {
      resource?: string;
    };
    realm?: string;
    scope?: string;
  };
}
declare class AuthenticationMiddleware {
  private config;
  constructor(config?: AuthConfig);
  getScopeChallengeResponse(requiredScopes: string[], errorDescription?: string, requestId?: unknown): {
    body: string;
    headers: Record<string, string>;
    statusCode: number;
  };
  getUnauthorizedResponse(options?: {
    error?: string;
    error_description?: string;
    error_uri?: string;
    scope?: string;
  }): {
    body: string;
    headers: Record<string, string>;
  };
  validateRequest(req: IncomingMessage): boolean;
}
//#endregion
//#region src/InMemoryEventStore.d.ts
/**
 * Simple in-memory implementation of the EventStore interface for resumability
 * This is primarily intended for examples and testing, not for production use
 * where a persistent storage solution would be more appropriate.
 */
declare class InMemoryEventStore implements EventStore {
  private events;
  private lastTimestamp;
  private lastTimestampCounter;
  /**
   * Replays events that occurred after a specific event ID
   * Implements EventStore.replayEventsAfter
   */
  replayEventsAfter(lastEventId: string, {
    send
  }: {
    send: (eventId: string, message: JSONRPCMessage) => Promise<void>;
  }): Promise<string>;
  /**
   * Stores an event with a generated event ID
   * Implements EventStore.storeEvent
   */
  storeEvent(streamId: string, message: JSONRPCMessage): Promise<string>;
  /**
   * Generates a monotonic unique event ID in
   * `${streamId}_${timestamp}_${counter}_${random}` format.
   */
  private generateEventId;
  /**
   * Extracts the stream ID from an event ID
   */
  private getStreamIdFromEventId;
}
//#endregion
//#region src/proxyServer.d.ts
declare const proxyServer: ({
  client,
  requestTimeout,
  server,
  serverCapabilities
}: {
  client: Client;
  requestTimeout?: number;
  server: Server;
  serverCapabilities: ServerCapabilities;
}) => Promise<void>;
//#endregion
//#region src/startHTTPServer.d.ts
interface CorsOptions {
  allowedHeaders?: string | string[];
  credentials?: boolean;
  exposedHeaders?: string[];
  maxAge?: number;
  methods?: string[];
  origin?: ((origin: string) => boolean) | string | string[];
}
type SSEServer = {
  close: () => Promise<void>;
};
type ServerLike = {
  close: Server["close"];
  connect: Server["connect"];
};
declare const startHTTPServer: <T extends ServerLike>({
  apiKey,
  authenticate,
  cors,
  createServer,
  enableJsonResponse,
  eventStore,
  host,
  oauth,
  onClose,
  onConnect,
  onUnhandledRequest,
  port,
  sseEndpoint,
  sslCa,
  sslCert,
  sslKey,
  stateless,
  streamEndpoint
}: {
  apiKey?: string;
  authenticate?: (request: http.IncomingMessage) => Promise<unknown>;
  cors?: boolean | CorsOptions;
  createServer: (request: http.IncomingMessage) => Promise<T>;
  enableJsonResponse?: boolean;
  eventStore?: EventStore;
  host?: string;
  oauth?: AuthConfig["oauth"];
  onClose?: (server: T) => Promise<void>;
  onConnect?: (server: T) => Promise<void>;
  onUnhandledRequest?: (req: http.IncomingMessage, res: http.ServerResponse) => Promise<void>;
  port: number;
  sseEndpoint?: null | string;
  sslCa?: null | string;
  sslCert?: null | string;
  sslKey?: null | string;
  stateless?: boolean;
  streamEndpoint?: null | string;
}) => Promise<SSEServer>;
//#endregion
//#region src/startStdioServer.d.ts
declare enum ServerType {
  HTTPStream = "HTTPStream",
  SSE = "SSE",
}
declare const startStdioServer: ({
  initStdioServer,
  initStreamClient,
  serverType,
  transportOptions,
  url
}: {
  initStdioServer?: () => Promise<Server>;
  initStreamClient?: () => Promise<Client>;
  serverType: ServerType;
  transportOptions?: SSEClientTransportOptions | StreamableHTTPClientTransportOptions;
  url: string;
}) => Promise<Server>;
//#endregion
//#region src/tapTransport.d.ts
type TransportEvent = {
  error: Error;
  type: "onerror";
} | {
  message: JSONRPCMessage;
  type: "onmessage";
} | {
  message: JSONRPCMessage;
  type: "send";
} | {
  type: "close";
} | {
  type: "onclose";
} | {
  type: "start";
};
declare const tapTransport: (transport: Transport, eventHandler: (event: TransportEvent) => void) => Transport;
//#endregion
export { type AuthConfig, AuthenticationMiddleware, type CorsOptions, InMemoryEventStore, ServerType, proxyServer, startHTTPServer, startStdioServer, tapTransport };
//# sourceMappingURL=index.d.mts.map