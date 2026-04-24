import { C as NEVER, S as _coercedNumber, T as AuthenticationMiddleware, _ as looseObject, a as startHTTPServer, b as string, c as LATEST_PROTOCOL_VERSION, d as isJSONRPCResultResponse, f as ZodNumber, g as literal, h as boolean, i as Client, l as isInitializedNotification, m as array, n as serializeMessage, o as proxyServer, p as any, r as Server, s as JSONRPCMessageSchema, t as ReadBuffer, u as isJSONRPCRequest, v as number$1, w as InMemoryEventStore, x as url, y as object } from "./stdio-BmURZCbz.mjs";
import process from "node:process";

//#region node_modules/.pnpm/zod@3.25.76/node_modules/zod/v4/classic/compat.js
/** @deprecated Use the raw string literal codes instead, e.g. "invalid_type". */
const ZodIssueCode = {
	invalid_type: "invalid_type",
	too_big: "too_big",
	too_small: "too_small",
	invalid_format: "invalid_format",
	not_multiple_of: "not_multiple_of",
	unrecognized_keys: "unrecognized_keys",
	invalid_union: "invalid_union",
	invalid_key: "invalid_key",
	invalid_element: "invalid_element",
	invalid_value: "invalid_value",
	custom: "custom"
};

//#endregion
//#region node_modules/.pnpm/zod@3.25.76/node_modules/zod/v4/classic/coerce.js
function number(params) {
	return _coercedNumber(ZodNumber, params);
}

//#endregion
//#region node_modules/.pnpm/eventsource-parser@3.0.6/node_modules/eventsource-parser/dist/index.js
var ParseError = class extends Error {
	constructor(message, options) {
		super(message), this.name = "ParseError", this.type = options.type, this.field = options.field, this.value = options.value, this.line = options.line;
	}
};
function noop(_arg) {}
function createParser(callbacks) {
	if (typeof callbacks == "function") throw new TypeError("`callbacks` must be an object, got a function instead. Did you mean `{onEvent: fn}`?");
	const { onEvent = noop, onError = noop, onRetry = noop, onComment } = callbacks;
	let incompleteLine = "", isFirstChunk = !0, id, data = "", eventType = "";
	function feed(newChunk) {
		const chunk = isFirstChunk ? newChunk.replace(/^\xEF\xBB\xBF/, "") : newChunk, [complete, incomplete] = splitLines(`${incompleteLine}${chunk}`);
		for (const line of complete) parseLine(line);
		incompleteLine = incomplete, isFirstChunk = !1;
	}
	function parseLine(line) {
		if (line === "") {
			dispatchEvent();
			return;
		}
		if (line.startsWith(":")) {
			onComment && onComment(line.slice(line.startsWith(": ") ? 2 : 1));
			return;
		}
		const fieldSeparatorIndex = line.indexOf(":");
		if (fieldSeparatorIndex !== -1) {
			const field = line.slice(0, fieldSeparatorIndex), offset = line[fieldSeparatorIndex + 1] === " " ? 2 : 1;
			processField(field, line.slice(fieldSeparatorIndex + offset), line);
			return;
		}
		processField(line, "", line);
	}
	function processField(field, value, line) {
		switch (field) {
			case "event":
				eventType = value;
				break;
			case "data":
				data = `${data}${value}
`;
				break;
			case "id":
				id = value.includes("\0") ? void 0 : value;
				break;
			case "retry":
				/^\d+$/.test(value) ? onRetry(parseInt(value, 10)) : onError(new ParseError(`Invalid \`retry\` value: "${value}"`, {
					type: "invalid-retry",
					value,
					line
				}));
				break;
			default:
				onError(new ParseError(`Unknown field "${field.length > 20 ? `${field.slice(0, 20)}\u2026` : field}"`, {
					type: "unknown-field",
					field,
					value,
					line
				}));
				break;
		}
	}
	function dispatchEvent() {
		data.length > 0 && onEvent({
			id,
			event: eventType || void 0,
			data: data.endsWith(`
`) ? data.slice(0, -1) : data
		}), id = void 0, data = "", eventType = "";
	}
	function reset(options = {}) {
		incompleteLine && options.consume && parseLine(incompleteLine), isFirstChunk = !0, id = void 0, data = "", eventType = "", incompleteLine = "";
	}
	return {
		feed,
		reset
	};
}
function splitLines(chunk) {
	const lines = [];
	let incompleteLine = "", searchIndex = 0;
	for (; searchIndex < chunk.length;) {
		const crIndex = chunk.indexOf("\r", searchIndex), lfIndex = chunk.indexOf(`
`, searchIndex);
		let lineEnd = -1;
		if (crIndex !== -1 && lfIndex !== -1 ? lineEnd = Math.min(crIndex, lfIndex) : crIndex !== -1 ? crIndex === chunk.length - 1 ? lineEnd = -1 : lineEnd = crIndex : lfIndex !== -1 && (lineEnd = lfIndex), lineEnd === -1) {
			incompleteLine = chunk.slice(searchIndex);
			break;
		} else {
			const line = chunk.slice(searchIndex, lineEnd);
			lines.push(line), searchIndex = lineEnd + 1, chunk[searchIndex - 1] === "\r" && chunk[searchIndex] === `
` && searchIndex++;
		}
	}
	return [lines, incompleteLine];
}

//#endregion
//#region node_modules/.pnpm/eventsource@3.0.7/node_modules/eventsource/dist/index.js
var ErrorEvent = class extends Event {
	/**
	* Constructs a new `ErrorEvent` instance. This is typically not called directly,
	* but rather emitted by the `EventSource` object when an error occurs.
	*
	* @param type - The type of the event (should be "error")
	* @param errorEventInitDict - Optional properties to include in the error event
	*/
	constructor(type, errorEventInitDict) {
		var _a, _b;
		super(type), this.code = (_a = errorEventInitDict == null ? void 0 : errorEventInitDict.code) != null ? _a : void 0, this.message = (_b = errorEventInitDict == null ? void 0 : errorEventInitDict.message) != null ? _b : void 0;
	}
	/**
	* Node.js "hides" the `message` and `code` properties of the `ErrorEvent` instance,
	* when it is `console.log`'ed. This makes it harder to debug errors. To ease debugging,
	* we explicitly include the properties in the `inspect` method.
	*
	* This is automatically called by Node.js when you `console.log` an instance of this class.
	*
	* @param _depth - The current depth
	* @param options - The options passed to `util.inspect`
	* @param inspect - The inspect function to use (prevents having to import it from `util`)
	* @returns A string representation of the error
	*/
	[Symbol.for("nodejs.util.inspect.custom")](_depth, options, inspect) {
		return inspect(inspectableError(this), options);
	}
	/**
	* Deno "hides" the `message` and `code` properties of the `ErrorEvent` instance,
	* when it is `console.log`'ed. This makes it harder to debug errors. To ease debugging,
	* we explicitly include the properties in the `inspect` method.
	*
	* This is automatically called by Deno when you `console.log` an instance of this class.
	*
	* @param inspect - The inspect function to use (prevents having to import it from `util`)
	* @param options - The options passed to `Deno.inspect`
	* @returns A string representation of the error
	*/
	[Symbol.for("Deno.customInspect")](inspect, options) {
		return inspect(inspectableError(this), options);
	}
};
function syntaxError(message) {
	const DomException = globalThis.DOMException;
	return typeof DomException == "function" ? new DomException(message, "SyntaxError") : new SyntaxError(message);
}
function flattenError(err) {
	return err instanceof Error ? "errors" in err && Array.isArray(err.errors) ? err.errors.map(flattenError).join(", ") : "cause" in err && err.cause instanceof Error ? `${err}: ${flattenError(err.cause)}` : err.message : `${err}`;
}
function inspectableError(err) {
	return {
		type: err.type,
		message: err.message,
		code: err.code,
		defaultPrevented: err.defaultPrevented,
		cancelable: err.cancelable,
		timeStamp: err.timeStamp
	};
}
var __typeError = (msg) => {
	throw TypeError(msg);
}, __accessCheck = (obj, member, msg) => member.has(obj) || __typeError("Cannot " + msg), __privateGet = (obj, member, getter) => (__accessCheck(obj, member, "read from private field"), getter ? getter.call(obj) : member.get(obj)), __privateAdd = (obj, member, value) => member.has(obj) ? __typeError("Cannot add the same private member more than once") : member instanceof WeakSet ? member.add(obj) : member.set(obj, value), __privateSet = (obj, member, value, setter) => (__accessCheck(obj, member, "write to private field"), member.set(obj, value), value), __privateMethod = (obj, member, method) => (__accessCheck(obj, member, "access private method"), method), _readyState, _url, _redirectUrl, _withCredentials, _fetch, _reconnectInterval, _reconnectTimer, _lastEventId, _controller, _parser, _onError, _onMessage, _onOpen, _EventSource_instances, connect_fn, _onFetchResponse, _onFetchError, getRequestOptions_fn, _onEvent, _onRetryChange, failConnection_fn, scheduleReconnect_fn, _reconnect;
var EventSource = class extends EventTarget {
	constructor(url$1, eventSourceInitDict) {
		var _a, _b;
		super(), __privateAdd(this, _EventSource_instances), this.CONNECTING = 0, this.OPEN = 1, this.CLOSED = 2, __privateAdd(this, _readyState), __privateAdd(this, _url), __privateAdd(this, _redirectUrl), __privateAdd(this, _withCredentials), __privateAdd(this, _fetch), __privateAdd(this, _reconnectInterval), __privateAdd(this, _reconnectTimer), __privateAdd(this, _lastEventId, null), __privateAdd(this, _controller), __privateAdd(this, _parser), __privateAdd(this, _onError, null), __privateAdd(this, _onMessage, null), __privateAdd(this, _onOpen, null), __privateAdd(this, _onFetchResponse, async (response) => {
			var _a2;
			__privateGet(this, _parser).reset();
			const { body, redirected, status, headers } = response;
			if (status === 204) {
				__privateMethod(this, _EventSource_instances, failConnection_fn).call(this, "Server sent HTTP 204, not reconnecting", 204), this.close();
				return;
			}
			if (redirected ? __privateSet(this, _redirectUrl, new URL(response.url)) : __privateSet(this, _redirectUrl, void 0), status !== 200) {
				__privateMethod(this, _EventSource_instances, failConnection_fn).call(this, `Non-200 status code (${status})`, status);
				return;
			}
			if (!(headers.get("content-type") || "").startsWith("text/event-stream")) {
				__privateMethod(this, _EventSource_instances, failConnection_fn).call(this, "Invalid content type, expected \"text/event-stream\"", status);
				return;
			}
			if (__privateGet(this, _readyState) === this.CLOSED) return;
			__privateSet(this, _readyState, this.OPEN);
			const openEvent = new Event("open");
			if ((_a2 = __privateGet(this, _onOpen)) == null || _a2.call(this, openEvent), this.dispatchEvent(openEvent), typeof body != "object" || !body || !("getReader" in body)) {
				__privateMethod(this, _EventSource_instances, failConnection_fn).call(this, "Invalid response body, expected a web ReadableStream", status), this.close();
				return;
			}
			const decoder = new TextDecoder(), reader = body.getReader();
			let open = !0;
			do {
				const { done, value } = await reader.read();
				value && __privateGet(this, _parser).feed(decoder.decode(value, { stream: !done })), done && (open = !1, __privateGet(this, _parser).reset(), __privateMethod(this, _EventSource_instances, scheduleReconnect_fn).call(this));
			} while (open);
		}), __privateAdd(this, _onFetchError, (err) => {
			__privateSet(this, _controller, void 0), !(err.name === "AbortError" || err.type === "aborted") && __privateMethod(this, _EventSource_instances, scheduleReconnect_fn).call(this, flattenError(err));
		}), __privateAdd(this, _onEvent, (event) => {
			typeof event.id == "string" && __privateSet(this, _lastEventId, event.id);
			const messageEvent = new MessageEvent(event.event || "message", {
				data: event.data,
				origin: __privateGet(this, _redirectUrl) ? __privateGet(this, _redirectUrl).origin : __privateGet(this, _url).origin,
				lastEventId: event.id || ""
			});
			__privateGet(this, _onMessage) && (!event.event || event.event === "message") && __privateGet(this, _onMessage).call(this, messageEvent), this.dispatchEvent(messageEvent);
		}), __privateAdd(this, _onRetryChange, (value) => {
			__privateSet(this, _reconnectInterval, value);
		}), __privateAdd(this, _reconnect, () => {
			__privateSet(this, _reconnectTimer, void 0), __privateGet(this, _readyState) === this.CONNECTING && __privateMethod(this, _EventSource_instances, connect_fn).call(this);
		});
		try {
			if (url$1 instanceof URL) __privateSet(this, _url, url$1);
			else if (typeof url$1 == "string") __privateSet(this, _url, new URL(url$1, getBaseURL()));
			else throw new Error("Invalid URL");
		} catch {
			throw syntaxError("An invalid or illegal string was specified");
		}
		__privateSet(this, _parser, createParser({
			onEvent: __privateGet(this, _onEvent),
			onRetry: __privateGet(this, _onRetryChange)
		})), __privateSet(this, _readyState, this.CONNECTING), __privateSet(this, _reconnectInterval, 3e3), __privateSet(this, _fetch, (_a = eventSourceInitDict == null ? void 0 : eventSourceInitDict.fetch) != null ? _a : globalThis.fetch), __privateSet(this, _withCredentials, (_b = eventSourceInitDict == null ? void 0 : eventSourceInitDict.withCredentials) != null ? _b : !1), __privateMethod(this, _EventSource_instances, connect_fn).call(this);
	}
	/**
	* Returns the state of this EventSource object's connection. It can have the values described below.
	*
	* [MDN Reference](https://developer.mozilla.org/docs/Web/API/EventSource/readyState)
	*
	* Note: typed as `number` instead of `0 | 1 | 2` for compatibility with the `EventSource` interface,
	* defined in the TypeScript `dom` library.
	*
	* @public
	*/
	get readyState() {
		return __privateGet(this, _readyState);
	}
	/**
	* Returns the URL providing the event stream.
	*
	* [MDN Reference](https://developer.mozilla.org/docs/Web/API/EventSource/url)
	*
	* @public
	*/
	get url() {
		return __privateGet(this, _url).href;
	}
	/**
	* Returns true if the credentials mode for connection requests to the URL providing the event stream is set to "include", and false otherwise.
	*
	* [MDN Reference](https://developer.mozilla.org/docs/Web/API/EventSource/withCredentials)
	*/
	get withCredentials() {
		return __privateGet(this, _withCredentials);
	}
	/** [MDN Reference](https://developer.mozilla.org/docs/Web/API/EventSource/error_event) */
	get onerror() {
		return __privateGet(this, _onError);
	}
	set onerror(value) {
		__privateSet(this, _onError, value);
	}
	/** [MDN Reference](https://developer.mozilla.org/docs/Web/API/EventSource/message_event) */
	get onmessage() {
		return __privateGet(this, _onMessage);
	}
	set onmessage(value) {
		__privateSet(this, _onMessage, value);
	}
	/** [MDN Reference](https://developer.mozilla.org/docs/Web/API/EventSource/open_event) */
	get onopen() {
		return __privateGet(this, _onOpen);
	}
	set onopen(value) {
		__privateSet(this, _onOpen, value);
	}
	addEventListener(type, listener, options) {
		const listen = listener;
		super.addEventListener(type, listen, options);
	}
	removeEventListener(type, listener, options) {
		const listen = listener;
		super.removeEventListener(type, listen, options);
	}
	/**
	* Aborts any instances of the fetch algorithm started for this EventSource object, and sets the readyState attribute to CLOSED.
	*
	* [MDN Reference](https://developer.mozilla.org/docs/Web/API/EventSource/close)
	*
	* @public
	*/
	close() {
		__privateGet(this, _reconnectTimer) && clearTimeout(__privateGet(this, _reconnectTimer)), __privateGet(this, _readyState) !== this.CLOSED && (__privateGet(this, _controller) && __privateGet(this, _controller).abort(), __privateSet(this, _readyState, this.CLOSED), __privateSet(this, _controller, void 0));
	}
};
_readyState = /* @__PURE__ */ new WeakMap(), _url = /* @__PURE__ */ new WeakMap(), _redirectUrl = /* @__PURE__ */ new WeakMap(), _withCredentials = /* @__PURE__ */ new WeakMap(), _fetch = /* @__PURE__ */ new WeakMap(), _reconnectInterval = /* @__PURE__ */ new WeakMap(), _reconnectTimer = /* @__PURE__ */ new WeakMap(), _lastEventId = /* @__PURE__ */ new WeakMap(), _controller = /* @__PURE__ */ new WeakMap(), _parser = /* @__PURE__ */ new WeakMap(), _onError = /* @__PURE__ */ new WeakMap(), _onMessage = /* @__PURE__ */ new WeakMap(), _onOpen = /* @__PURE__ */ new WeakMap(), _EventSource_instances = /* @__PURE__ */ new WeakSet(), connect_fn = function() {
	__privateSet(this, _readyState, this.CONNECTING), __privateSet(this, _controller, new AbortController()), __privateGet(this, _fetch)(__privateGet(this, _url), __privateMethod(this, _EventSource_instances, getRequestOptions_fn).call(this)).then(__privateGet(this, _onFetchResponse)).catch(__privateGet(this, _onFetchError));
}, _onFetchResponse = /* @__PURE__ */ new WeakMap(), _onFetchError = /* @__PURE__ */ new WeakMap(), getRequestOptions_fn = function() {
	var _a;
	const init = {
		mode: "cors",
		redirect: "follow",
		headers: {
			Accept: "text/event-stream",
			...__privateGet(this, _lastEventId) ? { "Last-Event-ID": __privateGet(this, _lastEventId) } : void 0
		},
		cache: "no-store",
		signal: (_a = __privateGet(this, _controller)) == null ? void 0 : _a.signal
	};
	return "window" in globalThis && (init.credentials = this.withCredentials ? "include" : "same-origin"), init;
}, _onEvent = /* @__PURE__ */ new WeakMap(), _onRetryChange = /* @__PURE__ */ new WeakMap(), failConnection_fn = function(message, code) {
	var _a;
	__privateGet(this, _readyState) !== this.CLOSED && __privateSet(this, _readyState, this.CLOSED);
	const errorEvent = new ErrorEvent("error", {
		code,
		message
	});
	(_a = __privateGet(this, _onError)) == null || _a.call(this, errorEvent), this.dispatchEvent(errorEvent);
}, scheduleReconnect_fn = function(message, code) {
	var _a;
	if (__privateGet(this, _readyState) === this.CLOSED) return;
	__privateSet(this, _readyState, this.CONNECTING);
	const errorEvent = new ErrorEvent("error", {
		code,
		message
	});
	(_a = __privateGet(this, _onError)) == null || _a.call(this, errorEvent), this.dispatchEvent(errorEvent), __privateSet(this, _reconnectTimer, setTimeout(__privateGet(this, _reconnect), __privateGet(this, _reconnectInterval)));
}, _reconnect = /* @__PURE__ */ new WeakMap(), EventSource.CONNECTING = 0, EventSource.OPEN = 1, EventSource.CLOSED = 2;
function getBaseURL() {
	const doc = "document" in globalThis ? globalThis.document : void 0;
	return doc && typeof doc == "object" && "baseURI" in doc && typeof doc.baseURI == "string" ? doc.baseURI : void 0;
}

//#endregion
//#region node_modules/.pnpm/@modelcontextprotocol+sdk@1.27.1_zod@3.25.76/node_modules/@modelcontextprotocol/sdk/dist/esm/shared/transport.js
/**
* Normalizes HeadersInit to a plain Record<string, string> for manipulation.
* Handles Headers objects, arrays of tuples, and plain objects.
*/
function normalizeHeaders(headers) {
	if (!headers) return {};
	if (headers instanceof Headers) return Object.fromEntries(headers.entries());
	if (Array.isArray(headers)) return Object.fromEntries(headers);
	return { ...headers };
}
/**
* Creates a fetch function that includes base RequestInit options.
* This ensures requests inherit settings like credentials, mode, headers, etc. from the base init.
*
* @param baseFetch - The base fetch function to wrap (defaults to global fetch)
* @param baseInit - The base RequestInit to merge with each request
* @returns A wrapped fetch function that merges base options with call-specific options
*/
function createFetchWithInit(baseFetch = fetch, baseInit) {
	if (!baseInit) return baseFetch;
	return async (url$1, init) => {
		return baseFetch(url$1, {
			...baseInit,
			...init,
			headers: init?.headers ? {
				...normalizeHeaders(baseInit.headers),
				...normalizeHeaders(init.headers)
			} : baseInit.headers
		});
	};
}

//#endregion
//#region node_modules/.pnpm/pkce-challenge@5.0.1/node_modules/pkce-challenge/dist/index.node.js
let crypto;
crypto = globalThis.crypto?.webcrypto ?? globalThis.crypto ?? import("node:crypto").then((m) => m.webcrypto);
/**
* Creates an array of length `size` of random bytes
* @param size
* @returns Array of random ints (0 to 255)
*/
async function getRandomValues(size) {
	return (await crypto).getRandomValues(new Uint8Array(size));
}
/** Generate cryptographically strong random string
* @param size The desired length of the string
* @returns The random string
*/
async function random(size) {
	const mask = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
	const evenDistCutoff = Math.pow(2, 8) - Math.pow(2, 8) % 66;
	let result = "";
	while (result.length < size) {
		const randomBytes = await getRandomValues(size - result.length);
		for (const randomByte of randomBytes) if (randomByte < evenDistCutoff) result += mask[randomByte % 66];
	}
	return result;
}
/** Generate a PKCE challenge verifier
* @param length Length of the verifier
* @returns A random verifier `length` characters long
*/
async function generateVerifier(length) {
	return await random(length);
}
/** Generate a PKCE code challenge from a code verifier
* @param code_verifier
* @returns The base64 url encoded code challenge
*/
async function generateChallenge(code_verifier) {
	const buffer = await (await crypto).subtle.digest("SHA-256", new TextEncoder().encode(code_verifier));
	return btoa(String.fromCharCode(...new Uint8Array(buffer))).replace(/\//g, "_").replace(/\+/g, "-").replace(/=/g, "");
}
/** Generate a PKCE challenge pair
* @param length Length of the verifer (between 43-128). Defaults to 43.
* @returns PKCE challenge pair
*/
async function pkceChallenge(length) {
	if (!length) length = 43;
	if (length < 43 || length > 128) throw `Expected a length between 43 and 128. Received ${length}.`;
	const verifier = await generateVerifier(length);
	return {
		code_verifier: verifier,
		code_challenge: await generateChallenge(verifier)
	};
}

//#endregion
//#region node_modules/.pnpm/@modelcontextprotocol+sdk@1.27.1_zod@3.25.76/node_modules/@modelcontextprotocol/sdk/dist/esm/shared/auth.js
/**
* Reusable URL validation that disallows javascript: scheme
*/
const SafeUrlSchema = url().superRefine((val, ctx) => {
	if (!URL.canParse(val)) {
		ctx.addIssue({
			code: ZodIssueCode.custom,
			message: "URL must be parseable",
			fatal: true
		});
		return NEVER;
	}
}).refine((url$1) => {
	const u = new URL(url$1);
	return u.protocol !== "javascript:" && u.protocol !== "data:" && u.protocol !== "vbscript:";
}, { message: "URL cannot use javascript:, data:, or vbscript: scheme" });
/**
* RFC 9728 OAuth Protected Resource Metadata
*/
const OAuthProtectedResourceMetadataSchema = looseObject({
	resource: string().url(),
	authorization_servers: array(SafeUrlSchema).optional(),
	jwks_uri: string().url().optional(),
	scopes_supported: array(string()).optional(),
	bearer_methods_supported: array(string()).optional(),
	resource_signing_alg_values_supported: array(string()).optional(),
	resource_name: string().optional(),
	resource_documentation: string().optional(),
	resource_policy_uri: string().url().optional(),
	resource_tos_uri: string().url().optional(),
	tls_client_certificate_bound_access_tokens: boolean().optional(),
	authorization_details_types_supported: array(string()).optional(),
	dpop_signing_alg_values_supported: array(string()).optional(),
	dpop_bound_access_tokens_required: boolean().optional()
});
/**
* RFC 8414 OAuth 2.0 Authorization Server Metadata
*/
const OAuthMetadataSchema = looseObject({
	issuer: string(),
	authorization_endpoint: SafeUrlSchema,
	token_endpoint: SafeUrlSchema,
	registration_endpoint: SafeUrlSchema.optional(),
	scopes_supported: array(string()).optional(),
	response_types_supported: array(string()),
	response_modes_supported: array(string()).optional(),
	grant_types_supported: array(string()).optional(),
	token_endpoint_auth_methods_supported: array(string()).optional(),
	token_endpoint_auth_signing_alg_values_supported: array(string()).optional(),
	service_documentation: SafeUrlSchema.optional(),
	revocation_endpoint: SafeUrlSchema.optional(),
	revocation_endpoint_auth_methods_supported: array(string()).optional(),
	revocation_endpoint_auth_signing_alg_values_supported: array(string()).optional(),
	introspection_endpoint: string().optional(),
	introspection_endpoint_auth_methods_supported: array(string()).optional(),
	introspection_endpoint_auth_signing_alg_values_supported: array(string()).optional(),
	code_challenge_methods_supported: array(string()).optional(),
	client_id_metadata_document_supported: boolean().optional()
});
/**
* OpenID Connect Discovery 1.0 Provider Metadata
* see: https://openid.net/specs/openid-connect-discovery-1_0.html#ProviderMetadata
*/
const OpenIdProviderMetadataSchema = looseObject({
	issuer: string(),
	authorization_endpoint: SafeUrlSchema,
	token_endpoint: SafeUrlSchema,
	userinfo_endpoint: SafeUrlSchema.optional(),
	jwks_uri: SafeUrlSchema,
	registration_endpoint: SafeUrlSchema.optional(),
	scopes_supported: array(string()).optional(),
	response_types_supported: array(string()),
	response_modes_supported: array(string()).optional(),
	grant_types_supported: array(string()).optional(),
	acr_values_supported: array(string()).optional(),
	subject_types_supported: array(string()),
	id_token_signing_alg_values_supported: array(string()),
	id_token_encryption_alg_values_supported: array(string()).optional(),
	id_token_encryption_enc_values_supported: array(string()).optional(),
	userinfo_signing_alg_values_supported: array(string()).optional(),
	userinfo_encryption_alg_values_supported: array(string()).optional(),
	userinfo_encryption_enc_values_supported: array(string()).optional(),
	request_object_signing_alg_values_supported: array(string()).optional(),
	request_object_encryption_alg_values_supported: array(string()).optional(),
	request_object_encryption_enc_values_supported: array(string()).optional(),
	token_endpoint_auth_methods_supported: array(string()).optional(),
	token_endpoint_auth_signing_alg_values_supported: array(string()).optional(),
	display_values_supported: array(string()).optional(),
	claim_types_supported: array(string()).optional(),
	claims_supported: array(string()).optional(),
	service_documentation: string().optional(),
	claims_locales_supported: array(string()).optional(),
	ui_locales_supported: array(string()).optional(),
	claims_parameter_supported: boolean().optional(),
	request_parameter_supported: boolean().optional(),
	request_uri_parameter_supported: boolean().optional(),
	require_request_uri_registration: boolean().optional(),
	op_policy_uri: SafeUrlSchema.optional(),
	op_tos_uri: SafeUrlSchema.optional(),
	client_id_metadata_document_supported: boolean().optional()
});
/**
* OpenID Connect Discovery metadata that may include OAuth 2.0 fields
* This schema represents the real-world scenario where OIDC providers
* return a mix of OpenID Connect and OAuth 2.0 metadata fields
*/
const OpenIdProviderDiscoveryMetadataSchema = object({
	...OpenIdProviderMetadataSchema.shape,
	...OAuthMetadataSchema.pick({ code_challenge_methods_supported: true }).shape
});
/**
* OAuth 2.1 token response
*/
const OAuthTokensSchema = object({
	access_token: string(),
	id_token: string().optional(),
	token_type: string(),
	expires_in: number().optional(),
	scope: string().optional(),
	refresh_token: string().optional()
}).strip();
/**
* OAuth 2.1 error response
*/
const OAuthErrorResponseSchema = object({
	error: string(),
	error_description: string().optional(),
	error_uri: string().optional()
});
/**
* Optional version of SafeUrlSchema that allows empty string for retrocompatibility on tos_uri and logo_uri
*/
const OptionalSafeUrlSchema = SafeUrlSchema.optional().or(literal("").transform(() => void 0));
/**
* RFC 7591 OAuth 2.0 Dynamic Client Registration metadata
*/
const OAuthClientMetadataSchema = object({
	redirect_uris: array(SafeUrlSchema),
	token_endpoint_auth_method: string().optional(),
	grant_types: array(string()).optional(),
	response_types: array(string()).optional(),
	client_name: string().optional(),
	client_uri: SafeUrlSchema.optional(),
	logo_uri: OptionalSafeUrlSchema,
	scope: string().optional(),
	contacts: array(string()).optional(),
	tos_uri: OptionalSafeUrlSchema,
	policy_uri: string().optional(),
	jwks_uri: SafeUrlSchema.optional(),
	jwks: any().optional(),
	software_id: string().optional(),
	software_version: string().optional(),
	software_statement: string().optional()
}).strip();
/**
* RFC 7591 OAuth 2.0 Dynamic Client Registration client information
*/
const OAuthClientInformationSchema = object({
	client_id: string(),
	client_secret: string().optional(),
	client_id_issued_at: number$1().optional(),
	client_secret_expires_at: number$1().optional()
}).strip();
/**
* RFC 7591 OAuth 2.0 Dynamic Client Registration full response (client information plus metadata)
*/
const OAuthClientInformationFullSchema = OAuthClientMetadataSchema.merge(OAuthClientInformationSchema);
/**
* RFC 7591 OAuth 2.0 Dynamic Client Registration error response
*/
const OAuthClientRegistrationErrorSchema = object({
	error: string(),
	error_description: string().optional()
}).strip();
/**
* RFC 7009 OAuth 2.0 Token Revocation request
*/
const OAuthTokenRevocationRequestSchema = object({
	token: string(),
	token_type_hint: string().optional()
}).strip();

//#endregion
//#region node_modules/.pnpm/@modelcontextprotocol+sdk@1.27.1_zod@3.25.76/node_modules/@modelcontextprotocol/sdk/dist/esm/shared/auth-utils.js
/**
* Utilities for handling OAuth resource URIs.
*/
/**
* Converts a server URL to a resource URL by removing the fragment.
* RFC 8707 section 2 states that resource URIs "MUST NOT include a fragment component".
* Keeps everything else unchanged (scheme, domain, port, path, query).
*/
function resourceUrlFromServerUrl(url$1) {
	const resourceURL = typeof url$1 === "string" ? new URL(url$1) : new URL(url$1.href);
	resourceURL.hash = "";
	return resourceURL;
}
/**
* Checks if a requested resource URL matches a configured resource URL.
* A requested resource matches if it has the same scheme, domain, port,
* and its path starts with the configured resource's path.
*
* @param requestedResource The resource URL being requested
* @param configuredResource The resource URL that has been configured
* @returns true if the requested resource matches the configured resource, false otherwise
*/
function checkResourceAllowed({ requestedResource, configuredResource }) {
	const requested = typeof requestedResource === "string" ? new URL(requestedResource) : new URL(requestedResource.href);
	const configured = typeof configuredResource === "string" ? new URL(configuredResource) : new URL(configuredResource.href);
	if (requested.origin !== configured.origin) return false;
	if (requested.pathname.length < configured.pathname.length) return false;
	const requestedPath = requested.pathname.endsWith("/") ? requested.pathname : requested.pathname + "/";
	const configuredPath = configured.pathname.endsWith("/") ? configured.pathname : configured.pathname + "/";
	return requestedPath.startsWith(configuredPath);
}

//#endregion
//#region node_modules/.pnpm/@modelcontextprotocol+sdk@1.27.1_zod@3.25.76/node_modules/@modelcontextprotocol/sdk/dist/esm/server/auth/errors.js
/**
* Base class for all OAuth errors
*/
var OAuthError = class extends Error {
	constructor(message, errorUri) {
		super(message);
		this.errorUri = errorUri;
		this.name = this.constructor.name;
	}
	/**
	* Converts the error to a standard OAuth error response object
	*/
	toResponseObject() {
		const response = {
			error: this.errorCode,
			error_description: this.message
		};
		if (this.errorUri) response.error_uri = this.errorUri;
		return response;
	}
	get errorCode() {
		return this.constructor.errorCode;
	}
};
/**
* Invalid request error - The request is missing a required parameter,
* includes an invalid parameter value, includes a parameter more than once,
* or is otherwise malformed.
*/
var InvalidRequestError = class extends OAuthError {};
InvalidRequestError.errorCode = "invalid_request";
/**
* Invalid client error - Client authentication failed (e.g., unknown client, no client
* authentication included, or unsupported authentication method).
*/
var InvalidClientError = class extends OAuthError {};
InvalidClientError.errorCode = "invalid_client";
/**
* Invalid grant error - The provided authorization grant or refresh token is
* invalid, expired, revoked, does not match the redirection URI used in the
* authorization request, or was issued to another client.
*/
var InvalidGrantError = class extends OAuthError {};
InvalidGrantError.errorCode = "invalid_grant";
/**
* Unauthorized client error - The authenticated client is not authorized to use
* this authorization grant type.
*/
var UnauthorizedClientError = class extends OAuthError {};
UnauthorizedClientError.errorCode = "unauthorized_client";
/**
* Unsupported grant type error - The authorization grant type is not supported
* by the authorization server.
*/
var UnsupportedGrantTypeError = class extends OAuthError {};
UnsupportedGrantTypeError.errorCode = "unsupported_grant_type";
/**
* Invalid scope error - The requested scope is invalid, unknown, malformed, or
* exceeds the scope granted by the resource owner.
*/
var InvalidScopeError = class extends OAuthError {};
InvalidScopeError.errorCode = "invalid_scope";
/**
* Access denied error - The resource owner or authorization server denied the request.
*/
var AccessDeniedError = class extends OAuthError {};
AccessDeniedError.errorCode = "access_denied";
/**
* Server error - The authorization server encountered an unexpected condition
* that prevented it from fulfilling the request.
*/
var ServerError = class extends OAuthError {};
ServerError.errorCode = "server_error";
/**
* Temporarily unavailable error - The authorization server is currently unable to
* handle the request due to a temporary overloading or maintenance of the server.
*/
var TemporarilyUnavailableError = class extends OAuthError {};
TemporarilyUnavailableError.errorCode = "temporarily_unavailable";
/**
* Unsupported response type error - The authorization server does not support
* obtaining an authorization code using this method.
*/
var UnsupportedResponseTypeError = class extends OAuthError {};
UnsupportedResponseTypeError.errorCode = "unsupported_response_type";
/**
* Unsupported token type error - The authorization server does not support
* the requested token type.
*/
var UnsupportedTokenTypeError = class extends OAuthError {};
UnsupportedTokenTypeError.errorCode = "unsupported_token_type";
/**
* Invalid token error - The access token provided is expired, revoked, malformed,
* or invalid for other reasons.
*/
var InvalidTokenError = class extends OAuthError {};
InvalidTokenError.errorCode = "invalid_token";
/**
* Method not allowed error - The HTTP method used is not allowed for this endpoint.
* (Custom, non-standard error)
*/
var MethodNotAllowedError = class extends OAuthError {};
MethodNotAllowedError.errorCode = "method_not_allowed";
/**
* Too many requests error - Rate limit exceeded.
* (Custom, non-standard error based on RFC 6585)
*/
var TooManyRequestsError = class extends OAuthError {};
TooManyRequestsError.errorCode = "too_many_requests";
/**
* Invalid client metadata error - The client metadata is invalid.
* (Custom error for dynamic client registration - RFC 7591)
*/
var InvalidClientMetadataError = class extends OAuthError {};
InvalidClientMetadataError.errorCode = "invalid_client_metadata";
/**
* Insufficient scope error - The request requires higher privileges than provided by the access token.
*/
var InsufficientScopeError = class extends OAuthError {};
InsufficientScopeError.errorCode = "insufficient_scope";
/**
* Invalid target error - The requested resource is invalid, missing, unknown, or malformed.
* (Custom error for resource indicators - RFC 8707)
*/
var InvalidTargetError = class extends OAuthError {};
InvalidTargetError.errorCode = "invalid_target";
/**
* A full list of all OAuthErrors, enabling parsing from error responses
*/
const OAUTH_ERRORS = {
	[InvalidRequestError.errorCode]: InvalidRequestError,
	[InvalidClientError.errorCode]: InvalidClientError,
	[InvalidGrantError.errorCode]: InvalidGrantError,
	[UnauthorizedClientError.errorCode]: UnauthorizedClientError,
	[UnsupportedGrantTypeError.errorCode]: UnsupportedGrantTypeError,
	[InvalidScopeError.errorCode]: InvalidScopeError,
	[AccessDeniedError.errorCode]: AccessDeniedError,
	[ServerError.errorCode]: ServerError,
	[TemporarilyUnavailableError.errorCode]: TemporarilyUnavailableError,
	[UnsupportedResponseTypeError.errorCode]: UnsupportedResponseTypeError,
	[UnsupportedTokenTypeError.errorCode]: UnsupportedTokenTypeError,
	[InvalidTokenError.errorCode]: InvalidTokenError,
	[MethodNotAllowedError.errorCode]: MethodNotAllowedError,
	[TooManyRequestsError.errorCode]: TooManyRequestsError,
	[InvalidClientMetadataError.errorCode]: InvalidClientMetadataError,
	[InsufficientScopeError.errorCode]: InsufficientScopeError,
	[InvalidTargetError.errorCode]: InvalidTargetError
};

//#endregion
//#region node_modules/.pnpm/@modelcontextprotocol+sdk@1.27.1_zod@3.25.76/node_modules/@modelcontextprotocol/sdk/dist/esm/client/auth.js
var UnauthorizedError = class extends Error {
	constructor(message) {
		super(message ?? "Unauthorized");
	}
};
function isClientAuthMethod(method) {
	return [
		"client_secret_basic",
		"client_secret_post",
		"none"
	].includes(method);
}
const AUTHORIZATION_CODE_RESPONSE_TYPE = "code";
const AUTHORIZATION_CODE_CHALLENGE_METHOD = "S256";
/**
* Determines the best client authentication method to use based on server support and client configuration.
*
* Priority order (highest to lowest):
* 1. client_secret_basic (if client secret is available)
* 2. client_secret_post (if client secret is available)
* 3. none (for public clients)
*
* @param clientInformation - OAuth client information containing credentials
* @param supportedMethods - Authentication methods supported by the authorization server
* @returns The selected authentication method
*/
function selectClientAuthMethod(clientInformation, supportedMethods) {
	const hasClientSecret = clientInformation.client_secret !== void 0;
	if (supportedMethods.length === 0) return hasClientSecret ? "client_secret_post" : "none";
	if ("token_endpoint_auth_method" in clientInformation && clientInformation.token_endpoint_auth_method && isClientAuthMethod(clientInformation.token_endpoint_auth_method) && supportedMethods.includes(clientInformation.token_endpoint_auth_method)) return clientInformation.token_endpoint_auth_method;
	if (hasClientSecret && supportedMethods.includes("client_secret_basic")) return "client_secret_basic";
	if (hasClientSecret && supportedMethods.includes("client_secret_post")) return "client_secret_post";
	if (supportedMethods.includes("none")) return "none";
	return hasClientSecret ? "client_secret_post" : "none";
}
/**
* Applies client authentication to the request based on the specified method.
*
* Implements OAuth 2.1 client authentication methods:
* - client_secret_basic: HTTP Basic authentication (RFC 6749 Section 2.3.1)
* - client_secret_post: Credentials in request body (RFC 6749 Section 2.3.1)
* - none: Public client authentication (RFC 6749 Section 2.1)
*
* @param method - The authentication method to use
* @param clientInformation - OAuth client information containing credentials
* @param headers - HTTP headers object to modify
* @param params - URL search parameters to modify
* @throws {Error} When required credentials are missing
*/
function applyClientAuthentication(method, clientInformation, headers, params) {
	const { client_id, client_secret } = clientInformation;
	switch (method) {
		case "client_secret_basic":
			applyBasicAuth(client_id, client_secret, headers);
			return;
		case "client_secret_post":
			applyPostAuth(client_id, client_secret, params);
			return;
		case "none":
			applyPublicAuth(client_id, params);
			return;
		default: throw new Error(`Unsupported client authentication method: ${method}`);
	}
}
/**
* Applies HTTP Basic authentication (RFC 6749 Section 2.3.1)
*/
function applyBasicAuth(clientId, clientSecret, headers) {
	if (!clientSecret) throw new Error("client_secret_basic authentication requires a client_secret");
	const credentials = btoa(`${clientId}:${clientSecret}`);
	headers.set("Authorization", `Basic ${credentials}`);
}
/**
* Applies POST body authentication (RFC 6749 Section 2.3.1)
*/
function applyPostAuth(clientId, clientSecret, params) {
	params.set("client_id", clientId);
	if (clientSecret) params.set("client_secret", clientSecret);
}
/**
* Applies public client authentication (RFC 6749 Section 2.1)
*/
function applyPublicAuth(clientId, params) {
	params.set("client_id", clientId);
}
/**
* Parses an OAuth error response from a string or Response object.
*
* If the input is a standard OAuth2.0 error response, it will be parsed according to the spec
* and an instance of the appropriate OAuthError subclass will be returned.
* If parsing fails, it falls back to a generic ServerError that includes
* the response status (if available) and original content.
*
* @param input - A Response object or string containing the error response
* @returns A Promise that resolves to an OAuthError instance
*/
async function parseErrorResponse(input) {
	const statusCode = input instanceof Response ? input.status : void 0;
	const body = input instanceof Response ? await input.text() : input;
	try {
		const { error, error_description, error_uri } = OAuthErrorResponseSchema.parse(JSON.parse(body));
		return new (OAUTH_ERRORS[error] || ServerError)(error_description || "", error_uri);
	} catch (error) {
		return new ServerError(`${statusCode ? `HTTP ${statusCode}: ` : ""}Invalid OAuth error response: ${error}. Raw body: ${body}`);
	}
}
/**
* Orchestrates the full auth flow with a server.
*
* This can be used as a single entry point for all authorization functionality,
* instead of linking together the other lower-level functions in this module.
*/
async function auth(provider, options) {
	try {
		return await authInternal(provider, options);
	} catch (error) {
		if (error instanceof InvalidClientError || error instanceof UnauthorizedClientError) {
			await provider.invalidateCredentials?.("all");
			return await authInternal(provider, options);
		} else if (error instanceof InvalidGrantError) {
			await provider.invalidateCredentials?.("tokens");
			return await authInternal(provider, options);
		}
		throw error;
	}
}
async function authInternal(provider, { serverUrl, authorizationCode, scope, resourceMetadataUrl, fetchFn }) {
	const cachedState = await provider.discoveryState?.();
	let resourceMetadata;
	let authorizationServerUrl;
	let metadata;
	let effectiveResourceMetadataUrl = resourceMetadataUrl;
	if (!effectiveResourceMetadataUrl && cachedState?.resourceMetadataUrl) effectiveResourceMetadataUrl = new URL(cachedState.resourceMetadataUrl);
	if (cachedState?.authorizationServerUrl) {
		authorizationServerUrl = cachedState.authorizationServerUrl;
		resourceMetadata = cachedState.resourceMetadata;
		metadata = cachedState.authorizationServerMetadata ?? await discoverAuthorizationServerMetadata(authorizationServerUrl, { fetchFn });
		if (!resourceMetadata) try {
			resourceMetadata = await discoverOAuthProtectedResourceMetadata(serverUrl, { resourceMetadataUrl: effectiveResourceMetadataUrl }, fetchFn);
		} catch {}
		if (metadata !== cachedState.authorizationServerMetadata || resourceMetadata !== cachedState.resourceMetadata) await provider.saveDiscoveryState?.({
			authorizationServerUrl: String(authorizationServerUrl),
			resourceMetadataUrl: effectiveResourceMetadataUrl?.toString(),
			resourceMetadata,
			authorizationServerMetadata: metadata
		});
	} else {
		const serverInfo = await discoverOAuthServerInfo(serverUrl, {
			resourceMetadataUrl: effectiveResourceMetadataUrl,
			fetchFn
		});
		authorizationServerUrl = serverInfo.authorizationServerUrl;
		metadata = serverInfo.authorizationServerMetadata;
		resourceMetadata = serverInfo.resourceMetadata;
		await provider.saveDiscoveryState?.({
			authorizationServerUrl: String(authorizationServerUrl),
			resourceMetadataUrl: effectiveResourceMetadataUrl?.toString(),
			resourceMetadata,
			authorizationServerMetadata: metadata
		});
	}
	const resource = await selectResourceURL(serverUrl, provider, resourceMetadata);
	let clientInformation = await Promise.resolve(provider.clientInformation());
	if (!clientInformation) {
		if (authorizationCode !== void 0) throw new Error("Existing OAuth client information is required when exchanging an authorization code");
		const supportsUrlBasedClientId = metadata?.client_id_metadata_document_supported === true;
		const clientMetadataUrl = provider.clientMetadataUrl;
		if (clientMetadataUrl && !isHttpsUrl(clientMetadataUrl)) throw new InvalidClientMetadataError(`clientMetadataUrl must be a valid HTTPS URL with a non-root pathname, got: ${clientMetadataUrl}`);
		if (supportsUrlBasedClientId && clientMetadataUrl) {
			clientInformation = { client_id: clientMetadataUrl };
			await provider.saveClientInformation?.(clientInformation);
		} else {
			if (!provider.saveClientInformation) throw new Error("OAuth client information must be saveable for dynamic registration");
			const fullInformation = await registerClient(authorizationServerUrl, {
				metadata,
				clientMetadata: provider.clientMetadata,
				fetchFn
			});
			await provider.saveClientInformation(fullInformation);
			clientInformation = fullInformation;
		}
	}
	const nonInteractiveFlow = !provider.redirectUrl;
	if (authorizationCode !== void 0 || nonInteractiveFlow) {
		const tokens$1 = await fetchToken(provider, authorizationServerUrl, {
			metadata,
			resource,
			authorizationCode,
			fetchFn
		});
		await provider.saveTokens(tokens$1);
		return "AUTHORIZED";
	}
	const tokens = await provider.tokens();
	if (tokens?.refresh_token) try {
		const newTokens = await refreshAuthorization(authorizationServerUrl, {
			metadata,
			clientInformation,
			refreshToken: tokens.refresh_token,
			resource,
			addClientAuthentication: provider.addClientAuthentication,
			fetchFn
		});
		await provider.saveTokens(newTokens);
		return "AUTHORIZED";
	} catch (error) {
		if (!(error instanceof OAuthError) || error instanceof ServerError) {} else throw error;
	}
	const state = provider.state ? await provider.state() : void 0;
	const { authorizationUrl, codeVerifier } = await startAuthorization(authorizationServerUrl, {
		metadata,
		clientInformation,
		state,
		redirectUrl: provider.redirectUrl,
		scope: scope || resourceMetadata?.scopes_supported?.join(" ") || provider.clientMetadata.scope,
		resource
	});
	await provider.saveCodeVerifier(codeVerifier);
	await provider.redirectToAuthorization(authorizationUrl);
	return "REDIRECT";
}
/**
* SEP-991: URL-based Client IDs
* Validate that the client_id is a valid URL with https scheme
*/
function isHttpsUrl(value) {
	if (!value) return false;
	try {
		const url$1 = new URL(value);
		return url$1.protocol === "https:" && url$1.pathname !== "/";
	} catch {
		return false;
	}
}
async function selectResourceURL(serverUrl, provider, resourceMetadata) {
	const defaultResource = resourceUrlFromServerUrl(serverUrl);
	if (provider.validateResourceURL) return await provider.validateResourceURL(defaultResource, resourceMetadata?.resource);
	if (!resourceMetadata) return;
	if (!checkResourceAllowed({
		requestedResource: defaultResource,
		configuredResource: resourceMetadata.resource
	})) throw new Error(`Protected resource ${resourceMetadata.resource} does not match expected ${defaultResource} (or origin)`);
	return new URL(resourceMetadata.resource);
}
/**
* Extract resource_metadata, scope, and error from WWW-Authenticate header.
*/
function extractWWWAuthenticateParams(res) {
	const authenticateHeader = res.headers.get("WWW-Authenticate");
	if (!authenticateHeader) return {};
	const [type, scheme] = authenticateHeader.split(" ");
	if (type.toLowerCase() !== "bearer" || !scheme) return {};
	const resourceMetadataMatch = extractFieldFromWwwAuth(res, "resource_metadata") || void 0;
	let resourceMetadataUrl;
	if (resourceMetadataMatch) try {
		resourceMetadataUrl = new URL(resourceMetadataMatch);
	} catch {}
	const scope = extractFieldFromWwwAuth(res, "scope") || void 0;
	const error = extractFieldFromWwwAuth(res, "error") || void 0;
	return {
		resourceMetadataUrl,
		scope,
		error
	};
}
/**
* Extracts a specific field's value from the WWW-Authenticate header string.
*
* @param response The HTTP response object containing the headers.
* @param fieldName The name of the field to extract (e.g., "realm", "nonce").
* @returns The field value
*/
function extractFieldFromWwwAuth(response, fieldName) {
	const wwwAuthHeader = response.headers.get("WWW-Authenticate");
	if (!wwwAuthHeader) return null;
	const pattern = /* @__PURE__ */ new RegExp(`${fieldName}=(?:"([^"]+)"|([^\\s,]+))`);
	const match = wwwAuthHeader.match(pattern);
	if (match) return match[1] || match[2];
	return null;
}
/**
* Looks up RFC 9728 OAuth 2.0 Protected Resource Metadata.
*
* If the server returns a 404 for the well-known endpoint, this function will
* return `undefined`. Any other errors will be thrown as exceptions.
*/
async function discoverOAuthProtectedResourceMetadata(serverUrl, opts, fetchFn = fetch) {
	const response = await discoverMetadataWithFallback(serverUrl, "oauth-protected-resource", fetchFn, {
		protocolVersion: opts?.protocolVersion,
		metadataUrl: opts?.resourceMetadataUrl
	});
	if (!response || response.status === 404) {
		await response?.body?.cancel();
		throw new Error(`Resource server does not implement OAuth 2.0 Protected Resource Metadata.`);
	}
	if (!response.ok) {
		await response.body?.cancel();
		throw new Error(`HTTP ${response.status} trying to load well-known OAuth protected resource metadata.`);
	}
	return OAuthProtectedResourceMetadataSchema.parse(await response.json());
}
/**
* Helper function to handle fetch with CORS retry logic
*/
async function fetchWithCorsRetry(url$1, headers, fetchFn = fetch) {
	try {
		return await fetchFn(url$1, { headers });
	} catch (error) {
		if (error instanceof TypeError) if (headers) return fetchWithCorsRetry(url$1, void 0, fetchFn);
		else return;
		throw error;
	}
}
/**
* Constructs the well-known path for auth-related metadata discovery
*/
function buildWellKnownPath(wellKnownPrefix, pathname = "", options = {}) {
	if (pathname.endsWith("/")) pathname = pathname.slice(0, -1);
	return options.prependPathname ? `${pathname}/.well-known/${wellKnownPrefix}` : `/.well-known/${wellKnownPrefix}${pathname}`;
}
/**
* Tries to discover OAuth metadata at a specific URL
*/
async function tryMetadataDiscovery(url$1, protocolVersion, fetchFn = fetch) {
	return await fetchWithCorsRetry(url$1, { "MCP-Protocol-Version": protocolVersion }, fetchFn);
}
/**
* Determines if fallback to root discovery should be attempted
*/
function shouldAttemptFallback(response, pathname) {
	return !response || response.status >= 400 && response.status < 500 && pathname !== "/";
}
/**
* Generic function for discovering OAuth metadata with fallback support
*/
async function discoverMetadataWithFallback(serverUrl, wellKnownType, fetchFn, opts) {
	const issuer = new URL(serverUrl);
	const protocolVersion = opts?.protocolVersion ?? LATEST_PROTOCOL_VERSION;
	let url$1;
	if (opts?.metadataUrl) url$1 = new URL(opts.metadataUrl);
	else {
		const wellKnownPath = buildWellKnownPath(wellKnownType, issuer.pathname);
		url$1 = new URL(wellKnownPath, opts?.metadataServerUrl ?? issuer);
		url$1.search = issuer.search;
	}
	let response = await tryMetadataDiscovery(url$1, protocolVersion, fetchFn);
	if (!opts?.metadataUrl && shouldAttemptFallback(response, issuer.pathname)) response = await tryMetadataDiscovery(new URL(`/.well-known/${wellKnownType}`, issuer), protocolVersion, fetchFn);
	return response;
}
/**
* Builds a list of discovery URLs to try for authorization server metadata.
* URLs are returned in priority order:
* 1. OAuth metadata at the given URL
* 2. OIDC metadata endpoints at the given URL
*/
function buildDiscoveryUrls(authorizationServerUrl) {
	const url$1 = typeof authorizationServerUrl === "string" ? new URL(authorizationServerUrl) : authorizationServerUrl;
	const hasPath = url$1.pathname !== "/";
	const urlsToTry = [];
	if (!hasPath) {
		urlsToTry.push({
			url: new URL("/.well-known/oauth-authorization-server", url$1.origin),
			type: "oauth"
		});
		urlsToTry.push({
			url: new URL(`/.well-known/openid-configuration`, url$1.origin),
			type: "oidc"
		});
		return urlsToTry;
	}
	let pathname = url$1.pathname;
	if (pathname.endsWith("/")) pathname = pathname.slice(0, -1);
	urlsToTry.push({
		url: new URL(`/.well-known/oauth-authorization-server${pathname}`, url$1.origin),
		type: "oauth"
	});
	urlsToTry.push({
		url: new URL(`/.well-known/openid-configuration${pathname}`, url$1.origin),
		type: "oidc"
	});
	urlsToTry.push({
		url: new URL(`${pathname}/.well-known/openid-configuration`, url$1.origin),
		type: "oidc"
	});
	return urlsToTry;
}
/**
* Discovers authorization server metadata with support for RFC 8414 OAuth 2.0 Authorization Server Metadata
* and OpenID Connect Discovery 1.0 specifications.
*
* This function implements a fallback strategy for authorization server discovery:
* 1. Attempts RFC 8414 OAuth metadata discovery first
* 2. If OAuth discovery fails, falls back to OpenID Connect Discovery
*
* @param authorizationServerUrl - The authorization server URL obtained from the MCP Server's
*                                 protected resource metadata, or the MCP server's URL if the
*                                 metadata was not found.
* @param options - Configuration options
* @param options.fetchFn - Optional fetch function for making HTTP requests, defaults to global fetch
* @param options.protocolVersion - MCP protocol version to use, defaults to LATEST_PROTOCOL_VERSION
* @returns Promise resolving to authorization server metadata, or undefined if discovery fails
*/
async function discoverAuthorizationServerMetadata(authorizationServerUrl, { fetchFn = fetch, protocolVersion = LATEST_PROTOCOL_VERSION } = {}) {
	const headers = {
		"MCP-Protocol-Version": protocolVersion,
		Accept: "application/json"
	};
	const urlsToTry = buildDiscoveryUrls(authorizationServerUrl);
	for (const { url: endpointUrl, type } of urlsToTry) {
		const response = await fetchWithCorsRetry(endpointUrl, headers, fetchFn);
		if (!response)
 /**
		* CORS error occurred - don't throw as the endpoint may not allow CORS,
		* continue trying other possible endpoints
		*/
		continue;
		if (!response.ok) {
			await response.body?.cancel();
			if (response.status >= 400 && response.status < 500) continue;
			throw new Error(`HTTP ${response.status} trying to load ${type === "oauth" ? "OAuth" : "OpenID provider"} metadata from ${endpointUrl}`);
		}
		if (type === "oauth") return OAuthMetadataSchema.parse(await response.json());
		else return OpenIdProviderDiscoveryMetadataSchema.parse(await response.json());
	}
}
/**
* Discovers the authorization server for an MCP server following
* {@link https://datatracker.ietf.org/doc/html/rfc9728 | RFC 9728} (OAuth 2.0 Protected
* Resource Metadata), with fallback to treating the server URL as the
* authorization server.
*
* This function combines two discovery steps into one call:
* 1. Probes `/.well-known/oauth-protected-resource` on the MCP server to find the
*    authorization server URL (RFC 9728).
* 2. Fetches authorization server metadata from that URL (RFC 8414 / OpenID Connect Discovery).
*
* Use this when you need the authorization server metadata for operations outside the
* {@linkcode auth} orchestrator, such as token refresh or token revocation.
*
* @param serverUrl - The MCP resource server URL
* @param opts - Optional configuration
* @param opts.resourceMetadataUrl - Override URL for the protected resource metadata endpoint
* @param opts.fetchFn - Custom fetch function for HTTP requests
* @returns Authorization server URL, metadata, and resource metadata (if available)
*/
async function discoverOAuthServerInfo(serverUrl, opts) {
	let resourceMetadata;
	let authorizationServerUrl;
	try {
		resourceMetadata = await discoverOAuthProtectedResourceMetadata(serverUrl, { resourceMetadataUrl: opts?.resourceMetadataUrl }, opts?.fetchFn);
		if (resourceMetadata.authorization_servers && resourceMetadata.authorization_servers.length > 0) authorizationServerUrl = resourceMetadata.authorization_servers[0];
	} catch {}
	if (!authorizationServerUrl) authorizationServerUrl = String(new URL("/", serverUrl));
	const authorizationServerMetadata = await discoverAuthorizationServerMetadata(authorizationServerUrl, { fetchFn: opts?.fetchFn });
	return {
		authorizationServerUrl,
		authorizationServerMetadata,
		resourceMetadata
	};
}
/**
* Begins the authorization flow with the given server, by generating a PKCE challenge and constructing the authorization URL.
*/
async function startAuthorization(authorizationServerUrl, { metadata, clientInformation, redirectUrl, scope, state, resource }) {
	let authorizationUrl;
	if (metadata) {
		authorizationUrl = new URL(metadata.authorization_endpoint);
		if (!metadata.response_types_supported.includes(AUTHORIZATION_CODE_RESPONSE_TYPE)) throw new Error(`Incompatible auth server: does not support response type ${AUTHORIZATION_CODE_RESPONSE_TYPE}`);
		if (metadata.code_challenge_methods_supported && !metadata.code_challenge_methods_supported.includes(AUTHORIZATION_CODE_CHALLENGE_METHOD)) throw new Error(`Incompatible auth server: does not support code challenge method ${AUTHORIZATION_CODE_CHALLENGE_METHOD}`);
	} else authorizationUrl = new URL("/authorize", authorizationServerUrl);
	const challenge = await pkceChallenge();
	const codeVerifier = challenge.code_verifier;
	const codeChallenge = challenge.code_challenge;
	authorizationUrl.searchParams.set("response_type", AUTHORIZATION_CODE_RESPONSE_TYPE);
	authorizationUrl.searchParams.set("client_id", clientInformation.client_id);
	authorizationUrl.searchParams.set("code_challenge", codeChallenge);
	authorizationUrl.searchParams.set("code_challenge_method", AUTHORIZATION_CODE_CHALLENGE_METHOD);
	authorizationUrl.searchParams.set("redirect_uri", String(redirectUrl));
	if (state) authorizationUrl.searchParams.set("state", state);
	if (scope) authorizationUrl.searchParams.set("scope", scope);
	if (scope?.includes("offline_access")) authorizationUrl.searchParams.append("prompt", "consent");
	if (resource) authorizationUrl.searchParams.set("resource", resource.href);
	return {
		authorizationUrl,
		codeVerifier
	};
}
/**
* Prepares token request parameters for an authorization code exchange.
*
* This is the default implementation used by fetchToken when the provider
* doesn't implement prepareTokenRequest.
*
* @param authorizationCode - The authorization code received from the authorization endpoint
* @param codeVerifier - The PKCE code verifier
* @param redirectUri - The redirect URI used in the authorization request
* @returns URLSearchParams for the authorization_code grant
*/
function prepareAuthorizationCodeRequest(authorizationCode, codeVerifier, redirectUri) {
	return new URLSearchParams({
		grant_type: "authorization_code",
		code: authorizationCode,
		code_verifier: codeVerifier,
		redirect_uri: String(redirectUri)
	});
}
/**
* Internal helper to execute a token request with the given parameters.
* Used by exchangeAuthorization, refreshAuthorization, and fetchToken.
*/
async function executeTokenRequest(authorizationServerUrl, { metadata, tokenRequestParams, clientInformation, addClientAuthentication, resource, fetchFn }) {
	const tokenUrl = metadata?.token_endpoint ? new URL(metadata.token_endpoint) : new URL("/token", authorizationServerUrl);
	const headers = new Headers({
		"Content-Type": "application/x-www-form-urlencoded",
		Accept: "application/json"
	});
	if (resource) tokenRequestParams.set("resource", resource.href);
	if (addClientAuthentication) await addClientAuthentication(headers, tokenRequestParams, tokenUrl, metadata);
	else if (clientInformation) applyClientAuthentication(selectClientAuthMethod(clientInformation, metadata?.token_endpoint_auth_methods_supported ?? []), clientInformation, headers, tokenRequestParams);
	const response = await (fetchFn ?? fetch)(tokenUrl, {
		method: "POST",
		headers,
		body: tokenRequestParams
	});
	if (!response.ok) throw await parseErrorResponse(response);
	return OAuthTokensSchema.parse(await response.json());
}
/**
* Exchange a refresh token for an updated access token.
*
* Supports multiple client authentication methods as specified in OAuth 2.1:
* - Automatically selects the best authentication method based on server support
* - Preserves the original refresh token if a new one is not returned
*
* @param authorizationServerUrl - The authorization server's base URL
* @param options - Configuration object containing client info, refresh token, etc.
* @returns Promise resolving to OAuth tokens (preserves original refresh_token if not replaced)
* @throws {Error} When token refresh fails or authentication is invalid
*/
async function refreshAuthorization(authorizationServerUrl, { metadata, clientInformation, refreshToken, resource, addClientAuthentication, fetchFn }) {
	return {
		refresh_token: refreshToken,
		...await executeTokenRequest(authorizationServerUrl, {
			metadata,
			tokenRequestParams: new URLSearchParams({
				grant_type: "refresh_token",
				refresh_token: refreshToken
			}),
			clientInformation,
			addClientAuthentication,
			resource,
			fetchFn
		})
	};
}
/**
* Unified token fetching that works with any grant type via provider.prepareTokenRequest().
*
* This function provides a single entry point for obtaining tokens regardless of the
* OAuth grant type. The provider's prepareTokenRequest() method determines which grant
* to use and supplies the grant-specific parameters.
*
* @param provider - OAuth client provider that implements prepareTokenRequest()
* @param authorizationServerUrl - The authorization server's base URL
* @param options - Configuration for the token request
* @returns Promise resolving to OAuth tokens
* @throws {Error} When provider doesn't implement prepareTokenRequest or token fetch fails
*
* @example
* // Provider for client_credentials:
* class MyProvider implements OAuthClientProvider {
*   prepareTokenRequest(scope) {
*     const params = new URLSearchParams({ grant_type: 'client_credentials' });
*     if (scope) params.set('scope', scope);
*     return params;
*   }
*   // ... other methods
* }
*
* const tokens = await fetchToken(provider, authServerUrl, { metadata });
*/
async function fetchToken(provider, authorizationServerUrl, { metadata, resource, authorizationCode, fetchFn } = {}) {
	const scope = provider.clientMetadata.scope;
	let tokenRequestParams;
	if (provider.prepareTokenRequest) tokenRequestParams = await provider.prepareTokenRequest(scope);
	if (!tokenRequestParams) {
		if (!authorizationCode) throw new Error("Either provider.prepareTokenRequest() or authorizationCode is required");
		if (!provider.redirectUrl) throw new Error("redirectUrl is required for authorization_code flow");
		tokenRequestParams = prepareAuthorizationCodeRequest(authorizationCode, await provider.codeVerifier(), provider.redirectUrl);
	}
	const clientInformation = await provider.clientInformation();
	return executeTokenRequest(authorizationServerUrl, {
		metadata,
		tokenRequestParams,
		clientInformation: clientInformation ?? void 0,
		addClientAuthentication: provider.addClientAuthentication,
		resource,
		fetchFn
	});
}
/**
* Performs OAuth 2.0 Dynamic Client Registration according to RFC 7591.
*/
async function registerClient(authorizationServerUrl, { metadata, clientMetadata, fetchFn }) {
	let registrationUrl;
	if (metadata) {
		if (!metadata.registration_endpoint) throw new Error("Incompatible auth server: does not support dynamic client registration");
		registrationUrl = new URL(metadata.registration_endpoint);
	} else registrationUrl = new URL("/register", authorizationServerUrl);
	const response = await (fetchFn ?? fetch)(registrationUrl, {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(clientMetadata)
	});
	if (!response.ok) throw await parseErrorResponse(response);
	return OAuthClientInformationFullSchema.parse(await response.json());
}

//#endregion
//#region node_modules/.pnpm/@modelcontextprotocol+sdk@1.27.1_zod@3.25.76/node_modules/@modelcontextprotocol/sdk/dist/esm/client/sse.js
var SseError = class extends Error {
	constructor(code, message, event) {
		super(`SSE error: ${message}`);
		this.code = code;
		this.event = event;
	}
};
/**
* Client transport for SSE: this will connect to a server using Server-Sent Events for receiving
* messages and make separate POST requests for sending messages.
* @deprecated SSEClientTransport is deprecated. Prefer to use StreamableHTTPClientTransport where possible instead. Note that because some servers are still using SSE, clients may need to support both transports during the migration period.
*/
var SSEClientTransport = class {
	constructor(url$1, opts) {
		this._url = url$1;
		this._resourceMetadataUrl = void 0;
		this._scope = void 0;
		this._eventSourceInit = opts?.eventSourceInit;
		this._requestInit = opts?.requestInit;
		this._authProvider = opts?.authProvider;
		this._fetch = opts?.fetch;
		this._fetchWithInit = createFetchWithInit(opts?.fetch, opts?.requestInit);
	}
	async _authThenStart() {
		if (!this._authProvider) throw new UnauthorizedError("No auth provider");
		let result;
		try {
			result = await auth(this._authProvider, {
				serverUrl: this._url,
				resourceMetadataUrl: this._resourceMetadataUrl,
				scope: this._scope,
				fetchFn: this._fetchWithInit
			});
		} catch (error) {
			this.onerror?.(error);
			throw error;
		}
		if (result !== "AUTHORIZED") throw new UnauthorizedError();
		return await this._startOrAuth();
	}
	async _commonHeaders() {
		const headers = {};
		if (this._authProvider) {
			const tokens = await this._authProvider.tokens();
			if (tokens) headers["Authorization"] = `Bearer ${tokens.access_token}`;
		}
		if (this._protocolVersion) headers["mcp-protocol-version"] = this._protocolVersion;
		const extraHeaders = normalizeHeaders(this._requestInit?.headers);
		return new Headers({
			...headers,
			...extraHeaders
		});
	}
	_startOrAuth() {
		const fetchImpl = this?._eventSourceInit?.fetch ?? this._fetch ?? fetch;
		return new Promise((resolve, reject) => {
			this._eventSource = new EventSource(this._url.href, {
				...this._eventSourceInit,
				fetch: async (url$1, init) => {
					const headers = await this._commonHeaders();
					headers.set("Accept", "text/event-stream");
					const response = await fetchImpl(url$1, {
						...init,
						headers
					});
					if (response.status === 401 && response.headers.has("www-authenticate")) {
						const { resourceMetadataUrl, scope } = extractWWWAuthenticateParams(response);
						this._resourceMetadataUrl = resourceMetadataUrl;
						this._scope = scope;
					}
					return response;
				}
			});
			this._abortController = new AbortController();
			this._eventSource.onerror = (event) => {
				if (event.code === 401 && this._authProvider) {
					this._authThenStart().then(resolve, reject);
					return;
				}
				const error = new SseError(event.code, event.message, event);
				reject(error);
				this.onerror?.(error);
			};
			this._eventSource.onopen = () => {};
			this._eventSource.addEventListener("endpoint", (event) => {
				const messageEvent = event;
				try {
					this._endpoint = new URL(messageEvent.data, this._url);
					if (this._endpoint.origin !== this._url.origin) throw new Error(`Endpoint origin does not match connection origin: ${this._endpoint.origin}`);
				} catch (error) {
					reject(error);
					this.onerror?.(error);
					this.close();
					return;
				}
				resolve();
			});
			this._eventSource.onmessage = (event) => {
				const messageEvent = event;
				let message;
				try {
					message = JSONRPCMessageSchema.parse(JSON.parse(messageEvent.data));
				} catch (error) {
					this.onerror?.(error);
					return;
				}
				this.onmessage?.(message);
			};
		});
	}
	async start() {
		if (this._eventSource) throw new Error("SSEClientTransport already started! If using Client class, note that connect() calls start() automatically.");
		return await this._startOrAuth();
	}
	/**
	* Call this method after the user has finished authorizing via their user agent and is redirected back to the MCP client application. This will exchange the authorization code for an access token, enabling the next connection attempt to successfully auth.
	*/
	async finishAuth(authorizationCode) {
		if (!this._authProvider) throw new UnauthorizedError("No auth provider");
		if (await auth(this._authProvider, {
			serverUrl: this._url,
			authorizationCode,
			resourceMetadataUrl: this._resourceMetadataUrl,
			scope: this._scope,
			fetchFn: this._fetchWithInit
		}) !== "AUTHORIZED") throw new UnauthorizedError("Failed to authorize");
	}
	async close() {
		this._abortController?.abort();
		this._eventSource?.close();
		this.onclose?.();
	}
	async send(message) {
		if (!this._endpoint) throw new Error("Not connected");
		try {
			const headers = await this._commonHeaders();
			headers.set("content-type", "application/json");
			const init = {
				...this._requestInit,
				method: "POST",
				headers,
				body: JSON.stringify(message),
				signal: this._abortController?.signal
			};
			const response = await (this._fetch ?? fetch)(this._endpoint, init);
			if (!response.ok) {
				const text = await response.text().catch(() => null);
				if (response.status === 401 && this._authProvider) {
					const { resourceMetadataUrl, scope } = extractWWWAuthenticateParams(response);
					this._resourceMetadataUrl = resourceMetadataUrl;
					this._scope = scope;
					if (await auth(this._authProvider, {
						serverUrl: this._url,
						resourceMetadataUrl: this._resourceMetadataUrl,
						scope: this._scope,
						fetchFn: this._fetchWithInit
					}) !== "AUTHORIZED") throw new UnauthorizedError();
					return this.send(message);
				}
				throw new Error(`Error POSTing to endpoint (HTTP ${response.status}): ${text}`);
			}
			await response.body?.cancel();
		} catch (error) {
			this.onerror?.(error);
			throw error;
		}
	}
	setProtocolVersion(version) {
		this._protocolVersion = version;
	}
};

//#endregion
//#region node_modules/.pnpm/eventsource-parser@3.0.6/node_modules/eventsource-parser/dist/stream.js
var EventSourceParserStream = class extends TransformStream {
	constructor({ onError, onRetry, onComment } = {}) {
		let parser;
		super({
			start(controller) {
				parser = createParser({
					onEvent: (event) => {
						controller.enqueue(event);
					},
					onError(error) {
						onError === "terminate" ? controller.error(error) : typeof onError == "function" && onError(error);
					},
					onRetry,
					onComment
				});
			},
			transform(chunk) {
				parser.feed(chunk);
			}
		});
	}
};

//#endregion
//#region node_modules/.pnpm/@modelcontextprotocol+sdk@1.27.1_zod@3.25.76/node_modules/@modelcontextprotocol/sdk/dist/esm/client/streamableHttp.js
const DEFAULT_STREAMABLE_HTTP_RECONNECTION_OPTIONS = {
	initialReconnectionDelay: 1e3,
	maxReconnectionDelay: 3e4,
	reconnectionDelayGrowFactor: 1.5,
	maxRetries: 2
};
var StreamableHTTPError = class extends Error {
	constructor(code, message) {
		super(`Streamable HTTP error: ${message}`);
		this.code = code;
	}
};
/**
* Client transport for Streamable HTTP: this implements the MCP Streamable HTTP transport specification.
* It will connect to a server using HTTP POST for sending messages and HTTP GET with Server-Sent Events
* for receiving messages.
*/
var StreamableHTTPClientTransport = class {
	constructor(url$1, opts) {
		this._hasCompletedAuthFlow = false;
		this._url = url$1;
		this._resourceMetadataUrl = void 0;
		this._scope = void 0;
		this._requestInit = opts?.requestInit;
		this._authProvider = opts?.authProvider;
		this._fetch = opts?.fetch;
		this._fetchWithInit = createFetchWithInit(opts?.fetch, opts?.requestInit);
		this._sessionId = opts?.sessionId;
		this._reconnectionOptions = opts?.reconnectionOptions ?? DEFAULT_STREAMABLE_HTTP_RECONNECTION_OPTIONS;
	}
	async _authThenStart() {
		if (!this._authProvider) throw new UnauthorizedError("No auth provider");
		let result;
		try {
			result = await auth(this._authProvider, {
				serverUrl: this._url,
				resourceMetadataUrl: this._resourceMetadataUrl,
				scope: this._scope,
				fetchFn: this._fetchWithInit
			});
		} catch (error) {
			this.onerror?.(error);
			throw error;
		}
		if (result !== "AUTHORIZED") throw new UnauthorizedError();
		return await this._startOrAuthSse({ resumptionToken: void 0 });
	}
	async _commonHeaders() {
		const headers = {};
		if (this._authProvider) {
			const tokens = await this._authProvider.tokens();
			if (tokens) headers["Authorization"] = `Bearer ${tokens.access_token}`;
		}
		if (this._sessionId) headers["mcp-session-id"] = this._sessionId;
		if (this._protocolVersion) headers["mcp-protocol-version"] = this._protocolVersion;
		const extraHeaders = normalizeHeaders(this._requestInit?.headers);
		return new Headers({
			...headers,
			...extraHeaders
		});
	}
	async _startOrAuthSse(options) {
		const { resumptionToken } = options;
		try {
			const headers = await this._commonHeaders();
			headers.set("Accept", "text/event-stream");
			if (resumptionToken) headers.set("last-event-id", resumptionToken);
			const response = await (this._fetch ?? fetch)(this._url, {
				method: "GET",
				headers,
				signal: this._abortController?.signal
			});
			if (!response.ok) {
				await response.body?.cancel();
				if (response.status === 401 && this._authProvider) return await this._authThenStart();
				if (response.status === 405) return;
				throw new StreamableHTTPError(response.status, `Failed to open SSE stream: ${response.statusText}`);
			}
			this._handleSseStream(response.body, options, true);
		} catch (error) {
			this.onerror?.(error);
			throw error;
		}
	}
	/**
	* Calculates the next reconnection delay using  backoff algorithm
	*
	* @param attempt Current reconnection attempt count for the specific stream
	* @returns Time to wait in milliseconds before next reconnection attempt
	*/
	_getNextReconnectionDelay(attempt) {
		if (this._serverRetryMs !== void 0) return this._serverRetryMs;
		const initialDelay = this._reconnectionOptions.initialReconnectionDelay;
		const growFactor = this._reconnectionOptions.reconnectionDelayGrowFactor;
		const maxDelay = this._reconnectionOptions.maxReconnectionDelay;
		return Math.min(initialDelay * Math.pow(growFactor, attempt), maxDelay);
	}
	/**
	* Schedule a reconnection attempt using server-provided retry interval or backoff
	*
	* @param lastEventId The ID of the last received event for resumability
	* @param attemptCount Current reconnection attempt count for this specific stream
	*/
	_scheduleReconnection(options, attemptCount = 0) {
		const maxRetries = this._reconnectionOptions.maxRetries;
		if (attemptCount >= maxRetries) {
			this.onerror?.(/* @__PURE__ */ new Error(`Maximum reconnection attempts (${maxRetries}) exceeded.`));
			return;
		}
		const delay = this._getNextReconnectionDelay(attemptCount);
		this._reconnectionTimeout = setTimeout(() => {
			this._startOrAuthSse(options).catch((error) => {
				this.onerror?.(/* @__PURE__ */ new Error(`Failed to reconnect SSE stream: ${error instanceof Error ? error.message : String(error)}`));
				this._scheduleReconnection(options, attemptCount + 1);
			});
		}, delay);
	}
	_handleSseStream(stream, options, isReconnectable) {
		if (!stream) return;
		const { onresumptiontoken, replayMessageId } = options;
		let lastEventId;
		let hasPrimingEvent = false;
		let receivedResponse = false;
		const processStream = async () => {
			try {
				const reader = stream.pipeThrough(new TextDecoderStream()).pipeThrough(new EventSourceParserStream({ onRetry: (retryMs) => {
					this._serverRetryMs = retryMs;
				} })).getReader();
				while (true) {
					const { value: event, done } = await reader.read();
					if (done) break;
					if (event.id) {
						lastEventId = event.id;
						hasPrimingEvent = true;
						onresumptiontoken?.(event.id);
					}
					if (!event.data) continue;
					if (!event.event || event.event === "message") try {
						const message = JSONRPCMessageSchema.parse(JSON.parse(event.data));
						if (isJSONRPCResultResponse(message)) {
							receivedResponse = true;
							if (replayMessageId !== void 0) message.id = replayMessageId;
						}
						this.onmessage?.(message);
					} catch (error) {
						this.onerror?.(error);
					}
				}
				if ((isReconnectable || hasPrimingEvent) && !receivedResponse && this._abortController && !this._abortController.signal.aborted) this._scheduleReconnection({
					resumptionToken: lastEventId,
					onresumptiontoken,
					replayMessageId
				}, 0);
			} catch (error) {
				this.onerror?.(/* @__PURE__ */ new Error(`SSE stream disconnected: ${error}`));
				if ((isReconnectable || hasPrimingEvent) && !receivedResponse && this._abortController && !this._abortController.signal.aborted) try {
					this._scheduleReconnection({
						resumptionToken: lastEventId,
						onresumptiontoken,
						replayMessageId
					}, 0);
				} catch (error$1) {
					this.onerror?.(/* @__PURE__ */ new Error(`Failed to reconnect: ${error$1 instanceof Error ? error$1.message : String(error$1)}`));
				}
			}
		};
		processStream();
	}
	async start() {
		if (this._abortController) throw new Error("StreamableHTTPClientTransport already started! If using Client class, note that connect() calls start() automatically.");
		this._abortController = new AbortController();
	}
	/**
	* Call this method after the user has finished authorizing via their user agent and is redirected back to the MCP client application. This will exchange the authorization code for an access token, enabling the next connection attempt to successfully auth.
	*/
	async finishAuth(authorizationCode) {
		if (!this._authProvider) throw new UnauthorizedError("No auth provider");
		if (await auth(this._authProvider, {
			serverUrl: this._url,
			authorizationCode,
			resourceMetadataUrl: this._resourceMetadataUrl,
			scope: this._scope,
			fetchFn: this._fetchWithInit
		}) !== "AUTHORIZED") throw new UnauthorizedError("Failed to authorize");
	}
	async close() {
		if (this._reconnectionTimeout) {
			clearTimeout(this._reconnectionTimeout);
			this._reconnectionTimeout = void 0;
		}
		this._abortController?.abort();
		this.onclose?.();
	}
	async send(message, options) {
		try {
			const { resumptionToken, onresumptiontoken } = options || {};
			if (resumptionToken) {
				this._startOrAuthSse({
					resumptionToken,
					replayMessageId: isJSONRPCRequest(message) ? message.id : void 0
				}).catch((err) => this.onerror?.(err));
				return;
			}
			const headers = await this._commonHeaders();
			headers.set("content-type", "application/json");
			headers.set("accept", "application/json, text/event-stream");
			const init = {
				...this._requestInit,
				method: "POST",
				headers,
				body: JSON.stringify(message),
				signal: this._abortController?.signal
			};
			const response = await (this._fetch ?? fetch)(this._url, init);
			const sessionId = response.headers.get("mcp-session-id");
			if (sessionId) this._sessionId = sessionId;
			if (!response.ok) {
				const text = await response.text().catch(() => null);
				if (response.status === 401 && this._authProvider) {
					if (this._hasCompletedAuthFlow) throw new StreamableHTTPError(401, "Server returned 401 after successful authentication");
					const { resourceMetadataUrl, scope } = extractWWWAuthenticateParams(response);
					this._resourceMetadataUrl = resourceMetadataUrl;
					this._scope = scope;
					if (await auth(this._authProvider, {
						serverUrl: this._url,
						resourceMetadataUrl: this._resourceMetadataUrl,
						scope: this._scope,
						fetchFn: this._fetchWithInit
					}) !== "AUTHORIZED") throw new UnauthorizedError();
					this._hasCompletedAuthFlow = true;
					return this.send(message);
				}
				if (response.status === 403 && this._authProvider) {
					const { resourceMetadataUrl, scope, error } = extractWWWAuthenticateParams(response);
					if (error === "insufficient_scope") {
						const wwwAuthHeader = response.headers.get("WWW-Authenticate");
						if (this._lastUpscopingHeader === wwwAuthHeader) throw new StreamableHTTPError(403, "Server returned 403 after trying upscoping");
						if (scope) this._scope = scope;
						if (resourceMetadataUrl) this._resourceMetadataUrl = resourceMetadataUrl;
						this._lastUpscopingHeader = wwwAuthHeader ?? void 0;
						if (await auth(this._authProvider, {
							serverUrl: this._url,
							resourceMetadataUrl: this._resourceMetadataUrl,
							scope: this._scope,
							fetchFn: this._fetch
						}) !== "AUTHORIZED") throw new UnauthorizedError();
						return this.send(message);
					}
				}
				throw new StreamableHTTPError(response.status, `Error POSTing to endpoint: ${text}`);
			}
			this._hasCompletedAuthFlow = false;
			this._lastUpscopingHeader = void 0;
			if (response.status === 202) {
				await response.body?.cancel();
				if (isInitializedNotification(message)) this._startOrAuthSse({ resumptionToken: void 0 }).catch((err) => this.onerror?.(err));
				return;
			}
			const hasRequests = (Array.isArray(message) ? message : [message]).filter((msg) => "method" in msg && "id" in msg && msg.id !== void 0).length > 0;
			const contentType = response.headers.get("content-type");
			if (hasRequests) if (contentType?.includes("text/event-stream")) this._handleSseStream(response.body, { onresumptiontoken }, false);
			else if (contentType?.includes("application/json")) {
				const data = await response.json();
				const responseMessages = Array.isArray(data) ? data.map((msg) => JSONRPCMessageSchema.parse(msg)) : [JSONRPCMessageSchema.parse(data)];
				for (const msg of responseMessages) this.onmessage?.(msg);
			} else {
				await response.body?.cancel();
				throw new StreamableHTTPError(-1, `Unexpected content type: ${contentType}`);
			}
			else await response.body?.cancel();
		} catch (error) {
			this.onerror?.(error);
			throw error;
		}
	}
	get sessionId() {
		return this._sessionId;
	}
	/**
	* Terminates the current session by sending a DELETE request to the server.
	*
	* Clients that no longer need a particular session
	* (e.g., because the user is leaving the client application) SHOULD send an
	* HTTP DELETE to the MCP endpoint with the Mcp-Session-Id header to explicitly
	* terminate the session.
	*
	* The server MAY respond with HTTP 405 Method Not Allowed, indicating that
	* the server does not allow clients to terminate sessions.
	*/
	async terminateSession() {
		if (!this._sessionId) return;
		try {
			const headers = await this._commonHeaders();
			const init = {
				...this._requestInit,
				method: "DELETE",
				headers,
				signal: this._abortController?.signal
			};
			const response = await (this._fetch ?? fetch)(this._url, init);
			await response.body?.cancel();
			if (!response.ok && response.status !== 405) throw new StreamableHTTPError(response.status, `Failed to terminate session: ${response.statusText}`);
			this._sessionId = void 0;
		} catch (error) {
			this.onerror?.(error);
			throw error;
		}
	}
	setProtocolVersion(version) {
		this._protocolVersion = version;
	}
	get protocolVersion() {
		return this._protocolVersion;
	}
	/**
	* Resume an SSE stream from a previous event ID.
	* Opens a GET SSE connection with Last-Event-ID header to replay missed events.
	*
	* @param lastEventId The event ID to resume from
	* @param options Optional callback to receive new resumption tokens
	*/
	async resumeStream(lastEventId, options) {
		await this._startOrAuthSse({
			resumptionToken: lastEventId,
			onresumptiontoken: options?.onresumptiontoken
		});
	}
};

//#endregion
//#region node_modules/.pnpm/@modelcontextprotocol+sdk@1.27.1_zod@3.25.76/node_modules/@modelcontextprotocol/sdk/dist/esm/server/stdio.js
/**
* Server transport for stdio: this communicates with an MCP client by reading from the current process' stdin and writing to stdout.
*
* This transport is only available in Node.js environments.
*/
var StdioServerTransport = class {
	constructor(_stdin = process.stdin, _stdout = process.stdout) {
		this._stdin = _stdin;
		this._stdout = _stdout;
		this._readBuffer = new ReadBuffer();
		this._started = false;
		this._ondata = (chunk) => {
			this._readBuffer.append(chunk);
			this.processReadBuffer();
		};
		this._onerror = (error) => {
			this.onerror?.(error);
		};
	}
	/**
	* Starts listening for messages on stdin.
	*/
	async start() {
		if (this._started) throw new Error("StdioServerTransport already started! If using Server class, note that connect() calls start() automatically.");
		this._started = true;
		this._stdin.on("data", this._ondata);
		this._stdin.on("error", this._onerror);
	}
	processReadBuffer() {
		while (true) try {
			const message = this._readBuffer.readMessage();
			if (message === null) break;
			this.onmessage?.(message);
		} catch (error) {
			this.onerror?.(error);
		}
	}
	async close() {
		this._stdin.off("data", this._ondata);
		this._stdin.off("error", this._onerror);
		if (this._stdin.listenerCount("data") === 0) this._stdin.pause();
		this._readBuffer.clear();
		this.onclose?.();
	}
	send(message) {
		return new Promise((resolve) => {
			const json = serializeMessage(message);
			if (this._stdout.write(json)) resolve();
			else this._stdout.once("drain", resolve);
		});
	}
};

//#endregion
//#region src/startStdioServer.ts
let ServerType = /* @__PURE__ */ function(ServerType$1) {
	ServerType$1["HTTPStream"] = "HTTPStream";
	ServerType$1["SSE"] = "SSE";
	return ServerType$1;
}({});
const startStdioServer = async ({ initStdioServer, initStreamClient, serverType, transportOptions = {}, url: url$1 }) => {
	let transport;
	switch (serverType) {
		case ServerType.SSE:
			transport = new SSEClientTransport(new URL(url$1), transportOptions);
			break;
		default: transport = new StreamableHTTPClientTransport(new URL(url$1), transportOptions);
	}
	const streamClient = initStreamClient ? await initStreamClient() : new Client({
		name: "mcp-proxy",
		version: "1.0.0"
	}, { capabilities: {} });
	await streamClient.connect(transport);
	const serverVersion = streamClient.getServerVersion();
	const serverCapabilities = streamClient.getServerCapabilities();
	const stdioServer = initStdioServer ? await initStdioServer() : new Server(serverVersion, { capabilities: serverCapabilities });
	const stdioTransport = new StdioServerTransport();
	await stdioServer.connect(stdioTransport);
	await proxyServer({
		client: streamClient,
		server: stdioServer,
		serverCapabilities
	});
	return stdioServer;
};

//#endregion
//#region src/tapTransport.ts
const tapTransport = (transport, eventHandler) => {
	const originalClose = transport.close.bind(transport);
	const originalOnClose = transport.onclose?.bind(transport);
	const originalOnError = transport.onerror?.bind(transport);
	const originalOnMessage = transport.onmessage?.bind(transport);
	const originalSend = transport.send.bind(transport);
	const originalStart = transport.start.bind(transport);
	transport.close = async () => {
		eventHandler({ type: "close" });
		return originalClose?.();
	};
	transport.onclose = async () => {
		eventHandler({ type: "onclose" });
		return originalOnClose?.();
	};
	transport.onerror = async (error) => {
		eventHandler({
			error,
			type: "onerror"
		});
		return originalOnError?.(error);
	};
	transport.onmessage = async (message) => {
		eventHandler({
			message,
			type: "onmessage"
		});
		return originalOnMessage?.(message);
	};
	transport.send = async (message) => {
		eventHandler({
			message,
			type: "send"
		});
		return originalSend?.(message);
	};
	transport.start = async () => {
		eventHandler({ type: "start" });
		return originalStart?.();
	};
	return transport;
};

//#endregion
export { AuthenticationMiddleware, InMemoryEventStore, ServerType, proxyServer, startHTTPServer, startStdioServer, tapTransport };
//# sourceMappingURL=index.mjs.map