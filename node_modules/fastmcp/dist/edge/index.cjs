"use strict";Object.defineProperty(exports, "__esModule", {value: true}); function _nullishCoalesce(lhs, rhsFn) { if (lhs != null) { return lhs; } else { return rhsFn(); } } async function _asyncNullishCoalesce(lhs, rhsFn) { if (lhs != null) { return lhs; } else { return await rhsFn(); } } function _optionalChain(ops) { let lastAccessLHS = undefined; let value = ops[0]; let i = 1; while (i < ops.length) { const op = ops[i]; const fn = ops[i + 1]; i += 2; if ((op === 'optionalAccess' || op === 'optionalCall') && value == null) { return undefined; } if (op === 'access' || op === 'optionalAccess') { lastAccessLHS = value; value = fn(value); } else if (op === 'call' || op === 'optionalCall') { value = fn((...args) => value.call(lastAccessLHS, ...args)); lastAccessLHS = undefined; } } return value; } var _class;// src/edge/index.ts



var _typesjs = require('@modelcontextprotocol/sdk/types.js');
var _hono = require('hono');
var _zod = require('zod');
var _zodtojsonschema = require('zod-to-json-schema');

// src/edge/WebStreamableHTTPServerTransport.ts






var MAXIMUM_MESSAGE_SIZE = 4 * 1024 * 1024;
var WebStreamableHTTPServerTransport = (_class = class {
  
  
  
  
  __init() {this._enableJsonResponse = false}
  __init2() {this._encoder = new TextEncoder()}
  
  
  
  __init3() {this._pendingResponses = []}
  __init4() {this._requestToStreamMapping = /* @__PURE__ */ new Map()}
  __init5() {this._standaloneSseStreamId = "_GET_stream"}
  __init6() {this._started = false}
  __init7() {this._streamMapping = /* @__PURE__ */ new Map()}
  
  constructor(options) {;_class.prototype.__init.call(this);_class.prototype.__init2.call(this);_class.prototype.__init3.call(this);_class.prototype.__init4.call(this);_class.prototype.__init5.call(this);_class.prototype.__init6.call(this);_class.prototype.__init7.call(this);
    this.sessionIdGenerator = options.sessionIdGenerator;
    this._enableJsonResponse = _nullishCoalesce(options.enableJsonResponse, () => ( false));
    this._eventStore = options.eventStore;
    this._onsessioninitialized = options.onsessioninitialized;
    this._onsessionclosed = options.onsessionclosed;
  }
  /**
   * Close the transport
   */
  async close() {
    for (const writer of this._streamMapping.values()) {
      try {
        await writer.close();
      } catch (e) {
      }
    }
    this._streamMapping.clear();
    this._started = false;
    _optionalChain([this, 'access', _ => _.onclose, 'optionalCall', _2 => _2()]);
  }
  /**
   * Handles an incoming web Request and returns a Response
   */
  async handleRequest(request, parsedBody) {
    const method = request.method;
    if (method === "POST") {
      return this.handlePostRequest(request, parsedBody);
    } else if (method === "GET") {
      return this.handleGetRequest(request);
    } else if (method === "DELETE") {
      return this.handleDeleteRequest(request);
    } else {
      return this.handleUnsupportedRequest();
    }
  }
  /**
   * Send a message to connected clients
   */
  async send(message, options) {
    this._pendingResponses.push(message);
    const streamId = _optionalChain([options, 'optionalAccess', _3 => _3.relatedRequestId]) ? this._requestToStreamMapping.get(options.relatedRequestId) : this._standaloneSseStreamId;
    if (streamId) {
      const writer = this._streamMapping.get(streamId);
      if (writer) {
        try {
          if (this._eventStore) {
            const eventId = await this._eventStore.storeEvent(
              streamId,
              message
            );
            await this.writeSSEEventWithId(writer, eventId, message);
          } else {
            await this.writeSSEEvent(writer, message);
          }
        } catch (error) {
          _optionalChain([this, 'access', _4 => _4.onerror, 'optionalCall', _5 => _5(
            error instanceof Error ? error : new Error(String(error))
          )]);
        }
      }
    }
  }
  async start() {
    if (this._started) {
      throw new Error("Transport already started");
    }
    this._started = true;
  }
  /**
   * Create an error response
   */
  createErrorResponse(status, code, message) {
    return new Response(
      JSON.stringify({
        error: { code, message },
        id: null,
        jsonrpc: "2.0"
      }),
      {
        headers: {
          ...this.getResponseHeaders(),
          "Content-Type": "application/json"
        },
        status
      }
    );
  }
  /**
   * Get common response headers
   */
  getResponseHeaders() {
    const headers = {};
    if (this.sessionId) {
      headers["mcp-session-id"] = this.sessionId;
    }
    return headers;
  }
  /**
   * Handles DELETE requests to terminate sessions
   */
  async handleDeleteRequest(request) {
    const sessionId = request.headers.get("mcp-session-id");
    if (this.sessionIdGenerator) {
      if (!sessionId) {
        return this.createErrorResponse(
          400,
          -32e3,
          "Bad Request: Mcp-Session-Id header is required"
        );
      }
      if (this.sessionId !== sessionId) {
        return this.createErrorResponse(404, -32001, "Session not found");
      }
    }
    for (const writer of this._streamMapping.values()) {
      try {
        await writer.close();
      } catch (e2) {
      }
    }
    this._streamMapping.clear();
    await _optionalChain([this, 'access', _6 => _6._onsessionclosed, 'optionalCall', _7 => _7(_nullishCoalesce(this.sessionId, () => ( "")))]);
    this.sessionId = void 0;
    return new Response(null, {
      headers: this.getResponseHeaders(),
      status: 204
    });
  }
  /**
   * Handles GET requests for SSE stream
   */
  async handleGetRequest(request) {
    const acceptHeader = request.headers.get("accept");
    if (!_optionalChain([acceptHeader, 'optionalAccess', _8 => _8.includes, 'call', _9 => _9("text/event-stream")])) {
      return this.createErrorResponse(
        406,
        -32e3,
        "Not Acceptable: Client must accept text/event-stream"
      );
    }
    const sessionId = request.headers.get("mcp-session-id");
    if (this.sessionIdGenerator && !sessionId) {
      return this.createErrorResponse(
        400,
        -32e3,
        "Bad Request: Mcp-Session-Id header is required"
      );
    }
    if (this.sessionIdGenerator && this.sessionId !== sessionId) {
      return this.createErrorResponse(404, -32001, "Session not found");
    }
    if (this._streamMapping.has(this._standaloneSseStreamId)) {
      return this.createErrorResponse(
        409,
        -32e3,
        "Conflict: SSE stream already exists for this session"
      );
    }
    if (this._eventStore) {
      const lastEventId = request.headers.get("last-event-id");
      if (lastEventId) {
        return this.handleReplayEvents(lastEventId);
      }
    }
    const { readable, writable } = new TransformStream();
    const writer = writable.getWriter();
    this._streamMapping.set(this._standaloneSseStreamId, writer);
    return new Response(readable, {
      headers: {
        ...this.getResponseHeaders(),
        "Cache-Control": "no-cache, no-transform",
        Connection: "keep-alive",
        "Content-Type": "text/event-stream"
      },
      status: 200
    });
  }
  /**
   * Handles POST requests containing JSON-RPC messages
   */
  async handlePostRequest(request, parsedBody) {
    const acceptHeader = request.headers.get("accept");
    if (!_optionalChain([acceptHeader, 'optionalAccess', _10 => _10.includes, 'call', _11 => _11("application/json")]) && !_optionalChain([acceptHeader, 'optionalAccess', _12 => _12.includes, 'call', _13 => _13("text/event-stream")])) {
      return this.createErrorResponse(
        406,
        -32e3,
        "Not Acceptable: Client must accept application/json or text/event-stream"
      );
    }
    const contentType = request.headers.get("content-type");
    if (!_optionalChain([contentType, 'optionalAccess', _14 => _14.includes, 'call', _15 => _15("application/json")])) {
      return this.createErrorResponse(
        415,
        -32e3,
        "Unsupported Media Type: Content-Type must be application/json"
      );
    }
    const contentLength = parseInt(
      _nullishCoalesce(request.headers.get("content-length"), () => ( "0")),
      10
    );
    if (contentLength > MAXIMUM_MESSAGE_SIZE) {
      return this.createErrorResponse(
        413,
        -32e3,
        `Request body too large. Maximum size is ${MAXIMUM_MESSAGE_SIZE} bytes`
      );
    }
    let rawMessage;
    try {
      rawMessage = await _asyncNullishCoalesce(parsedBody, async () => ( await request.json()));
    } catch (e3) {
      return this.createErrorResponse(400, -32700, "Parse error: Invalid JSON");
    }
    const arrayMessage = Array.isArray(rawMessage) ? rawMessage : [rawMessage];
    const messages = [];
    for (const msg of arrayMessage) {
      const result = _typesjs.JSONRPCMessageSchema.safeParse(msg);
      if (!result.success) {
        return this.createErrorResponse(
          400,
          -32700,
          "Parse error: Invalid JSON-RPC message"
        );
      }
      messages.push(result.data);
    }
    const requestSessionId = request.headers.get("mcp-session-id");
    const hasInitRequest = messages.some((msg) => _typesjs.isInitializeRequest.call(void 0, msg));
    if (hasInitRequest && requestSessionId) {
      return this.createErrorResponse(
        400,
        -32600,
        "Invalid Request: Initialization requests must not include a sessionId"
      );
    }
    if (hasInitRequest && messages.length > 1) {
      return this.createErrorResponse(
        400,
        -32600,
        "Invalid Request: Only one initialization request is allowed"
      );
    }
    if (!hasInitRequest && !requestSessionId && this.sessionIdGenerator) {
      return this.createErrorResponse(
        400,
        -32e3,
        "Bad Request: Mcp-Session-Id header is required"
      );
    }
    if (hasInitRequest && this.sessionIdGenerator) {
      this.sessionId = this.sessionIdGenerator();
      await _optionalChain([this, 'access', _16 => _16._onsessioninitialized, 'optionalCall', _17 => _17(this.sessionId)]);
    } else if (requestSessionId) {
      if (this.sessionIdGenerator && this.sessionId !== requestSessionId) {
        return this.createErrorResponse(404, -32001, "Session not found");
      }
    }
    this._pendingResponses = [];
    for (const message of messages) {
      _optionalChain([this, 'access', _18 => _18.onmessage, 'optionalCall', _19 => _19(message, { authInfo: void 0 })]);
    }
    if (messages.every(
      (msg) => _typesjs.isJSONRPCNotification.call(void 0, msg) || _typesjs.isJSONRPCResponse.call(void 0, msg)
    )) {
      return new Response(null, {
        headers: this.getResponseHeaders(),
        status: 202
      });
    }
    if (this._enableJsonResponse && _optionalChain([acceptHeader, 'optionalAccess', _20 => _20.includes, 'call', _21 => _21("application/json")])) {
      await new Promise((resolve) => setTimeout(resolve, 0));
      const responseBody = this._pendingResponses.length === 1 ? JSON.stringify(this._pendingResponses[0]) : JSON.stringify(this._pendingResponses);
      return new Response(responseBody, {
        headers: {
          ...this.getResponseHeaders(),
          "Content-Type": "application/json"
        },
        status: 200
      });
    }
    const { readable, writable } = new TransformStream();
    const writer = writable.getWriter();
    const streamId = `post_${Date.now()}`;
    this._streamMapping.set(streamId, writer);
    (async () => {
      try {
        for (const response of this._pendingResponses) {
          await this.writeSSEEvent(writer, response);
        }
      } catch (error) {
        _optionalChain([this, 'access', _22 => _22.onerror, 'optionalCall', _23 => _23(
          error instanceof Error ? error : new Error(String(error))
        )]);
      }
    })();
    return new Response(readable, {
      headers: {
        ...this.getResponseHeaders(),
        "Cache-Control": "no-cache, no-transform",
        Connection: "keep-alive",
        "Content-Type": "text/event-stream"
      },
      status: 200
    });
  }
  /**
   * Replay events for resumability
   */
  async handleReplayEvents(lastEventId) {
    if (!this._eventStore) {
      return this.createErrorResponse(
        400,
        -32e3,
        "Resumability not supported"
      );
    }
    const { readable, writable } = new TransformStream();
    const writer = writable.getWriter();
    try {
      const streamId = await this._eventStore.replayEventsAfter(lastEventId, {
        send: async (eventId, message) => {
          await this.writeSSEEventWithId(writer, eventId, message);
        }
      });
      this._streamMapping.set(streamId, writer);
    } catch (error) {
      await writer.close();
      return this.createErrorResponse(500, -32e3, `Replay failed: ${error}`);
    }
    return new Response(readable, {
      headers: {
        ...this.getResponseHeaders(),
        "Cache-Control": "no-cache, no-transform",
        Connection: "keep-alive",
        "Content-Type": "text/event-stream"
      },
      status: 200
    });
  }
  /**
   * Handles unsupported HTTP methods
   */
  handleUnsupportedRequest() {
    return this.createErrorResponse(405, -32e3, "Method not allowed");
  }
  /**
   * Write an SSE event to the stream
   */
  async writeSSEEvent(writer, message) {
    const data = `data: ${JSON.stringify(message)}

`;
    await writer.write(this._encoder.encode(data));
  }
  /**
   * Write an SSE event with ID to the stream
   */
  async writeSSEEventWithId(writer, eventId, message) {
    const data = `id: ${eventId}
data: ${JSON.stringify(message)}

`;
    await writer.write(this._encoder.encode(data));
  }
}, _class);

// src/edge/index.ts
var EdgeFastMCP = class {
  #honoApp = new (0, _hono.Hono)();
  #logger;
  #mcpPath;
  #name;
  #prompts = [];
  #resources = [];
  #tools = [];
  #version;
  constructor(options) {
    this.#name = options.name;
    this.#version = options.version;
    this.#logger = _nullishCoalesce(options.logger, () => ( console));
    this.#mcpPath = _nullishCoalesce(options.mcpPath, () => ( "/mcp"));
    this.#setupRoutes();
  }
  /**
   * Add a prompt to the server
   */
  addPrompt(prompt) {
    this.#prompts.push(prompt);
    return this;
  }
  /**
   * Add a resource to the server
   */
  addResource(resource) {
    this.#resources.push(resource);
    return this;
  }
  /**
   * Add a tool to the server
   */
  addTool(tool) {
    this.#tools.push(tool);
    return this;
  }
  /**
   * Handle an incoming request (main entry point for edge runtimes)
   */
  async fetch(request) {
    return this.#honoApp.fetch(request);
  }
  /**
   * Get the Hono app for adding custom routes
   */
  getApp() {
    return this.#honoApp;
  }
  /**
   * Create an error HTTP response
   */
  #errorResponse(status, code, message) {
    return new Response(
      JSON.stringify({
        error: { code, message },
        id: null,
        jsonrpc: "2.0"
      }),
      {
        headers: { "Content-Type": "application/json" },
        status
      }
    );
  }
  /**
   * Handle initialize request
   */
  #handleInitialize(id) {
    return {
      id,
      jsonrpc: "2.0",
      result: {
        capabilities: {
          prompts: this.#prompts.length > 0 ? {} : void 0,
          resources: this.#resources.length > 0 ? {} : void 0,
          tools: this.#tools.length > 0 ? {} : void 0
        },
        protocolVersion: _typesjs.LATEST_PROTOCOL_VERSION,
        serverInfo: {
          name: this.#name,
          version: this.#version
        }
      }
    };
  }
  /**
   * Handle MCP POST requests
   */
  async #handleMcpRequest(request) {
    const acceptHeader = request.headers.get("accept");
    if (!_optionalChain([acceptHeader, 'optionalAccess', _24 => _24.includes, 'call', _25 => _25("application/json")]) && !_optionalChain([acceptHeader, 'optionalAccess', _26 => _26.includes, 'call', _27 => _27("text/event-stream")])) {
      return this.#errorResponse(
        406,
        -32e3,
        "Not Acceptable: Client must accept application/json or text/event-stream"
      );
    }
    const contentType = request.headers.get("content-type");
    if (!_optionalChain([contentType, 'optionalAccess', _28 => _28.includes, 'call', _29 => _29("application/json")])) {
      return this.#errorResponse(
        415,
        -32e3,
        "Unsupported Media Type: Content-Type must be application/json"
      );
    }
    let body;
    try {
      body = await request.json();
    } catch (e4) {
      return this.#errorResponse(400, -32700, "Parse error: Invalid JSON");
    }
    const messages = Array.isArray(body) ? body : [body];
    const responses = [];
    for (const message of messages) {
      const response = await this.#handleMessage(message);
      if (response) {
        responses.push(response);
      }
    }
    if (responses.length === 0) {
      return new Response(null, { status: 202 });
    }
    const responseBody = responses.length === 1 ? JSON.stringify(responses[0]) : JSON.stringify(responses);
    return new Response(responseBody, {
      headers: {
        "Content-Type": "application/json"
      },
      status: 200
    });
  }
  /**
   * Handle SSE GET requests
   */
  async #handleMcpSseRequest(request) {
    const acceptHeader = request.headers.get("accept");
    if (!_optionalChain([acceptHeader, 'optionalAccess', _30 => _30.includes, 'call', _31 => _31("text/event-stream")])) {
      return this.#errorResponse(
        406,
        -32e3,
        "Not Acceptable: Client must accept text/event-stream"
      );
    }
    return this.#errorResponse(
      405,
      -32e3,
      "Method Not Allowed: SSE streams not supported in stateless mode"
    );
  }
  /**
   * Handle individual MCP messages
   */
  async #handleMessage(message) {
    if (!message || typeof message !== "object") {
      return this.#rpcError(null, -32700, "Parse error: Invalid message");
    }
    const msg = message;
    if (msg.jsonrpc !== "2.0") {
      return this.#rpcError(
        _nullishCoalesce(msg.id, () => ( null)),
        -32600,
        "Invalid Request: jsonrpc must be 2.0"
      );
    }
    if (!("id" in msg) || msg.id === void 0) {
      return null;
    }
    const method = msg.method;
    const id = msg.id;
    const params = msg.params;
    try {
      switch (method) {
        case "initialize":
          return this.#handleInitialize(id);
        case "ping":
          return { id, jsonrpc: "2.0", result: {} };
        case "prompts/get":
          return this.#handlePromptsGet(id, params);
        case "prompts/list":
          return this.#handlePromptsList(id);
        case "resources/list":
          return this.#handleResourcesList(id);
        case "resources/read":
          return this.#handleResourcesRead(id, params);
        case "tools/call":
          return this.#handleToolsCall(id, params);
        case "tools/list":
          return this.#handleToolsList(id);
        default:
          return this.#rpcError(id, -32601, `Method not found: ${method}`);
      }
    } catch (error) {
      this.#logger.error(`Error handling ${method}:`, error);
      return this.#rpcError(
        id,
        -32603,
        `Internal error: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }
  /**
   * Handle prompts/get request
   */
  async #handlePromptsGet(id, params) {
    const promptName = _optionalChain([params, 'optionalAccess', _32 => _32.name]);
    const promptArgs = _optionalChain([params, 'optionalAccess', _33 => _33.arguments]);
    const prompt = this.#prompts.find((p) => p.name === promptName);
    if (!prompt) {
      return this.#rpcError(
        id,
        _typesjs.ErrorCode.InvalidParams,
        `Prompt not found: ${promptName}`
      );
    }
    try {
      const result = await prompt.load(_nullishCoalesce(promptArgs, () => ( {})));
      const messages = typeof result === "string" ? [{ content: { text: result, type: "text" }, role: "user" }] : result.messages;
      return {
        id,
        jsonrpc: "2.0",
        result: { messages }
      };
    } catch (error) {
      return this.#rpcError(
        id,
        _typesjs.ErrorCode.InternalError,
        `Prompt load failed: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }
  /**
   * Handle prompts/list request
   */
  #handlePromptsList(id) {
    return {
      id,
      jsonrpc: "2.0",
      result: {
        prompts: this.#prompts.map((p) => ({
          arguments: p.arguments,
          description: p.description,
          name: p.name
        }))
      }
    };
  }
  /**
   * Handle resources/list request
   */
  #handleResourcesList(id) {
    return {
      id,
      jsonrpc: "2.0",
      result: {
        resources: this.#resources.map((r) => ({
          description: r.description,
          mimeType: r.mimeType,
          name: r.name,
          uri: r.uri
        }))
      }
    };
  }
  /**
   * Handle resources/read request
   */
  async #handleResourcesRead(id, params) {
    const uri = _optionalChain([params, 'optionalAccess', _34 => _34.uri]);
    const resource = this.#resources.find((r) => r.uri === uri);
    if (!resource) {
      return this.#rpcError(
        id,
        _typesjs.ErrorCode.InvalidParams,
        `Resource not found: ${uri}`
      );
    }
    try {
      const result = await resource.load();
      const content = typeof result === "string" ? { mimeType: _nullishCoalesce(resource.mimeType, () => ( "text/plain")), text: result, uri } : { uri, ...result };
      return {
        id,
        jsonrpc: "2.0",
        result: { contents: [content] }
      };
    } catch (error) {
      return this.#rpcError(
        id,
        _typesjs.ErrorCode.InternalError,
        `Resource load failed: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }
  /**
   * Handle tools/call request
   */
  async #handleToolsCall(id, params) {
    const toolName = _optionalChain([params, 'optionalAccess', _35 => _35.name]);
    const toolArgs = _optionalChain([params, 'optionalAccess', _36 => _36.arguments]);
    const tool = this.#tools.find((t) => t.name === toolName);
    if (!tool) {
      return this.#rpcError(
        id,
        _typesjs.ErrorCode.InvalidParams,
        `Tool not found: ${toolName}`
      );
    }
    try {
      const result = await tool.execute(_nullishCoalesce(toolArgs, () => ( {})));
      const content = typeof result === "string" ? [{ text: result, type: "text" }] : result.content;
      return {
        id,
        jsonrpc: "2.0",
        result: { content }
      };
    } catch (error) {
      return this.#rpcError(
        id,
        _typesjs.ErrorCode.InternalError,
        `Tool execution failed: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }
  /**
   * Handle tools/list request
   */
  #handleToolsList(id) {
    return {
      id,
      jsonrpc: "2.0",
      result: {
        tools: this.#tools.map((tool) => ({
          description: tool.description,
          inputSchema: tool.parameters ? this.#schemaToJsonSchema(tool.parameters) : { type: "object" },
          name: tool.name
        }))
      }
    };
  }
  /**
   * Create an RPC error message
   */
  #rpcError(id, code, message) {
    return {
      error: { code, message },
      id,
      jsonrpc: "2.0"
    };
  }
  /**
   * Convert schema to JSON Schema
   */
  #schemaToJsonSchema(schema) {
    try {
      if (typeof _zod.z.toJSONSchema === "function") {
        return _zod.z.toJSONSchema(schema);
      }
      if ("_def" in schema || schema instanceof _zod.z.ZodType) {
        return _zodtojsonschema.zodToJsonSchema.call(void 0, schema, { target: "openApi3" });
      }
      return { type: "object" };
    } catch (e5) {
      return { type: "object" };
    }
  }
  /**
   * Set up MCP and health routes
   */
  #setupRoutes() {
    this.#honoApp.get("/health", (c) => c.text("\u2713 Ok"));
    this.#honoApp.post(this.#mcpPath, async (c) => {
      return this.#handleMcpRequest(c.req.raw);
    });
    this.#honoApp.get(this.#mcpPath, async (c) => {
      return this.#handleMcpSseRequest(c.req.raw);
    });
    this.#honoApp.delete(this.#mcpPath, async () => {
      return new Response(null, { status: 204 });
    });
  }
};



exports.EdgeFastMCP = EdgeFastMCP; exports.WebStreamableHTTPServerTransport = WebStreamableHTTPServerTransport;
//# sourceMappingURL=index.cjs.map