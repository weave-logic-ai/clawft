// src/FastMCP.ts
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  CompleteRequestSchema,
  ErrorCode,
  GetPromptRequestSchema,
  ListPromptsRequestSchema,
  ListResourcesRequestSchema,
  ListResourceTemplatesRequestSchema,
  ListToolsRequestSchema,
  McpError,
  ReadResourceRequestSchema,
  RootsListChangedNotificationSchema,
  SetLevelRequestSchema
} from "@modelcontextprotocol/sdk/types.js";
import { EventEmitter } from "events";
import { readFile } from "fs/promises";
import Fuse from "fuse.js";
import { Hono } from "hono";
import { startHTTPServer } from "mcp-proxy";
import { setTimeout as delay } from "timers/promises";
import { fetch as fetch2 } from "undici";
import parseURITemplate from "uri-templates";
import { toJsonSchema } from "xsschema";
import { z } from "zod";

// src/DiscoveryDocumentCache.ts
var DiscoveryDocumentCache = class {
  get size() {
    return this.#cache.size;
  }
  #cache = /* @__PURE__ */ new Map();
  #inFlight = /* @__PURE__ */ new Map();
  #ttl;
  /**
   * @param options - configuration options
   * @param options.ttl - time-to-live in miliseconds
   */
  constructor(options = {}) {
    this.#ttl = options.ttl ?? 36e5;
  }
  /**
   * @param url - optional URL to clear. if omitted, clears all cached documents.
   */
  clear(url) {
    if (url) {
      this.#cache.delete(url);
    } else {
      this.#cache.clear();
    }
  }
  /**
   * fetches a discovery document from the given URL.
   * uses cached value if available and not expired.
   * coalesces concurrent requests for the same URL to prevent duplicate fetches.
   *
   * @param url - the discovery document URL (e.g., /.well-known/openid-configuration)
   * @returns the discovery document as a JSON object
   * @throws Error if the fetch fails or returns non-OK status
   */
  async get(url) {
    const now = Date.now();
    const cached = this.#cache.get(url);
    if (cached && cached.expiresAt > now) {
      return cached.data;
    }
    const inFlight = this.#inFlight.get(url);
    if (inFlight) {
      return inFlight;
    }
    const fetchPromise = this.#fetchAndCache(url);
    this.#inFlight.set(url, fetchPromise);
    try {
      const data = await fetchPromise;
      return data;
    } finally {
      this.#inFlight.delete(url);
    }
  }
  /**
   * @param url - the URL to check
   * @returns true if the URL is cached and nott expired
   */
  has(url) {
    const cached = this.#cache.get(url);
    if (!cached) {
      return false;
    }
    const now = Date.now();
    if (cached.expiresAt <= now) {
      this.#cache.delete(url);
      return false;
    }
    return true;
  }
  async #fetchAndCache(url) {
    const res = await fetch(url);
    if (!res.ok) {
      throw new Error(
        `Failed to fetch discovery document from ${url}: ${res.status} ${res.statusText}`
      );
    }
    const data = await res.json();
    const expiresAt = Date.now() + this.#ttl;
    this.#cache.set(url, {
      data,
      expiresAt
    });
    return data;
  }
};

// src/FastMCP.ts
var imageContent = async (input) => {
  let rawData;
  try {
    if ("url" in input) {
      try {
        const response = await fetch2(input.url);
        if (!response.ok) {
          throw new Error(
            `Server responded with status: ${response.status} - ${response.statusText}`
          );
        }
        rawData = Buffer.from(await response.arrayBuffer());
      } catch (error) {
        throw new Error(
          `Failed to fetch image from URL (${input.url}): ${error instanceof Error ? error.message : String(error)}`
        );
      }
    } else if ("path" in input) {
      try {
        rawData = await readFile(input.path);
      } catch (error) {
        throw new Error(
          `Failed to read image from path (${input.path}): ${error instanceof Error ? error.message : String(error)}`
        );
      }
    } else if ("buffer" in input) {
      rawData = input.buffer;
    } else {
      throw new Error(
        "Invalid input: Provide a valid 'url', 'path', or 'buffer'"
      );
    }
    const { fileTypeFromBuffer } = await import("file-type");
    const mimeType = await fileTypeFromBuffer(rawData);
    if (!mimeType || !mimeType.mime.startsWith("image/")) {
      console.warn(
        `Warning: Content may not be a valid image. Detected MIME: ${mimeType?.mime || "unknown"}`
      );
    }
    const base64Data = rawData.toString("base64");
    return {
      data: base64Data,
      mimeType: mimeType?.mime ?? "image/png",
      type: "image"
    };
  } catch (error) {
    if (error instanceof Error) {
      throw error;
    } else {
      throw new Error(`Unexpected error processing image: ${String(error)}`);
    }
  }
};
var audioContent = async (input) => {
  let rawData;
  try {
    if ("url" in input) {
      try {
        const response = await fetch2(input.url);
        if (!response.ok) {
          throw new Error(
            `Server responded with status: ${response.status} - ${response.statusText}`
          );
        }
        rawData = Buffer.from(await response.arrayBuffer());
      } catch (error) {
        throw new Error(
          `Failed to fetch audio from URL (${input.url}): ${error instanceof Error ? error.message : String(error)}`
        );
      }
    } else if ("path" in input) {
      try {
        rawData = await readFile(input.path);
      } catch (error) {
        throw new Error(
          `Failed to read audio from path (${input.path}): ${error instanceof Error ? error.message : String(error)}`
        );
      }
    } else if ("buffer" in input) {
      rawData = input.buffer;
    } else {
      throw new Error(
        "Invalid input: Provide a valid 'url', 'path', or 'buffer'"
      );
    }
    const { fileTypeFromBuffer } = await import("file-type");
    const mimeType = await fileTypeFromBuffer(rawData);
    if (!mimeType || !mimeType.mime.startsWith("audio/")) {
      console.warn(
        `Warning: Content may not be a valid audio file. Detected MIME: ${mimeType?.mime || "unknown"}`
      );
    }
    const base64Data = rawData.toString("base64");
    return {
      data: base64Data,
      mimeType: mimeType?.mime ?? "audio/mpeg",
      type: "audio"
    };
  } catch (error) {
    if (error instanceof Error) {
      throw error;
    } else {
      throw new Error(`Unexpected error processing audio: ${String(error)}`);
    }
  }
};
var FastMCPError = class extends Error {
  constructor(message) {
    super(message);
    this.name = new.target.name;
  }
};
var UnexpectedStateError = class extends FastMCPError {
  extras;
  constructor(message, extras) {
    super(message);
    this.name = new.target.name;
    this.extras = extras;
  }
};
var UserError = class extends UnexpectedStateError {
};
var TextContentZodSchema = z.object({
  /**
   * The text content of the message.
   */
  text: z.string(),
  type: z.literal("text")
}).strict();
var ImageContentZodSchema = z.object({
  /**
   * The base64-encoded image data.
   */
  data: z.string().base64(),
  /**
   * The MIME type of the image. Different providers may support different image types.
   */
  mimeType: z.string(),
  type: z.literal("image")
}).strict();
var AudioContentZodSchema = z.object({
  /**
   * The base64-encoded audio data.
   */
  data: z.string().base64(),
  mimeType: z.string(),
  type: z.literal("audio")
}).strict();
var ResourceContentZodSchema = z.object({
  resource: z.object({
    blob: z.string().optional(),
    mimeType: z.string().optional(),
    text: z.string().optional(),
    uri: z.string()
  }),
  type: z.literal("resource")
}).strict();
var ResourceLinkZodSchema = z.object({
  description: z.string().optional(),
  mimeType: z.string().optional(),
  name: z.string(),
  title: z.string().optional(),
  type: z.literal("resource_link"),
  uri: z.string()
});
var ContentZodSchema = z.discriminatedUnion("type", [
  TextContentZodSchema,
  ImageContentZodSchema,
  AudioContentZodSchema,
  ResourceContentZodSchema,
  ResourceLinkZodSchema
]);
var ContentResultZodSchema = z.object({
  content: ContentZodSchema.array(),
  isError: z.boolean().optional()
}).strict();
var CompletionZodSchema = z.object({
  /**
   * Indicates whether there are additional completion options beyond those provided in the current response, even if the exact total is unknown.
   */
  hasMore: z.optional(z.boolean()),
  /**
   * The total number of completion options available. This can exceed the number of values actually sent in the response.
   */
  total: z.optional(z.number().int()),
  /**
   * An array of completion values. Must not exceed 100 items.
   */
  values: z.array(z.string()).max(100)
});
var FastMCPSessionEventEmitterBase = EventEmitter;
var ServerState = /* @__PURE__ */ ((ServerState2) => {
  ServerState2["Error"] = "error";
  ServerState2["Running"] = "running";
  ServerState2["Stopped"] = "stopped";
  return ServerState2;
})(ServerState || {});
var FastMCPSessionEventEmitter = class extends FastMCPSessionEventEmitterBase {
};
var FastMCPSession = class extends FastMCPSessionEventEmitter {
  get clientCapabilities() {
    return this.#clientCapabilities ?? null;
  }
  get isReady() {
    return this.#connectionState === "ready";
  }
  get loggingLevel() {
    return this.#loggingLevel;
  }
  get roots() {
    return this.#roots;
  }
  get server() {
    return this.#server;
  }
  get sessionId() {
    return this.#sessionId;
  }
  set sessionId(value) {
    this.#sessionId = value;
  }
  #auth;
  #capabilities = {};
  #clientCapabilities;
  #connectionState = "connecting";
  #logger;
  #loggingLevel = "info";
  #needsEventLoopFlush = false;
  #pingConfig;
  #pingInterval = null;
  #prompts = /* @__PURE__ */ new Map();
  #resources = /* @__PURE__ */ new Map();
  #resourceTemplates = /* @__PURE__ */ new Map();
  #roots = [];
  #rootsConfig;
  #server;
  /**
   * Session ID from the Mcp-Session-Id header (HTTP transports only).
   * Used to track per-session state across multiple requests.
   */
  #sessionId;
  #utils;
  constructor({
    auth,
    instructions,
    logger,
    name,
    ping,
    prompts,
    resources,
    resourcesTemplates,
    roots,
    sessionId,
    tools,
    transportType,
    utils,
    version
  }) {
    super();
    this.#auth = auth;
    this.#logger = logger;
    this.#pingConfig = ping;
    this.#rootsConfig = roots;
    this.#sessionId = sessionId;
    this.#needsEventLoopFlush = transportType === "httpStream";
    if (tools.length) {
      this.#capabilities.tools = {};
    }
    if (resources.length || resourcesTemplates.length) {
      this.#capabilities.resources = {};
    }
    if (prompts.length) {
      for (const prompt of prompts) {
        this.addPrompt(prompt);
      }
      this.#capabilities.prompts = {};
    }
    this.#capabilities.logging = {};
    this.#capabilities.completions = {};
    this.#server = new Server(
      { name, version },
      { capabilities: this.#capabilities, instructions }
    );
    this.#utils = utils;
    this.setupErrorHandling();
    this.setupLoggingHandlers();
    this.setupRootsHandlers();
    this.setupCompleteHandlers();
    if (tools.length) {
      this.setupToolHandlers(tools);
    }
    if (resources.length || resourcesTemplates.length) {
      for (const resource of resources) {
        this.addResource(resource);
      }
      this.setupResourceHandlers();
      if (resourcesTemplates.length) {
        for (const resourceTemplate of resourcesTemplates) {
          this.addResourceTemplate(resourceTemplate);
        }
        this.setupResourceTemplateHandlers();
      }
    }
    if (prompts.length) {
      this.setupPromptHandlers();
    }
  }
  async close() {
    this.#connectionState = "closed";
    if (this.#pingInterval) {
      clearInterval(this.#pingInterval);
    }
    try {
      await this.#server.close();
    } catch (error) {
      this.#logger.error("[FastMCP error]", "could not close server", error);
    }
  }
  async connect(transport) {
    if (this.#server.transport) {
      throw new UnexpectedStateError("Server is already connected");
    }
    this.#connectionState = "connecting";
    try {
      await this.#server.connect(transport);
      if ("sessionId" in transport) {
        const transportWithSessionId = transport;
        if (typeof transportWithSessionId.sessionId === "string") {
          this.#sessionId = transportWithSessionId.sessionId;
        }
      }
      let attempt = 0;
      const maxAttempts = 10;
      const retryDelay = 100;
      while (attempt++ < maxAttempts) {
        const capabilities = this.#server.getClientCapabilities();
        if (capabilities) {
          this.#clientCapabilities = capabilities;
          break;
        }
        await delay(retryDelay);
      }
      if (!this.#clientCapabilities) {
        this.#logger.warn(
          `[FastMCP warning] could not infer client capabilities after ${maxAttempts} attempts. Connection may be unstable.`
        );
      }
      if (this.#rootsConfig?.enabled !== false && this.#clientCapabilities?.roots?.listChanged && typeof this.#server.listRoots === "function") {
        try {
          const roots = await this.#server.listRoots();
          this.#roots = roots?.roots || [];
        } catch (e) {
          if (e instanceof McpError && e.code === ErrorCode.MethodNotFound) {
            this.#logger.debug(
              "[FastMCP debug] listRoots method not supported by client"
            );
          } else {
            this.#logger.error(
              `[FastMCP error] received error listing roots.

${e instanceof Error ? e.stack : JSON.stringify(e)}`
            );
          }
        }
      }
      if (this.#clientCapabilities) {
        const pingConfig = this.#getPingConfig(transport);
        if (pingConfig.enabled) {
          this.#pingInterval = setInterval(async () => {
            try {
              await this.#server.ping();
            } catch {
              const logLevel = pingConfig.logLevel;
              if (logLevel === "debug") {
                this.#logger.debug("[FastMCP debug] server ping failed");
              } else if (logLevel === "warning") {
                this.#logger.warn(
                  "[FastMCP warning] server is not responding to ping"
                );
              } else if (logLevel === "error") {
                this.#logger.error(
                  "[FastMCP error] server is not responding to ping"
                );
              } else {
                this.#logger.info("[FastMCP info] server ping failed");
              }
            }
          }, pingConfig.intervalMs);
        }
      }
      this.#connectionState = "ready";
      this.emit("ready");
    } catch (error) {
      this.#connectionState = "error";
      const errorEvent = {
        error: error instanceof Error ? error : new Error(String(error))
      };
      this.emit("error", errorEvent);
      throw error;
    }
  }
  promptsListChanged(prompts) {
    this.#prompts.clear();
    for (const prompt of prompts) {
      this.addPrompt(prompt);
    }
    this.setupPromptHandlers();
    this.triggerListChangedNotification("notifications/prompts/list_changed");
  }
  async requestSampling(message, options) {
    return this.#server.createMessage(message, options);
  }
  resourcesListChanged(resources) {
    this.#resources.clear();
    for (const resource of resources) {
      this.addResource(resource);
    }
    this.setupResourceHandlers();
    this.triggerListChangedNotification("notifications/resources/list_changed");
  }
  resourceTemplatesListChanged(resourceTemplates) {
    this.#resourceTemplates.clear();
    for (const resourceTemplate of resourceTemplates) {
      this.addResourceTemplate(resourceTemplate);
    }
    this.setupResourceTemplateHandlers();
    this.triggerListChangedNotification("notifications/resources/list_changed");
  }
  toolsListChanged(tools) {
    const allowedTools = tools.filter(
      (tool) => tool.canAccess ? tool.canAccess(this.#auth) : true
    );
    this.setupToolHandlers(allowedTools);
    this.triggerListChangedNotification("notifications/tools/list_changed");
  }
  async triggerListChangedNotification(method) {
    try {
      await this.#server.notification({
        method
      });
    } catch (error) {
      this.#logger.error(
        `[FastMCP error] failed to send ${method} notification.

${error instanceof Error ? error.stack : JSON.stringify(error)}`
      );
    }
  }
  /**
   * Update the session's authentication context.
   * Called by mcp-proxy when a new token is validated on subsequent requests.
   */
  updateAuth(auth) {
    this.#auth = auth;
  }
  waitForReady() {
    if (this.isReady) {
      return Promise.resolve();
    }
    if (this.#connectionState === "error" || this.#connectionState === "closed") {
      return Promise.reject(
        new Error(`Connection is in ${this.#connectionState} state`)
      );
    }
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(
          new Error(
            "Connection timeout: Session failed to become ready within 5 seconds"
          )
        );
      }, 5e3);
      this.once("ready", () => {
        clearTimeout(timeout);
        resolve();
      });
      this.once("error", (event) => {
        clearTimeout(timeout);
        reject(event.error);
      });
    });
  }
  #getPingConfig(transport) {
    const pingConfig = this.#pingConfig || {};
    let defaultEnabled = false;
    if ("type" in transport) {
      if (transport.type === "httpStream") {
        defaultEnabled = true;
      }
    }
    return {
      enabled: pingConfig.enabled !== void 0 ? pingConfig.enabled : defaultEnabled,
      intervalMs: pingConfig.intervalMs || 5e3,
      logLevel: pingConfig.logLevel || "debug"
    };
  }
  addPrompt(inputPrompt) {
    const completers = {};
    const enums = {};
    const fuseInstances = {};
    for (const argument of inputPrompt.arguments ?? []) {
      if (argument.complete) {
        completers[argument.name] = argument.complete;
      }
      if (argument.enum) {
        enums[argument.name] = argument.enum;
        fuseInstances[argument.name] = new Fuse(argument.enum, {
          includeScore: true,
          threshold: 0.3
          // More flexible matching!
        });
      }
    }
    const prompt = {
      ...inputPrompt,
      complete: async (name, value, auth) => {
        if (completers[name]) {
          return await completers[name](value, auth);
        }
        if (inputPrompt.complete) {
          return await inputPrompt.complete(name, value, auth);
        }
        if (fuseInstances[name]) {
          const result = fuseInstances[name].search(value);
          return {
            total: result.length,
            values: result.map((item) => item.item)
          };
        }
        return {
          values: []
        };
      }
    };
    this.#prompts.set(prompt.name, prompt);
  }
  addResource(inputResource) {
    this.#resources.set(inputResource.uri, inputResource);
  }
  addResourceTemplate(inputResourceTemplate) {
    const completers = {};
    for (const argument of inputResourceTemplate.arguments ?? []) {
      if (argument.complete) {
        completers[argument.name] = argument.complete;
      }
    }
    const resourceTemplate = {
      ...inputResourceTemplate,
      complete: async (name, value, auth) => {
        if (completers[name]) {
          return await completers[name](value, auth);
        }
        if (inputResourceTemplate.complete) {
          return await inputResourceTemplate.complete(name, value, auth);
        }
        return {
          values: []
        };
      }
    };
    this.#resourceTemplates.set(resourceTemplate.name, resourceTemplate);
  }
  setupCompleteHandlers() {
    this.#server.setRequestHandler(CompleteRequestSchema, async (request) => {
      if (request.params.ref.type === "ref/prompt") {
        const ref = request.params.ref;
        const prompt = "name" in ref && this.#prompts.get(ref.name);
        if (!prompt) {
          throw new UnexpectedStateError("Unknown prompt", {
            request
          });
        }
        if (!prompt.complete) {
          throw new UnexpectedStateError("Prompt does not support completion", {
            request
          });
        }
        const completion = CompletionZodSchema.parse(
          await prompt.complete(
            request.params.argument.name,
            request.params.argument.value,
            this.#auth
          )
        );
        return {
          completion
        };
      }
      if (request.params.ref.type === "ref/resource") {
        const ref = request.params.ref;
        const resource = "uri" in ref && Array.from(this.#resourceTemplates.values()).find(
          (resource2) => resource2.uriTemplate === ref.uri
        );
        if (!resource) {
          throw new UnexpectedStateError("Unknown resource", {
            request
          });
        }
        if (!("uriTemplate" in resource)) {
          throw new UnexpectedStateError("Unexpected resource");
        }
        if (!resource.complete) {
          throw new UnexpectedStateError(
            "Resource does not support completion",
            {
              request
            }
          );
        }
        const completion = CompletionZodSchema.parse(
          await resource.complete(
            request.params.argument.name,
            request.params.argument.value,
            this.#auth
          )
        );
        return {
          completion
        };
      }
      throw new UnexpectedStateError("Unexpected completion request", {
        request
      });
    });
  }
  setupErrorHandling() {
    this.#server.onerror = (error) => {
      this.#logger.error("[FastMCP error]", error);
    };
  }
  setupLoggingHandlers() {
    this.#server.setRequestHandler(SetLevelRequestSchema, (request) => {
      this.#loggingLevel = request.params.level;
      return {};
    });
  }
  setupPromptHandlers() {
    let cachedPromptsList = null;
    this.#server.setRequestHandler(ListPromptsRequestSchema, async () => {
      if (cachedPromptsList) {
        return {
          prompts: cachedPromptsList
        };
      }
      cachedPromptsList = Array.from(this.#prompts.values()).map((prompt) => {
        return {
          arguments: prompt.arguments,
          complete: prompt.complete,
          description: prompt.description,
          name: prompt.name
        };
      });
      return {
        prompts: cachedPromptsList
      };
    });
    this.#server.setRequestHandler(GetPromptRequestSchema, async (request) => {
      const prompt = this.#prompts.get(request.params.name);
      if (!prompt) {
        throw new McpError(
          ErrorCode.MethodNotFound,
          `Unknown prompt: ${request.params.name}`
        );
      }
      const args = request.params.arguments;
      for (const arg of prompt.arguments ?? []) {
        if (arg.required && !(args && arg.name in args)) {
          throw new McpError(
            ErrorCode.InvalidRequest,
            `Prompt '${request.params.name}' requires argument '${arg.name}': ${arg.description || "No description provided"}`
          );
        }
      }
      let result;
      try {
        result = await prompt.load(
          args,
          this.#auth
        );
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        throw new McpError(
          ErrorCode.InternalError,
          `Failed to load prompt '${request.params.name}': ${errorMessage}`
        );
      }
      if (typeof result === "string") {
        return {
          description: prompt.description,
          messages: [
            {
              content: { text: result, type: "text" },
              role: "user"
            }
          ]
        };
      } else {
        return {
          description: prompt.description,
          messages: result.messages
        };
      }
    });
  }
  setupResourceHandlers() {
    let cachedResourcesList = null;
    this.#server.setRequestHandler(ListResourcesRequestSchema, async () => {
      if (cachedResourcesList) {
        return {
          resources: cachedResourcesList
        };
      }
      cachedResourcesList = Array.from(this.#resources.values()).map(
        (resource) => ({
          description: resource.description,
          mimeType: resource.mimeType,
          name: resource.name,
          uri: resource.uri
        })
      );
      return {
        resources: cachedResourcesList
      };
    });
    this.#server.setRequestHandler(
      ReadResourceRequestSchema,
      async (request) => {
        if ("uri" in request.params) {
          const resource = this.#resources.get(request.params.uri);
          if (!resource) {
            for (const resourceTemplate of this.#resourceTemplates.values()) {
              const uriTemplate = parseURITemplate(
                resourceTemplate.uriTemplate
              );
              const match = uriTemplate.fromUri(request.params.uri);
              if (!match) {
                continue;
              }
              const uri = uriTemplate.fill(match);
              const result = await resourceTemplate.load(match, this.#auth);
              const resources = Array.isArray(result) ? result : [result];
              return {
                contents: resources.map((resource2) => ({
                  ...resource2,
                  description: resourceTemplate.description,
                  mimeType: resource2.mimeType ?? resourceTemplate.mimeType,
                  name: resourceTemplate.name,
                  uri: resource2.uri ?? uri
                }))
              };
            }
            throw new McpError(
              ErrorCode.MethodNotFound,
              `Resource not found: '${request.params.uri}'. Available resources: ${Array.from(this.#resources.values()).map((r) => r.uri).join(", ") || "none"}`
            );
          }
          if (!("uri" in resource)) {
            throw new UnexpectedStateError("Resource does not support reading");
          }
          let maybeArrayResult;
          try {
            maybeArrayResult = await resource.load(this.#auth);
          } catch (error) {
            const errorMessage = error instanceof Error ? error.message : String(error);
            throw new McpError(
              ErrorCode.InternalError,
              `Failed to load resource '${resource.name}' (${resource.uri}): ${errorMessage}`,
              {
                uri: resource.uri
              }
            );
          }
          const resourceResults = Array.isArray(maybeArrayResult) ? maybeArrayResult : [maybeArrayResult];
          return {
            contents: resourceResults.map((result) => ({
              ...result,
              mimeType: result.mimeType ?? resource.mimeType,
              name: resource.name,
              uri: result.uri ?? resource.uri
            }))
          };
        }
        throw new UnexpectedStateError("Unknown resource request", {
          request
        });
      }
    );
  }
  setupResourceTemplateHandlers() {
    let cachedResourceTemplatesList = null;
    this.#server.setRequestHandler(
      ListResourceTemplatesRequestSchema,
      async () => {
        if (cachedResourceTemplatesList) {
          return {
            resourceTemplates: cachedResourceTemplatesList
          };
        }
        cachedResourceTemplatesList = Array.from(
          this.#resourceTemplates.values()
        ).map((resourceTemplate) => ({
          description: resourceTemplate.description,
          mimeType: resourceTemplate.mimeType,
          name: resourceTemplate.name,
          uriTemplate: resourceTemplate.uriTemplate
        }));
        return {
          resourceTemplates: cachedResourceTemplatesList
        };
      }
    );
  }
  setupRootsHandlers() {
    if (this.#rootsConfig?.enabled === false) {
      this.#logger.debug(
        "[FastMCP debug] roots capability explicitly disabled via config"
      );
      return;
    }
    if (typeof this.#server.listRoots === "function") {
      this.#server.setNotificationHandler(
        RootsListChangedNotificationSchema,
        () => {
          this.#server.listRoots().then((roots) => {
            this.#roots = roots.roots;
            this.emit("rootsChanged", {
              roots: roots.roots
            });
          }).catch((error) => {
            if (error instanceof McpError && error.code === ErrorCode.MethodNotFound) {
              this.#logger.debug(
                "[FastMCP debug] listRoots method not supported by client"
              );
            } else {
              this.#logger.error(
                `[FastMCP error] received error listing roots.

${error instanceof Error ? error.stack : JSON.stringify(error)}`
              );
            }
          });
        }
      );
    } else {
      this.#logger.debug(
        "[FastMCP debug] roots capability not available, not setting up notification handler"
      );
    }
  }
  setupToolHandlers(tools) {
    const toolsMap = new Map(tools.map((tool) => [tool.name, tool]));
    let cachedToolsList = null;
    this.#server.setRequestHandler(ListToolsRequestSchema, async () => {
      if (cachedToolsList) {
        return {
          tools: cachedToolsList
        };
      }
      cachedToolsList = await Promise.all(
        tools.map(async (tool) => {
          return {
            annotations: tool.annotations,
            description: tool.description,
            inputSchema: tool.parameters ? await toJsonSchema(tool.parameters) : {
              additionalProperties: false,
              properties: {},
              type: "object"
            },
            name: tool.name,
            ...tool.outputSchema && {
              outputSchema: await toJsonSchema(
                tool.outputSchema
              )
            },
            // Pass through _meta for MCP ext-apps UI support (issue #229)
            ...tool._meta && { _meta: tool._meta }
          };
        })
      );
      return {
        tools: cachedToolsList
      };
    });
    this.#server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const tool = toolsMap.get(request.params.name);
      if (!tool) {
        throw new McpError(
          ErrorCode.MethodNotFound,
          `Unknown tool: ${request.params.name}`
        );
      }
      let args = void 0;
      if (tool.parameters) {
        const parsed = await tool.parameters["~standard"].validate(
          request.params.arguments
        );
        if (parsed.issues) {
          const friendlyErrors = this.#utils?.formatInvalidParamsErrorMessage ? this.#utils.formatInvalidParamsErrorMessage(parsed.issues) : parsed.issues.map((issue) => {
            const path = issue.path?.join(".") || "root";
            return `${path}: ${issue.message}`;
          }).join(", ");
          throw new McpError(
            ErrorCode.InvalidParams,
            `Tool '${request.params.name}' parameter validation failed: ${friendlyErrors}. Please check the parameter types and values according to the tool's schema.`
          );
        }
        args = parsed.value;
      }
      const progressToken = request.params?._meta?.progressToken;
      let result;
      try {
        const reportProgress = async (progress) => {
          try {
            await this.#server.notification({
              method: "notifications/progress",
              params: {
                ...progress,
                progressToken
              }
            });
            if (this.#needsEventLoopFlush) {
              await new Promise((resolve) => setImmediate(resolve));
            }
          } catch (progressError) {
            this.#logger.warn(
              `[FastMCP warning] Failed to report progress for tool '${request.params.name}':`,
              progressError instanceof Error ? progressError.message : String(progressError)
            );
          }
        };
        const log = {
          debug: (message, context) => {
            this.#server.sendLoggingMessage({
              data: {
                context,
                message
              },
              level: "debug"
            });
          },
          error: (message, context) => {
            this.#server.sendLoggingMessage({
              data: {
                context,
                message
              },
              level: "error"
            });
          },
          info: (message, context) => {
            this.#server.sendLoggingMessage({
              data: {
                context,
                message
              },
              level: "info"
            });
          },
          warn: (message, context) => {
            this.#server.sendLoggingMessage({
              data: {
                context,
                message
              },
              level: "warning"
            });
          }
        };
        const streamContent = async (content) => {
          const contentArray = Array.isArray(content) ? content : [content];
          try {
            await this.#server.notification({
              method: "notifications/tool/streamContent",
              params: {
                content: contentArray,
                toolName: request.params.name
              }
            });
            if (this.#needsEventLoopFlush) {
              await new Promise((resolve) => setImmediate(resolve));
            }
          } catch (streamError) {
            this.#logger.warn(
              `[FastMCP warning] Failed to stream content for tool '${request.params.name}':`,
              streamError instanceof Error ? streamError.message : String(streamError)
            );
          }
        };
        const executeToolPromise = tool.execute(args, {
          client: {
            version: this.#server.getClientVersion()
          },
          log,
          reportProgress,
          requestId: typeof request.params?._meta?.requestId === "string" ? request.params._meta.requestId : void 0,
          session: this.#auth,
          sessionId: this.#sessionId,
          streamContent
        });
        const maybeStringResult = await (tool.timeoutMs ? Promise.race([
          executeToolPromise,
          new Promise((_, reject) => {
            const timeoutId = setTimeout(() => {
              reject(
                new UserError(
                  `Tool '${request.params.name}' timed out after ${tool.timeoutMs}ms. Consider increasing timeoutMs or optimizing the tool implementation.`
                )
              );
            }, tool.timeoutMs);
            executeToolPromise.then(
              () => clearTimeout(timeoutId),
              () => clearTimeout(timeoutId)
            );
          })
        ]) : executeToolPromise);
        await delay(1);
        if (maybeStringResult === void 0 || maybeStringResult === null) {
          result = ContentResultZodSchema.parse({
            content: []
          });
        } else if (typeof maybeStringResult === "string") {
          result = ContentResultZodSchema.parse({
            content: [{ text: maybeStringResult, type: "text" }]
          });
        } else if ("type" in maybeStringResult) {
          result = ContentResultZodSchema.parse({
            content: [maybeStringResult]
          });
        } else {
          result = ContentResultZodSchema.parse(maybeStringResult);
        }
      } catch (error) {
        if (error instanceof UserError) {
          return {
            content: [{ text: error.message, type: "text" }],
            isError: true,
            ...error.extras ? { structuredContent: error.extras } : {}
          };
        }
        const errorMessage = error instanceof Error ? error.message : String(error);
        return {
          content: [
            {
              text: `Tool '${request.params.name}' execution failed: ${errorMessage}`,
              type: "text"
            }
          ],
          isError: true
        };
      }
      return result;
    });
  }
};
function camelToSnakeCase(str) {
  return str.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);
}
function convertObjectToSnakeCase(obj) {
  const result = {};
  for (const [key, value] of Object.entries(obj)) {
    const snakeKey = camelToSnakeCase(key);
    result[snakeKey] = value;
  }
  return result;
}
function parseBasicAuthHeader(authHeader) {
  const basicMatch = authHeader?.match(/^Basic\s+(.+)$/);
  if (!basicMatch) return null;
  try {
    const credentials = Buffer.from(basicMatch[1], "base64").toString("utf-8");
    const credMatch = credentials.match(/^([^:]+):(.*)$/);
    if (!credMatch) return null;
    return { clientId: credMatch[1], clientSecret: credMatch[2] };
  } catch {
    return null;
  }
}
var FastMCPEventEmitterBase = EventEmitter;
var FastMCPEventEmitter = class extends FastMCPEventEmitterBase {
};
var FastMCP = class extends FastMCPEventEmitter {
  constructor(options) {
    super();
    this.options = options;
    this.#options = options;
    this.#logger = options.logger || console;
    if (options.auth) {
      if (!options.authenticate) {
        this.#authenticate = ((request) => options.auth.authenticate(request));
      } else {
        this.#authenticate = options.authenticate;
      }
      if (!options.oauth) {
        this.#options = {
          ...options,
          oauth: options.auth.getOAuthConfig()
        };
      }
    } else {
      this.#authenticate = options.authenticate;
    }
  }
  get serverState() {
    return this.#serverState;
  }
  get sessions() {
    return this.#sessions;
  }
  #authenticate;
  #honoApp = new Hono();
  #httpStreamServer = null;
  #logger;
  #options;
  #prompts = [];
  #resources = [];
  #resourcesTemplates = [];
  #serverState = "stopped" /* Stopped */;
  #sessions = [];
  #tools = [];
  /**
   * Adds a prompt to the server.
   */
  addPrompt(prompt) {
    this.#prompts = this.#prompts.filter((p) => p.name !== prompt.name);
    this.#prompts.push(prompt);
    if (this.#serverState === "running" /* Running */) {
      this.#promptsListChanged(this.#prompts);
    }
  }
  /**
   * Adds prompts to the server.
   */
  addPrompts(prompts) {
    const newPromptNames = new Set(prompts.map((prompt) => prompt.name));
    this.#prompts = this.#prompts.filter((p) => !newPromptNames.has(p.name));
    this.#prompts.push(...prompts);
    if (this.#serverState === "running" /* Running */) {
      this.#promptsListChanged(this.#prompts);
    }
  }
  /**
   * Adds a resource to the server.
   */
  addResource(resource) {
    this.#resources = this.#resources.filter((r) => r.name !== resource.name);
    this.#resources.push(resource);
    if (this.#serverState === "running" /* Running */) {
      this.#resourcesListChanged(this.#resources);
    }
  }
  /**
   * Adds resources to the server.
   */
  addResources(resources) {
    const newResourceNames = new Set(
      resources.map((resource) => resource.name)
    );
    this.#resources = this.#resources.filter(
      (r) => !newResourceNames.has(r.name)
    );
    this.#resources.push(...resources);
    if (this.#serverState === "running" /* Running */) {
      this.#resourcesListChanged(this.#resources);
    }
  }
  /**
   * Adds a resource template to the server.
   */
  addResourceTemplate(resource) {
    this.#resourcesTemplates = this.#resourcesTemplates.filter(
      (t) => t.name !== resource.name
    );
    this.#resourcesTemplates.push(resource);
    if (this.#serverState === "running" /* Running */) {
      this.#resourceTemplatesListChanged(this.#resourcesTemplates);
    }
  }
  /**
   * Adds resource templates to the server.
   */
  addResourceTemplates(resources) {
    const newResourceTemplateNames = new Set(
      resources.map((resource) => resource.name)
    );
    this.#resourcesTemplates = this.#resourcesTemplates.filter(
      (t) => !newResourceTemplateNames.has(t.name)
    );
    this.#resourcesTemplates.push(...resources);
    if (this.#serverState === "running" /* Running */) {
      this.#resourceTemplatesListChanged(this.#resourcesTemplates);
    }
  }
  /**
   * Adds a tool to the server.
   */
  addTool(tool) {
    this.#tools = this.#tools.filter((t) => t.name !== tool.name);
    this.#tools.push(tool);
    if (this.#serverState === "running" /* Running */) {
      this.#toolsListChanged(this.#tools);
    }
  }
  /**
   * Adds tools to the server.
   */
  addTools(tools) {
    const newToolNames = new Set(tools.map((tool) => tool.name));
    this.#tools = this.#tools.filter((t) => !newToolNames.has(t.name));
    this.#tools.push(...tools);
    if (this.#serverState === "running" /* Running */) {
      this.#toolsListChanged(this.#tools);
    }
  }
  /**
   * Embeds a resource by URI, making it easy to include resources in tool responses.
   *
   * @param uri - The URI of the resource to embed
   * @returns Promise<ResourceContent> - The embedded resource content
   */
  async embedded(uri) {
    const directResource = this.#resources.find(
      (resource) => resource.uri === uri
    );
    if (directResource) {
      const result = await directResource.load();
      const results = Array.isArray(result) ? result : [result];
      const firstResult = results[0];
      const resourceData = {
        mimeType: directResource.mimeType,
        uri
      };
      if ("text" in firstResult) {
        resourceData.text = firstResult.text;
      }
      if ("blob" in firstResult) {
        resourceData.blob = firstResult.blob;
      }
      return resourceData;
    }
    for (const template of this.#resourcesTemplates) {
      const parsedTemplate = parseURITemplate(template.uriTemplate);
      const params = parsedTemplate.fromUri(uri);
      if (!params) {
        continue;
      }
      const result = await template.load(
        params
      );
      const resourceData = {
        mimeType: template.mimeType,
        uri
      };
      if ("text" in result) {
        resourceData.text = result.text;
      }
      if ("blob" in result) {
        resourceData.blob = result.blob;
      }
      return resourceData;
    }
    throw new UnexpectedStateError(`Resource not found: ${uri}`, { uri });
  }
  /**
   * Returns the underlying Hono app instance for direct access to Hono's native API.
   * This allows you to add custom routes, middleware, and handlers using Hono's standard methods.
   *
   * @returns The Hono app instance
   *
   * @example
   * ```typescript
   * const app = server.getApp();
   *
   * // Add routes using native Hono API
   * app.get('/api/users', async (c) => {
   *   return c.json({ users: [] });
   * });
   *
   * app.post('/api/users/:id', async (c) => {
   *   const id = c.req.param('id');
   *   return c.json({ id });
   * });
   * ```
   */
  getApp() {
    return this.#honoApp;
  }
  /**
   * Removes a prompt from the server.
   */
  removePrompt(name) {
    this.#prompts = this.#prompts.filter((p) => p.name !== name);
    if (this.#serverState === "running" /* Running */) {
      this.#promptsListChanged(this.#prompts);
    }
  }
  /**
   * Removes prompts from the server.
   */
  removePrompts(names) {
    for (const name of names) {
      this.#prompts = this.#prompts.filter((p) => p.name !== name);
    }
    if (this.#serverState === "running" /* Running */) {
      this.#promptsListChanged(this.#prompts);
    }
  }
  /**
   * Removes a resource from the server.
   */
  removeResource(name) {
    this.#resources = this.#resources.filter((r) => r.name !== name);
    if (this.#serverState === "running" /* Running */) {
      this.#resourcesListChanged(this.#resources);
    }
  }
  /**
   * Removes resources from the server.
   */
  removeResources(names) {
    for (const name of names) {
      this.#resources = this.#resources.filter((r) => r.name !== name);
    }
    if (this.#serverState === "running" /* Running */) {
      this.#resourcesListChanged(this.#resources);
    }
  }
  /**
   * Removes a resource template from the server.
   */
  removeResourceTemplate(name) {
    this.#resourcesTemplates = this.#resourcesTemplates.filter(
      (t) => t.name !== name
    );
    if (this.#serverState === "running" /* Running */) {
      this.#resourceTemplatesListChanged(this.#resourcesTemplates);
    }
  }
  /**
   * Removes resource templates from the server.
   */
  removeResourceTemplates(names) {
    for (const name of names) {
      this.#resourcesTemplates = this.#resourcesTemplates.filter(
        (t) => t.name !== name
      );
    }
    if (this.#serverState === "running" /* Running */) {
      this.#resourceTemplatesListChanged(this.#resourcesTemplates);
    }
  }
  /**
   * Removes a tool from the server.
   */
  removeTool(name) {
    this.#tools = this.#tools.filter((t) => t.name !== name);
    if (this.#serverState === "running" /* Running */) {
      this.#toolsListChanged(this.#tools);
    }
  }
  /**
   * Removes tools from the server.
   */
  removeTools(names) {
    for (const name of names) {
      this.#tools = this.#tools.filter((t) => t.name !== name);
    }
    if (this.#serverState === "running" /* Running */) {
      this.#toolsListChanged(this.#tools);
    }
  }
  /**
   * Starts the server.
   */
  async start(options) {
    const config = this.#parseRuntimeConfig(options);
    if (config.transportType === "stdio") {
      const transport = new StdioServerTransport();
      let auth;
      if (this.#authenticate) {
        try {
          auth = await this.#authenticate(
            void 0
          );
        } catch (error) {
          this.#logger.error(
            "[FastMCP error] Authentication failed for stdio transport:",
            error instanceof Error ? error.message : String(error)
          );
        }
      }
      const session = new FastMCPSession({
        auth,
        instructions: this.#options.instructions,
        logger: this.#logger,
        name: this.#options.name,
        ping: this.#options.ping,
        prompts: this.#prompts,
        resources: this.#resources,
        resourcesTemplates: this.#resourcesTemplates,
        roots: this.#options.roots,
        tools: this.#tools,
        transportType: "stdio",
        utils: this.#options.utils,
        version: this.#options.version
      });
      await session.connect(transport);
      this.#sessions.push(session);
      session.once("error", () => {
        this.#removeSession(session);
      });
      if (transport.onclose) {
        const originalOnClose = transport.onclose;
        transport.onclose = () => {
          this.#removeSession(session);
          if (originalOnClose) {
            originalOnClose();
          }
        };
      } else {
        transport.onclose = () => {
          this.#removeSession(session);
        };
      }
      this.emit("connect", {
        session
      });
      this.#serverState = "running" /* Running */;
    } else if (config.transportType === "httpStream") {
      const httpConfig = config.httpStream;
      const protocol = httpConfig.sslCert || httpConfig.sslKey ? "https" : "http";
      if (httpConfig.stateless) {
        this.#logger.info(
          `[FastMCP info] Starting server in stateless mode on HTTP Stream at ${protocol}://${httpConfig.host}:${httpConfig.port}${httpConfig.endpoint}`
        );
        this.#httpStreamServer = await startHTTPServer({
          ...this.#authenticate ? { authenticate: this.#authenticate } : {},
          createServer: async (request) => {
            let auth;
            if (this.#authenticate) {
              auth = await this.#authenticate(request);
              if (auth === void 0 || auth === null) {
                throw new Error("Authentication required");
              }
            }
            const sessionId = Array.isArray(request.headers["mcp-session-id"]) ? request.headers["mcp-session-id"][0] : request.headers["mcp-session-id"];
            return this.#createSession(auth, sessionId);
          },
          enableJsonResponse: httpConfig.enableJsonResponse,
          eventStore: httpConfig.eventStore,
          host: httpConfig.host,
          ...this.#options.oauth?.enabled && this.#options.oauth.protectedResource?.resource ? {
            oauth: {
              protectedResource: {
                resource: this.#options.oauth.protectedResource.resource
              }
            }
          } : {},
          // In stateless mode, we don't track sessions
          onClose: async () => {
          },
          onConnect: async () => {
            this.#logger.debug(
              `[FastMCP debug] Stateless HTTP Stream request handled`
            );
          },
          onUnhandledRequest: async (req, res) => {
            await this.#handleUnhandledRequest(
              req,
              res,
              true,
              httpConfig.host,
              httpConfig.endpoint
            );
          },
          port: httpConfig.port,
          sslCa: httpConfig.sslCa,
          sslCert: httpConfig.sslCert,
          sslKey: httpConfig.sslKey,
          stateless: true,
          streamEndpoint: httpConfig.endpoint
        });
      } else {
        this.#httpStreamServer = await startHTTPServer({
          ...this.#authenticate ? { authenticate: this.#authenticate } : {},
          createServer: async (request) => {
            let auth;
            if (this.#authenticate) {
              auth = await this.#authenticate(request);
            }
            const sessionId = Array.isArray(request.headers["mcp-session-id"]) ? request.headers["mcp-session-id"][0] : request.headers["mcp-session-id"];
            return this.#createSession(auth, sessionId);
          },
          enableJsonResponse: httpConfig.enableJsonResponse,
          eventStore: httpConfig.eventStore,
          host: httpConfig.host,
          ...this.#options.oauth?.enabled && this.#options.oauth.protectedResource?.resource ? {
            oauth: {
              protectedResource: {
                resource: this.#options.oauth.protectedResource.resource
              }
            }
          } : {},
          onClose: async (session) => {
            const sessionIndex = this.#sessions.indexOf(session);
            if (sessionIndex !== -1) this.#sessions.splice(sessionIndex, 1);
            this.emit("disconnect", {
              session
            });
          },
          onConnect: async (session) => {
            this.#sessions.push(session);
            this.#logger.info(`[FastMCP info] HTTP Stream session established`);
            this.emit("connect", {
              session
            });
          },
          onUnhandledRequest: async (req, res) => {
            await this.#handleUnhandledRequest(
              req,
              res,
              false,
              httpConfig.host,
              httpConfig.endpoint
            );
          },
          port: httpConfig.port,
          sslCa: httpConfig.sslCa,
          sslCert: httpConfig.sslCert,
          sslKey: httpConfig.sslKey,
          stateless: httpConfig.stateless,
          streamEndpoint: httpConfig.endpoint
        });
        this.#logger.info(
          `[FastMCP info] server is running on HTTP Stream at ${protocol}://${httpConfig.host}:${httpConfig.port}${httpConfig.endpoint}`
        );
      }
      this.#serverState = "running" /* Running */;
    } else {
      throw new Error("Invalid transport type");
    }
  }
  /**
   * Stops the server.
   */
  async stop() {
    if (this.#httpStreamServer) {
      await this.#httpStreamServer.close();
    }
    this.#serverState = "stopped" /* Stopped */;
  }
  /**
   * Creates a new FastMCPSession instance with the current configuration.
   * Used both for regular sessions and stateless requests.
   */
  #createSession(auth, sessionId) {
    if (auth && typeof auth === "object" && "authenticated" in auth && !auth.authenticated) {
      const errorMessage = "error" in auth && typeof auth.error === "string" ? auth.error : "Authentication failed";
      throw new Error(errorMessage);
    }
    const allowedTools = auth ? this.#tools.filter(
      (tool) => tool.canAccess ? tool.canAccess(auth) : true
    ) : this.#tools;
    return new FastMCPSession({
      auth,
      instructions: this.#options.instructions,
      logger: this.#logger,
      name: this.#options.name,
      ping: this.#options.ping,
      prompts: this.#prompts,
      resources: this.#resources,
      resourcesTemplates: this.#resourcesTemplates,
      roots: this.#options.roots,
      sessionId,
      tools: allowedTools,
      transportType: "httpStream",
      utils: this.#options.utils,
      version: this.#options.version
    });
  }
  /**
   * Handles unhandled HTTP requests with health, readiness, OAuth endpoints, and custom routes
   */
  #handleUnhandledRequest = async (req, res, isStateless = false, host, streamEndpoint) => {
    const url = new URL(req.url || "", `http://${host}`);
    try {
      const webRequest = this.#nodeRequestToWebRequest(req, url);
      const honoResponse = await this.#honoApp.fetch(webRequest, {
        incoming: req,
        outgoing: res
      });
      if (honoResponse.status !== 404) {
        if (!res.headersSent) {
          res.statusCode = honoResponse.status;
          honoResponse.headers.forEach((value, key) => {
            res.setHeader(key, value);
          });
          if (honoResponse.body) {
            const reader = honoResponse.body.getReader();
            try {
              while (true) {
                const { done, value } = await reader.read();
                if (done) break;
                res.write(value);
              }
            } finally {
              reader.releaseLock();
            }
          }
          res.end();
        }
        return;
      }
    } catch (error) {
      this.#logger.debug("[FastMCP debug] Hono route not matched", error);
    }
    const healthConfig = this.#options.health ?? {};
    const enabled = healthConfig.enabled === void 0 ? true : healthConfig.enabled;
    if (enabled) {
      const path = healthConfig.path ?? "/health";
      const url2 = new URL(req.url || "", `http://${host}`);
      try {
        if (req.method === "GET" && url2.pathname === path) {
          res.writeHead(healthConfig.status ?? 200, {
            "Content-Type": "text/plain"
          }).end(healthConfig.message ?? "\u2713 Ok");
          return;
        }
        if (req.method === "GET" && url2.pathname === "/ready") {
          if (isStateless) {
            const response = {
              mode: "stateless",
              ready: 1,
              status: "ready",
              total: 1
            };
            res.writeHead(200, {
              "Content-Type": "application/json"
            }).end(JSON.stringify(response));
          } else {
            const readySessions = this.#sessions.filter(
              (s) => s.isReady
            ).length;
            const totalSessions = this.#sessions.length;
            const allReady = readySessions === totalSessions && totalSessions > 0;
            const response = {
              ready: readySessions,
              status: allReady ? "ready" : totalSessions === 0 ? "no_sessions" : "initializing",
              total: totalSessions
            };
            res.writeHead(allReady ? 200 : 503, {
              "Content-Type": "application/json"
            }).end(JSON.stringify(response));
          }
          return;
        }
      } catch (error) {
        this.#logger.error("[FastMCP error] health endpoint error", error);
      }
    }
    const oauthConfig = this.#options.oauth;
    if (oauthConfig?.enabled && req.method === "GET") {
      const url2 = new URL(req.url || "", `http://${host}`);
      if (url2.pathname === "/.well-known/oauth-authorization-server" && oauthConfig.authorizationServer) {
        const metadata = convertObjectToSnakeCase(
          oauthConfig.authorizationServer
        );
        res.writeHead(200, {
          "Content-Type": "application/json"
        }).end(JSON.stringify(metadata));
        return;
      }
      if (oauthConfig.protectedResource) {
        const wellKnownBase = "/.well-known/oauth-protected-resource";
        let shouldServeMetadata = false;
        if (streamEndpoint && url2.pathname === `${wellKnownBase}${streamEndpoint}`) {
          shouldServeMetadata = true;
        } else if (url2.pathname === wellKnownBase) {
          shouldServeMetadata = true;
        }
        if (shouldServeMetadata) {
          const metadata = convertObjectToSnakeCase(
            oauthConfig.protectedResource
          );
          res.writeHead(200, {
            "Content-Type": "application/json"
          }).end(JSON.stringify(metadata));
          return;
        }
      }
    }
    const oauthProxy = oauthConfig?.proxy;
    if (oauthProxy && oauthConfig?.enabled) {
      const url2 = new URL(req.url || "", `http://${host}`);
      try {
        if (req.method === "POST" && url2.pathname === "/oauth/register") {
          let body = "";
          req.on("data", (chunk) => body += chunk);
          req.on("end", async () => {
            try {
              const request = JSON.parse(body);
              const response = await oauthProxy.registerClient(request);
              res.writeHead(201, { "Content-Type": "application/json" }).end(JSON.stringify(response));
            } catch (error) {
              const statusCode = error.statusCode || 400;
              res.writeHead(statusCode, { "Content-Type": "application/json" }).end(
                JSON.stringify(
                  error.toJSON?.() || {
                    error: "invalid_request"
                  }
                )
              );
            }
          });
          return;
        }
        if (req.method === "GET" && url2.pathname === "/oauth/authorize") {
          try {
            const params = Object.fromEntries(url2.searchParams.entries());
            const response = await oauthProxy.authorize(
              params
            );
            const location = response.headers.get("Location");
            if (location) {
              res.writeHead(response.status, { Location: location }).end();
            } else {
              const html = await response.text();
              res.writeHead(response.status, { "Content-Type": "text/html" }).end(html);
            }
          } catch (error) {
            res.writeHead(400, { "Content-Type": "application/json" }).end(
              JSON.stringify(
                error.toJSON?.() || {
                  error: "invalid_request"
                }
              )
            );
          }
          return;
        }
        if (req.method === "GET" && url2.pathname === "/oauth/callback") {
          try {
            const mockRequest = new Request(`http://${host}${req.url}`);
            const response = await oauthProxy.handleCallback(mockRequest);
            const location = response.headers.get("Location");
            if (location) {
              res.writeHead(response.status, { Location: location }).end();
            } else {
              const text = await response.text();
              res.writeHead(response.status).end(text);
            }
          } catch (error) {
            res.writeHead(400, { "Content-Type": "application/json" }).end(
              JSON.stringify(
                error.toJSON?.() || {
                  error: "server_error"
                }
              )
            );
          }
          return;
        }
        if (req.method === "POST" && url2.pathname === "/oauth/consent") {
          let body = "";
          req.on("data", (chunk) => body += chunk);
          req.on("end", async () => {
            try {
              const mockRequest = new Request(`http://${host}/oauth/consent`, {
                body,
                headers: {
                  "Content-Type": "application/x-www-form-urlencoded"
                },
                method: "POST"
              });
              const response = await oauthProxy.handleConsent(mockRequest);
              const location = response.headers.get("Location");
              if (location) {
                res.writeHead(response.status, { Location: location }).end();
              } else {
                const text = await response.text();
                res.writeHead(response.status).end(text);
              }
            } catch (error) {
              res.writeHead(400, { "Content-Type": "application/json" }).end(
                JSON.stringify(
                  error.toJSON?.() || {
                    error: "server_error"
                  }
                )
              );
            }
          });
          return;
        }
        if (req.method === "POST" && url2.pathname === "/oauth/token") {
          let body = "";
          req.on("data", (chunk) => body += chunk);
          req.on("end", async () => {
            try {
              const params = new URLSearchParams(body);
              const grantType = params.get("grant_type");
              const basicAuth = parseBasicAuthHeader(req.headers.authorization);
              const clientId = basicAuth?.clientId || params.get("client_id") || "";
              const clientSecret = basicAuth?.clientSecret ?? params.get("client_secret") ?? void 0;
              let response;
              if (grantType === "authorization_code") {
                response = await oauthProxy.exchangeAuthorizationCode({
                  client_id: clientId,
                  client_secret: clientSecret,
                  code: params.get("code") || "",
                  code_verifier: params.get("code_verifier") || void 0,
                  grant_type: "authorization_code",
                  redirect_uri: params.get("redirect_uri") || ""
                });
              } else if (grantType === "refresh_token") {
                response = await oauthProxy.exchangeRefreshToken({
                  client_id: clientId,
                  client_secret: clientSecret,
                  grant_type: "refresh_token",
                  refresh_token: params.get("refresh_token") || "",
                  scope: params.get("scope") || void 0
                });
              } else {
                throw {
                  statusCode: 400,
                  toJSON: () => ({ error: "unsupported_grant_type" })
                };
              }
              res.writeHead(200, { "Content-Type": "application/json" }).end(JSON.stringify(response));
            } catch (error) {
              const statusCode = error.statusCode || 400;
              res.writeHead(statusCode, { "Content-Type": "application/json" }).end(
                JSON.stringify(
                  error.toJSON?.() || {
                    error: "invalid_request"
                  }
                )
              );
            }
          });
          return;
        }
      } catch (error) {
        this.#logger.error("[FastMCP error] OAuth Proxy endpoint error", error);
        res.writeHead(500).end();
        return;
      }
    }
    res.writeHead(404).end();
  };
  /**
   * Converts Node.js IncomingMessage to Web Request for Hono
   */
  #nodeRequestToWebRequest(req, url) {
    const method = req.method || "GET";
    const headers = new Headers();
    for (const [key, value] of Object.entries(req.headers)) {
      if (value) {
        if (Array.isArray(value)) {
          for (const v of value) {
            headers.append(key, v);
          }
        } else {
          headers.set(key, value);
        }
      }
    }
    const hasBody = method !== "GET" && method !== "HEAD";
    if (hasBody) {
      return new Request(url.toString(), {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        body: req,
        // Node.js IncomingMessage is readable stream
        duplex: "half",
        // Required for streaming bodies
        headers,
        method
      });
    } else {
      return new Request(url.toString(), {
        headers,
        method
      });
    }
  }
  #parseRuntimeConfig(overrides) {
    const args = process.argv.slice(2);
    const getArg = (name) => {
      const index = args.findIndex((arg) => arg === `--${name}`);
      return index !== -1 && index + 1 < args.length ? args[index + 1] : void 0;
    };
    const transportArg = getArg("transport");
    const portArg = getArg("port");
    const endpointArg = getArg("endpoint");
    const statelessArg = getArg("stateless");
    const hostArg = getArg("host");
    const envTransport = process.env.FASTMCP_TRANSPORT;
    const envPort = process.env.FASTMCP_PORT;
    const envEndpoint = process.env.FASTMCP_ENDPOINT;
    const envStateless = process.env.FASTMCP_STATELESS;
    const envHost = process.env.FASTMCP_HOST;
    const transportType = overrides?.transportType || (transportArg === "http-stream" ? "httpStream" : transportArg) || envTransport || "stdio";
    if (transportType === "httpStream") {
      const port = parseInt(
        overrides?.httpStream?.port?.toString() || portArg || envPort || "8080"
      );
      const host = overrides?.httpStream?.host || hostArg || envHost || "localhost";
      const endpoint = overrides?.httpStream?.endpoint || endpointArg || envEndpoint || "/mcp";
      const enableJsonResponse = overrides?.httpStream?.enableJsonResponse || false;
      const stateless = overrides?.httpStream?.stateless || statelessArg === "true" || envStateless === "true" || false;
      const eventStore = overrides?.httpStream?.eventStore;
      const sslCa = overrides?.httpStream?.sslCa;
      const sslCert = overrides?.httpStream?.sslCert;
      const sslKey = overrides?.httpStream?.sslKey;
      return {
        httpStream: {
          enableJsonResponse,
          endpoint,
          eventStore,
          host,
          port,
          sslCa,
          sslCert,
          sslKey,
          stateless
        },
        transportType: "httpStream"
      };
    }
    return { transportType: "stdio" };
  }
  /**
   * Notifies all sessions that the prompts list has changed.
   */
  #promptsListChanged(prompts) {
    for (const session of this.#sessions) {
      session.promptsListChanged(prompts);
    }
  }
  #removeSession(session) {
    const sessionIndex = this.#sessions.indexOf(session);
    if (sessionIndex !== -1) {
      this.#sessions.splice(sessionIndex, 1);
      this.emit("disconnect", {
        session
      });
    }
  }
  /**
   * Notifies all sessions that the resources list has changed.
   */
  #resourcesListChanged(resources) {
    for (const session of this.#sessions) {
      session.resourcesListChanged(resources);
    }
  }
  /**
   * Notifies all sessions that the resource templates list has changed.
   */
  #resourceTemplatesListChanged(templates) {
    for (const session of this.#sessions) {
      session.resourceTemplatesListChanged(templates);
    }
  }
  /**
   * Notifies all sessions that the tools list has changed.
   */
  #toolsListChanged(tools) {
    for (const session of this.#sessions) {
      session.toolsListChanged(tools);
    }
  }
};

export {
  DiscoveryDocumentCache,
  imageContent,
  audioContent,
  UnexpectedStateError,
  UserError,
  ServerState,
  FastMCPSession,
  FastMCP
};
//# sourceMappingURL=chunk-DMSZ2FEE.js.map