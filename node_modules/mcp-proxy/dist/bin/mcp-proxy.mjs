#!/usr/bin/env node
import { D as __toESM, E as __commonJSMin, a as startHTTPServer, i as Client, n as serializeMessage, o as proxyServer, r as Server, t as ReadBuffer, w as InMemoryEventStore } from "../stdio-BmURZCbz.mjs";
import { createRequire } from "node:module";
import { basename, dirname, extname, join, normalize, relative, resolve } from "path";
import { format, inspect } from "util";
import { readFileSync, readdirSync, statSync, writeFile } from "fs";
import { setTimeout as setTimeout$1 } from "node:timers";
import util from "node:util";
import { pipenet } from "pipenet";
import { notStrictEqual, strictEqual } from "assert";
import { fileURLToPath } from "url";
import { readFileSync as readFileSync$1, readdirSync as readdirSync$1 } from "node:fs";
import { spawn } from "node:child_process";
import { PassThrough, Transform } from "node:stream";

//#region node_modules/.pnpm/eventsource-parser@3.0.2/node_modules/eventsource-parser/dist/index.js
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
		if (crIndex !== -1 && lfIndex !== -1 ? lineEnd = Math.min(crIndex, lfIndex) : crIndex !== -1 ? lineEnd = crIndex : lfIndex !== -1 && (lineEnd = lfIndex), lineEnd === -1) {
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
//#region node_modules/.pnpm/eventsource@4.1.0/node_modules/eventsource/dist/index.js
var ErrorEvent = class extends Event {
	/**
	* Constructs a new `ErrorEvent` instance. This is typically not called directly,
	* but rather emitted by the `EventSource` object when an error occurs.
	*
	* @param type - The type of the event (should be "error")
	* @param errorEventInitDict - Optional properties to include in the error event
	*/
	constructor(type, errorEventInitDict) {
		var _a$1, _b$1;
		super(type), this.code = (_a$1 = errorEventInitDict == null ? void 0 : errorEventInitDict.code) != null ? _a$1 : void 0, this.message = (_b$1 = errorEventInitDict == null ? void 0 : errorEventInitDict.message) != null ? _b$1 : void 0;
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
	[Symbol.for("nodejs.util.inspect.custom")](_depth, options, inspect$1) {
		return inspect$1(inspectableError(this), options);
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
	[Symbol.for("Deno.customInspect")](inspect$1, options) {
		return inspect$1(inspectableError(this), options);
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
	constructor(url, eventSourceInitDict) {
		var _a$1, _b$1;
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
			if (url instanceof URL) __privateSet(this, _url, url);
			else if (typeof url == "string") __privateSet(this, _url, new URL(url, getBaseURL()));
			else throw new Error("Invalid URL");
		} catch {
			throw syntaxError("An invalid or illegal string was specified");
		}
		__privateSet(this, _parser, createParser({
			onEvent: __privateGet(this, _onEvent),
			onRetry: __privateGet(this, _onRetryChange)
		})), __privateSet(this, _readyState, this.CONNECTING), __privateSet(this, _reconnectInterval, 3e3), __privateSet(this, _fetch, (_a$1 = eventSourceInitDict == null ? void 0 : eventSourceInitDict.fetch) != null ? _a$1 : globalThis.fetch), __privateSet(this, _withCredentials, (_b$1 = eventSourceInitDict == null ? void 0 : eventSourceInitDict.withCredentials) != null ? _b$1 : !1), __privateMethod(this, _EventSource_instances, connect_fn).call(this);
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
	var _a$1;
	const init = {
		mode: "cors",
		redirect: "follow",
		headers: {
			Accept: "text/event-stream",
			...__privateGet(this, _lastEventId) ? { "Last-Event-ID": __privateGet(this, _lastEventId) } : void 0
		},
		cache: "no-store",
		signal: (_a$1 = __privateGet(this, _controller)) == null ? void 0 : _a$1.signal
	};
	return "window" in globalThis && (init.credentials = this.withCredentials ? "include" : "same-origin"), init;
}, _onEvent = /* @__PURE__ */ new WeakMap(), _onRetryChange = /* @__PURE__ */ new WeakMap(), failConnection_fn = function(message, code) {
	var _a$1;
	__privateGet(this, _readyState) !== this.CLOSED && __privateSet(this, _readyState, this.CLOSED);
	const errorEvent = new ErrorEvent("error", {
		code,
		message
	});
	(_a$1 = __privateGet(this, _onError)) == null || _a$1.call(this, errorEvent), this.dispatchEvent(errorEvent);
}, scheduleReconnect_fn = function(message, code) {
	var _a$1;
	if (__privateGet(this, _readyState) === this.CLOSED) return;
	__privateSet(this, _readyState, this.CONNECTING);
	const errorEvent = new ErrorEvent("error", {
		code,
		message
	});
	(_a$1 = __privateGet(this, _onError)) == null || _a$1.call(this, errorEvent), this.dispatchEvent(errorEvent), __privateSet(this, _reconnectTimer, setTimeout(__privateGet(this, _reconnect), __privateGet(this, _reconnectInterval)));
}, _reconnect = /* @__PURE__ */ new WeakMap(), EventSource.CONNECTING = 0, EventSource.OPEN = 1, EventSource.CLOSED = 2;
Object.defineProperty(EventSource, Symbol.for("eventsource.supports-fetch-override"), {
	value: !0,
	writable: !1,
	configurable: !1,
	enumerable: !1
});
function getBaseURL() {
	const doc = "document" in globalThis ? globalThis.document : void 0;
	return doc && typeof doc == "object" && "baseURI" in doc && typeof doc.baseURI == "string" ? doc.baseURI : void 0;
}

//#endregion
//#region node_modules/.pnpm/cliui@9.0.1/node_modules/cliui/build/lib/index.js
const align = {
	right: alignRight,
	center: alignCenter
};
const top = 0;
const right = 1;
const bottom = 2;
const left = 3;
var UI = class {
	constructor(opts) {
		var _a$1;
		this.width = opts.width;
		this.wrap = (_a$1 = opts.wrap) !== null && _a$1 !== void 0 ? _a$1 : true;
		this.rows = [];
	}
	span(...args) {
		const cols = this.div(...args);
		cols.span = true;
	}
	resetOutput() {
		this.rows = [];
	}
	div(...args) {
		if (args.length === 0) this.div("");
		if (this.wrap && this.shouldApplyLayoutDSL(...args) && typeof args[0] === "string") return this.applyLayoutDSL(args[0]);
		const cols = args.map((arg) => {
			if (typeof arg === "string") return this.colFromString(arg);
			return arg;
		});
		this.rows.push(cols);
		return cols;
	}
	shouldApplyLayoutDSL(...args) {
		return args.length === 1 && typeof args[0] === "string" && /[\t\n]/.test(args[0]);
	}
	applyLayoutDSL(str) {
		const rows = str.split("\n").map((row) => row.split("	"));
		let leftColumnWidth = 0;
		rows.forEach((columns) => {
			if (columns.length > 1 && mixin$1.stringWidth(columns[0]) > leftColumnWidth) leftColumnWidth = Math.min(Math.floor(this.width * .5), mixin$1.stringWidth(columns[0]));
		});
		rows.forEach((columns) => {
			this.div(...columns.map((r, i) => {
				return {
					text: r.trim(),
					padding: this.measurePadding(r),
					width: i === 0 && columns.length > 1 ? leftColumnWidth : void 0
				};
			}));
		});
		return this.rows[this.rows.length - 1];
	}
	colFromString(text) {
		return {
			text,
			padding: this.measurePadding(text)
		};
	}
	measurePadding(str) {
		const noAnsi = mixin$1.stripAnsi(str);
		return [
			0,
			noAnsi.match(/\s*$/)[0].length,
			0,
			noAnsi.match(/^\s*/)[0].length
		];
	}
	toString() {
		const lines = [];
		this.rows.forEach((row) => {
			this.rowToString(row, lines);
		});
		return lines.filter((line) => !line.hidden).map((line) => line.text).join("\n");
	}
	rowToString(row, lines) {
		this.rasterize(row).forEach((rrow, r) => {
			let str = "";
			rrow.forEach((col, c) => {
				const { width } = row[c];
				const wrapWidth = this.negatePadding(row[c]);
				let ts = col;
				if (wrapWidth > mixin$1.stringWidth(col)) ts += " ".repeat(wrapWidth - mixin$1.stringWidth(col));
				if (row[c].align && row[c].align !== "left" && this.wrap) {
					const fn = align[row[c].align];
					ts = fn(ts, wrapWidth);
					if (mixin$1.stringWidth(ts) < wrapWidth) ts += " ".repeat((width || 0) - mixin$1.stringWidth(ts) - 1);
				}
				const padding = row[c].padding || [
					0,
					0,
					0,
					0
				];
				if (padding[left]) str += " ".repeat(padding[left]);
				str += addBorder(row[c], ts, "| ");
				str += ts;
				str += addBorder(row[c], ts, " |");
				if (padding[right]) str += " ".repeat(padding[right]);
				if (r === 0 && lines.length > 0) str = this.renderInline(str, lines[lines.length - 1]);
			});
			lines.push({
				text: str.replace(/ +$/, ""),
				span: row.span
			});
		});
		return lines;
	}
	renderInline(source, previousLine) {
		const match = source.match(/^ */);
		const leadingWhitespace = match ? match[0].length : 0;
		const target = previousLine.text;
		const targetTextWidth = mixin$1.stringWidth(target.trimRight());
		if (!previousLine.span) return source;
		if (!this.wrap) {
			previousLine.hidden = true;
			return target + source;
		}
		if (leadingWhitespace < targetTextWidth) return source;
		previousLine.hidden = true;
		return target.trimRight() + " ".repeat(leadingWhitespace - targetTextWidth) + source.trimLeft();
	}
	rasterize(row) {
		const rrows = [];
		const widths = this.columnWidths(row);
		let wrapped;
		row.forEach((col, c) => {
			col.width = widths[c];
			if (this.wrap) wrapped = mixin$1.wrap(col.text, this.negatePadding(col), { hard: true }).split("\n");
			else wrapped = col.text.split("\n");
			if (col.border) {
				wrapped.unshift("." + "-".repeat(this.negatePadding(col) + 2) + ".");
				wrapped.push("'" + "-".repeat(this.negatePadding(col) + 2) + "'");
			}
			if (col.padding) {
				wrapped.unshift(...new Array(col.padding[top] || 0).fill(""));
				wrapped.push(...new Array(col.padding[bottom] || 0).fill(""));
			}
			wrapped.forEach((str, r) => {
				if (!rrows[r]) rrows.push([]);
				const rrow = rrows[r];
				for (let i = 0; i < c; i++) if (rrow[i] === void 0) rrow.push("");
				rrow.push(str);
			});
		});
		return rrows;
	}
	negatePadding(col) {
		let wrapWidth = col.width || 0;
		if (col.padding) wrapWidth -= (col.padding[left] || 0) + (col.padding[right] || 0);
		if (col.border) wrapWidth -= 4;
		return wrapWidth;
	}
	columnWidths(row) {
		if (!this.wrap) return row.map((col) => {
			return col.width || mixin$1.stringWidth(col.text);
		});
		let unset = row.length;
		let remainingWidth = this.width;
		const widths = row.map((col) => {
			if (col.width) {
				unset--;
				remainingWidth -= col.width;
				return col.width;
			}
		});
		const unsetWidth = unset ? Math.floor(remainingWidth / unset) : 0;
		return widths.map((w, i) => {
			if (w === void 0) return Math.max(unsetWidth, _minWidth(row[i]));
			return w;
		});
	}
};
function addBorder(col, ts, style) {
	if (col.border) {
		if (/[.']-+[.']/.test(ts)) return "";
		if (ts.trim().length !== 0) return style;
		return "  ";
	}
	return "";
}
function _minWidth(col) {
	const padding = col.padding || [];
	const minWidth = 1 + (padding[left] || 0) + (padding[right] || 0);
	if (col.border) return minWidth + 4;
	return minWidth;
}
function getWindowWidth() {
	/* c8 ignore next 5: depends on terminal */
	if (typeof process === "object" && process.stdout && process.stdout.columns) return process.stdout.columns;
	return 80;
}
function alignRight(str, width) {
	str = str.trim();
	const strWidth = mixin$1.stringWidth(str);
	if (strWidth < width) return " ".repeat(width - strWidth) + str;
	return str;
}
function alignCenter(str, width) {
	str = str.trim();
	const strWidth = mixin$1.stringWidth(str);
	/* c8 ignore next 3 */
	if (strWidth >= width) return str;
	return " ".repeat(width - strWidth >> 1) + str;
}
let mixin$1;
function cliui(opts, _mixin) {
	mixin$1 = _mixin;
	return new UI({
		width: (opts === null || opts === void 0 ? void 0 : opts.width) || getWindowWidth(),
		wrap: opts === null || opts === void 0 ? void 0 : opts.wrap
	});
}

//#endregion
//#region node_modules/.pnpm/ansi-regex@6.1.0/node_modules/ansi-regex/index.js
function ansiRegex({ onlyFirst = false } = {}) {
	const pattern = [`[\\u001B\\u009B][[\\]()#;?]*(?:(?:(?:(?:;[-a-zA-Z\\d\\/#&.:=?%@~_]+)*|[a-zA-Z\\d]+(?:;[-a-zA-Z\\d\\/#&.:=?%@~_]*)*)?(?:\\u0007|\\u001B\\u005C|\\u009C))`, "(?:(?:\\d{1,4}(?:;\\d{0,4})*)?[\\dA-PR-TZcf-nq-uy=><~]))"].join("|");
	return new RegExp(pattern, onlyFirst ? void 0 : "g");
}

//#endregion
//#region node_modules/.pnpm/strip-ansi@7.1.0/node_modules/strip-ansi/index.js
const regex = ansiRegex();
function stripAnsi(string) {
	if (typeof string !== "string") throw new TypeError(`Expected a \`string\`, got \`${typeof string}\``);
	return string.replace(regex, "");
}

//#endregion
//#region node_modules/.pnpm/get-east-asian-width@1.3.0/node_modules/get-east-asian-width/lookup.js
function isAmbiguous(x) {
	return x === 161 || x === 164 || x === 167 || x === 168 || x === 170 || x === 173 || x === 174 || x >= 176 && x <= 180 || x >= 182 && x <= 186 || x >= 188 && x <= 191 || x === 198 || x === 208 || x === 215 || x === 216 || x >= 222 && x <= 225 || x === 230 || x >= 232 && x <= 234 || x === 236 || x === 237 || x === 240 || x === 242 || x === 243 || x >= 247 && x <= 250 || x === 252 || x === 254 || x === 257 || x === 273 || x === 275 || x === 283 || x === 294 || x === 295 || x === 299 || x >= 305 && x <= 307 || x === 312 || x >= 319 && x <= 322 || x === 324 || x >= 328 && x <= 331 || x === 333 || x === 338 || x === 339 || x === 358 || x === 359 || x === 363 || x === 462 || x === 464 || x === 466 || x === 468 || x === 470 || x === 472 || x === 474 || x === 476 || x === 593 || x === 609 || x === 708 || x === 711 || x >= 713 && x <= 715 || x === 717 || x === 720 || x >= 728 && x <= 731 || x === 733 || x === 735 || x >= 768 && x <= 879 || x >= 913 && x <= 929 || x >= 931 && x <= 937 || x >= 945 && x <= 961 || x >= 963 && x <= 969 || x === 1025 || x >= 1040 && x <= 1103 || x === 1105 || x === 8208 || x >= 8211 && x <= 8214 || x === 8216 || x === 8217 || x === 8220 || x === 8221 || x >= 8224 && x <= 8226 || x >= 8228 && x <= 8231 || x === 8240 || x === 8242 || x === 8243 || x === 8245 || x === 8251 || x === 8254 || x === 8308 || x === 8319 || x >= 8321 && x <= 8324 || x === 8364 || x === 8451 || x === 8453 || x === 8457 || x === 8467 || x === 8470 || x === 8481 || x === 8482 || x === 8486 || x === 8491 || x === 8531 || x === 8532 || x >= 8539 && x <= 8542 || x >= 8544 && x <= 8555 || x >= 8560 && x <= 8569 || x === 8585 || x >= 8592 && x <= 8601 || x === 8632 || x === 8633 || x === 8658 || x === 8660 || x === 8679 || x === 8704 || x === 8706 || x === 8707 || x === 8711 || x === 8712 || x === 8715 || x === 8719 || x === 8721 || x === 8725 || x === 8730 || x >= 8733 && x <= 8736 || x === 8739 || x === 8741 || x >= 8743 && x <= 8748 || x === 8750 || x >= 8756 && x <= 8759 || x === 8764 || x === 8765 || x === 8776 || x === 8780 || x === 8786 || x === 8800 || x === 8801 || x >= 8804 && x <= 8807 || x === 8810 || x === 8811 || x === 8814 || x === 8815 || x === 8834 || x === 8835 || x === 8838 || x === 8839 || x === 8853 || x === 8857 || x === 8869 || x === 8895 || x === 8978 || x >= 9312 && x <= 9449 || x >= 9451 && x <= 9547 || x >= 9552 && x <= 9587 || x >= 9600 && x <= 9615 || x >= 9618 && x <= 9621 || x === 9632 || x === 9633 || x >= 9635 && x <= 9641 || x === 9650 || x === 9651 || x === 9654 || x === 9655 || x === 9660 || x === 9661 || x === 9664 || x === 9665 || x >= 9670 && x <= 9672 || x === 9675 || x >= 9678 && x <= 9681 || x >= 9698 && x <= 9701 || x === 9711 || x === 9733 || x === 9734 || x === 9737 || x === 9742 || x === 9743 || x === 9756 || x === 9758 || x === 9792 || x === 9794 || x === 9824 || x === 9825 || x >= 9827 && x <= 9829 || x >= 9831 && x <= 9834 || x === 9836 || x === 9837 || x === 9839 || x === 9886 || x === 9887 || x === 9919 || x >= 9926 && x <= 9933 || x >= 9935 && x <= 9939 || x >= 9941 && x <= 9953 || x === 9955 || x === 9960 || x === 9961 || x >= 9963 && x <= 9969 || x === 9972 || x >= 9974 && x <= 9977 || x === 9979 || x === 9980 || x === 9982 || x === 9983 || x === 10045 || x >= 10102 && x <= 10111 || x >= 11094 && x <= 11097 || x >= 12872 && x <= 12879 || x >= 57344 && x <= 63743 || x >= 65024 && x <= 65039 || x === 65533 || x >= 127232 && x <= 127242 || x >= 127248 && x <= 127277 || x >= 127280 && x <= 127337 || x >= 127344 && x <= 127373 || x === 127375 || x === 127376 || x >= 127387 && x <= 127404 || x >= 917760 && x <= 917999 || x >= 983040 && x <= 1048573 || x >= 1048576 && x <= 1114109;
}
function isFullWidth(x) {
	return x === 12288 || x >= 65281 && x <= 65376 || x >= 65504 && x <= 65510;
}
function isWide(x) {
	return x >= 4352 && x <= 4447 || x === 8986 || x === 8987 || x === 9001 || x === 9002 || x >= 9193 && x <= 9196 || x === 9200 || x === 9203 || x === 9725 || x === 9726 || x === 9748 || x === 9749 || x >= 9776 && x <= 9783 || x >= 9800 && x <= 9811 || x === 9855 || x >= 9866 && x <= 9871 || x === 9875 || x === 9889 || x === 9898 || x === 9899 || x === 9917 || x === 9918 || x === 9924 || x === 9925 || x === 9934 || x === 9940 || x === 9962 || x === 9970 || x === 9971 || x === 9973 || x === 9978 || x === 9981 || x === 9989 || x === 9994 || x === 9995 || x === 10024 || x === 10060 || x === 10062 || x >= 10067 && x <= 10069 || x === 10071 || x >= 10133 && x <= 10135 || x === 10160 || x === 10175 || x === 11035 || x === 11036 || x === 11088 || x === 11093 || x >= 11904 && x <= 11929 || x >= 11931 && x <= 12019 || x >= 12032 && x <= 12245 || x >= 12272 && x <= 12287 || x >= 12289 && x <= 12350 || x >= 12353 && x <= 12438 || x >= 12441 && x <= 12543 || x >= 12549 && x <= 12591 || x >= 12593 && x <= 12686 || x >= 12688 && x <= 12773 || x >= 12783 && x <= 12830 || x >= 12832 && x <= 12871 || x >= 12880 && x <= 42124 || x >= 42128 && x <= 42182 || x >= 43360 && x <= 43388 || x >= 44032 && x <= 55203 || x >= 63744 && x <= 64255 || x >= 65040 && x <= 65049 || x >= 65072 && x <= 65106 || x >= 65108 && x <= 65126 || x >= 65128 && x <= 65131 || x >= 94176 && x <= 94180 || x === 94192 || x === 94193 || x >= 94208 && x <= 100343 || x >= 100352 && x <= 101589 || x >= 101631 && x <= 101640 || x >= 110576 && x <= 110579 || x >= 110581 && x <= 110587 || x === 110589 || x === 110590 || x >= 110592 && x <= 110882 || x === 110898 || x >= 110928 && x <= 110930 || x === 110933 || x >= 110948 && x <= 110951 || x >= 110960 && x <= 111355 || x >= 119552 && x <= 119638 || x >= 119648 && x <= 119670 || x === 126980 || x === 127183 || x === 127374 || x >= 127377 && x <= 127386 || x >= 127488 && x <= 127490 || x >= 127504 && x <= 127547 || x >= 127552 && x <= 127560 || x === 127568 || x === 127569 || x >= 127584 && x <= 127589 || x >= 127744 && x <= 127776 || x >= 127789 && x <= 127797 || x >= 127799 && x <= 127868 || x >= 127870 && x <= 127891 || x >= 127904 && x <= 127946 || x >= 127951 && x <= 127955 || x >= 127968 && x <= 127984 || x === 127988 || x >= 127992 && x <= 128062 || x === 128064 || x >= 128066 && x <= 128252 || x >= 128255 && x <= 128317 || x >= 128331 && x <= 128334 || x >= 128336 && x <= 128359 || x === 128378 || x === 128405 || x === 128406 || x === 128420 || x >= 128507 && x <= 128591 || x >= 128640 && x <= 128709 || x === 128716 || x >= 128720 && x <= 128722 || x >= 128725 && x <= 128727 || x >= 128732 && x <= 128735 || x === 128747 || x === 128748 || x >= 128756 && x <= 128764 || x >= 128992 && x <= 129003 || x === 129008 || x >= 129292 && x <= 129338 || x >= 129340 && x <= 129349 || x >= 129351 && x <= 129535 || x >= 129648 && x <= 129660 || x >= 129664 && x <= 129673 || x >= 129679 && x <= 129734 || x >= 129742 && x <= 129756 || x >= 129759 && x <= 129769 || x >= 129776 && x <= 129784 || x >= 131072 && x <= 196605 || x >= 196608 && x <= 262141;
}

//#endregion
//#region node_modules/.pnpm/get-east-asian-width@1.3.0/node_modules/get-east-asian-width/index.js
function validate(codePoint) {
	if (!Number.isSafeInteger(codePoint)) throw new TypeError(`Expected a code point, got \`${typeof codePoint}\`.`);
}
function eastAsianWidth(codePoint, { ambiguousAsWide = false } = {}) {
	validate(codePoint);
	if (isFullWidth(codePoint) || isWide(codePoint) || ambiguousAsWide && isAmbiguous(codePoint)) return 2;
	return 1;
}

//#endregion
//#region node_modules/.pnpm/emoji-regex@10.4.0/node_modules/emoji-regex/index.js
var require_emoji_regex = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	module.exports = () => {
		return /[#*0-9]\uFE0F?\u20E3|[\xA9\xAE\u203C\u2049\u2122\u2139\u2194-\u2199\u21A9\u21AA\u231A\u231B\u2328\u23CF\u23ED-\u23EF\u23F1\u23F2\u23F8-\u23FA\u24C2\u25AA\u25AB\u25B6\u25C0\u25FB\u25FC\u25FE\u2600-\u2604\u260E\u2611\u2614\u2615\u2618\u2620\u2622\u2623\u2626\u262A\u262E\u262F\u2638-\u263A\u2640\u2642\u2648-\u2653\u265F\u2660\u2663\u2665\u2666\u2668\u267B\u267E\u267F\u2692\u2694-\u2697\u2699\u269B\u269C\u26A0\u26A7\u26AA\u26B0\u26B1\u26BD\u26BE\u26C4\u26C8\u26CF\u26D1\u26E9\u26F0-\u26F5\u26F7\u26F8\u26FA\u2702\u2708\u2709\u270F\u2712\u2714\u2716\u271D\u2721\u2733\u2734\u2744\u2747\u2757\u2763\u27A1\u2934\u2935\u2B05-\u2B07\u2B1B\u2B1C\u2B55\u3030\u303D\u3297\u3299]\uFE0F?|[\u261D\u270C\u270D](?:\uD83C[\uDFFB-\uDFFF]|\uFE0F)?|[\u270A\u270B](?:\uD83C[\uDFFB-\uDFFF])?|[\u23E9-\u23EC\u23F0\u23F3\u25FD\u2693\u26A1\u26AB\u26C5\u26CE\u26D4\u26EA\u26FD\u2705\u2728\u274C\u274E\u2753-\u2755\u2795-\u2797\u27B0\u27BF\u2B50]|\u26D3\uFE0F?(?:\u200D\uD83D\uDCA5)?|\u26F9(?:\uD83C[\uDFFB-\uDFFF]|\uFE0F)?(?:\u200D[\u2640\u2642]\uFE0F?)?|\u2764\uFE0F?(?:\u200D(?:\uD83D\uDD25|\uD83E\uDE79))?|\uD83C(?:[\uDC04\uDD70\uDD71\uDD7E\uDD7F\uDE02\uDE37\uDF21\uDF24-\uDF2C\uDF36\uDF7D\uDF96\uDF97\uDF99-\uDF9B\uDF9E\uDF9F\uDFCD\uDFCE\uDFD4-\uDFDF\uDFF5\uDFF7]\uFE0F?|[\uDF85\uDFC2\uDFC7](?:\uD83C[\uDFFB-\uDFFF])?|[\uDFC4\uDFCA](?:\uD83C[\uDFFB-\uDFFF])?(?:\u200D[\u2640\u2642]\uFE0F?)?|[\uDFCB\uDFCC](?:\uD83C[\uDFFB-\uDFFF]|\uFE0F)?(?:\u200D[\u2640\u2642]\uFE0F?)?|[\uDCCF\uDD8E\uDD91-\uDD9A\uDE01\uDE1A\uDE2F\uDE32-\uDE36\uDE38-\uDE3A\uDE50\uDE51\uDF00-\uDF20\uDF2D-\uDF35\uDF37-\uDF43\uDF45-\uDF4A\uDF4C-\uDF7C\uDF7E-\uDF84\uDF86-\uDF93\uDFA0-\uDFC1\uDFC5\uDFC6\uDFC8\uDFC9\uDFCF-\uDFD3\uDFE0-\uDFF0\uDFF8-\uDFFF]|\uDDE6\uD83C[\uDDE8-\uDDEC\uDDEE\uDDF1\uDDF2\uDDF4\uDDF6-\uDDFA\uDDFC\uDDFD\uDDFF]|\uDDE7\uD83C[\uDDE6\uDDE7\uDDE9-\uDDEF\uDDF1-\uDDF4\uDDF6-\uDDF9\uDDFB\uDDFC\uDDFE\uDDFF]|\uDDE8\uD83C[\uDDE6\uDDE8\uDDE9\uDDEB-\uDDEE\uDDF0-\uDDF7\uDDFA-\uDDFF]|\uDDE9\uD83C[\uDDEA\uDDEC\uDDEF\uDDF0\uDDF2\uDDF4\uDDFF]|\uDDEA\uD83C[\uDDE6\uDDE8\uDDEA\uDDEC\uDDED\uDDF7-\uDDFA]|\uDDEB\uD83C[\uDDEE-\uDDF0\uDDF2\uDDF4\uDDF7]|\uDDEC\uD83C[\uDDE6\uDDE7\uDDE9-\uDDEE\uDDF1-\uDDF3\uDDF5-\uDDFA\uDDFC\uDDFE]|\uDDED\uD83C[\uDDF0\uDDF2\uDDF3\uDDF7\uDDF9\uDDFA]|\uDDEE\uD83C[\uDDE8-\uDDEA\uDDF1-\uDDF4\uDDF6-\uDDF9]|\uDDEF\uD83C[\uDDEA\uDDF2\uDDF4\uDDF5]|\uDDF0\uD83C[\uDDEA\uDDEC-\uDDEE\uDDF2\uDDF3\uDDF5\uDDF7\uDDFC\uDDFE\uDDFF]|\uDDF1\uD83C[\uDDE6-\uDDE8\uDDEE\uDDF0\uDDF7-\uDDFB\uDDFE]|\uDDF2\uD83C[\uDDE6\uDDE8-\uDDED\uDDF0-\uDDFF]|\uDDF3\uD83C[\uDDE6\uDDE8\uDDEA-\uDDEC\uDDEE\uDDF1\uDDF4\uDDF5\uDDF7\uDDFA\uDDFF]|\uDDF4\uD83C\uDDF2|\uDDF5\uD83C[\uDDE6\uDDEA-\uDDED\uDDF0-\uDDF3\uDDF7-\uDDF9\uDDFC\uDDFE]|\uDDF6\uD83C\uDDE6|\uDDF7\uD83C[\uDDEA\uDDF4\uDDF8\uDDFA\uDDFC]|\uDDF8\uD83C[\uDDE6-\uDDEA\uDDEC-\uDDF4\uDDF7-\uDDF9\uDDFB\uDDFD-\uDDFF]|\uDDF9\uD83C[\uDDE6\uDDE8\uDDE9\uDDEB-\uDDED\uDDEF-\uDDF4\uDDF7\uDDF9\uDDFB\uDDFC\uDDFF]|\uDDFA\uD83C[\uDDE6\uDDEC\uDDF2\uDDF3\uDDF8\uDDFE\uDDFF]|\uDDFB\uD83C[\uDDE6\uDDE8\uDDEA\uDDEC\uDDEE\uDDF3\uDDFA]|\uDDFC\uD83C[\uDDEB\uDDF8]|\uDDFD\uD83C\uDDF0|\uDDFE\uD83C[\uDDEA\uDDF9]|\uDDFF\uD83C[\uDDE6\uDDF2\uDDFC]|\uDF44(?:\u200D\uD83D\uDFEB)?|\uDF4B(?:\u200D\uD83D\uDFE9)?|\uDFC3(?:\uD83C[\uDFFB-\uDFFF])?(?:\u200D(?:[\u2640\u2642]\uFE0F?(?:\u200D\u27A1\uFE0F?)?|\u27A1\uFE0F?))?|\uDFF3\uFE0F?(?:\u200D(?:\u26A7\uFE0F?|\uD83C\uDF08))?|\uDFF4(?:\u200D\u2620\uFE0F?|\uDB40\uDC67\uDB40\uDC62\uDB40(?:\uDC65\uDB40\uDC6E\uDB40\uDC67|\uDC73\uDB40\uDC63\uDB40\uDC74|\uDC77\uDB40\uDC6C\uDB40\uDC73)\uDB40\uDC7F)?)|\uD83D(?:[\uDC3F\uDCFD\uDD49\uDD4A\uDD6F\uDD70\uDD73\uDD76-\uDD79\uDD87\uDD8A-\uDD8D\uDDA5\uDDA8\uDDB1\uDDB2\uDDBC\uDDC2-\uDDC4\uDDD1-\uDDD3\uDDDC-\uDDDE\uDDE1\uDDE3\uDDE8\uDDEF\uDDF3\uDDFA\uDECB\uDECD-\uDECF\uDEE0-\uDEE5\uDEE9\uDEF0\uDEF3]\uFE0F?|[\uDC42\uDC43\uDC46-\uDC50\uDC66\uDC67\uDC6B-\uDC6D\uDC72\uDC74-\uDC76\uDC78\uDC7C\uDC83\uDC85\uDC8F\uDC91\uDCAA\uDD7A\uDD95\uDD96\uDE4C\uDE4F\uDEC0\uDECC](?:\uD83C[\uDFFB-\uDFFF])?|[\uDC6E\uDC70\uDC71\uDC73\uDC77\uDC81\uDC82\uDC86\uDC87\uDE45-\uDE47\uDE4B\uDE4D\uDE4E\uDEA3\uDEB4\uDEB5](?:\uD83C[\uDFFB-\uDFFF])?(?:\u200D[\u2640\u2642]\uFE0F?)?|[\uDD74\uDD90](?:\uD83C[\uDFFB-\uDFFF]|\uFE0F)?|[\uDC00-\uDC07\uDC09-\uDC14\uDC16-\uDC25\uDC27-\uDC3A\uDC3C-\uDC3E\uDC40\uDC44\uDC45\uDC51-\uDC65\uDC6A\uDC79-\uDC7B\uDC7D-\uDC80\uDC84\uDC88-\uDC8E\uDC90\uDC92-\uDCA9\uDCAB-\uDCFC\uDCFF-\uDD3D\uDD4B-\uDD4E\uDD50-\uDD67\uDDA4\uDDFB-\uDE2D\uDE2F-\uDE34\uDE37-\uDE41\uDE43\uDE44\uDE48-\uDE4A\uDE80-\uDEA2\uDEA4-\uDEB3\uDEB7-\uDEBF\uDEC1-\uDEC5\uDED0-\uDED2\uDED5-\uDED7\uDEDC-\uDEDF\uDEEB\uDEEC\uDEF4-\uDEFC\uDFE0-\uDFEB\uDFF0]|\uDC08(?:\u200D\u2B1B)?|\uDC15(?:\u200D\uD83E\uDDBA)?|\uDC26(?:\u200D(?:\u2B1B|\uD83D\uDD25))?|\uDC3B(?:\u200D\u2744\uFE0F?)?|\uDC41\uFE0F?(?:\u200D\uD83D\uDDE8\uFE0F?)?|\uDC68(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D\uD83D(?:\uDC8B\u200D\uD83D)?\uDC68|\uD83C[\uDF3E\uDF73\uDF7C\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D(?:[\uDC68\uDC69]\u200D\uD83D(?:\uDC66(?:\u200D\uD83D\uDC66)?|\uDC67(?:\u200D\uD83D[\uDC66\uDC67])?)|[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uDC66(?:\u200D\uD83D\uDC66)?|\uDC67(?:\u200D\uD83D[\uDC66\uDC67])?)|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]))|\uD83C(?:\uDFFB(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D\uD83D(?:\uDC8B\u200D\uD83D)?\uDC68\uD83C[\uDFFB-\uDFFF]|\uD83C[\uDF3E\uDF73\uDF7C\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]|\uDD1D\u200D\uD83D\uDC68\uD83C[\uDFFC-\uDFFF])))?|\uDFFC(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D\uD83D(?:\uDC8B\u200D\uD83D)?\uDC68\uD83C[\uDFFB-\uDFFF]|\uD83C[\uDF3E\uDF73\uDF7C\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]|\uDD1D\u200D\uD83D\uDC68\uD83C[\uDFFB\uDFFD-\uDFFF])))?|\uDFFD(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D\uD83D(?:\uDC8B\u200D\uD83D)?\uDC68\uD83C[\uDFFB-\uDFFF]|\uD83C[\uDF3E\uDF73\uDF7C\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]|\uDD1D\u200D\uD83D\uDC68\uD83C[\uDFFB\uDFFC\uDFFE\uDFFF])))?|\uDFFE(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D\uD83D(?:\uDC8B\u200D\uD83D)?\uDC68\uD83C[\uDFFB-\uDFFF]|\uD83C[\uDF3E\uDF73\uDF7C\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]|\uDD1D\u200D\uD83D\uDC68\uD83C[\uDFFB-\uDFFD\uDFFF])))?|\uDFFF(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D\uD83D(?:\uDC8B\u200D\uD83D)?\uDC68\uD83C[\uDFFB-\uDFFF]|\uD83C[\uDF3E\uDF73\uDF7C\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]|\uDD1D\u200D\uD83D\uDC68\uD83C[\uDFFB-\uDFFE])))?))?|\uDC69(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D\uD83D(?:\uDC8B\u200D\uD83D)?[\uDC68\uDC69]|\uD83C[\uDF3E\uDF73\uDF7C\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D(?:[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uDC66(?:\u200D\uD83D\uDC66)?|\uDC67(?:\u200D\uD83D[\uDC66\uDC67])?|\uDC69\u200D\uD83D(?:\uDC66(?:\u200D\uD83D\uDC66)?|\uDC67(?:\u200D\uD83D[\uDC66\uDC67])?))|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]))|\uD83C(?:\uDFFB(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D\uD83D(?:[\uDC68\uDC69]|\uDC8B\u200D\uD83D[\uDC68\uDC69])\uD83C[\uDFFB-\uDFFF]|\uD83C[\uDF3E\uDF73\uDF7C\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]|\uDD1D\u200D\uD83D[\uDC68\uDC69]\uD83C[\uDFFC-\uDFFF])))?|\uDFFC(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D\uD83D(?:[\uDC68\uDC69]|\uDC8B\u200D\uD83D[\uDC68\uDC69])\uD83C[\uDFFB-\uDFFF]|\uD83C[\uDF3E\uDF73\uDF7C\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]|\uDD1D\u200D\uD83D[\uDC68\uDC69]\uD83C[\uDFFB\uDFFD-\uDFFF])))?|\uDFFD(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D\uD83D(?:[\uDC68\uDC69]|\uDC8B\u200D\uD83D[\uDC68\uDC69])\uD83C[\uDFFB-\uDFFF]|\uD83C[\uDF3E\uDF73\uDF7C\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]|\uDD1D\u200D\uD83D[\uDC68\uDC69]\uD83C[\uDFFB\uDFFC\uDFFE\uDFFF])))?|\uDFFE(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D\uD83D(?:[\uDC68\uDC69]|\uDC8B\u200D\uD83D[\uDC68\uDC69])\uD83C[\uDFFB-\uDFFF]|\uD83C[\uDF3E\uDF73\uDF7C\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]|\uDD1D\u200D\uD83D[\uDC68\uDC69]\uD83C[\uDFFB-\uDFFD\uDFFF])))?|\uDFFF(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D\uD83D(?:[\uDC68\uDC69]|\uDC8B\u200D\uD83D[\uDC68\uDC69])\uD83C[\uDFFB-\uDFFF]|\uD83C[\uDF3E\uDF73\uDF7C\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]|\uDD1D\u200D\uD83D[\uDC68\uDC69]\uD83C[\uDFFB-\uDFFE])))?))?|\uDC6F(?:\u200D[\u2640\u2642]\uFE0F?)?|\uDD75(?:\uD83C[\uDFFB-\uDFFF]|\uFE0F)?(?:\u200D[\u2640\u2642]\uFE0F?)?|\uDE2E(?:\u200D\uD83D\uDCA8)?|\uDE35(?:\u200D\uD83D\uDCAB)?|\uDE36(?:\u200D\uD83C\uDF2B\uFE0F?)?|\uDE42(?:\u200D[\u2194\u2195]\uFE0F?)?|\uDEB6(?:\uD83C[\uDFFB-\uDFFF])?(?:\u200D(?:[\u2640\u2642]\uFE0F?(?:\u200D\u27A1\uFE0F?)?|\u27A1\uFE0F?))?)|\uD83E(?:[\uDD0C\uDD0F\uDD18-\uDD1F\uDD30-\uDD34\uDD36\uDD77\uDDB5\uDDB6\uDDBB\uDDD2\uDDD3\uDDD5\uDEC3-\uDEC5\uDEF0\uDEF2-\uDEF8](?:\uD83C[\uDFFB-\uDFFF])?|[\uDD26\uDD35\uDD37-\uDD39\uDD3D\uDD3E\uDDB8\uDDB9\uDDCD\uDDCF\uDDD4\uDDD6-\uDDDD](?:\uD83C[\uDFFB-\uDFFF])?(?:\u200D[\u2640\u2642]\uFE0F?)?|[\uDDDE\uDDDF](?:\u200D[\u2640\u2642]\uFE0F?)?|[\uDD0D\uDD0E\uDD10-\uDD17\uDD20-\uDD25\uDD27-\uDD2F\uDD3A\uDD3F-\uDD45\uDD47-\uDD76\uDD78-\uDDB4\uDDB7\uDDBA\uDDBC-\uDDCC\uDDD0\uDDE0-\uDDFF\uDE70-\uDE7C\uDE80-\uDE89\uDE8F-\uDEC2\uDEC6\uDECE-\uDEDC\uDEDF-\uDEE9]|\uDD3C(?:\u200D[\u2640\u2642]\uFE0F?|\uD83C[\uDFFB-\uDFFF])?|\uDDCE(?:\uD83C[\uDFFB-\uDFFF])?(?:\u200D(?:[\u2640\u2642]\uFE0F?(?:\u200D\u27A1\uFE0F?)?|\u27A1\uFE0F?))?|\uDDD1(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\uD83C[\uDF3E\uDF73\uDF7C\uDF84\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]|\uDD1D\u200D\uD83E\uDDD1|\uDDD1\u200D\uD83E\uDDD2(?:\u200D\uD83E\uDDD2)?|\uDDD2(?:\u200D\uD83E\uDDD2)?))|\uD83C(?:\uDFFB(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D(?:\uD83D\uDC8B\u200D)?\uD83E\uDDD1\uD83C[\uDFFC-\uDFFF]|\uD83C[\uDF3E\uDF73\uDF7C\uDF84\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]|\uDD1D\u200D\uD83E\uDDD1\uD83C[\uDFFB-\uDFFF])))?|\uDFFC(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D(?:\uD83D\uDC8B\u200D)?\uD83E\uDDD1\uD83C[\uDFFB\uDFFD-\uDFFF]|\uD83C[\uDF3E\uDF73\uDF7C\uDF84\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]|\uDD1D\u200D\uD83E\uDDD1\uD83C[\uDFFB-\uDFFF])))?|\uDFFD(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D(?:\uD83D\uDC8B\u200D)?\uD83E\uDDD1\uD83C[\uDFFB\uDFFC\uDFFE\uDFFF]|\uD83C[\uDF3E\uDF73\uDF7C\uDF84\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]|\uDD1D\u200D\uD83E\uDDD1\uD83C[\uDFFB-\uDFFF])))?|\uDFFE(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D(?:\uD83D\uDC8B\u200D)?\uD83E\uDDD1\uD83C[\uDFFB-\uDFFD\uDFFF]|\uD83C[\uDF3E\uDF73\uDF7C\uDF84\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]|\uDD1D\u200D\uD83E\uDDD1\uD83C[\uDFFB-\uDFFF])))?|\uDFFF(?:\u200D(?:[\u2695\u2696\u2708]\uFE0F?|\u2764\uFE0F?\u200D(?:\uD83D\uDC8B\u200D)?\uD83E\uDDD1\uD83C[\uDFFB-\uDFFE]|\uD83C[\uDF3E\uDF73\uDF7C\uDF84\uDF93\uDFA4\uDFA8\uDFEB\uDFED]|\uD83D[\uDCBB\uDCBC\uDD27\uDD2C\uDE80\uDE92]|\uD83E(?:[\uDDAF\uDDBC\uDDBD](?:\u200D\u27A1\uFE0F?)?|[\uDDB0-\uDDB3]|\uDD1D\u200D\uD83E\uDDD1\uD83C[\uDFFB-\uDFFF])))?))?|\uDEF1(?:\uD83C(?:\uDFFB(?:\u200D\uD83E\uDEF2\uD83C[\uDFFC-\uDFFF])?|\uDFFC(?:\u200D\uD83E\uDEF2\uD83C[\uDFFB\uDFFD-\uDFFF])?|\uDFFD(?:\u200D\uD83E\uDEF2\uD83C[\uDFFB\uDFFC\uDFFE\uDFFF])?|\uDFFE(?:\u200D\uD83E\uDEF2\uD83C[\uDFFB-\uDFFD\uDFFF])?|\uDFFF(?:\u200D\uD83E\uDEF2\uD83C[\uDFFB-\uDFFE])?))?)/g;
	};
}));

//#endregion
//#region node_modules/.pnpm/string-width@7.2.0/node_modules/string-width/index.js
var import_emoji_regex = /* @__PURE__ */ __toESM(require_emoji_regex(), 1);
const segmenter = new Intl.Segmenter();
const defaultIgnorableCodePointRegex = /^\p{Default_Ignorable_Code_Point}$/u;
function stringWidth(string, options = {}) {
	if (typeof string !== "string" || string.length === 0) return 0;
	const { ambiguousIsNarrow = true, countAnsiEscapeCodes = false } = options;
	if (!countAnsiEscapeCodes) string = stripAnsi(string);
	if (string.length === 0) return 0;
	let width = 0;
	const eastAsianWidthOptions = { ambiguousAsWide: !ambiguousIsNarrow };
	for (const { segment: character } of segmenter.segment(string)) {
		const codePoint = character.codePointAt(0);
		if (codePoint <= 31 || codePoint >= 127 && codePoint <= 159) continue;
		if (codePoint >= 8203 && codePoint <= 8207 || codePoint === 65279) continue;
		if (codePoint >= 768 && codePoint <= 879 || codePoint >= 6832 && codePoint <= 6911 || codePoint >= 7616 && codePoint <= 7679 || codePoint >= 8400 && codePoint <= 8447 || codePoint >= 65056 && codePoint <= 65071) continue;
		if (codePoint >= 55296 && codePoint <= 57343) continue;
		if (codePoint >= 65024 && codePoint <= 65039) continue;
		if (defaultIgnorableCodePointRegex.test(character)) continue;
		if ((0, import_emoji_regex.default)().test(character)) {
			width += 2;
			continue;
		}
		width += eastAsianWidth(codePoint, eastAsianWidthOptions);
	}
	return width;
}

//#endregion
//#region node_modules/.pnpm/ansi-styles@6.2.1/node_modules/ansi-styles/index.js
const ANSI_BACKGROUND_OFFSET = 10;
const wrapAnsi16 = (offset = 0) => (code) => `\u001B[${code + offset}m`;
const wrapAnsi256 = (offset = 0) => (code) => `\u001B[${38 + offset};5;${code}m`;
const wrapAnsi16m = (offset = 0) => (red, green, blue) => `\u001B[${38 + offset};2;${red};${green};${blue}m`;
const styles = {
	modifier: {
		reset: [0, 0],
		bold: [1, 22],
		dim: [2, 22],
		italic: [3, 23],
		underline: [4, 24],
		overline: [53, 55],
		inverse: [7, 27],
		hidden: [8, 28],
		strikethrough: [9, 29]
	},
	color: {
		black: [30, 39],
		red: [31, 39],
		green: [32, 39],
		yellow: [33, 39],
		blue: [34, 39],
		magenta: [35, 39],
		cyan: [36, 39],
		white: [37, 39],
		blackBright: [90, 39],
		gray: [90, 39],
		grey: [90, 39],
		redBright: [91, 39],
		greenBright: [92, 39],
		yellowBright: [93, 39],
		blueBright: [94, 39],
		magentaBright: [95, 39],
		cyanBright: [96, 39],
		whiteBright: [97, 39]
	},
	bgColor: {
		bgBlack: [40, 49],
		bgRed: [41, 49],
		bgGreen: [42, 49],
		bgYellow: [43, 49],
		bgBlue: [44, 49],
		bgMagenta: [45, 49],
		bgCyan: [46, 49],
		bgWhite: [47, 49],
		bgBlackBright: [100, 49],
		bgGray: [100, 49],
		bgGrey: [100, 49],
		bgRedBright: [101, 49],
		bgGreenBright: [102, 49],
		bgYellowBright: [103, 49],
		bgBlueBright: [104, 49],
		bgMagentaBright: [105, 49],
		bgCyanBright: [106, 49],
		bgWhiteBright: [107, 49]
	}
};
const modifierNames = Object.keys(styles.modifier);
const foregroundColorNames = Object.keys(styles.color);
const backgroundColorNames = Object.keys(styles.bgColor);
const colorNames = [...foregroundColorNames, ...backgroundColorNames];
function assembleStyles() {
	const codes = /* @__PURE__ */ new Map();
	for (const [groupName, group] of Object.entries(styles)) {
		for (const [styleName, style] of Object.entries(group)) {
			styles[styleName] = {
				open: `\u001B[${style[0]}m`,
				close: `\u001B[${style[1]}m`
			};
			group[styleName] = styles[styleName];
			codes.set(style[0], style[1]);
		}
		Object.defineProperty(styles, groupName, {
			value: group,
			enumerable: false
		});
	}
	Object.defineProperty(styles, "codes", {
		value: codes,
		enumerable: false
	});
	styles.color.close = "\x1B[39m";
	styles.bgColor.close = "\x1B[49m";
	styles.color.ansi = wrapAnsi16();
	styles.color.ansi256 = wrapAnsi256();
	styles.color.ansi16m = wrapAnsi16m();
	styles.bgColor.ansi = wrapAnsi16(ANSI_BACKGROUND_OFFSET);
	styles.bgColor.ansi256 = wrapAnsi256(ANSI_BACKGROUND_OFFSET);
	styles.bgColor.ansi16m = wrapAnsi16m(ANSI_BACKGROUND_OFFSET);
	Object.defineProperties(styles, {
		rgbToAnsi256: {
			value: (red, green, blue) => {
				if (red === green && green === blue) {
					if (red < 8) return 16;
					if (red > 248) return 231;
					return Math.round((red - 8) / 247 * 24) + 232;
				}
				return 16 + 36 * Math.round(red / 255 * 5) + 6 * Math.round(green / 255 * 5) + Math.round(blue / 255 * 5);
			},
			enumerable: false
		},
		hexToRgb: {
			value: (hex) => {
				const matches = /[a-f\d]{6}|[a-f\d]{3}/i.exec(hex.toString(16));
				if (!matches) return [
					0,
					0,
					0
				];
				let [colorString] = matches;
				if (colorString.length === 3) colorString = [...colorString].map((character) => character + character).join("");
				const integer = Number.parseInt(colorString, 16);
				return [
					integer >> 16 & 255,
					integer >> 8 & 255,
					integer & 255
				];
			},
			enumerable: false
		},
		hexToAnsi256: {
			value: (hex) => styles.rgbToAnsi256(...styles.hexToRgb(hex)),
			enumerable: false
		},
		ansi256ToAnsi: {
			value: (code) => {
				if (code < 8) return 30 + code;
				if (code < 16) return 90 + (code - 8);
				let red;
				let green;
				let blue;
				if (code >= 232) {
					red = ((code - 232) * 10 + 8) / 255;
					green = red;
					blue = red;
				} else {
					code -= 16;
					const remainder = code % 36;
					red = Math.floor(code / 36) / 5;
					green = Math.floor(remainder / 6) / 5;
					blue = remainder % 6 / 5;
				}
				const value = Math.max(red, green, blue) * 2;
				if (value === 0) return 30;
				let result = 30 + (Math.round(blue) << 2 | Math.round(green) << 1 | Math.round(red));
				if (value === 2) result += 60;
				return result;
			},
			enumerable: false
		},
		rgbToAnsi: {
			value: (red, green, blue) => styles.ansi256ToAnsi(styles.rgbToAnsi256(red, green, blue)),
			enumerable: false
		},
		hexToAnsi: {
			value: (hex) => styles.ansi256ToAnsi(styles.hexToAnsi256(hex)),
			enumerable: false
		}
	});
	return styles;
}
const ansiStyles = assembleStyles();
var ansi_styles_default = ansiStyles;

//#endregion
//#region node_modules/.pnpm/wrap-ansi@9.0.0/node_modules/wrap-ansi/index.js
const ESCAPES = new Set(["\x1B", ""]);
const END_CODE = 39;
const ANSI_ESCAPE_BELL = "\x07";
const ANSI_CSI = "[";
const ANSI_OSC = "]";
const ANSI_SGR_TERMINATOR = "m";
const ANSI_ESCAPE_LINK = `${ANSI_OSC}8;;`;
const wrapAnsiCode = (code) => `${ESCAPES.values().next().value}${ANSI_CSI}${code}${ANSI_SGR_TERMINATOR}`;
const wrapAnsiHyperlink = (url) => `${ESCAPES.values().next().value}${ANSI_ESCAPE_LINK}${url}${ANSI_ESCAPE_BELL}`;
const wordLengths = (string) => string.split(" ").map((character) => stringWidth(character));
const wrapWord = (rows, word, columns) => {
	const characters = [...word];
	let isInsideEscape = false;
	let isInsideLinkEscape = false;
	let visible = stringWidth(stripAnsi(rows.at(-1)));
	for (const [index, character] of characters.entries()) {
		const characterLength = stringWidth(character);
		if (visible + characterLength <= columns) rows[rows.length - 1] += character;
		else {
			rows.push(character);
			visible = 0;
		}
		if (ESCAPES.has(character)) {
			isInsideEscape = true;
			isInsideLinkEscape = characters.slice(index + 1, index + 1 + ANSI_ESCAPE_LINK.length).join("") === ANSI_ESCAPE_LINK;
		}
		if (isInsideEscape) {
			if (isInsideLinkEscape) {
				if (character === ANSI_ESCAPE_BELL) {
					isInsideEscape = false;
					isInsideLinkEscape = false;
				}
			} else if (character === ANSI_SGR_TERMINATOR) isInsideEscape = false;
			continue;
		}
		visible += characterLength;
		if (visible === columns && index < characters.length - 1) {
			rows.push("");
			visible = 0;
		}
	}
	if (!visible && rows.at(-1).length > 0 && rows.length > 1) rows[rows.length - 2] += rows.pop();
};
const stringVisibleTrimSpacesRight = (string) => {
	const words = string.split(" ");
	let last = words.length;
	while (last > 0) {
		if (stringWidth(words[last - 1]) > 0) break;
		last--;
	}
	if (last === words.length) return string;
	return words.slice(0, last).join(" ") + words.slice(last).join("");
};
const exec = (string, columns, options = {}) => {
	if (options.trim !== false && string.trim() === "") return "";
	let returnValue = "";
	let escapeCode;
	let escapeUrl;
	const lengths = wordLengths(string);
	let rows = [""];
	for (const [index, word] of string.split(" ").entries()) {
		if (options.trim !== false) rows[rows.length - 1] = rows.at(-1).trimStart();
		let rowLength = stringWidth(rows.at(-1));
		if (index !== 0) {
			if (rowLength >= columns && (options.wordWrap === false || options.trim === false)) {
				rows.push("");
				rowLength = 0;
			}
			if (rowLength > 0 || options.trim === false) {
				rows[rows.length - 1] += " ";
				rowLength++;
			}
		}
		if (options.hard && lengths[index] > columns) {
			const remainingColumns = columns - rowLength;
			const breaksStartingThisLine = 1 + Math.floor((lengths[index] - remainingColumns - 1) / columns);
			if (Math.floor((lengths[index] - 1) / columns) < breaksStartingThisLine) rows.push("");
			wrapWord(rows, word, columns);
			continue;
		}
		if (rowLength + lengths[index] > columns && rowLength > 0 && lengths[index] > 0) {
			if (options.wordWrap === false && rowLength < columns) {
				wrapWord(rows, word, columns);
				continue;
			}
			rows.push("");
		}
		if (rowLength + lengths[index] > columns && options.wordWrap === false) {
			wrapWord(rows, word, columns);
			continue;
		}
		rows[rows.length - 1] += word;
	}
	if (options.trim !== false) rows = rows.map((row) => stringVisibleTrimSpacesRight(row));
	const preString = rows.join("\n");
	const pre = [...preString];
	let preStringIndex = 0;
	for (const [index, character] of pre.entries()) {
		returnValue += character;
		if (ESCAPES.has(character)) {
			const { groups } = (/* @__PURE__ */ new RegExp(`(?:\\${ANSI_CSI}(?<code>\\d+)m|\\${ANSI_ESCAPE_LINK}(?<uri>.*)${ANSI_ESCAPE_BELL})`)).exec(preString.slice(preStringIndex)) || { groups: {} };
			if (groups.code !== void 0) {
				const code$1 = Number.parseFloat(groups.code);
				escapeCode = code$1 === END_CODE ? void 0 : code$1;
			} else if (groups.uri !== void 0) escapeUrl = groups.uri.length === 0 ? void 0 : groups.uri;
		}
		const code = ansi_styles_default.codes.get(Number(escapeCode));
		if (pre[index + 1] === "\n") {
			if (escapeUrl) returnValue += wrapAnsiHyperlink("");
			if (escapeCode && code) returnValue += wrapAnsiCode(code);
		} else if (character === "\n") {
			if (escapeCode && code) returnValue += wrapAnsiCode(escapeCode);
			if (escapeUrl) returnValue += wrapAnsiHyperlink(escapeUrl);
		}
		preStringIndex += character.length;
	}
	return returnValue;
};
function wrapAnsi(string, columns, options) {
	return String(string).normalize().replaceAll("\r\n", "\n").split("\n").map((line) => exec(line, columns, options)).join("\n");
}

//#endregion
//#region node_modules/.pnpm/cliui@9.0.1/node_modules/cliui/index.mjs
function ui(opts) {
	return cliui(opts, {
		stringWidth,
		stripAnsi,
		wrap: wrapAnsi
	});
}

//#endregion
//#region node_modules/.pnpm/escalade@3.2.0/node_modules/escalade/sync/index.mjs
function sync_default(start, callback) {
	let dir = resolve(".", start);
	let tmp;
	if (!statSync(dir).isDirectory()) dir = dirname(dir);
	while (true) {
		tmp = callback(dir, readdirSync(dir));
		if (tmp) return resolve(dir, tmp);
		dir = dirname(tmp = dir);
		if (tmp === dir) break;
	}
}

//#endregion
//#region node_modules/.pnpm/yargs-parser@22.0.0/node_modules/yargs-parser/build/lib/string-utils.js
/**
* @license
* Copyright (c) 2016, Contributors
* SPDX-License-Identifier: ISC
*/
function camelCase(str) {
	if (!(str !== str.toLowerCase() && str !== str.toUpperCase())) str = str.toLowerCase();
	if (str.indexOf("-") === -1 && str.indexOf("_") === -1) return str;
	else {
		let camelcase = "";
		let nextChrUpper = false;
		const leadingHyphens = str.match(/^-+/);
		for (let i = leadingHyphens ? leadingHyphens[0].length : 0; i < str.length; i++) {
			let chr = str.charAt(i);
			if (nextChrUpper) {
				nextChrUpper = false;
				chr = chr.toUpperCase();
			}
			if (i !== 0 && (chr === "-" || chr === "_")) nextChrUpper = true;
			else if (chr !== "-" && chr !== "_") camelcase += chr;
		}
		return camelcase;
	}
}
function decamelize(str, joinString) {
	const lowercase = str.toLowerCase();
	joinString = joinString || "-";
	let notCamelcase = "";
	for (let i = 0; i < str.length; i++) {
		const chrLower = lowercase.charAt(i);
		const chrString = str.charAt(i);
		if (chrLower !== chrString && i > 0) notCamelcase += `${joinString}${lowercase.charAt(i)}`;
		else notCamelcase += chrString;
	}
	return notCamelcase;
}
function looksLikeNumber(x) {
	if (x === null || x === void 0) return false;
	if (typeof x === "number") return true;
	if (/^0x[0-9a-f]+$/i.test(x)) return true;
	if (/^0[^.]/.test(x)) return false;
	return /^[-]?(?:\d+(?:\.\d*)?|\.\d+)(e[-+]?\d+)?$/.test(x);
}

//#endregion
//#region node_modules/.pnpm/yargs-parser@22.0.0/node_modules/yargs-parser/build/lib/tokenize-arg-string.js
/**
* @license
* Copyright (c) 2016, Contributors
* SPDX-License-Identifier: ISC
*/
function tokenizeArgString(argString) {
	if (Array.isArray(argString)) return argString.map((e) => typeof e !== "string" ? e + "" : e);
	argString = argString.trim();
	let i = 0;
	let prevC = null;
	let c = null;
	let opening = null;
	const args = [];
	for (let ii = 0; ii < argString.length; ii++) {
		prevC = c;
		c = argString.charAt(ii);
		if (c === " " && !opening) {
			if (!(prevC === " ")) i++;
			continue;
		}
		if (c === opening) opening = null;
		else if ((c === "'" || c === "\"") && !opening) opening = c;
		if (!args[i]) args[i] = "";
		args[i] += c;
	}
	return args;
}

//#endregion
//#region node_modules/.pnpm/yargs-parser@22.0.0/node_modules/yargs-parser/build/lib/yargs-parser-types.js
/**
* @license
* Copyright (c) 2016, Contributors
* SPDX-License-Identifier: ISC
*/
var DefaultValuesForTypeKey;
(function(DefaultValuesForTypeKey$1) {
	DefaultValuesForTypeKey$1["BOOLEAN"] = "boolean";
	DefaultValuesForTypeKey$1["STRING"] = "string";
	DefaultValuesForTypeKey$1["NUMBER"] = "number";
	DefaultValuesForTypeKey$1["ARRAY"] = "array";
})(DefaultValuesForTypeKey || (DefaultValuesForTypeKey = {}));

//#endregion
//#region node_modules/.pnpm/yargs-parser@22.0.0/node_modules/yargs-parser/build/lib/yargs-parser.js
/**
* @license
* Copyright (c) 2016, Contributors
* SPDX-License-Identifier: ISC
*/
let mixin;
var YargsParser = class {
	constructor(_mixin) {
		mixin = _mixin;
	}
	parse(argsInput, options) {
		const opts = Object.assign({
			alias: void 0,
			array: void 0,
			boolean: void 0,
			config: void 0,
			configObjects: void 0,
			configuration: void 0,
			coerce: void 0,
			count: void 0,
			default: void 0,
			envPrefix: void 0,
			narg: void 0,
			normalize: void 0,
			string: void 0,
			number: void 0,
			__: void 0,
			key: void 0
		}, options);
		const args = tokenizeArgString(argsInput);
		const inputIsString = typeof argsInput === "string";
		const aliases = combineAliases(Object.assign(Object.create(null), opts.alias));
		const configuration = Object.assign({
			"boolean-negation": true,
			"camel-case-expansion": true,
			"combine-arrays": false,
			"dot-notation": true,
			"duplicate-arguments-array": true,
			"flatten-duplicate-arrays": true,
			"greedy-arrays": true,
			"halt-at-non-option": false,
			"nargs-eats-options": false,
			"negation-prefix": "no-",
			"parse-numbers": true,
			"parse-positional-numbers": true,
			"populate--": false,
			"set-placeholder-key": false,
			"short-option-groups": true,
			"strip-aliased": false,
			"strip-dashed": false,
			"unknown-options-as-args": false
		}, opts.configuration);
		const defaults = Object.assign(Object.create(null), opts.default);
		const configObjects = opts.configObjects || [];
		const envPrefix = opts.envPrefix;
		const notFlagsOption = configuration["populate--"];
		const notFlagsArgv = notFlagsOption ? "--" : "_";
		const newAliases = Object.create(null);
		const defaulted = Object.create(null);
		const __ = opts.__ || mixin.format;
		const flags = {
			aliases: Object.create(null),
			arrays: Object.create(null),
			bools: Object.create(null),
			strings: Object.create(null),
			numbers: Object.create(null),
			counts: Object.create(null),
			normalize: Object.create(null),
			configs: Object.create(null),
			nargs: Object.create(null),
			coercions: Object.create(null),
			keys: []
		};
		const negative = /^-([0-9]+(\.[0-9]+)?|\.[0-9]+)$/;
		const negatedBoolean = /* @__PURE__ */ new RegExp("^--" + configuration["negation-prefix"] + "(.+)");
		[].concat(opts.array || []).filter(Boolean).forEach(function(opt) {
			const key = typeof opt === "object" ? opt.key : opt;
			const assignment = Object.keys(opt).map(function(key$1) {
				return {
					boolean: "bools",
					string: "strings",
					number: "numbers"
				}[key$1];
			}).filter(Boolean).pop();
			if (assignment) flags[assignment][key] = true;
			flags.arrays[key] = true;
			flags.keys.push(key);
		});
		[].concat(opts.boolean || []).filter(Boolean).forEach(function(key) {
			flags.bools[key] = true;
			flags.keys.push(key);
		});
		[].concat(opts.string || []).filter(Boolean).forEach(function(key) {
			flags.strings[key] = true;
			flags.keys.push(key);
		});
		[].concat(opts.number || []).filter(Boolean).forEach(function(key) {
			flags.numbers[key] = true;
			flags.keys.push(key);
		});
		[].concat(opts.count || []).filter(Boolean).forEach(function(key) {
			flags.counts[key] = true;
			flags.keys.push(key);
		});
		[].concat(opts.normalize || []).filter(Boolean).forEach(function(key) {
			flags.normalize[key] = true;
			flags.keys.push(key);
		});
		if (typeof opts.narg === "object") Object.entries(opts.narg).forEach(([key, value]) => {
			if (typeof value === "number") {
				flags.nargs[key] = value;
				flags.keys.push(key);
			}
		});
		if (typeof opts.coerce === "object") Object.entries(opts.coerce).forEach(([key, value]) => {
			if (typeof value === "function") {
				flags.coercions[key] = value;
				flags.keys.push(key);
			}
		});
		if (typeof opts.config !== "undefined") {
			if (Array.isArray(opts.config) || typeof opts.config === "string") [].concat(opts.config).filter(Boolean).forEach(function(key) {
				flags.configs[key] = true;
			});
			else if (typeof opts.config === "object") Object.entries(opts.config).forEach(([key, value]) => {
				if (typeof value === "boolean" || typeof value === "function") flags.configs[key] = value;
			});
		}
		extendAliases(opts.key, aliases, opts.default, flags.arrays);
		Object.keys(defaults).forEach(function(key) {
			(flags.aliases[key] || []).forEach(function(alias) {
				defaults[alias] = defaults[key];
			});
		});
		let error = null;
		checkConfiguration();
		let notFlags = [];
		const argv$1 = Object.assign(Object.create(null), { _: [] });
		const argvReturn = {};
		for (let i = 0; i < args.length; i++) {
			const arg = args[i];
			const truncatedArg = arg.replace(/^-{3,}/, "---");
			let broken;
			let key;
			let letters;
			let m;
			let next;
			let value;
			if (arg !== "--" && /^-/.test(arg) && isUnknownOptionAsArg(arg)) pushPositional(arg);
			else if (truncatedArg.match(/^---+(=|$)/)) {
				pushPositional(arg);
				continue;
			} else if (arg.match(/^--.+=/) || !configuration["short-option-groups"] && arg.match(/^-.+=/)) {
				m = arg.match(/^--?([^=]+)=([\s\S]*)$/);
				if (m !== null && Array.isArray(m) && m.length >= 3) if (checkAllAliases(m[1], flags.arrays)) i = eatArray(i, m[1], args, m[2]);
				else if (checkAllAliases(m[1], flags.nargs) !== false) i = eatNargs(i, m[1], args, m[2]);
				else setArg(m[1], m[2], true);
			} else if (arg.match(negatedBoolean) && configuration["boolean-negation"]) {
				m = arg.match(negatedBoolean);
				if (m !== null && Array.isArray(m) && m.length >= 2) {
					key = m[1];
					setArg(key, checkAllAliases(key, flags.arrays) ? [false] : false);
				}
			} else if (arg.match(/^--.+/) || !configuration["short-option-groups"] && arg.match(/^-[^-]+/)) {
				m = arg.match(/^--?(.+)/);
				if (m !== null && Array.isArray(m) && m.length >= 2) {
					key = m[1];
					if (checkAllAliases(key, flags.arrays)) i = eatArray(i, key, args);
					else if (checkAllAliases(key, flags.nargs) !== false) i = eatNargs(i, key, args);
					else {
						next = args[i + 1];
						if (next !== void 0 && (!next.match(/^-/) || next.match(negative)) && !checkAllAliases(key, flags.bools) && !checkAllAliases(key, flags.counts)) {
							setArg(key, next);
							i++;
						} else if (/^(true|false)$/.test(next)) {
							setArg(key, next);
							i++;
						} else setArg(key, defaultValue(key));
					}
				}
			} else if (arg.match(/^-.\..+=/)) {
				m = arg.match(/^-([^=]+)=([\s\S]*)$/);
				if (m !== null && Array.isArray(m) && m.length >= 3) setArg(m[1], m[2]);
			} else if (arg.match(/^-.\..+/) && !arg.match(negative)) {
				next = args[i + 1];
				m = arg.match(/^-(.\..+)/);
				if (m !== null && Array.isArray(m) && m.length >= 2) {
					key = m[1];
					if (next !== void 0 && !next.match(/^-/) && !checkAllAliases(key, flags.bools) && !checkAllAliases(key, flags.counts)) {
						setArg(key, next);
						i++;
					} else setArg(key, defaultValue(key));
				}
			} else if (arg.match(/^-[^-]+/) && !arg.match(negative)) {
				letters = arg.slice(1, -1).split("");
				broken = false;
				for (let j = 0; j < letters.length; j++) {
					next = arg.slice(j + 2);
					if (letters[j + 1] && letters[j + 1] === "=") {
						value = arg.slice(j + 3);
						key = letters[j];
						if (checkAllAliases(key, flags.arrays)) i = eatArray(i, key, args, value);
						else if (checkAllAliases(key, flags.nargs) !== false) i = eatNargs(i, key, args, value);
						else setArg(key, value);
						broken = true;
						break;
					}
					if (next === "-") {
						setArg(letters[j], next);
						continue;
					}
					if (/[A-Za-z]/.test(letters[j]) && /^-?\d+(\.\d*)?(e-?\d+)?$/.test(next) && checkAllAliases(next, flags.bools) === false) {
						setArg(letters[j], next);
						broken = true;
						break;
					}
					if (letters[j + 1] && letters[j + 1].match(/\W/)) {
						setArg(letters[j], next);
						broken = true;
						break;
					} else setArg(letters[j], defaultValue(letters[j]));
				}
				key = arg.slice(-1)[0];
				if (!broken && key !== "-") if (checkAllAliases(key, flags.arrays)) i = eatArray(i, key, args);
				else if (checkAllAliases(key, flags.nargs) !== false) i = eatNargs(i, key, args);
				else {
					next = args[i + 1];
					if (next !== void 0 && (!/^(-|--)[^-]/.test(next) || next.match(negative)) && !checkAllAliases(key, flags.bools) && !checkAllAliases(key, flags.counts)) {
						setArg(key, next);
						i++;
					} else if (/^(true|false)$/.test(next)) {
						setArg(key, next);
						i++;
					} else setArg(key, defaultValue(key));
				}
			} else if (arg.match(/^-[0-9]$/) && arg.match(negative) && checkAllAliases(arg.slice(1), flags.bools)) {
				key = arg.slice(1);
				setArg(key, defaultValue(key));
			} else if (arg === "--") {
				notFlags = args.slice(i + 1);
				break;
			} else if (configuration["halt-at-non-option"]) {
				notFlags = args.slice(i);
				break;
			} else pushPositional(arg);
		}
		applyEnvVars(argv$1, true);
		applyEnvVars(argv$1, false);
		setConfig(argv$1);
		setConfigObjects();
		applyDefaultsAndAliases(argv$1, flags.aliases, defaults, true);
		applyCoercions(argv$1);
		if (configuration["set-placeholder-key"]) setPlaceholderKeys(argv$1);
		Object.keys(flags.counts).forEach(function(key) {
			if (!hasKey(argv$1, key.split("."))) setArg(key, 0);
		});
		if (notFlagsOption && notFlags.length) argv$1[notFlagsArgv] = [];
		notFlags.forEach(function(key) {
			argv$1[notFlagsArgv].push(key);
		});
		if (configuration["camel-case-expansion"] && configuration["strip-dashed"]) Object.keys(argv$1).filter((key) => key !== "--" && key.includes("-")).forEach((key) => {
			delete argv$1[key];
		});
		if (configuration["strip-aliased"]) [].concat(...Object.keys(aliases).map((k) => aliases[k])).forEach((alias) => {
			if (configuration["camel-case-expansion"] && alias.includes("-")) delete argv$1[alias.split(".").map((prop) => camelCase(prop)).join(".")];
			delete argv$1[alias];
		});
		function pushPositional(arg) {
			const maybeCoercedNumber = maybeCoerceNumber("_", arg);
			if (typeof maybeCoercedNumber === "string" || typeof maybeCoercedNumber === "number") argv$1._.push(maybeCoercedNumber);
		}
		function eatNargs(i, key, args$1, argAfterEqualSign) {
			let ii;
			let toEat = checkAllAliases(key, flags.nargs);
			toEat = typeof toEat !== "number" || isNaN(toEat) ? 1 : toEat;
			if (toEat === 0) {
				if (!isUndefined(argAfterEqualSign)) error = Error(__("Argument unexpected for: %s", key));
				setArg(key, defaultValue(key));
				return i;
			}
			let available = isUndefined(argAfterEqualSign) ? 0 : 1;
			if (configuration["nargs-eats-options"]) {
				if (args$1.length - (i + 1) + available < toEat) error = Error(__("Not enough arguments following: %s", key));
				available = toEat;
			} else {
				for (ii = i + 1; ii < args$1.length; ii++) if (!args$1[ii].match(/^-[^0-9]/) || args$1[ii].match(negative) || isUnknownOptionAsArg(args$1[ii])) available++;
				else break;
				if (available < toEat) error = Error(__("Not enough arguments following: %s", key));
			}
			let consumed = Math.min(available, toEat);
			if (!isUndefined(argAfterEqualSign) && consumed > 0) {
				setArg(key, argAfterEqualSign);
				consumed--;
			}
			for (ii = i + 1; ii < consumed + i + 1; ii++) setArg(key, args$1[ii]);
			return i + consumed;
		}
		function eatArray(i, key, args$1, argAfterEqualSign) {
			let argsToSet = [];
			let next = argAfterEqualSign || args$1[i + 1];
			const nargsCount = checkAllAliases(key, flags.nargs);
			if (checkAllAliases(key, flags.bools) && !/^(true|false)$/.test(next)) argsToSet.push(true);
			else if (isUndefined(next) || isUndefined(argAfterEqualSign) && /^-/.test(next) && !negative.test(next) && !isUnknownOptionAsArg(next)) {
				if (defaults[key] !== void 0) {
					const defVal = defaults[key];
					argsToSet = Array.isArray(defVal) ? defVal : [defVal];
				}
			} else {
				if (!isUndefined(argAfterEqualSign)) argsToSet.push(processValue(key, argAfterEqualSign, true));
				for (let ii = i + 1; ii < args$1.length; ii++) {
					if (!configuration["greedy-arrays"] && argsToSet.length > 0 || nargsCount && typeof nargsCount === "number" && argsToSet.length >= nargsCount) break;
					next = args$1[ii];
					if (/^-/.test(next) && !negative.test(next) && !isUnknownOptionAsArg(next)) break;
					i = ii;
					argsToSet.push(processValue(key, next, inputIsString));
				}
			}
			if (typeof nargsCount === "number" && (nargsCount && argsToSet.length < nargsCount || isNaN(nargsCount) && argsToSet.length === 0)) error = Error(__("Not enough arguments following: %s", key));
			setArg(key, argsToSet);
			return i;
		}
		function setArg(key, val, shouldStripQuotes = inputIsString) {
			if (/-/.test(key) && configuration["camel-case-expansion"]) addNewAlias(key, key.split(".").map(function(prop) {
				return camelCase(prop);
			}).join("."));
			const value = processValue(key, val, shouldStripQuotes);
			const splitKey = key.split(".");
			setKey(argv$1, splitKey, value);
			if (flags.aliases[key]) flags.aliases[key].forEach(function(x) {
				setKey(argv$1, x.split("."), value);
			});
			if (splitKey.length > 1 && configuration["dot-notation"]) (flags.aliases[splitKey[0]] || []).forEach(function(x) {
				let keyProperties = x.split(".");
				const a = [].concat(splitKey);
				a.shift();
				keyProperties = keyProperties.concat(a);
				if (!(flags.aliases[key] || []).includes(keyProperties.join("."))) setKey(argv$1, keyProperties, value);
			});
			if (checkAllAliases(key, flags.normalize) && !checkAllAliases(key, flags.arrays)) [key].concat(flags.aliases[key] || []).forEach(function(key$1) {
				Object.defineProperty(argvReturn, key$1, {
					enumerable: true,
					get() {
						return val;
					},
					set(value$1) {
						val = typeof value$1 === "string" ? mixin.normalize(value$1) : value$1;
					}
				});
			});
		}
		function addNewAlias(key, alias) {
			if (!(flags.aliases[key] && flags.aliases[key].length)) {
				flags.aliases[key] = [alias];
				newAliases[alias] = true;
			}
			if (!(flags.aliases[alias] && flags.aliases[alias].length)) addNewAlias(alias, key);
		}
		function processValue(key, val, shouldStripQuotes) {
			if (shouldStripQuotes) val = stripQuotes(val);
			if (checkAllAliases(key, flags.bools) || checkAllAliases(key, flags.counts)) {
				if (typeof val === "string") val = val === "true";
			}
			let value = Array.isArray(val) ? val.map(function(v) {
				return maybeCoerceNumber(key, v);
			}) : maybeCoerceNumber(key, val);
			if (checkAllAliases(key, flags.counts) && (isUndefined(value) || typeof value === "boolean")) value = increment();
			if (checkAllAliases(key, flags.normalize) && checkAllAliases(key, flags.arrays)) if (Array.isArray(val)) value = val.map((val$1) => {
				return mixin.normalize(val$1);
			});
			else value = mixin.normalize(val);
			return value;
		}
		function maybeCoerceNumber(key, value) {
			if (!configuration["parse-positional-numbers"] && key === "_") return value;
			if (!checkAllAliases(key, flags.strings) && !checkAllAliases(key, flags.bools) && !Array.isArray(value)) {
				if (looksLikeNumber(value) && configuration["parse-numbers"] && Number.isSafeInteger(Math.floor(parseFloat(`${value}`))) || !isUndefined(value) && checkAllAliases(key, flags.numbers)) value = Number(value);
			}
			return value;
		}
		function setConfig(argv$2) {
			const configLookup = Object.create(null);
			applyDefaultsAndAliases(configLookup, flags.aliases, defaults);
			Object.keys(flags.configs).forEach(function(configKey) {
				const configPath = argv$2[configKey] || configLookup[configKey];
				if (configPath) try {
					let config = null;
					const resolvedConfigPath = mixin.resolve(mixin.cwd(), configPath);
					const resolveConfig = flags.configs[configKey];
					if (typeof resolveConfig === "function") {
						try {
							config = resolveConfig(resolvedConfigPath);
						} catch (e) {
							config = e;
						}
						if (config instanceof Error) {
							error = config;
							return;
						}
					} else config = mixin.require(resolvedConfigPath);
					setConfigObject(config);
				} catch (ex) {
					if (ex.name === "PermissionDenied") error = ex;
					else if (argv$2[configKey]) error = Error(__("Invalid JSON config file: %s", configPath));
				}
			});
		}
		function setConfigObject(config, prev) {
			Object.keys(config).forEach(function(key) {
				const value = config[key];
				const fullKey = prev ? prev + "." + key : key;
				if (typeof value === "object" && value !== null && !Array.isArray(value) && configuration["dot-notation"]) setConfigObject(value, fullKey);
				else if (!hasKey(argv$1, fullKey.split(".")) || checkAllAliases(fullKey, flags.arrays) && configuration["combine-arrays"]) setArg(fullKey, value);
			});
		}
		function setConfigObjects() {
			if (typeof configObjects !== "undefined") configObjects.forEach(function(configObject) {
				setConfigObject(configObject);
			});
		}
		function applyEnvVars(argv$2, configOnly) {
			if (typeof envPrefix === "undefined") return;
			const prefix = typeof envPrefix === "string" ? envPrefix : "";
			const env$1 = mixin.env();
			Object.keys(env$1).forEach(function(envVar) {
				if (prefix === "" || envVar.lastIndexOf(prefix, 0) === 0) {
					const keys = envVar.split("__").map(function(key, i) {
						if (i === 0) key = key.substring(prefix.length);
						return camelCase(key);
					});
					if ((configOnly && flags.configs[keys.join(".")] || !configOnly) && !hasKey(argv$2, keys)) setArg(keys.join("."), env$1[envVar]);
				}
			});
		}
		function applyCoercions(argv$2) {
			let coerce;
			const applied = /* @__PURE__ */ new Set();
			Object.keys(argv$2).forEach(function(key) {
				if (!applied.has(key)) {
					coerce = checkAllAliases(key, flags.coercions);
					if (typeof coerce === "function") try {
						const value = maybeCoerceNumber(key, coerce(argv$2[key]));
						[].concat(flags.aliases[key] || [], key).forEach((ali) => {
							applied.add(ali);
							argv$2[ali] = value;
						});
					} catch (err) {
						error = err;
					}
				}
			});
		}
		function setPlaceholderKeys(argv$2) {
			flags.keys.forEach((key) => {
				if (~key.indexOf(".")) return;
				if (typeof argv$2[key] === "undefined") argv$2[key] = void 0;
			});
			return argv$2;
		}
		function applyDefaultsAndAliases(obj, aliases$1, defaults$1, canLog = false) {
			Object.keys(defaults$1).forEach(function(key) {
				if (!hasKey(obj, key.split("."))) {
					setKey(obj, key.split("."), defaults$1[key]);
					if (canLog) defaulted[key] = true;
					(aliases$1[key] || []).forEach(function(x) {
						if (hasKey(obj, x.split("."))) return;
						setKey(obj, x.split("."), defaults$1[key]);
					});
				}
			});
		}
		function hasKey(obj, keys) {
			let o = obj;
			if (!configuration["dot-notation"]) keys = [keys.join(".")];
			keys.slice(0, -1).forEach(function(key$1) {
				o = o[key$1] || {};
			});
			const key = keys[keys.length - 1];
			if (typeof o !== "object") return false;
			else return key in o;
		}
		function setKey(obj, keys, value) {
			let o = obj;
			if (!configuration["dot-notation"]) keys = [keys.join(".")];
			keys.slice(0, -1).forEach(function(key$1) {
				key$1 = sanitizeKey(key$1);
				if (typeof o === "object" && o[key$1] === void 0) o[key$1] = {};
				if (typeof o[key$1] !== "object" || Array.isArray(o[key$1])) {
					if (Array.isArray(o[key$1])) o[key$1].push({});
					else o[key$1] = [o[key$1], {}];
					o = o[key$1][o[key$1].length - 1];
				} else o = o[key$1];
			});
			const key = sanitizeKey(keys[keys.length - 1]);
			const isTypeArray = checkAllAliases(keys.join("."), flags.arrays);
			const isValueArray = Array.isArray(value);
			let duplicate = configuration["duplicate-arguments-array"];
			if (!duplicate && checkAllAliases(key, flags.nargs)) {
				duplicate = true;
				if (!isUndefined(o[key]) && flags.nargs[key] === 1 || Array.isArray(o[key]) && o[key].length === flags.nargs[key]) o[key] = void 0;
			}
			if (value === increment()) o[key] = increment(o[key]);
			else if (Array.isArray(o[key])) if (duplicate && isTypeArray && isValueArray) o[key] = configuration["flatten-duplicate-arrays"] ? o[key].concat(value) : (Array.isArray(o[key][0]) ? o[key] : [o[key]]).concat([value]);
			else if (!duplicate && Boolean(isTypeArray) === Boolean(isValueArray)) o[key] = value;
			else o[key] = o[key].concat([value]);
			else if (o[key] === void 0 && isTypeArray) o[key] = isValueArray ? value : [value];
			else if (duplicate && !(o[key] === void 0 || checkAllAliases(key, flags.counts) || checkAllAliases(key, flags.bools))) o[key] = [o[key], value];
			else o[key] = value;
		}
		function extendAliases(...args$1) {
			args$1.forEach(function(obj) {
				Object.keys(obj || {}).forEach(function(key) {
					if (flags.aliases[key]) return;
					flags.aliases[key] = [].concat(aliases[key] || []);
					flags.aliases[key].concat(key).forEach(function(x) {
						if (/-/.test(x) && configuration["camel-case-expansion"]) {
							const c = camelCase(x);
							if (c !== key && flags.aliases[key].indexOf(c) === -1) {
								flags.aliases[key].push(c);
								newAliases[c] = true;
							}
						}
					});
					flags.aliases[key].concat(key).forEach(function(x) {
						if (x.length > 1 && /[A-Z]/.test(x) && configuration["camel-case-expansion"]) {
							const c = decamelize(x, "-");
							if (c !== key && flags.aliases[key].indexOf(c) === -1) {
								flags.aliases[key].push(c);
								newAliases[c] = true;
							}
						}
					});
					flags.aliases[key].forEach(function(x) {
						flags.aliases[x] = [key].concat(flags.aliases[key].filter(function(y) {
							return x !== y;
						}));
					});
				});
			});
		}
		function checkAllAliases(key, flag) {
			const toCheck = [].concat(flags.aliases[key] || [], key);
			const keys = Object.keys(flag);
			const setAlias = toCheck.find((key$1) => keys.includes(key$1));
			return setAlias ? flag[setAlias] : false;
		}
		function hasAnyFlag(key) {
			const flagsKeys = Object.keys(flags);
			return [].concat(flagsKeys.map((k) => flags[k])).some(function(flag) {
				return Array.isArray(flag) ? flag.includes(key) : flag[key];
			});
		}
		function hasFlagsMatching(arg, ...patterns) {
			return [].concat(...patterns).some(function(pattern) {
				const match = arg.match(pattern);
				return match && hasAnyFlag(match[1]);
			});
		}
		function hasAllShortFlags(arg) {
			if (arg.match(negative) || !arg.match(/^-[^-]+/)) return false;
			let hasAllFlags = true;
			let next;
			const letters = arg.slice(1).split("");
			for (let j = 0; j < letters.length; j++) {
				next = arg.slice(j + 2);
				if (!hasAnyFlag(letters[j])) {
					hasAllFlags = false;
					break;
				}
				if (letters[j + 1] && letters[j + 1] === "=" || next === "-" || /[A-Za-z]/.test(letters[j]) && /^-?\d+(\.\d*)?(e-?\d+)?$/.test(next) || letters[j + 1] && letters[j + 1].match(/\W/)) break;
			}
			return hasAllFlags;
		}
		function isUnknownOptionAsArg(arg) {
			return configuration["unknown-options-as-args"] && isUnknownOption(arg);
		}
		function isUnknownOption(arg) {
			arg = arg.replace(/^-{3,}/, "--");
			if (arg.match(negative)) return false;
			if (hasAllShortFlags(arg)) return false;
			return !hasFlagsMatching(arg, /^-+([^=]+?)=[\s\S]*$/, negatedBoolean, /^-+([^=]+?)$/, /^-+([^=]+?)-$/, /^-+([^=]+?\d+)$/, /^-+([^=]+?)\W+.*$/);
		}
		function defaultValue(key) {
			if (!checkAllAliases(key, flags.bools) && !checkAllAliases(key, flags.counts) && `${key}` in defaults) return defaults[key];
			else return defaultForType(guessType$1(key));
		}
		function defaultForType(type) {
			return {
				[DefaultValuesForTypeKey.BOOLEAN]: true,
				[DefaultValuesForTypeKey.STRING]: "",
				[DefaultValuesForTypeKey.NUMBER]: void 0,
				[DefaultValuesForTypeKey.ARRAY]: []
			}[type];
		}
		function guessType$1(key) {
			let type = DefaultValuesForTypeKey.BOOLEAN;
			if (checkAllAliases(key, flags.strings)) type = DefaultValuesForTypeKey.STRING;
			else if (checkAllAliases(key, flags.numbers)) type = DefaultValuesForTypeKey.NUMBER;
			else if (checkAllAliases(key, flags.bools)) type = DefaultValuesForTypeKey.BOOLEAN;
			else if (checkAllAliases(key, flags.arrays)) type = DefaultValuesForTypeKey.ARRAY;
			return type;
		}
		function isUndefined(num) {
			return num === void 0;
		}
		function checkConfiguration() {
			Object.keys(flags.counts).find((key) => {
				if (checkAllAliases(key, flags.arrays)) {
					error = Error(__("Invalid configuration: %s, opts.count excludes opts.array.", key));
					return true;
				} else if (checkAllAliases(key, flags.nargs)) {
					error = Error(__("Invalid configuration: %s, opts.count excludes opts.narg.", key));
					return true;
				}
				return false;
			});
		}
		return {
			aliases: Object.assign({}, flags.aliases),
			argv: Object.assign(argvReturn, argv$1),
			configuration,
			defaulted: Object.assign({}, defaulted),
			error,
			newAliases: Object.assign({}, newAliases)
		};
	}
};
function combineAliases(aliases) {
	const aliasArrays = [];
	const combined = Object.create(null);
	let change = true;
	Object.keys(aliases).forEach(function(key) {
		aliasArrays.push([].concat(aliases[key], key));
	});
	while (change) {
		change = false;
		for (let i = 0; i < aliasArrays.length; i++) for (let ii = i + 1; ii < aliasArrays.length; ii++) if (aliasArrays[i].filter(function(v) {
			return aliasArrays[ii].indexOf(v) !== -1;
		}).length) {
			aliasArrays[i] = aliasArrays[i].concat(aliasArrays[ii]);
			aliasArrays.splice(ii, 1);
			change = true;
			break;
		}
	}
	aliasArrays.forEach(function(aliasArray) {
		aliasArray = aliasArray.filter(function(v, i, self) {
			return self.indexOf(v) === i;
		});
		const lastAlias = aliasArray.pop();
		if (lastAlias !== void 0 && typeof lastAlias === "string") combined[lastAlias] = aliasArray;
	});
	return combined;
}
function increment(orig) {
	return orig !== void 0 ? orig + 1 : 1;
}
function sanitizeKey(key) {
	if (key === "__proto__") return "___proto___";
	return key;
}
function stripQuotes(val) {
	return typeof val === "string" && (val[0] === "'" || val[0] === "\"") && val[val.length - 1] === val[0] ? val.substring(1, val.length - 1) : val;
}

//#endregion
//#region node_modules/.pnpm/yargs-parser@22.0.0/node_modules/yargs-parser/build/lib/index.js
/**
* @fileoverview Main entrypoint for libraries using yargs-parser in Node.js
*
* @license
* Copyright (c) 2016, Contributors
* SPDX-License-Identifier: ISC
*/
var _a, _b, _c;
const minNodeVersion = process && process.env && process.env.YARGS_MIN_NODE_VERSION ? Number(process.env.YARGS_MIN_NODE_VERSION) : 20;
const nodeVersion = (_b = (_a = process === null || process === void 0 ? void 0 : process.versions) === null || _a === void 0 ? void 0 : _a.node) !== null && _b !== void 0 ? _b : (_c = process === null || process === void 0 ? void 0 : process.version) === null || _c === void 0 ? void 0 : _c.slice(1);
if (nodeVersion) {
	if (Number(nodeVersion.match(/^([^.]+)/)[1]) < minNodeVersion) throw Error(`yargs parser supports a minimum Node.js version of ${minNodeVersion}. Read our version support policy: https://github.com/yargs/yargs-parser#supported-nodejs-versions`);
}
const env = process ? process.env : {};
const require$1 = createRequire ? createRequire(import.meta.url) : void 0;
const parser = new YargsParser({
	cwd: process.cwd,
	env: () => {
		return env;
	},
	format,
	normalize,
	resolve,
	require: (path) => {
		if (typeof require$1 !== "undefined") return require$1(path);
		else if (path.match(/\.json$/)) return JSON.parse(readFileSync(path, "utf8"));
		else throw Error("only .json config files are supported in ESM");
	}
});
const yargsParser = function Parser(args, opts) {
	return parser.parse(args.slice(), opts).argv;
};
yargsParser.detailed = function(args, opts) {
	return parser.parse(args.slice(), opts);
};
yargsParser.camelCase = camelCase;
yargsParser.decamelize = decamelize;
yargsParser.looksLikeNumber = looksLikeNumber;
var lib_default = yargsParser;

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/utils/process-argv.js
function getProcessArgvBinIndex() {
	if (isBundledElectronApp()) return 0;
	return 1;
}
function isBundledElectronApp() {
	return isElectronApp() && !process.defaultApp;
}
function isElectronApp() {
	return !!process.versions.electron;
}
function hideBin(argv$1) {
	return argv$1.slice(getProcessArgvBinIndex() + 1);
}
function getProcessArgvBin() {
	return process.argv[getProcessArgvBinIndex()];
}

//#endregion
//#region node_modules/.pnpm/y18n@5.0.8/node_modules/y18n/build/lib/platform-shims/node.js
var node_default = {
	fs: {
		readFileSync,
		writeFile
	},
	format,
	resolve,
	exists: (file) => {
		try {
			return statSync(file).isFile();
		} catch (err) {
			return false;
		}
	}
};

//#endregion
//#region node_modules/.pnpm/y18n@5.0.8/node_modules/y18n/build/lib/index.js
let shim$1;
var Y18N = class {
	constructor(opts) {
		opts = opts || {};
		this.directory = opts.directory || "./locales";
		this.updateFiles = typeof opts.updateFiles === "boolean" ? opts.updateFiles : true;
		this.locale = opts.locale || "en";
		this.fallbackToLanguage = typeof opts.fallbackToLanguage === "boolean" ? opts.fallbackToLanguage : true;
		this.cache = Object.create(null);
		this.writeQueue = [];
	}
	__(...args) {
		if (typeof arguments[0] !== "string") return this._taggedLiteral(arguments[0], ...arguments);
		const str = args.shift();
		let cb = function() {};
		if (typeof args[args.length - 1] === "function") cb = args.pop();
		cb = cb || function() {};
		if (!this.cache[this.locale]) this._readLocaleFile();
		if (!this.cache[this.locale][str] && this.updateFiles) {
			this.cache[this.locale][str] = str;
			this._enqueueWrite({
				directory: this.directory,
				locale: this.locale,
				cb
			});
		} else cb();
		return shim$1.format.apply(shim$1.format, [this.cache[this.locale][str] || str].concat(args));
	}
	__n() {
		const args = Array.prototype.slice.call(arguments);
		const singular = args.shift();
		const plural = args.shift();
		const quantity = args.shift();
		let cb = function() {};
		if (typeof args[args.length - 1] === "function") cb = args.pop();
		if (!this.cache[this.locale]) this._readLocaleFile();
		let str = quantity === 1 ? singular : plural;
		if (this.cache[this.locale][singular]) str = this.cache[this.locale][singular][quantity === 1 ? "one" : "other"];
		if (!this.cache[this.locale][singular] && this.updateFiles) {
			this.cache[this.locale][singular] = {
				one: singular,
				other: plural
			};
			this._enqueueWrite({
				directory: this.directory,
				locale: this.locale,
				cb
			});
		} else cb();
		const values = [str];
		if (~str.indexOf("%d")) values.push(quantity);
		return shim$1.format.apply(shim$1.format, values.concat(args));
	}
	setLocale(locale) {
		this.locale = locale;
	}
	getLocale() {
		return this.locale;
	}
	updateLocale(obj) {
		if (!this.cache[this.locale]) this._readLocaleFile();
		for (const key in obj) if (Object.prototype.hasOwnProperty.call(obj, key)) this.cache[this.locale][key] = obj[key];
	}
	_taggedLiteral(parts, ...args) {
		let str = "";
		parts.forEach(function(part, i) {
			const arg = args[i + 1];
			str += part;
			if (typeof arg !== "undefined") str += "%s";
		});
		return this.__.apply(this, [str].concat([].slice.call(args, 1)));
	}
	_enqueueWrite(work) {
		this.writeQueue.push(work);
		if (this.writeQueue.length === 1) this._processWriteQueue();
	}
	_processWriteQueue() {
		const _this = this;
		const work = this.writeQueue[0];
		const directory = work.directory;
		const locale = work.locale;
		const cb = work.cb;
		const languageFile = this._resolveLocaleFile(directory, locale);
		const serializedLocale = JSON.stringify(this.cache[locale], null, 2);
		shim$1.fs.writeFile(languageFile, serializedLocale, "utf-8", function(err) {
			_this.writeQueue.shift();
			if (_this.writeQueue.length > 0) _this._processWriteQueue();
			cb(err);
		});
	}
	_readLocaleFile() {
		let localeLookup = {};
		const languageFile = this._resolveLocaleFile(this.directory, this.locale);
		try {
			if (shim$1.fs.readFileSync) localeLookup = JSON.parse(shim$1.fs.readFileSync(languageFile, "utf-8"));
		} catch (err) {
			if (err instanceof SyntaxError) err.message = "syntax error in " + languageFile;
			if (err.code === "ENOENT") localeLookup = {};
			else throw err;
		}
		this.cache[this.locale] = localeLookup;
	}
	_resolveLocaleFile(directory, locale) {
		let file = shim$1.resolve(directory, "./", locale + ".json");
		if (this.fallbackToLanguage && !this._fileExistsSync(file) && ~locale.lastIndexOf("_")) {
			const languageFile = shim$1.resolve(directory, "./", locale.split("_")[0] + ".json");
			if (this._fileExistsSync(languageFile)) file = languageFile;
		}
		return file;
	}
	_fileExistsSync(file) {
		return shim$1.exists(file);
	}
};
function y18n(opts, _shim) {
	shim$1 = _shim;
	const y18n$2 = new Y18N(opts);
	return {
		__: y18n$2.__.bind(y18n$2),
		__n: y18n$2.__n.bind(y18n$2),
		setLocale: y18n$2.setLocale.bind(y18n$2),
		getLocale: y18n$2.getLocale.bind(y18n$2),
		updateLocale: y18n$2.updateLocale.bind(y18n$2),
		locale: y18n$2.locale
	};
}

//#endregion
//#region node_modules/.pnpm/y18n@5.0.8/node_modules/y18n/index.mjs
const y18n$1 = (opts) => {
	return y18n(opts, node_default);
};
var y18n_default = y18n$1;

//#endregion
//#region node_modules/.pnpm/get-caller-file@2.0.5/node_modules/get-caller-file/index.js
var require_get_caller_file = /* @__PURE__ */ __commonJSMin(((exports, module) => {
	module.exports = function getCallerFile$1(position) {
		if (position === void 0) position = 2;
		if (position >= Error.stackTraceLimit) throw new TypeError("getCallerFile(position) requires position be less then Error.stackTraceLimit but position was: `" + position + "` and Error.stackTraceLimit was: `" + Error.stackTraceLimit + "`");
		var oldPrepareStackTrace = Error.prepareStackTrace;
		Error.prepareStackTrace = function(_, stack$1) {
			return stack$1;
		};
		var stack = (/* @__PURE__ */ new Error()).stack;
		Error.prepareStackTrace = oldPrepareStackTrace;
		if (stack !== null && typeof stack === "object") return stack[position] ? stack[position].getFileName() : void 0;
	};
}));

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/lib/platform-shims/esm.mjs
var import_get_caller_file = /* @__PURE__ */ __toESM(require_get_caller_file(), 1);
const __dirname = fileURLToPath(import.meta.url);
const mainFilename = __dirname.substring(0, __dirname.lastIndexOf("node_modules"));
const require = createRequire(import.meta.url);
var esm_default = {
	assert: {
		notStrictEqual,
		strictEqual
	},
	cliui: ui,
	findUp: sync_default,
	getEnv: (key) => {
		return process.env[key];
	},
	inspect,
	getProcessArgvBin,
	mainFilename: mainFilename || process.cwd(),
	Parser: lib_default,
	path: {
		basename,
		dirname,
		extname,
		relative,
		resolve,
		join
	},
	process: {
		argv: () => process.argv,
		cwd: process.cwd,
		emitWarning: (warning, type) => process.emitWarning(warning, type),
		execPath: () => process.execPath,
		exit: (code) => {
			process.exit(code);
		},
		nextTick: process.nextTick,
		stdColumns: typeof process.stdout.columns !== "undefined" ? process.stdout.columns : null
	},
	readFileSync: readFileSync$1,
	readdirSync: readdirSync$1,
	require,
	getCallerFile: () => {
		const callerFile = (0, import_get_caller_file.default)(3);
		return callerFile.match(/^file:\/\//) ? fileURLToPath(callerFile) : callerFile;
	},
	stringWidth,
	y18n: y18n_default({
		directory: resolve(__dirname, "../../../locales"),
		updateFiles: false
	})
};

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/typings/common-types.js
function assertNotStrictEqual(actual, expected, shim$2, message) {
	shim$2.assert.notStrictEqual(actual, expected, message);
}
function assertSingleKey(actual, shim$2) {
	shim$2.assert.strictEqual(typeof actual, "string");
}
function objectKeys(object) {
	return Object.keys(object);
}

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/utils/is-promise.js
function isPromise(maybePromise) {
	return !!maybePromise && !!maybePromise.then && typeof maybePromise.then === "function";
}

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/yerror.js
var YError = class YError extends Error {
	constructor(msg) {
		super(msg || "yargs error");
		this.name = "YError";
		if (Error.captureStackTrace) Error.captureStackTrace(this, YError);
	}
};

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/parse-command.js
function parseCommand(cmd) {
	const splitCommand = cmd.replace(/\s{2,}/g, " ").split(/\s+(?![^[]*]|[^<]*>)/);
	const bregex = /\.*[\][<>]/g;
	const firstCommand = splitCommand.shift();
	if (!firstCommand) throw new Error(`No command found in: ${cmd}`);
	const parsedCommand = {
		cmd: firstCommand.replace(bregex, ""),
		demanded: [],
		optional: []
	};
	splitCommand.forEach((cmd$1, i) => {
		let variadic = false;
		cmd$1 = cmd$1.replace(/\s/g, "");
		if (/\.+[\]>]/.test(cmd$1) && i === splitCommand.length - 1) variadic = true;
		if (/^\[/.test(cmd$1)) parsedCommand.optional.push({
			cmd: cmd$1.replace(bregex, "").split("|"),
			variadic
		});
		else parsedCommand.demanded.push({
			cmd: cmd$1.replace(bregex, "").split("|"),
			variadic
		});
	});
	return parsedCommand;
}

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/argsert.js
const positionName = [
	"first",
	"second",
	"third",
	"fourth",
	"fifth",
	"sixth"
];
function argsert(arg1, arg2, arg3) {
	function parseArgs() {
		return typeof arg1 === "object" ? [
			{
				demanded: [],
				optional: []
			},
			arg1,
			arg2
		] : [
			parseCommand(`cmd ${arg1}`),
			arg2,
			arg3
		];
	}
	try {
		let position = 0;
		const [parsed, callerArguments, _length] = parseArgs();
		const args = [].slice.call(callerArguments);
		while (args.length && args[args.length - 1] === void 0) args.pop();
		const length = _length || args.length;
		if (length < parsed.demanded.length) throw new YError(`Not enough arguments provided. Expected ${parsed.demanded.length} but received ${args.length}.`);
		const totalCommands = parsed.demanded.length + parsed.optional.length;
		if (length > totalCommands) throw new YError(`Too many arguments provided. Expected max ${totalCommands} but received ${length}.`);
		parsed.demanded.forEach((demanded) => {
			const observedType = guessType(args.shift());
			if (demanded.cmd.filter((type) => type === observedType || type === "*").length === 0) argumentTypeError(observedType, demanded.cmd, position);
			position += 1;
		});
		parsed.optional.forEach((optional) => {
			if (args.length === 0) return;
			const observedType = guessType(args.shift());
			if (optional.cmd.filter((type) => type === observedType || type === "*").length === 0) argumentTypeError(observedType, optional.cmd, position);
			position += 1;
		});
	} catch (err) {
		console.warn(err.stack);
	}
}
function guessType(arg) {
	if (Array.isArray(arg)) return "array";
	else if (arg === null) return "null";
	return typeof arg;
}
function argumentTypeError(observedType, allowedTypes, position) {
	throw new YError(`Invalid ${positionName[position] || "manyith"} argument. Expected ${allowedTypes.join(" or ")} but received ${observedType}.`);
}

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/middleware.js
var GlobalMiddleware = class {
	constructor(yargs) {
		this.globalMiddleware = [];
		this.frozens = [];
		this.yargs = yargs;
	}
	addMiddleware(callback, applyBeforeValidation, global$1 = true, mutates = false) {
		argsert("<array|function> [boolean] [boolean] [boolean]", [
			callback,
			applyBeforeValidation,
			global$1
		], arguments.length);
		if (Array.isArray(callback)) {
			for (let i = 0; i < callback.length; i++) {
				if (typeof callback[i] !== "function") throw Error("middleware must be a function");
				const m = callback[i];
				m.applyBeforeValidation = applyBeforeValidation;
				m.global = global$1;
			}
			Array.prototype.push.apply(this.globalMiddleware, callback);
		} else if (typeof callback === "function") {
			const m = callback;
			m.applyBeforeValidation = applyBeforeValidation;
			m.global = global$1;
			m.mutates = mutates;
			this.globalMiddleware.push(callback);
		}
		return this.yargs;
	}
	addCoerceMiddleware(callback, option) {
		const aliases = this.yargs.getAliases();
		this.globalMiddleware = this.globalMiddleware.filter((m) => {
			const toCheck = [...aliases[option] || [], option];
			if (!m.option) return true;
			else return !toCheck.includes(m.option);
		});
		callback.option = option;
		return this.addMiddleware(callback, true, true, true);
	}
	getMiddleware() {
		return this.globalMiddleware;
	}
	freeze() {
		this.frozens.push([...this.globalMiddleware]);
	}
	unfreeze() {
		const frozen = this.frozens.pop();
		if (frozen !== void 0) this.globalMiddleware = frozen;
	}
	reset() {
		this.globalMiddleware = this.globalMiddleware.filter((m) => m.global);
	}
};
function commandMiddlewareFactory(commandMiddleware) {
	if (!commandMiddleware) return [];
	return commandMiddleware.map((middleware) => {
		middleware.applyBeforeValidation = false;
		return middleware;
	});
}
function applyMiddleware(argv$1, yargs, middlewares, beforeValidation) {
	return middlewares.reduce((acc, middleware) => {
		if (middleware.applyBeforeValidation !== beforeValidation) return acc;
		if (middleware.mutates) {
			if (middleware.applied) return acc;
			middleware.applied = true;
		}
		if (isPromise(acc)) return acc.then((initialObj) => Promise.all([initialObj, middleware(initialObj, yargs)])).then(([initialObj, middlewareObj]) => Object.assign(initialObj, middlewareObj));
		else {
			const result = middleware(acc, yargs);
			return isPromise(result) ? result.then((middlewareObj) => Object.assign(acc, middlewareObj)) : Object.assign(acc, result);
		}
	}, argv$1);
}

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/utils/maybe-async-result.js
function maybeAsyncResult(getResult, resultHandler, errorHandler = (err) => {
	throw err;
}) {
	try {
		const result = isFunction(getResult) ? getResult() : getResult;
		return isPromise(result) ? result.then((result$1) => resultHandler(result$1)) : resultHandler(result);
	} catch (err) {
		return errorHandler(err);
	}
}
function isFunction(arg) {
	return typeof arg === "function";
}

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/command.js
const DEFAULT_MARKER = /(^\*)|(^\$0)/;
var CommandInstance = class {
	constructor(usage$1, validation$1, globalMiddleware, shim$2) {
		this.requireCache = /* @__PURE__ */ new Set();
		this.handlers = {};
		this.aliasMap = {};
		this.frozens = [];
		this.shim = shim$2;
		this.usage = usage$1;
		this.globalMiddleware = globalMiddleware;
		this.validation = validation$1;
	}
	addDirectory(dir, req, callerFile, opts) {
		opts = opts || {};
		this.requireCache.add(callerFile);
		const fullDirPath = this.shim.path.resolve(this.shim.path.dirname(callerFile), dir);
		const files = this.shim.readdirSync(fullDirPath, { recursive: opts.recurse ? true : false });
		if (!Array.isArray(opts.extensions)) opts.extensions = ["js"];
		const visit = typeof opts.visit === "function" ? opts.visit : (o) => o;
		for (const fileb of files) {
			const file = fileb.toString();
			if (opts.exclude) {
				let exclude = false;
				if (typeof opts.exclude === "function") exclude = opts.exclude(file);
				else exclude = opts.exclude.test(file);
				if (exclude) continue;
			}
			if (opts.include) {
				let include = false;
				if (typeof opts.include === "function") include = opts.include(file);
				else include = opts.include.test(file);
				if (!include) continue;
			}
			let supportedExtension = false;
			for (const ext of opts.extensions) if (file.endsWith(ext)) supportedExtension = true;
			if (supportedExtension) {
				const joined = this.shim.path.join(fullDirPath, file);
				const module$1 = req(joined);
				const extendableModule = Object.create(null, Object.getOwnPropertyDescriptors({ ...module$1 }));
				if (visit(extendableModule, joined, file)) {
					if (this.requireCache.has(joined)) continue;
					else this.requireCache.add(joined);
					if (!extendableModule.command) extendableModule.command = this.shim.path.basename(joined, this.shim.path.extname(joined));
					this.addHandler(extendableModule);
				}
			}
		}
	}
	addHandler(cmd, description, builder, handler, commandMiddleware, deprecated) {
		let aliases = [];
		const middlewares = commandMiddlewareFactory(commandMiddleware);
		handler = handler || (() => {});
		if (Array.isArray(cmd)) if (isCommandAndAliases(cmd)) [cmd, ...aliases] = cmd;
		else for (const command$1 of cmd) this.addHandler(command$1);
		else if (isCommandHandlerDefinition(cmd)) {
			let command$1 = Array.isArray(cmd.command) || typeof cmd.command === "string" ? cmd.command : null;
			if (command$1 === null) throw new Error(`No command name given for module: ${this.shim.inspect(cmd)}`);
			if (cmd.aliases) command$1 = [].concat(command$1).concat(cmd.aliases);
			this.addHandler(command$1, this.extractDesc(cmd), cmd.builder, cmd.handler, cmd.middlewares, cmd.deprecated);
			return;
		} else if (isCommandBuilderDefinition(builder)) {
			this.addHandler([cmd].concat(aliases), description, builder.builder, builder.handler, builder.middlewares, builder.deprecated);
			return;
		}
		if (typeof cmd === "string") {
			const parsedCommand = parseCommand(cmd);
			aliases = aliases.map((alias) => parseCommand(alias).cmd);
			let isDefault = false;
			const parsedAliases = [parsedCommand.cmd].concat(aliases).filter((c) => {
				if (DEFAULT_MARKER.test(c)) {
					isDefault = true;
					return false;
				}
				return true;
			});
			if (parsedAliases.length === 0 && isDefault) parsedAliases.push("$0");
			if (isDefault) {
				parsedCommand.cmd = parsedAliases[0];
				aliases = parsedAliases.slice(1);
				cmd = cmd.replace(DEFAULT_MARKER, parsedCommand.cmd);
			}
			aliases.forEach((alias) => {
				this.aliasMap[alias] = parsedCommand.cmd;
			});
			if (description !== false) this.usage.command(cmd, description, isDefault, aliases, deprecated);
			this.handlers[parsedCommand.cmd] = {
				original: cmd,
				description,
				handler,
				builder: builder || {},
				middlewares,
				deprecated,
				demanded: parsedCommand.demanded,
				optional: parsedCommand.optional
			};
			if (isDefault) this.defaultCommand = this.handlers[parsedCommand.cmd];
		}
	}
	getCommandHandlers() {
		return this.handlers;
	}
	getCommands() {
		return Object.keys(this.handlers).concat(Object.keys(this.aliasMap));
	}
	hasDefaultCommand() {
		return !!this.defaultCommand;
	}
	runCommand(command$1, yargs, parsed, commandIndex, helpOnly, helpOrVersionSet) {
		const commandHandler = this.handlers[command$1] || this.handlers[this.aliasMap[command$1]] || this.defaultCommand;
		const currentContext = yargs.getInternalMethods().getContext();
		const parentCommands = currentContext.commands.slice();
		const isDefaultCommand = !command$1;
		if (command$1) {
			currentContext.commands.push(command$1);
			currentContext.fullCommands.push(commandHandler.original);
		}
		const builderResult = this.applyBuilderUpdateUsageAndParse(isDefaultCommand, commandHandler, yargs, parsed.aliases, parentCommands, commandIndex, helpOnly, helpOrVersionSet);
		return isPromise(builderResult) ? builderResult.then((result) => this.applyMiddlewareAndGetResult(isDefaultCommand, commandHandler, result.innerArgv, currentContext, helpOnly, result.aliases, yargs)) : this.applyMiddlewareAndGetResult(isDefaultCommand, commandHandler, builderResult.innerArgv, currentContext, helpOnly, builderResult.aliases, yargs);
	}
	applyBuilderUpdateUsageAndParse(isDefaultCommand, commandHandler, yargs, aliases, parentCommands, commandIndex, helpOnly, helpOrVersionSet) {
		const builder = commandHandler.builder;
		let innerYargs = yargs;
		if (isCommandBuilderCallback(builder)) {
			yargs.getInternalMethods().getUsageInstance().freeze();
			const builderOutput = builder(yargs.getInternalMethods().reset(aliases), helpOrVersionSet);
			if (isPromise(builderOutput)) return builderOutput.then((output) => {
				innerYargs = isYargsInstance(output) ? output : yargs;
				return this.parseAndUpdateUsage(isDefaultCommand, commandHandler, innerYargs, parentCommands, commandIndex, helpOnly);
			});
		} else if (isCommandBuilderOptionDefinitions(builder)) {
			yargs.getInternalMethods().getUsageInstance().freeze();
			innerYargs = yargs.getInternalMethods().reset(aliases);
			Object.keys(commandHandler.builder).forEach((key) => {
				innerYargs.option(key, builder[key]);
			});
		}
		return this.parseAndUpdateUsage(isDefaultCommand, commandHandler, innerYargs, parentCommands, commandIndex, helpOnly);
	}
	parseAndUpdateUsage(isDefaultCommand, commandHandler, innerYargs, parentCommands, commandIndex, helpOnly) {
		if (isDefaultCommand) innerYargs.getInternalMethods().getUsageInstance().unfreeze(true);
		if (this.shouldUpdateUsage(innerYargs)) innerYargs.getInternalMethods().getUsageInstance().usage(this.usageFromParentCommandsCommandHandler(parentCommands, commandHandler), commandHandler.description);
		const innerArgv = innerYargs.getInternalMethods().runYargsParserAndExecuteCommands(null, void 0, true, commandIndex, helpOnly);
		return isPromise(innerArgv) ? innerArgv.then((argv$1) => ({
			aliases: innerYargs.parsed.aliases,
			innerArgv: argv$1
		})) : {
			aliases: innerYargs.parsed.aliases,
			innerArgv
		};
	}
	shouldUpdateUsage(yargs) {
		return !yargs.getInternalMethods().getUsageInstance().getUsageDisabled() && yargs.getInternalMethods().getUsageInstance().getUsage().length === 0;
	}
	usageFromParentCommandsCommandHandler(parentCommands, commandHandler) {
		const c = DEFAULT_MARKER.test(commandHandler.original) ? commandHandler.original.replace(DEFAULT_MARKER, "").trim() : commandHandler.original;
		const pc = parentCommands.filter((c$1) => {
			return !DEFAULT_MARKER.test(c$1);
		});
		pc.push(c);
		return `$0 ${pc.join(" ")}`;
	}
	handleValidationAndGetResult(isDefaultCommand, commandHandler, innerArgv, currentContext, aliases, yargs, middlewares, positionalMap) {
		if (!yargs.getInternalMethods().getHasOutput()) {
			const validation$1 = yargs.getInternalMethods().runValidation(aliases, positionalMap, yargs.parsed.error, isDefaultCommand);
			innerArgv = maybeAsyncResult(innerArgv, (result) => {
				validation$1(result);
				return result;
			});
		}
		if (commandHandler.handler && !yargs.getInternalMethods().getHasOutput()) {
			yargs.getInternalMethods().setHasOutput();
			const populateDoubleDash = !!yargs.getOptions().configuration["populate--"];
			yargs.getInternalMethods().postProcess(innerArgv, populateDoubleDash, false, false);
			innerArgv = applyMiddleware(innerArgv, yargs, middlewares, false);
			innerArgv = maybeAsyncResult(innerArgv, (result) => {
				const handlerResult = commandHandler.handler(result);
				return isPromise(handlerResult) ? handlerResult.then(() => result) : result;
			});
			if (!isDefaultCommand) yargs.getInternalMethods().getUsageInstance().cacheHelpMessage();
			if (isPromise(innerArgv) && !yargs.getInternalMethods().hasParseCallback()) innerArgv.catch((error) => {
				try {
					yargs.getInternalMethods().getUsageInstance().fail(null, error);
				} catch (_err) {}
			});
		}
		if (!isDefaultCommand) {
			currentContext.commands.pop();
			currentContext.fullCommands.pop();
		}
		return innerArgv;
	}
	applyMiddlewareAndGetResult(isDefaultCommand, commandHandler, innerArgv, currentContext, helpOnly, aliases, yargs) {
		let positionalMap = {};
		if (helpOnly) return innerArgv;
		if (!yargs.getInternalMethods().getHasOutput()) positionalMap = this.populatePositionals(commandHandler, innerArgv, currentContext, yargs);
		const middlewares = this.globalMiddleware.getMiddleware().slice(0).concat(commandHandler.middlewares);
		const maybePromiseArgv = applyMiddleware(innerArgv, yargs, middlewares, true);
		return isPromise(maybePromiseArgv) ? maybePromiseArgv.then((resolvedInnerArgv) => this.handleValidationAndGetResult(isDefaultCommand, commandHandler, resolvedInnerArgv, currentContext, aliases, yargs, middlewares, positionalMap)) : this.handleValidationAndGetResult(isDefaultCommand, commandHandler, maybePromiseArgv, currentContext, aliases, yargs, middlewares, positionalMap);
	}
	populatePositionals(commandHandler, argv$1, context, yargs) {
		argv$1._ = argv$1._.slice(context.commands.length);
		const demanded = commandHandler.demanded.slice(0);
		const optional = commandHandler.optional.slice(0);
		const positionalMap = {};
		this.validation.positionalCount(demanded.length, argv$1._.length);
		while (demanded.length) {
			const demand = demanded.shift();
			this.populatePositional(demand, argv$1, positionalMap);
		}
		while (optional.length) {
			const maybe = optional.shift();
			this.populatePositional(maybe, argv$1, positionalMap);
		}
		argv$1._ = context.commands.concat(argv$1._.map((a) => "" + a));
		this.postProcessPositionals(argv$1, positionalMap, this.cmdToParseOptions(commandHandler.original), yargs);
		return positionalMap;
	}
	populatePositional(positional, argv$1, positionalMap) {
		const cmd = positional.cmd[0];
		if (positional.variadic) positionalMap[cmd] = argv$1._.splice(0).map(String);
		else if (argv$1._.length) positionalMap[cmd] = [String(argv$1._.shift())];
	}
	cmdToParseOptions(cmdString) {
		const parseOptions = {
			array: [],
			default: {},
			alias: {},
			demand: {}
		};
		const parsed = parseCommand(cmdString);
		parsed.demanded.forEach((d) => {
			const [cmd, ...aliases] = d.cmd;
			if (d.variadic) {
				parseOptions.array.push(cmd);
				parseOptions.default[cmd] = [];
			}
			parseOptions.alias[cmd] = aliases;
			parseOptions.demand[cmd] = true;
		});
		parsed.optional.forEach((o) => {
			const [cmd, ...aliases] = o.cmd;
			if (o.variadic) {
				parseOptions.array.push(cmd);
				parseOptions.default[cmd] = [];
			}
			parseOptions.alias[cmd] = aliases;
		});
		return parseOptions;
	}
	postProcessPositionals(argv$1, positionalMap, parseOptions, yargs) {
		const options = Object.assign({}, yargs.getOptions());
		options.default = Object.assign(parseOptions.default, options.default);
		for (const key of Object.keys(parseOptions.alias)) options.alias[key] = (options.alias[key] || []).concat(parseOptions.alias[key]);
		options.array = options.array.concat(parseOptions.array);
		options.config = {};
		const unparsed = [];
		Object.keys(positionalMap).forEach((key) => {
			positionalMap[key].map((value) => {
				if (options.configuration["unknown-options-as-args"]) options.key[key] = true;
				unparsed.push(`--${key}`);
				unparsed.push(value);
			});
		});
		if (!unparsed.length) return;
		const config = Object.assign({}, options.configuration, { "populate--": false });
		const parsed = this.shim.Parser.detailed(unparsed, Object.assign({}, options, { configuration: config }));
		if (parsed.error) yargs.getInternalMethods().getUsageInstance().fail(parsed.error.message, parsed.error);
		else {
			const positionalKeys = Object.keys(positionalMap);
			Object.keys(positionalMap).forEach((key) => {
				positionalKeys.push(...parsed.aliases[key]);
			});
			Object.keys(parsed.argv).forEach((key) => {
				if (positionalKeys.includes(key)) {
					if (!positionalMap[key]) positionalMap[key] = parsed.argv[key];
					if (!this.isInConfigs(yargs, key) && !this.isDefaulted(yargs, key) && Object.prototype.hasOwnProperty.call(argv$1, key) && Object.prototype.hasOwnProperty.call(parsed.argv, key) && (Array.isArray(argv$1[key]) || Array.isArray(parsed.argv[key]))) argv$1[key] = [].concat(argv$1[key], parsed.argv[key]);
					else argv$1[key] = parsed.argv[key];
				}
			});
		}
	}
	isDefaulted(yargs, key) {
		const { default: defaults } = yargs.getOptions();
		return Object.prototype.hasOwnProperty.call(defaults, key) || Object.prototype.hasOwnProperty.call(defaults, this.shim.Parser.camelCase(key));
	}
	isInConfigs(yargs, key) {
		const { configObjects } = yargs.getOptions();
		return configObjects.some((c) => Object.prototype.hasOwnProperty.call(c, key)) || configObjects.some((c) => Object.prototype.hasOwnProperty.call(c, this.shim.Parser.camelCase(key)));
	}
	runDefaultBuilderOn(yargs) {
		if (!this.defaultCommand) return;
		if (this.shouldUpdateUsage(yargs)) {
			const commandString = DEFAULT_MARKER.test(this.defaultCommand.original) ? this.defaultCommand.original : this.defaultCommand.original.replace(/^[^[\]<>]*/, "$0 ");
			yargs.getInternalMethods().getUsageInstance().usage(commandString, this.defaultCommand.description);
		}
		const builder = this.defaultCommand.builder;
		if (isCommandBuilderCallback(builder)) return builder(yargs, true);
		else if (!isCommandBuilderDefinition(builder)) Object.keys(builder).forEach((key) => {
			yargs.option(key, builder[key]);
		});
	}
	extractDesc({ describe, description, desc }) {
		for (const test of [
			describe,
			description,
			desc
		]) {
			if (typeof test === "string" || test === false) return test;
			assertNotStrictEqual(test, true, this.shim);
		}
		return false;
	}
	freeze() {
		this.frozens.push({
			handlers: this.handlers,
			aliasMap: this.aliasMap,
			defaultCommand: this.defaultCommand
		});
	}
	unfreeze() {
		const frozen = this.frozens.pop();
		assertNotStrictEqual(frozen, void 0, this.shim);
		({handlers: this.handlers, aliasMap: this.aliasMap, defaultCommand: this.defaultCommand} = frozen);
	}
	reset() {
		this.handlers = {};
		this.aliasMap = {};
		this.defaultCommand = void 0;
		this.requireCache = /* @__PURE__ */ new Set();
		return this;
	}
};
function command(usage$1, validation$1, globalMiddleware, shim$2) {
	return new CommandInstance(usage$1, validation$1, globalMiddleware, shim$2);
}
function isCommandBuilderDefinition(builder) {
	return typeof builder === "object" && !!builder.builder && typeof builder.handler === "function";
}
function isCommandAndAliases(cmd) {
	return cmd.every((c) => typeof c === "string");
}
function isCommandBuilderCallback(builder) {
	return typeof builder === "function";
}
function isCommandBuilderOptionDefinitions(builder) {
	return typeof builder === "object";
}
function isCommandHandlerDefinition(cmd) {
	return typeof cmd === "object" && !Array.isArray(cmd);
}

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/utils/obj-filter.js
function objFilter(original = {}, filter = () => true) {
	const obj = {};
	objectKeys(original).forEach((key) => {
		if (filter(key, original[key])) obj[key] = original[key];
	});
	return obj;
}

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/utils/set-blocking.js
function setBlocking(blocking) {
	if (typeof process === "undefined") return;
	[process.stdout, process.stderr].forEach((_stream) => {
		const stream = _stream;
		if (stream._handle && stream.isTTY && typeof stream._handle.setBlocking === "function") stream._handle.setBlocking(blocking);
	});
}

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/usage.js
function isBoolean(fail) {
	return typeof fail === "boolean";
}
function usage(yargs, shim$2) {
	const __ = shim$2.y18n.__;
	const self = {};
	const fails = [];
	self.failFn = function failFn(f) {
		fails.push(f);
	};
	let failMessage = null;
	let globalFailMessage = null;
	let showHelpOnFail = true;
	self.showHelpOnFail = function showHelpOnFailFn(arg1 = true, arg2) {
		const [enabled, message] = typeof arg1 === "string" ? [true, arg1] : [arg1, arg2];
		if (yargs.getInternalMethods().isGlobalContext()) globalFailMessage = message;
		failMessage = message;
		showHelpOnFail = enabled;
		return self;
	};
	let failureOutput = false;
	self.fail = function fail(msg, err) {
		const logger = yargs.getInternalMethods().getLoggerInstance();
		if (fails.length) for (let i = fails.length - 1; i >= 0; --i) {
			const fail$1 = fails[i];
			if (isBoolean(fail$1)) {
				if (err) throw err;
				else if (msg) throw Error(msg);
			} else fail$1(msg, err, self);
		}
		else {
			if (yargs.getExitProcess()) setBlocking(true);
			if (!failureOutput) {
				failureOutput = true;
				if (showHelpOnFail) {
					yargs.showHelp("error");
					logger.error();
				}
				if (msg || err) logger.error(msg || err);
				const globalOrCommandFailMessage = failMessage || globalFailMessage;
				if (globalOrCommandFailMessage) {
					if (msg || err) logger.error("");
					logger.error(globalOrCommandFailMessage);
				}
			}
			err = err || new YError(msg);
			if (yargs.getExitProcess()) return yargs.exit(1);
			else if (yargs.getInternalMethods().hasParseCallback()) return yargs.exit(1, err);
			else throw err;
		}
	};
	let usages = [];
	let usageDisabled = false;
	self.usage = (msg, description) => {
		if (msg === null) {
			usageDisabled = true;
			usages = [];
			return self;
		}
		usageDisabled = false;
		usages.push([msg, description || ""]);
		return self;
	};
	self.getUsage = () => {
		return usages;
	};
	self.getUsageDisabled = () => {
		return usageDisabled;
	};
	self.getPositionalGroupName = () => {
		return __("Positionals:");
	};
	let examples = [];
	self.example = (cmd, description) => {
		examples.push([cmd, description || ""]);
	};
	let commands = [];
	self.command = function command$1(cmd, description, isDefault, aliases, deprecated = false) {
		if (isDefault) commands = commands.map((cmdArray) => {
			cmdArray[2] = false;
			return cmdArray;
		});
		commands.push([
			cmd,
			description || "",
			isDefault,
			aliases,
			deprecated
		]);
	};
	self.getCommands = () => commands;
	let descriptions = {};
	self.describe = function describe(keyOrKeys, desc) {
		if (Array.isArray(keyOrKeys)) keyOrKeys.forEach((k) => {
			self.describe(k, desc);
		});
		else if (typeof keyOrKeys === "object") Object.keys(keyOrKeys).forEach((k) => {
			self.describe(k, keyOrKeys[k]);
		});
		else descriptions[keyOrKeys] = desc;
	};
	self.getDescriptions = () => descriptions;
	let epilogs = [];
	self.epilog = (msg) => {
		epilogs.push(msg);
	};
	let wrapSet = false;
	let wrap;
	self.wrap = (cols) => {
		wrapSet = true;
		wrap = cols;
	};
	self.getWrap = () => {
		if (shim$2.getEnv("YARGS_DISABLE_WRAP")) return null;
		if (!wrapSet) {
			wrap = windowWidth();
			wrapSet = true;
		}
		return wrap;
	};
	const deferY18nLookupPrefix = "__yargsString__:";
	self.deferY18nLookup = (str) => deferY18nLookupPrefix + str;
	self.help = function help() {
		if (cachedHelpMessage) return cachedHelpMessage;
		normalizeAliases();
		const base$0 = yargs.customScriptName ? yargs.$0 : shim$2.path.basename(yargs.$0);
		const demandedOptions = yargs.getDemandedOptions();
		const demandedCommands = yargs.getDemandedCommands();
		const deprecatedOptions = yargs.getDeprecatedOptions();
		const groups = yargs.getGroups();
		const options = yargs.getOptions();
		let keys = [];
		keys = keys.concat(Object.keys(descriptions));
		keys = keys.concat(Object.keys(demandedOptions));
		keys = keys.concat(Object.keys(demandedCommands));
		keys = keys.concat(Object.keys(options.default));
		keys = keys.filter(filterHiddenOptions);
		keys = Object.keys(keys.reduce((acc, key) => {
			if (key !== "_") acc[key] = true;
			return acc;
		}, {}));
		const theWrap = self.getWrap();
		const ui$1 = shim$2.cliui({
			width: theWrap,
			wrap: !!theWrap
		});
		if (!usageDisabled) {
			if (usages.length) {
				usages.forEach((usage$1) => {
					ui$1.div({ text: `${usage$1[0].replace(/\$0/g, base$0)}` });
					if (usage$1[1]) ui$1.div({
						text: `${usage$1[1]}`,
						padding: [
							1,
							0,
							0,
							0
						]
					});
				});
				ui$1.div();
			} else if (commands.length) {
				let u = null;
				if (demandedCommands._) u = `${base$0} <${__("command")}>\n`;
				else u = `${base$0} [${__("command")}]\n`;
				ui$1.div(`${u}`);
			}
		}
		if (commands.length > 1 || commands.length === 1 && !commands[0][2]) {
			ui$1.div(__("Commands:"));
			const context = yargs.getInternalMethods().getContext();
			const parentCommands = context.commands.length ? `${context.commands.join(" ")} ` : "";
			if (yargs.getInternalMethods().getParserConfiguration()["sort-commands"] === true) commands = commands.sort((a, b) => a[0].localeCompare(b[0]));
			const prefix = base$0 ? `${base$0} ` : "";
			commands.forEach((command$1) => {
				const commandString = `${prefix}${parentCommands}${command$1[0].replace(/^\$0 ?/, "")}`;
				ui$1.span({
					text: commandString,
					padding: [
						0,
						2,
						0,
						2
					],
					width: maxWidth(commands, theWrap, `${base$0}${parentCommands}`) + 4
				}, { text: command$1[1] });
				const hints = [];
				if (command$1[2]) hints.push(`[${__("default")}]`);
				if (command$1[3] && command$1[3].length) hints.push(`[${__("aliases:")} ${command$1[3].join(", ")}]`);
				if (command$1[4]) if (typeof command$1[4] === "string") hints.push(`[${__("deprecated: %s", command$1[4])}]`);
				else hints.push(`[${__("deprecated")}]`);
				if (hints.length) ui$1.div({
					text: hints.join(" "),
					padding: [
						0,
						0,
						0,
						2
					],
					align: "right"
				});
				else ui$1.div();
			});
			ui$1.div();
		}
		const aliasKeys = (Object.keys(options.alias) || []).concat(Object.keys(yargs.parsed.newAliases) || []);
		keys = keys.filter((key) => !yargs.parsed.newAliases[key] && aliasKeys.every((alias) => (options.alias[alias] || []).indexOf(key) === -1));
		const defaultGroup = __("Options:");
		if (!groups[defaultGroup]) groups[defaultGroup] = [];
		addUngroupedKeys(keys, options.alias, groups, defaultGroup);
		const isLongSwitch = (sw) => /^--/.test(getText(sw));
		const displayedGroups = Object.keys(groups).filter((groupName) => groups[groupName].length > 0).map((groupName) => {
			return {
				groupName,
				normalizedKeys: groups[groupName].filter(filterHiddenOptions).map((key) => {
					if (aliasKeys.includes(key)) return key;
					for (let i = 0, aliasKey; (aliasKey = aliasKeys[i]) !== void 0; i++) if ((options.alias[aliasKey] || []).includes(key)) return aliasKey;
					return key;
				})
			};
		}).filter(({ normalizedKeys }) => normalizedKeys.length > 0).map(({ groupName, normalizedKeys }) => {
			return {
				groupName,
				normalizedKeys,
				switches: normalizedKeys.reduce((acc, key) => {
					acc[key] = [key].concat(options.alias[key] || []).map((sw) => {
						if (groupName === self.getPositionalGroupName()) return sw;
						else return (/^[0-9]$/.test(sw) ? options.boolean.includes(key) ? "-" : "--" : sw.length > 1 ? "--" : "-") + sw;
					}).sort((sw1, sw2) => isLongSwitch(sw1) === isLongSwitch(sw2) ? 0 : isLongSwitch(sw1) ? 1 : -1).join(", ");
					return acc;
				}, {})
			};
		});
		if (displayedGroups.filter(({ groupName }) => groupName !== self.getPositionalGroupName()).some(({ normalizedKeys, switches }) => !normalizedKeys.every((key) => isLongSwitch(switches[key])))) displayedGroups.filter(({ groupName }) => groupName !== self.getPositionalGroupName()).forEach(({ normalizedKeys, switches }) => {
			normalizedKeys.forEach((key) => {
				if (isLongSwitch(switches[key])) switches[key] = addIndentation(switches[key], 4);
			});
		});
		displayedGroups.forEach(({ groupName, normalizedKeys, switches }) => {
			ui$1.div(groupName);
			normalizedKeys.forEach((key) => {
				const kswitch = switches[key];
				let desc = descriptions[key] || "";
				let type = null;
				if (desc.includes(deferY18nLookupPrefix)) desc = __(desc.substring(16));
				if (options.boolean.includes(key)) type = `[${__("boolean")}]`;
				if (options.count.includes(key)) type = `[${__("count")}]`;
				if (options.string.includes(key)) type = `[${__("string")}]`;
				if (options.normalize.includes(key)) type = `[${__("string")}]`;
				if (options.array.includes(key)) type = `[${__("array")}]`;
				if (options.number.includes(key)) type = `[${__("number")}]`;
				const deprecatedExtra = (deprecated) => typeof deprecated === "string" ? `[${__("deprecated: %s", deprecated)}]` : `[${__("deprecated")}]`;
				const extra = [
					key in deprecatedOptions ? deprecatedExtra(deprecatedOptions[key]) : null,
					type,
					key in demandedOptions ? `[${__("required")}]` : null,
					options.choices && options.choices[key] ? `[${__("choices:")} ${self.stringifiedValues(options.choices[key])}]` : null,
					defaultString(options.default[key], options.defaultDescription[key])
				].filter(Boolean).join(" ");
				ui$1.span({
					text: getText(kswitch),
					padding: [
						0,
						2,
						0,
						2 + getIndentation(kswitch)
					],
					width: maxWidth(switches, theWrap) + 4
				}, desc);
				const shouldHideOptionExtras = yargs.getInternalMethods().getUsageConfiguration()["hide-types"] === true;
				if (extra && !shouldHideOptionExtras) ui$1.div({
					text: extra,
					padding: [
						0,
						0,
						0,
						2
					],
					align: "right"
				});
				else ui$1.div();
			});
			ui$1.div();
		});
		if (examples.length) {
			ui$1.div(__("Examples:"));
			examples.forEach((example) => {
				example[0] = example[0].replace(/\$0/g, base$0);
			});
			examples.forEach((example) => {
				if (example[1] === "") ui$1.div({
					text: example[0],
					padding: [
						0,
						2,
						0,
						2
					]
				});
				else ui$1.div({
					text: example[0],
					padding: [
						0,
						2,
						0,
						2
					],
					width: maxWidth(examples, theWrap) + 4
				}, { text: example[1] });
			});
			ui$1.div();
		}
		if (epilogs.length > 0) {
			const e = epilogs.map((epilog) => epilog.replace(/\$0/g, base$0)).join("\n");
			ui$1.div(`${e}\n`);
		}
		return ui$1.toString().replace(/\s*$/, "");
	};
	function maxWidth(table, theWrap, modifier) {
		let width = 0;
		if (!Array.isArray(table)) table = Object.values(table).map((v) => [v]);
		table.forEach((v) => {
			width = Math.max(shim$2.stringWidth(modifier ? `${modifier} ${getText(v[0])}` : getText(v[0])) + getIndentation(v[0]), width);
		});
		if (theWrap) width = Math.min(width, parseInt((theWrap * .5).toString(), 10));
		return width;
	}
	function normalizeAliases() {
		const demandedOptions = yargs.getDemandedOptions();
		const options = yargs.getOptions();
		(Object.keys(options.alias) || []).forEach((key) => {
			options.alias[key].forEach((alias) => {
				if (descriptions[alias]) self.describe(key, descriptions[alias]);
				if (alias in demandedOptions) yargs.demandOption(key, demandedOptions[alias]);
				if (options.boolean.includes(alias)) yargs.boolean(key);
				if (options.count.includes(alias)) yargs.count(key);
				if (options.string.includes(alias)) yargs.string(key);
				if (options.normalize.includes(alias)) yargs.normalize(key);
				if (options.array.includes(alias)) yargs.array(key);
				if (options.number.includes(alias)) yargs.number(key);
			});
		});
	}
	let cachedHelpMessage;
	self.cacheHelpMessage = function() {
		cachedHelpMessage = this.help();
	};
	self.clearCachedHelpMessage = function() {
		cachedHelpMessage = void 0;
	};
	self.hasCachedHelpMessage = function() {
		return !!cachedHelpMessage;
	};
	function addUngroupedKeys(keys, aliases, groups, defaultGroup) {
		let groupedKeys = [];
		let toCheck = null;
		Object.keys(groups).forEach((group) => {
			groupedKeys = groupedKeys.concat(groups[group]);
		});
		keys.forEach((key) => {
			toCheck = [key].concat(aliases[key]);
			if (!toCheck.some((k) => groupedKeys.indexOf(k) !== -1)) groups[defaultGroup].push(key);
		});
		return groupedKeys;
	}
	function filterHiddenOptions(key) {
		return yargs.getOptions().hiddenOptions.indexOf(key) < 0 || yargs.parsed.argv[yargs.getOptions().showHiddenOpt];
	}
	self.showHelp = (level) => {
		const logger = yargs.getInternalMethods().getLoggerInstance();
		if (!level) level = "error";
		(typeof level === "function" ? level : logger[level])(self.help());
	};
	self.functionDescription = (fn) => {
		return [
			"(",
			fn.name ? shim$2.Parser.decamelize(fn.name, "-") : __("generated-value"),
			")"
		].join("");
	};
	self.stringifiedValues = function stringifiedValues(values, separator) {
		let string = "";
		const sep = separator || ", ";
		const array = [].concat(values);
		if (!values || !array.length) return string;
		array.forEach((value) => {
			if (string.length) string += sep;
			string += JSON.stringify(value);
		});
		return string;
	};
	function defaultString(value, defaultDescription) {
		let string = `[${__("default:")} `;
		if (value === void 0 && !defaultDescription) return null;
		if (defaultDescription) string += defaultDescription;
		else switch (typeof value) {
			case "string":
				string += `"${value}"`;
				break;
			case "object":
				string += JSON.stringify(value);
				break;
			default: string += value;
		}
		return `${string}]`;
	}
	function windowWidth() {
		const maxWidth$1 = 80;
		if (shim$2.process.stdColumns) return Math.min(maxWidth$1, shim$2.process.stdColumns);
		else return maxWidth$1;
	}
	let version = null;
	self.version = (ver) => {
		version = ver;
	};
	self.showVersion = (level) => {
		const logger = yargs.getInternalMethods().getLoggerInstance();
		if (!level) level = "error";
		(typeof level === "function" ? level : logger[level])(version);
	};
	self.reset = function reset(localLookup) {
		failMessage = null;
		failureOutput = false;
		usages = [];
		usageDisabled = false;
		epilogs = [];
		examples = [];
		commands = [];
		descriptions = objFilter(descriptions, (k) => !localLookup[k]);
		return self;
	};
	const frozens = [];
	self.freeze = function freeze() {
		frozens.push({
			failMessage,
			failureOutput,
			usages,
			usageDisabled,
			epilogs,
			examples,
			commands,
			descriptions
		});
	};
	self.unfreeze = function unfreeze(defaultCommand = false) {
		const frozen = frozens.pop();
		if (!frozen) return;
		if (defaultCommand) {
			descriptions = {
				...frozen.descriptions,
				...descriptions
			};
			commands = [...frozen.commands, ...commands];
			usages = [...frozen.usages, ...usages];
			examples = [...frozen.examples, ...examples];
			epilogs = [...frozen.epilogs, ...epilogs];
		} else ({failMessage, failureOutput, usages, usageDisabled, epilogs, examples, commands, descriptions} = frozen);
	};
	return self;
}
function isIndentedText(text) {
	return typeof text === "object";
}
function addIndentation(text, indent) {
	return isIndentedText(text) ? {
		text: text.text,
		indentation: text.indentation + indent
	} : {
		text,
		indentation: indent
	};
}
function getIndentation(text) {
	return isIndentedText(text) ? text.indentation : 0;
}
function getText(text) {
	return isIndentedText(text) ? text.text : text;
}

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/completion-templates.js
const completionShTemplate = `###-begin-{{app_name}}-completions-###
#
# yargs command completion script
#
# Installation: {{app_path}} {{completion_command}} >> ~/.bashrc
#    or {{app_path}} {{completion_command}} >> ~/.bash_profile on OSX.
#
_{{app_name}}_yargs_completions()
{
    local cur_word args type_list

    cur_word="\${COMP_WORDS[COMP_CWORD]}"
    args=("\${COMP_WORDS[@]}")

    # ask yargs to generate completions.
    # see https://stackoverflow.com/a/40944195/7080036 for the spaces-handling awk
    mapfile -t type_list < <({{app_path}} --get-yargs-completions "\${args[@]}")
    mapfile -t COMPREPLY < <(compgen -W "$( printf '%q ' "\${type_list[@]}" )" -- "\${cur_word}" |
        awk '/ / { print "\\""$0"\\"" } /^[^ ]+$/ { print $0 }')

    # if no match was found, fall back to filename completion
    if [ \${#COMPREPLY[@]} -eq 0 ]; then
      COMPREPLY=()
    fi

    return 0
}
complete -o bashdefault -o default -F _{{app_name}}_yargs_completions {{app_name}}
###-end-{{app_name}}-completions-###
`;
const completionZshTemplate = `#compdef {{app_name}}
###-begin-{{app_name}}-completions-###
#
# yargs command completion script
#
# Installation: {{app_path}} {{completion_command}} >> ~/.zshrc
#    or {{app_path}} {{completion_command}} >> ~/.zprofile on OSX.
#
_{{app_name}}_yargs_completions()
{
  local reply
  local si=$IFS
  IFS=$'\n' reply=($(COMP_CWORD="$((CURRENT-1))" COMP_LINE="$BUFFER" COMP_POINT="$CURSOR" {{app_path}} --get-yargs-completions "\${words[@]}"))
  IFS=$si
  if [[ \${#reply} -gt 0 ]]; then
    _describe 'values' reply
  else
    _default
  fi
}
if [[ "'\${zsh_eval_context[-1]}" == "loadautofunc" ]]; then
  _{{app_name}}_yargs_completions "$@"
else
  compdef _{{app_name}}_yargs_completions {{app_name}}
fi
###-end-{{app_name}}-completions-###
`;

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/completion.js
var Completion = class {
	constructor(yargs, usage$1, command$1, shim$2) {
		var _a$1, _b$1, _c$1;
		this.yargs = yargs;
		this.usage = usage$1;
		this.command = command$1;
		this.shim = shim$2;
		this.completionKey = "get-yargs-completions";
		this.aliases = null;
		this.customCompletionFunction = null;
		this.indexAfterLastReset = 0;
		this.zshShell = (_c$1 = ((_a$1 = this.shim.getEnv("SHELL")) === null || _a$1 === void 0 ? void 0 : _a$1.includes("zsh")) || ((_b$1 = this.shim.getEnv("ZSH_NAME")) === null || _b$1 === void 0 ? void 0 : _b$1.includes("zsh"))) !== null && _c$1 !== void 0 ? _c$1 : false;
	}
	defaultCompletion(args, argv$1, current, done) {
		const handlers = this.command.getCommandHandlers();
		for (let i = 0, ii = args.length; i < ii; ++i) if (handlers[args[i]] && handlers[args[i]].builder) {
			const builder = handlers[args[i]].builder;
			if (isCommandBuilderCallback(builder)) {
				this.indexAfterLastReset = i + 1;
				const y = this.yargs.getInternalMethods().reset();
				builder(y, true);
				return y.argv;
			}
		}
		const completions = [];
		this.commandCompletions(completions, args, current);
		this.optionCompletions(completions, args, argv$1, current);
		this.choicesFromOptionsCompletions(completions, args, argv$1, current);
		this.choicesFromPositionalsCompletions(completions, args, argv$1, current);
		done(null, completions);
	}
	commandCompletions(completions, args, current) {
		const parentCommands = this.yargs.getInternalMethods().getContext().commands;
		if (!current.match(/^-/) && parentCommands[parentCommands.length - 1] !== current && !this.previousArgHasChoices(args)) this.usage.getCommands().forEach((usageCommand) => {
			const commandName = parseCommand(usageCommand[0]).cmd;
			if (args.indexOf(commandName) === -1) if (!this.zshShell) completions.push(commandName);
			else {
				const desc = usageCommand[1] || "";
				completions.push(commandName.replace(/:/g, "\\:") + ":" + desc);
			}
		});
	}
	optionCompletions(completions, args, argv$1, current) {
		if ((current.match(/^-/) || current === "" && completions.length === 0) && !this.previousArgHasChoices(args)) {
			const options = this.yargs.getOptions();
			const positionalKeys = this.yargs.getGroups()[this.usage.getPositionalGroupName()] || [];
			Object.keys(options.key).forEach((key) => {
				const negable = !!options.configuration["boolean-negation"] && options.boolean.includes(key);
				if (!positionalKeys.includes(key) && !options.hiddenOptions.includes(key) && !this.argsContainKey(args, key, negable)) this.completeOptionKey(key, completions, current, negable && !!options.default[key]);
			});
		}
	}
	choicesFromOptionsCompletions(completions, args, argv$1, current) {
		if (this.previousArgHasChoices(args)) {
			const choices = this.getPreviousArgChoices(args);
			if (choices && choices.length > 0) completions.push(...choices.map((c) => c.replace(/:/g, "\\:")));
		}
	}
	choicesFromPositionalsCompletions(completions, args, argv$1, current) {
		if (current === "" && completions.length > 0 && this.previousArgHasChoices(args)) return;
		const positionalKeys = this.yargs.getGroups()[this.usage.getPositionalGroupName()] || [];
		const offset = Math.max(this.indexAfterLastReset, this.yargs.getInternalMethods().getContext().commands.length + 1);
		const positionalKey = positionalKeys[argv$1._.length - offset - 1];
		if (!positionalKey) return;
		const choices = this.yargs.getOptions().choices[positionalKey] || [];
		for (const choice of choices) if (choice.startsWith(current)) completions.push(choice.replace(/:/g, "\\:"));
	}
	getPreviousArgChoices(args) {
		if (args.length < 1) return;
		let previousArg = args[args.length - 1];
		let filter = "";
		if (!previousArg.startsWith("-") && args.length > 1) {
			filter = previousArg;
			previousArg = args[args.length - 2];
		}
		if (!previousArg.startsWith("-")) return;
		const previousArgKey = previousArg.replace(/^-+/, "");
		const options = this.yargs.getOptions();
		const possibleAliases = [previousArgKey, ...this.yargs.getAliases()[previousArgKey] || []];
		let choices;
		for (const possibleAlias of possibleAliases) if (Object.prototype.hasOwnProperty.call(options.key, possibleAlias) && Array.isArray(options.choices[possibleAlias])) {
			choices = options.choices[possibleAlias];
			break;
		}
		if (choices) return choices.filter((choice) => !filter || choice.startsWith(filter));
	}
	previousArgHasChoices(args) {
		const choices = this.getPreviousArgChoices(args);
		return choices !== void 0 && choices.length > 0;
	}
	argsContainKey(args, key, negable) {
		const argsContains = (s) => args.indexOf((/^[^0-9]$/.test(s) ? "-" : "--") + s) !== -1;
		if (argsContains(key)) return true;
		if (negable && argsContains(`no-${key}`)) return true;
		if (this.aliases) {
			for (const alias of this.aliases[key]) if (argsContains(alias)) return true;
		}
		return false;
	}
	completeOptionKey(key, completions, current, negable) {
		var _a$1, _b$1, _c$1, _d;
		let keyWithDesc = key;
		if (this.zshShell) {
			const descs = this.usage.getDescriptions();
			const aliasKey = (_b$1 = (_a$1 = this === null || this === void 0 ? void 0 : this.aliases) === null || _a$1 === void 0 ? void 0 : _a$1[key]) === null || _b$1 === void 0 ? void 0 : _b$1.find((alias) => {
				const desc$1 = descs[alias];
				return typeof desc$1 === "string" && desc$1.length > 0;
			});
			const descFromAlias = aliasKey ? descs[aliasKey] : void 0;
			const desc = (_d = (_c$1 = descs[key]) !== null && _c$1 !== void 0 ? _c$1 : descFromAlias) !== null && _d !== void 0 ? _d : "";
			keyWithDesc = `${key.replace(/:/g, "\\:")}:${desc.replace("__yargsString__:", "").replace(/(\r\n|\n|\r)/gm, " ")}`;
		}
		const startsByTwoDashes = (s) => /^--/.test(s);
		const isShortOption = (s) => /^[^0-9]$/.test(s);
		const dashes = !startsByTwoDashes(current) && isShortOption(key) ? "-" : "--";
		completions.push(dashes + keyWithDesc);
		if (negable) completions.push(dashes + "no-" + keyWithDesc);
	}
	customCompletion(args, argv$1, current, done) {
		assertNotStrictEqual(this.customCompletionFunction, null, this.shim);
		if (isSyncCompletionFunction(this.customCompletionFunction)) {
			const result = this.customCompletionFunction(current, argv$1);
			if (isPromise(result)) return result.then((list) => {
				this.shim.process.nextTick(() => {
					done(null, list);
				});
			}).catch((err) => {
				this.shim.process.nextTick(() => {
					done(err, void 0);
				});
			});
			return done(null, result);
		} else if (isFallbackCompletionFunction(this.customCompletionFunction)) return this.customCompletionFunction(current, argv$1, (onCompleted = done) => this.defaultCompletion(args, argv$1, current, onCompleted), (completions) => {
			done(null, completions);
		});
		else return this.customCompletionFunction(current, argv$1, (completions) => {
			done(null, completions);
		});
	}
	getCompletion(args, done) {
		const current = args.length ? args[args.length - 1] : "";
		const argv$1 = this.yargs.parse(args, true);
		const completionFunction = this.customCompletionFunction ? (argv$2) => this.customCompletion(args, argv$2, current, done) : (argv$2) => this.defaultCompletion(args, argv$2, current, done);
		return isPromise(argv$1) ? argv$1.then(completionFunction) : completionFunction(argv$1);
	}
	generateCompletionScript($0, cmd) {
		let script = this.zshShell ? completionZshTemplate : completionShTemplate;
		const name = this.shim.path.basename($0);
		if ($0.match(/\.js$/)) $0 = `./${$0}`;
		script = script.replace(/{{app_name}}/g, name);
		script = script.replace(/{{completion_command}}/g, cmd);
		return script.replace(/{{app_path}}/g, $0);
	}
	registerFunction(fn) {
		this.customCompletionFunction = fn;
	}
	setParsed(parsed) {
		this.aliases = parsed.aliases;
	}
};
function completion(yargs, usage$1, command$1, shim$2) {
	return new Completion(yargs, usage$1, command$1, shim$2);
}
function isSyncCompletionFunction(completionFunction) {
	return completionFunction.length < 3;
}
function isFallbackCompletionFunction(completionFunction) {
	return completionFunction.length > 3;
}

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/utils/levenshtein.js
function levenshtein(a, b) {
	if (a.length === 0) return b.length;
	if (b.length === 0) return a.length;
	const matrix = [];
	let i;
	for (i = 0; i <= b.length; i++) matrix[i] = [i];
	let j;
	for (j = 0; j <= a.length; j++) matrix[0][j] = j;
	for (i = 1; i <= b.length; i++) for (j = 1; j <= a.length; j++) if (b.charAt(i - 1) === a.charAt(j - 1)) matrix[i][j] = matrix[i - 1][j - 1];
	else if (i > 1 && j > 1 && b.charAt(i - 2) === a.charAt(j - 1) && b.charAt(i - 1) === a.charAt(j - 2)) matrix[i][j] = matrix[i - 2][j - 2] + 1;
	else matrix[i][j] = Math.min(matrix[i - 1][j - 1] + 1, Math.min(matrix[i][j - 1] + 1, matrix[i - 1][j] + 1));
	return matrix[b.length][a.length];
}

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/validation.js
const specialKeys = [
	"$0",
	"--",
	"_"
];
function validation(yargs, usage$1, shim$2) {
	const __ = shim$2.y18n.__;
	const __n = shim$2.y18n.__n;
	const self = {};
	self.nonOptionCount = function nonOptionCount(argv$1) {
		const demandedCommands = yargs.getDemandedCommands();
		const _s = argv$1._.length + (argv$1["--"] ? argv$1["--"].length : 0) - yargs.getInternalMethods().getContext().commands.length;
		if (demandedCommands._ && (_s < demandedCommands._.min || _s > demandedCommands._.max)) {
			if (_s < demandedCommands._.min) if (demandedCommands._.minMsg !== void 0) usage$1.fail(demandedCommands._.minMsg ? demandedCommands._.minMsg.replace(/\$0/g, _s.toString()).replace(/\$1/, demandedCommands._.min.toString()) : null);
			else usage$1.fail(__n("Not enough non-option arguments: got %s, need at least %s", "Not enough non-option arguments: got %s, need at least %s", _s, _s.toString(), demandedCommands._.min.toString()));
			else if (_s > demandedCommands._.max) if (demandedCommands._.maxMsg !== void 0) usage$1.fail(demandedCommands._.maxMsg ? demandedCommands._.maxMsg.replace(/\$0/g, _s.toString()).replace(/\$1/, demandedCommands._.max.toString()) : null);
			else usage$1.fail(__n("Too many non-option arguments: got %s, maximum of %s", "Too many non-option arguments: got %s, maximum of %s", _s, _s.toString(), demandedCommands._.max.toString()));
		}
	};
	self.positionalCount = function positionalCount(required, observed) {
		if (observed < required) usage$1.fail(__n("Not enough non-option arguments: got %s, need at least %s", "Not enough non-option arguments: got %s, need at least %s", observed, observed + "", required + ""));
	};
	self.requiredArguments = function requiredArguments(argv$1, demandedOptions) {
		let missing = null;
		for (const key of Object.keys(demandedOptions)) if (!Object.prototype.hasOwnProperty.call(argv$1, key) || typeof argv$1[key] === "undefined") {
			missing = missing || {};
			missing[key] = demandedOptions[key];
		}
		if (missing) {
			const customMsgs = [];
			for (const key of Object.keys(missing)) {
				const msg = missing[key];
				if (msg && customMsgs.indexOf(msg) < 0) customMsgs.push(msg);
			}
			const customMsg = customMsgs.length ? `\n${customMsgs.join("\n")}` : "";
			usage$1.fail(__n("Missing required argument: %s", "Missing required arguments: %s", Object.keys(missing).length, Object.keys(missing).join(", ") + customMsg));
		}
	};
	self.unknownArguments = function unknownArguments(argv$1, aliases, positionalMap, isDefaultCommand, checkPositionals = true) {
		var _a$1;
		const commandKeys = yargs.getInternalMethods().getCommandInstance().getCommands();
		const unknown = [];
		const currentContext = yargs.getInternalMethods().getContext();
		Object.keys(argv$1).forEach((key) => {
			if (!specialKeys.includes(key) && !Object.prototype.hasOwnProperty.call(positionalMap, key) && !Object.prototype.hasOwnProperty.call(yargs.getInternalMethods().getParseContext(), key) && !self.isValidAndSomeAliasIsNotNew(key, aliases)) unknown.push(key);
		});
		if (checkPositionals && (currentContext.commands.length > 0 || commandKeys.length > 0 || isDefaultCommand)) argv$1._.slice(currentContext.commands.length).forEach((key) => {
			if (!commandKeys.includes("" + key)) unknown.push("" + key);
		});
		if (checkPositionals) {
			const maxNonOptDemanded = ((_a$1 = yargs.getDemandedCommands()._) === null || _a$1 === void 0 ? void 0 : _a$1.max) || 0;
			const expected = currentContext.commands.length + maxNonOptDemanded;
			if (expected < argv$1._.length) argv$1._.slice(expected).forEach((key) => {
				key = String(key);
				if (!currentContext.commands.includes(key) && !unknown.includes(key)) unknown.push(key);
			});
		}
		if (unknown.length) usage$1.fail(__n("Unknown argument: %s", "Unknown arguments: %s", unknown.length, unknown.map((s) => s.trim() ? s : `"${s}"`).join(", ")));
	};
	self.unknownCommands = function unknownCommands(argv$1) {
		const commandKeys = yargs.getInternalMethods().getCommandInstance().getCommands();
		const unknown = [];
		const currentContext = yargs.getInternalMethods().getContext();
		if (currentContext.commands.length > 0 || commandKeys.length > 0) argv$1._.slice(currentContext.commands.length).forEach((key) => {
			if (!commandKeys.includes("" + key)) unknown.push("" + key);
		});
		if (unknown.length > 0) {
			usage$1.fail(__n("Unknown command: %s", "Unknown commands: %s", unknown.length, unknown.join(", ")));
			return true;
		} else return false;
	};
	self.isValidAndSomeAliasIsNotNew = function isValidAndSomeAliasIsNotNew(key, aliases) {
		if (!Object.prototype.hasOwnProperty.call(aliases, key)) return false;
		const newAliases = yargs.parsed.newAliases;
		return [key, ...aliases[key]].some((a) => !Object.prototype.hasOwnProperty.call(newAliases, a) || !newAliases[key]);
	};
	self.limitedChoices = function limitedChoices(argv$1) {
		const options = yargs.getOptions();
		const invalid = {};
		if (!Object.keys(options.choices).length) return;
		Object.keys(argv$1).forEach((key) => {
			if (specialKeys.indexOf(key) === -1 && Object.prototype.hasOwnProperty.call(options.choices, key)) [].concat(argv$1[key]).forEach((value) => {
				if (options.choices[key].indexOf(value) === -1 && value !== void 0) invalid[key] = (invalid[key] || []).concat(value);
			});
		});
		const invalidKeys = Object.keys(invalid);
		if (!invalidKeys.length) return;
		let msg = __("Invalid values:");
		invalidKeys.forEach((key) => {
			msg += `\n  ${__("Argument: %s, Given: %s, Choices: %s", key, usage$1.stringifiedValues(invalid[key]), usage$1.stringifiedValues(options.choices[key]))}`;
		});
		usage$1.fail(msg);
	};
	let implied = {};
	self.implies = function implies(key, value) {
		argsert("<string|object> [array|number|string]", [key, value], arguments.length);
		if (typeof key === "object") Object.keys(key).forEach((k) => {
			self.implies(k, key[k]);
		});
		else {
			yargs.global(key);
			if (!implied[key]) implied[key] = [];
			if (Array.isArray(value)) value.forEach((i) => self.implies(key, i));
			else {
				assertNotStrictEqual(value, void 0, shim$2);
				implied[key].push(value);
			}
		}
	};
	self.getImplied = function getImplied() {
		return implied;
	};
	function keyExists(argv$1, val) {
		const num = Number(val);
		val = isNaN(num) ? val : num;
		if (typeof val === "number") val = argv$1._.length >= val;
		else if (val.match(/^--no-.+/)) {
			val = val.match(/^--no-(.+)/)[1];
			val = !Object.prototype.hasOwnProperty.call(argv$1, val);
		} else val = Object.prototype.hasOwnProperty.call(argv$1, val);
		return val;
	}
	self.implications = function implications(argv$1) {
		const implyFail = [];
		Object.keys(implied).forEach((key) => {
			const origKey = key;
			(implied[key] || []).forEach((value) => {
				let key$1 = origKey;
				const origValue = value;
				key$1 = keyExists(argv$1, key$1);
				value = keyExists(argv$1, value);
				if (key$1 && !value) implyFail.push(` ${origKey} -> ${origValue}`);
			});
		});
		if (implyFail.length) {
			let msg = `${__("Implications failed:")}\n`;
			implyFail.forEach((value) => {
				msg += value;
			});
			usage$1.fail(msg);
		}
	};
	let conflicting = {};
	self.conflicts = function conflicts(key, value) {
		argsert("<string|object> [array|string]", [key, value], arguments.length);
		if (typeof key === "object") Object.keys(key).forEach((k) => {
			self.conflicts(k, key[k]);
		});
		else {
			yargs.global(key);
			if (!conflicting[key]) conflicting[key] = [];
			if (Array.isArray(value)) value.forEach((i) => self.conflicts(key, i));
			else conflicting[key].push(value);
		}
	};
	self.getConflicting = () => conflicting;
	self.conflicting = function conflictingFn(argv$1) {
		Object.keys(argv$1).forEach((key) => {
			if (conflicting[key]) conflicting[key].forEach((value) => {
				if (value && argv$1[key] !== void 0 && argv$1[value] !== void 0) usage$1.fail(__("Arguments %s and %s are mutually exclusive", key, value));
			});
		});
		if (yargs.getInternalMethods().getParserConfiguration()["strip-dashed"]) Object.keys(conflicting).forEach((key) => {
			conflicting[key].forEach((value) => {
				if (value && argv$1[shim$2.Parser.camelCase(key)] !== void 0 && argv$1[shim$2.Parser.camelCase(value)] !== void 0) usage$1.fail(__("Arguments %s and %s are mutually exclusive", key, value));
			});
		});
	};
	self.recommendCommands = function recommendCommands(cmd, potentialCommands) {
		const threshold = 3;
		potentialCommands = potentialCommands.sort((a, b) => b.length - a.length);
		let recommended = null;
		let bestDistance = Infinity;
		for (let i = 0, candidate; (candidate = potentialCommands[i]) !== void 0; i++) {
			const d = levenshtein(cmd, candidate);
			if (d <= threshold && d < bestDistance) {
				bestDistance = d;
				recommended = candidate;
			}
		}
		if (recommended) usage$1.fail(__("Did you mean %s?", recommended));
	};
	self.reset = function reset(localLookup) {
		implied = objFilter(implied, (k) => !localLookup[k]);
		conflicting = objFilter(conflicting, (k) => !localLookup[k]);
		return self;
	};
	const frozens = [];
	self.freeze = function freeze() {
		frozens.push({
			implied,
			conflicting
		});
	};
	self.unfreeze = function unfreeze() {
		const frozen = frozens.pop();
		assertNotStrictEqual(frozen, void 0, shim$2);
		({implied, conflicting} = frozen);
	};
	return self;
}

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/utils/apply-extends.js
let previouslyVisitedConfigs = [];
let shim;
function applyExtends(config, cwd, mergeExtends, _shim) {
	shim = _shim;
	let defaultConfig = {};
	if (Object.prototype.hasOwnProperty.call(config, "extends")) {
		if (typeof config.extends !== "string") return defaultConfig;
		const isPath = /\.json|\..*rc$/.test(config.extends);
		let pathToDefault = null;
		if (!isPath) try {
			pathToDefault = import.meta.resolve(config.extends);
		} catch (_err) {
			return config;
		}
		else pathToDefault = getPathToDefaultConfig(cwd, config.extends);
		checkForCircularExtends(pathToDefault);
		previouslyVisitedConfigs.push(pathToDefault);
		defaultConfig = isPath ? JSON.parse(shim.readFileSync(pathToDefault, "utf8")) : _shim.require(config.extends);
		delete config.extends;
		defaultConfig = applyExtends(defaultConfig, shim.path.dirname(pathToDefault), mergeExtends, shim);
	}
	previouslyVisitedConfigs = [];
	return mergeExtends ? mergeDeep(defaultConfig, config) : Object.assign({}, defaultConfig, config);
}
function checkForCircularExtends(cfgPath) {
	if (previouslyVisitedConfigs.indexOf(cfgPath) > -1) throw new YError(`Circular extended configurations: '${cfgPath}'.`);
}
function getPathToDefaultConfig(cwd, pathToExtend) {
	return shim.path.resolve(cwd, pathToExtend);
}
function mergeDeep(config1, config2) {
	const target = {};
	function isObject(obj) {
		return obj && typeof obj === "object" && !Array.isArray(obj);
	}
	Object.assign(target, config1);
	for (const key of Object.keys(config2)) if (isObject(config2[key]) && isObject(target[key])) target[key] = mergeDeep(config1[key], config2[key]);
	else target[key] = config2[key];
	return target;
}

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/build/lib/yargs-factory.js
var __classPrivateFieldSet = void 0 && (void 0).__classPrivateFieldSet || function(receiver, state, value, kind, f) {
	if (kind === "m") throw new TypeError("Private method is not writable");
	if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a setter");
	if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
	return kind === "a" ? f.call(receiver, value) : f ? f.value = value : state.set(receiver, value), value;
};
var __classPrivateFieldGet = void 0 && (void 0).__classPrivateFieldGet || function(receiver, state, kind, f) {
	if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a getter");
	if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
	return kind === "m" ? f : kind === "a" ? f.call(receiver) : f ? f.value : state.get(receiver);
};
var _YargsInstance_command, _YargsInstance_cwd, _YargsInstance_context, _YargsInstance_completion, _YargsInstance_completionCommand, _YargsInstance_defaultShowHiddenOpt, _YargsInstance_exitError, _YargsInstance_detectLocale, _YargsInstance_emittedWarnings, _YargsInstance_exitProcess, _YargsInstance_frozens, _YargsInstance_globalMiddleware, _YargsInstance_groups, _YargsInstance_hasOutput, _YargsInstance_helpOpt, _YargsInstance_isGlobalContext, _YargsInstance_logger, _YargsInstance_output, _YargsInstance_options, _YargsInstance_parentRequire, _YargsInstance_parserConfig, _YargsInstance_parseFn, _YargsInstance_parseContext, _YargsInstance_pkgs, _YargsInstance_preservedGroups, _YargsInstance_processArgs, _YargsInstance_recommendCommands, _YargsInstance_shim, _YargsInstance_strict, _YargsInstance_strictCommands, _YargsInstance_strictOptions, _YargsInstance_usage, _YargsInstance_usageConfig, _YargsInstance_versionOpt, _YargsInstance_validation;
function YargsFactory(_shim) {
	return (processArgs = [], cwd = _shim.process.cwd(), parentRequire) => {
		const yargs = new YargsInstance(processArgs, cwd, parentRequire, _shim);
		Object.defineProperty(yargs, "argv", {
			get: () => {
				return yargs.parse();
			},
			enumerable: true
		});
		yargs.help();
		yargs.version();
		return yargs;
	};
}
const kCopyDoubleDash = Symbol("copyDoubleDash");
const kCreateLogger = Symbol("copyDoubleDash");
const kDeleteFromParserHintObject = Symbol("deleteFromParserHintObject");
const kEmitWarning = Symbol("emitWarning");
const kFreeze = Symbol("freeze");
const kGetDollarZero = Symbol("getDollarZero");
const kGetParserConfiguration = Symbol("getParserConfiguration");
const kGetUsageConfiguration = Symbol("getUsageConfiguration");
const kGuessLocale = Symbol("guessLocale");
const kGuessVersion = Symbol("guessVersion");
const kParsePositionalNumbers = Symbol("parsePositionalNumbers");
const kPkgUp = Symbol("pkgUp");
const kPopulateParserHintArray = Symbol("populateParserHintArray");
const kPopulateParserHintSingleValueDictionary = Symbol("populateParserHintSingleValueDictionary");
const kPopulateParserHintArrayDictionary = Symbol("populateParserHintArrayDictionary");
const kPopulateParserHintDictionary = Symbol("populateParserHintDictionary");
const kSanitizeKey = Symbol("sanitizeKey");
const kSetKey = Symbol("setKey");
const kUnfreeze = Symbol("unfreeze");
const kValidateAsync = Symbol("validateAsync");
const kGetCommandInstance = Symbol("getCommandInstance");
const kGetContext = Symbol("getContext");
const kGetHasOutput = Symbol("getHasOutput");
const kGetLoggerInstance = Symbol("getLoggerInstance");
const kGetParseContext = Symbol("getParseContext");
const kGetUsageInstance = Symbol("getUsageInstance");
const kGetValidationInstance = Symbol("getValidationInstance");
const kHasParseCallback = Symbol("hasParseCallback");
const kIsGlobalContext = Symbol("isGlobalContext");
const kPostProcess = Symbol("postProcess");
const kRebase = Symbol("rebase");
const kReset = Symbol("reset");
const kRunYargsParserAndExecuteCommands = Symbol("runYargsParserAndExecuteCommands");
const kRunValidation = Symbol("runValidation");
const kSetHasOutput = Symbol("setHasOutput");
const kTrackManuallySetKeys = Symbol("kTrackManuallySetKeys");
const DEFAULT_LOCALE = "en_US";
var YargsInstance = class {
	constructor(processArgs = [], cwd, parentRequire, shim$2) {
		this.customScriptName = false;
		this.parsed = false;
		_YargsInstance_command.set(this, void 0);
		_YargsInstance_cwd.set(this, void 0);
		_YargsInstance_context.set(this, {
			commands: [],
			fullCommands: []
		});
		_YargsInstance_completion.set(this, null);
		_YargsInstance_completionCommand.set(this, null);
		_YargsInstance_defaultShowHiddenOpt.set(this, "show-hidden");
		_YargsInstance_exitError.set(this, null);
		_YargsInstance_detectLocale.set(this, true);
		_YargsInstance_emittedWarnings.set(this, {});
		_YargsInstance_exitProcess.set(this, true);
		_YargsInstance_frozens.set(this, []);
		_YargsInstance_globalMiddleware.set(this, void 0);
		_YargsInstance_groups.set(this, {});
		_YargsInstance_hasOutput.set(this, false);
		_YargsInstance_helpOpt.set(this, null);
		_YargsInstance_isGlobalContext.set(this, true);
		_YargsInstance_logger.set(this, void 0);
		_YargsInstance_output.set(this, "");
		_YargsInstance_options.set(this, void 0);
		_YargsInstance_parentRequire.set(this, void 0);
		_YargsInstance_parserConfig.set(this, {});
		_YargsInstance_parseFn.set(this, null);
		_YargsInstance_parseContext.set(this, null);
		_YargsInstance_pkgs.set(this, {});
		_YargsInstance_preservedGroups.set(this, {});
		_YargsInstance_processArgs.set(this, void 0);
		_YargsInstance_recommendCommands.set(this, false);
		_YargsInstance_shim.set(this, void 0);
		_YargsInstance_strict.set(this, false);
		_YargsInstance_strictCommands.set(this, false);
		_YargsInstance_strictOptions.set(this, false);
		_YargsInstance_usage.set(this, void 0);
		_YargsInstance_usageConfig.set(this, {});
		_YargsInstance_versionOpt.set(this, null);
		_YargsInstance_validation.set(this, void 0);
		__classPrivateFieldSet(this, _YargsInstance_shim, shim$2, "f");
		__classPrivateFieldSet(this, _YargsInstance_processArgs, processArgs, "f");
		__classPrivateFieldSet(this, _YargsInstance_cwd, cwd, "f");
		__classPrivateFieldSet(this, _YargsInstance_parentRequire, parentRequire, "f");
		__classPrivateFieldSet(this, _YargsInstance_globalMiddleware, new GlobalMiddleware(this), "f");
		this.$0 = this[kGetDollarZero]();
		this[kReset]();
		__classPrivateFieldSet(this, _YargsInstance_command, __classPrivateFieldGet(this, _YargsInstance_command, "f"), "f");
		__classPrivateFieldSet(this, _YargsInstance_usage, __classPrivateFieldGet(this, _YargsInstance_usage, "f"), "f");
		__classPrivateFieldSet(this, _YargsInstance_validation, __classPrivateFieldGet(this, _YargsInstance_validation, "f"), "f");
		__classPrivateFieldSet(this, _YargsInstance_options, __classPrivateFieldGet(this, _YargsInstance_options, "f"), "f");
		__classPrivateFieldGet(this, _YargsInstance_options, "f").showHiddenOpt = __classPrivateFieldGet(this, _YargsInstance_defaultShowHiddenOpt, "f");
		__classPrivateFieldSet(this, _YargsInstance_logger, this[kCreateLogger](), "f");
		__classPrivateFieldGet(this, _YargsInstance_shim, "f").y18n.setLocale(DEFAULT_LOCALE);
	}
	addHelpOpt(opt, msg) {
		const defaultHelpOpt = "help";
		argsert("[string|boolean] [string]", [opt, msg], arguments.length);
		if (__classPrivateFieldGet(this, _YargsInstance_helpOpt, "f")) {
			this[kDeleteFromParserHintObject](__classPrivateFieldGet(this, _YargsInstance_helpOpt, "f"));
			__classPrivateFieldSet(this, _YargsInstance_helpOpt, null, "f");
		}
		if (opt === false && msg === void 0) return this;
		__classPrivateFieldSet(this, _YargsInstance_helpOpt, typeof opt === "string" ? opt : defaultHelpOpt, "f");
		this.boolean(__classPrivateFieldGet(this, _YargsInstance_helpOpt, "f"));
		this.describe(__classPrivateFieldGet(this, _YargsInstance_helpOpt, "f"), msg || __classPrivateFieldGet(this, _YargsInstance_usage, "f").deferY18nLookup("Show help"));
		return this;
	}
	help(opt, msg) {
		return this.addHelpOpt(opt, msg);
	}
	addShowHiddenOpt(opt, msg) {
		argsert("[string|boolean] [string]", [opt, msg], arguments.length);
		if (opt === false && msg === void 0) return this;
		const showHiddenOpt = typeof opt === "string" ? opt : __classPrivateFieldGet(this, _YargsInstance_defaultShowHiddenOpt, "f");
		this.boolean(showHiddenOpt);
		this.describe(showHiddenOpt, msg || __classPrivateFieldGet(this, _YargsInstance_usage, "f").deferY18nLookup("Show hidden options"));
		__classPrivateFieldGet(this, _YargsInstance_options, "f").showHiddenOpt = showHiddenOpt;
		return this;
	}
	showHidden(opt, msg) {
		return this.addShowHiddenOpt(opt, msg);
	}
	alias(key, value) {
		argsert("<object|string|array> [string|array]", [key, value], arguments.length);
		this[kPopulateParserHintArrayDictionary](this.alias.bind(this), "alias", key, value);
		return this;
	}
	array(keys) {
		argsert("<array|string>", [keys], arguments.length);
		this[kPopulateParserHintArray]("array", keys);
		this[kTrackManuallySetKeys](keys);
		return this;
	}
	boolean(keys) {
		argsert("<array|string>", [keys], arguments.length);
		this[kPopulateParserHintArray]("boolean", keys);
		this[kTrackManuallySetKeys](keys);
		return this;
	}
	check(f, global$1) {
		argsert("<function> [boolean]", [f, global$1], arguments.length);
		this.middleware((argv$1, _yargs) => {
			return maybeAsyncResult(() => {
				return f(argv$1, _yargs.getOptions());
			}, (result) => {
				if (!result) __classPrivateFieldGet(this, _YargsInstance_usage, "f").fail(__classPrivateFieldGet(this, _YargsInstance_shim, "f").y18n.__("Argument check failed: %s", f.toString()));
				else if (typeof result === "string" || result instanceof Error) __classPrivateFieldGet(this, _YargsInstance_usage, "f").fail(result.toString(), result);
				return argv$1;
			}, (err) => {
				__classPrivateFieldGet(this, _YargsInstance_usage, "f").fail(err.message ? err.message : err.toString(), err);
				return argv$1;
			});
		}, false, global$1);
		return this;
	}
	choices(key, value) {
		argsert("<object|string|array> [string|array]", [key, value], arguments.length);
		this[kPopulateParserHintArrayDictionary](this.choices.bind(this), "choices", key, value);
		return this;
	}
	coerce(keys, value) {
		argsert("<object|string|array> [function]", [keys, value], arguments.length);
		if (Array.isArray(keys)) {
			if (!value) throw new YError("coerce callback must be provided");
			for (const key of keys) this.coerce(key, value);
			return this;
		} else if (typeof keys === "object") {
			for (const key of Object.keys(keys)) this.coerce(key, keys[key]);
			return this;
		}
		if (!value) throw new YError("coerce callback must be provided");
		const coerceKey = keys;
		__classPrivateFieldGet(this, _YargsInstance_options, "f").key[coerceKey] = true;
		__classPrivateFieldGet(this, _YargsInstance_globalMiddleware, "f").addCoerceMiddleware((argv$1, yargs) => {
			var _a$1;
			const argvKeys = [coerceKey, ...(_a$1 = yargs.getAliases()[coerceKey]) !== null && _a$1 !== void 0 ? _a$1 : []].filter((key) => Object.prototype.hasOwnProperty.call(argv$1, key));
			if (argvKeys.length === 0) return argv$1;
			return maybeAsyncResult(() => {
				return value(argv$1[argvKeys[0]]);
			}, (result) => {
				argvKeys.forEach((key) => {
					argv$1[key] = result;
				});
				return argv$1;
			}, (err) => {
				throw new YError(err.message);
			});
		}, coerceKey);
		return this;
	}
	conflicts(key1, key2) {
		argsert("<string|object> [string|array]", [key1, key2], arguments.length);
		__classPrivateFieldGet(this, _YargsInstance_validation, "f").conflicts(key1, key2);
		return this;
	}
	config(key = "config", msg, parseFn) {
		argsert("[object|string] [string|function] [function]", [
			key,
			msg,
			parseFn
		], arguments.length);
		if (typeof key === "object" && !Array.isArray(key)) {
			key = applyExtends(key, __classPrivateFieldGet(this, _YargsInstance_cwd, "f"), this[kGetParserConfiguration]()["deep-merge-config"] || false, __classPrivateFieldGet(this, _YargsInstance_shim, "f"));
			__classPrivateFieldGet(this, _YargsInstance_options, "f").configObjects = (__classPrivateFieldGet(this, _YargsInstance_options, "f").configObjects || []).concat(key);
			return this;
		}
		if (typeof msg === "function") {
			parseFn = msg;
			msg = void 0;
		}
		this.describe(key, msg || __classPrivateFieldGet(this, _YargsInstance_usage, "f").deferY18nLookup("Path to JSON config file"));
		(Array.isArray(key) ? key : [key]).forEach((k) => {
			__classPrivateFieldGet(this, _YargsInstance_options, "f").config[k] = parseFn || true;
		});
		return this;
	}
	completion(cmd, desc, fn) {
		argsert("[string] [string|boolean|function] [function]", [
			cmd,
			desc,
			fn
		], arguments.length);
		if (typeof desc === "function") {
			fn = desc;
			desc = void 0;
		}
		__classPrivateFieldSet(this, _YargsInstance_completionCommand, cmd || __classPrivateFieldGet(this, _YargsInstance_completionCommand, "f") || "completion", "f");
		if (!desc && desc !== false) desc = "generate completion script";
		this.command(__classPrivateFieldGet(this, _YargsInstance_completionCommand, "f"), desc);
		if (fn) __classPrivateFieldGet(this, _YargsInstance_completion, "f").registerFunction(fn);
		return this;
	}
	command(cmd, description, builder, handler, middlewares, deprecated) {
		argsert("<string|array|object> [string|boolean] [function|object] [function] [array] [boolean|string]", [
			cmd,
			description,
			builder,
			handler,
			middlewares,
			deprecated
		], arguments.length);
		__classPrivateFieldGet(this, _YargsInstance_command, "f").addHandler(cmd, description, builder, handler, middlewares, deprecated);
		return this;
	}
	commands(cmd, description, builder, handler, middlewares, deprecated) {
		return this.command(cmd, description, builder, handler, middlewares, deprecated);
	}
	commandDir(dir, opts) {
		argsert("<string> [object]", [dir, opts], arguments.length);
		const req = __classPrivateFieldGet(this, _YargsInstance_parentRequire, "f") || __classPrivateFieldGet(this, _YargsInstance_shim, "f").require;
		__classPrivateFieldGet(this, _YargsInstance_command, "f").addDirectory(dir, req, __classPrivateFieldGet(this, _YargsInstance_shim, "f").getCallerFile(), opts);
		return this;
	}
	count(keys) {
		argsert("<array|string>", [keys], arguments.length);
		this[kPopulateParserHintArray]("count", keys);
		this[kTrackManuallySetKeys](keys);
		return this;
	}
	default(key, value, defaultDescription) {
		argsert("<object|string|array> [*] [string]", [
			key,
			value,
			defaultDescription
		], arguments.length);
		if (defaultDescription) {
			assertSingleKey(key, __classPrivateFieldGet(this, _YargsInstance_shim, "f"));
			__classPrivateFieldGet(this, _YargsInstance_options, "f").defaultDescription[key] = defaultDescription;
		}
		if (typeof value === "function") {
			assertSingleKey(key, __classPrivateFieldGet(this, _YargsInstance_shim, "f"));
			if (!__classPrivateFieldGet(this, _YargsInstance_options, "f").defaultDescription[key]) __classPrivateFieldGet(this, _YargsInstance_options, "f").defaultDescription[key] = __classPrivateFieldGet(this, _YargsInstance_usage, "f").functionDescription(value);
			value = value.call();
		}
		this[kPopulateParserHintSingleValueDictionary](this.default.bind(this), "default", key, value);
		return this;
	}
	defaults(key, value, defaultDescription) {
		return this.default(key, value, defaultDescription);
	}
	demandCommand(min = 1, max, minMsg, maxMsg) {
		argsert("[number] [number|string] [string|null|undefined] [string|null|undefined]", [
			min,
			max,
			minMsg,
			maxMsg
		], arguments.length);
		if (typeof max !== "number") {
			minMsg = max;
			max = Infinity;
		}
		this.global("_", false);
		__classPrivateFieldGet(this, _YargsInstance_options, "f").demandedCommands._ = {
			min,
			max,
			minMsg,
			maxMsg
		};
		return this;
	}
	demand(keys, max, msg) {
		if (Array.isArray(max)) {
			max.forEach((key) => {
				assertNotStrictEqual(msg, true, __classPrivateFieldGet(this, _YargsInstance_shim, "f"));
				this.demandOption(key, msg);
			});
			max = Infinity;
		} else if (typeof max !== "number") {
			msg = max;
			max = Infinity;
		}
		if (typeof keys === "number") {
			assertNotStrictEqual(msg, true, __classPrivateFieldGet(this, _YargsInstance_shim, "f"));
			this.demandCommand(keys, max, msg, msg);
		} else if (Array.isArray(keys)) keys.forEach((key) => {
			assertNotStrictEqual(msg, true, __classPrivateFieldGet(this, _YargsInstance_shim, "f"));
			this.demandOption(key, msg);
		});
		else if (typeof msg === "string") this.demandOption(keys, msg);
		else if (msg === true || typeof msg === "undefined") this.demandOption(keys);
		return this;
	}
	demandOption(keys, msg) {
		argsert("<object|string|array> [string]", [keys, msg], arguments.length);
		this[kPopulateParserHintSingleValueDictionary](this.demandOption.bind(this), "demandedOptions", keys, msg);
		return this;
	}
	deprecateOption(option, message) {
		argsert("<string> [string|boolean]", [option, message], arguments.length);
		__classPrivateFieldGet(this, _YargsInstance_options, "f").deprecatedOptions[option] = message;
		return this;
	}
	describe(keys, description) {
		argsert("<object|string|array> [string]", [keys, description], arguments.length);
		this[kSetKey](keys, true);
		__classPrivateFieldGet(this, _YargsInstance_usage, "f").describe(keys, description);
		return this;
	}
	detectLocale(detect) {
		argsert("<boolean>", [detect], arguments.length);
		__classPrivateFieldSet(this, _YargsInstance_detectLocale, detect, "f");
		return this;
	}
	env(prefix) {
		argsert("[string|boolean]", [prefix], arguments.length);
		if (prefix === false) delete __classPrivateFieldGet(this, _YargsInstance_options, "f").envPrefix;
		else __classPrivateFieldGet(this, _YargsInstance_options, "f").envPrefix = prefix || "";
		return this;
	}
	epilogue(msg) {
		argsert("<string>", [msg], arguments.length);
		__classPrivateFieldGet(this, _YargsInstance_usage, "f").epilog(msg);
		return this;
	}
	epilog(msg) {
		return this.epilogue(msg);
	}
	example(cmd, description) {
		argsert("<string|array> [string]", [cmd, description], arguments.length);
		if (Array.isArray(cmd)) cmd.forEach((exampleParams) => this.example(...exampleParams));
		else __classPrivateFieldGet(this, _YargsInstance_usage, "f").example(cmd, description);
		return this;
	}
	exit(code, err) {
		__classPrivateFieldSet(this, _YargsInstance_hasOutput, true, "f");
		__classPrivateFieldSet(this, _YargsInstance_exitError, err, "f");
		if (__classPrivateFieldGet(this, _YargsInstance_exitProcess, "f")) __classPrivateFieldGet(this, _YargsInstance_shim, "f").process.exit(code);
	}
	exitProcess(enabled = true) {
		argsert("[boolean]", [enabled], arguments.length);
		__classPrivateFieldSet(this, _YargsInstance_exitProcess, enabled, "f");
		return this;
	}
	fail(f) {
		argsert("<function|boolean>", [f], arguments.length);
		if (typeof f === "boolean" && f !== false) throw new YError("Invalid first argument. Expected function or boolean 'false'");
		__classPrivateFieldGet(this, _YargsInstance_usage, "f").failFn(f);
		return this;
	}
	getAliases() {
		return this.parsed ? this.parsed.aliases : {};
	}
	async getCompletion(args, done) {
		argsert("<array> [function]", [args, done], arguments.length);
		if (!done) return new Promise((resolve$1, reject) => {
			__classPrivateFieldGet(this, _YargsInstance_completion, "f").getCompletion(args, (err, completions) => {
				if (err) reject(err);
				else resolve$1(completions);
			});
		});
		else return __classPrivateFieldGet(this, _YargsInstance_completion, "f").getCompletion(args, done);
	}
	getDemandedOptions() {
		argsert([], 0);
		return __classPrivateFieldGet(this, _YargsInstance_options, "f").demandedOptions;
	}
	getDemandedCommands() {
		argsert([], 0);
		return __classPrivateFieldGet(this, _YargsInstance_options, "f").demandedCommands;
	}
	getDeprecatedOptions() {
		argsert([], 0);
		return __classPrivateFieldGet(this, _YargsInstance_options, "f").deprecatedOptions;
	}
	getDetectLocale() {
		return __classPrivateFieldGet(this, _YargsInstance_detectLocale, "f");
	}
	getExitProcess() {
		return __classPrivateFieldGet(this, _YargsInstance_exitProcess, "f");
	}
	getGroups() {
		return Object.assign({}, __classPrivateFieldGet(this, _YargsInstance_groups, "f"), __classPrivateFieldGet(this, _YargsInstance_preservedGroups, "f"));
	}
	getHelp() {
		__classPrivateFieldSet(this, _YargsInstance_hasOutput, true, "f");
		if (!__classPrivateFieldGet(this, _YargsInstance_usage, "f").hasCachedHelpMessage()) {
			if (!this.parsed) {
				const parse = this[kRunYargsParserAndExecuteCommands](__classPrivateFieldGet(this, _YargsInstance_processArgs, "f"), void 0, void 0, 0, true);
				if (isPromise(parse)) return parse.then(() => {
					return __classPrivateFieldGet(this, _YargsInstance_usage, "f").help();
				});
			}
			const builderResponse = __classPrivateFieldGet(this, _YargsInstance_command, "f").runDefaultBuilderOn(this);
			if (isPromise(builderResponse)) return builderResponse.then(() => {
				return __classPrivateFieldGet(this, _YargsInstance_usage, "f").help();
			});
		}
		return Promise.resolve(__classPrivateFieldGet(this, _YargsInstance_usage, "f").help());
	}
	getOptions() {
		return __classPrivateFieldGet(this, _YargsInstance_options, "f");
	}
	getStrict() {
		return __classPrivateFieldGet(this, _YargsInstance_strict, "f");
	}
	getStrictCommands() {
		return __classPrivateFieldGet(this, _YargsInstance_strictCommands, "f");
	}
	getStrictOptions() {
		return __classPrivateFieldGet(this, _YargsInstance_strictOptions, "f");
	}
	global(globals, global$1) {
		argsert("<string|array> [boolean]", [globals, global$1], arguments.length);
		globals = [].concat(globals);
		if (global$1 !== false) __classPrivateFieldGet(this, _YargsInstance_options, "f").local = __classPrivateFieldGet(this, _YargsInstance_options, "f").local.filter((l) => globals.indexOf(l) === -1);
		else globals.forEach((g) => {
			if (!__classPrivateFieldGet(this, _YargsInstance_options, "f").local.includes(g)) __classPrivateFieldGet(this, _YargsInstance_options, "f").local.push(g);
		});
		return this;
	}
	group(opts, groupName) {
		argsert("<string|array> <string>", [opts, groupName], arguments.length);
		const existing = __classPrivateFieldGet(this, _YargsInstance_preservedGroups, "f")[groupName] || __classPrivateFieldGet(this, _YargsInstance_groups, "f")[groupName];
		if (__classPrivateFieldGet(this, _YargsInstance_preservedGroups, "f")[groupName]) delete __classPrivateFieldGet(this, _YargsInstance_preservedGroups, "f")[groupName];
		const seen = {};
		__classPrivateFieldGet(this, _YargsInstance_groups, "f")[groupName] = (existing || []).concat(opts).filter((key) => {
			if (seen[key]) return false;
			return seen[key] = true;
		});
		return this;
	}
	hide(key) {
		argsert("<string>", [key], arguments.length);
		__classPrivateFieldGet(this, _YargsInstance_options, "f").hiddenOptions.push(key);
		return this;
	}
	implies(key, value) {
		argsert("<string|object> [number|string|array]", [key, value], arguments.length);
		__classPrivateFieldGet(this, _YargsInstance_validation, "f").implies(key, value);
		return this;
	}
	locale(locale) {
		argsert("[string]", [locale], arguments.length);
		if (locale === void 0) {
			this[kGuessLocale]();
			return __classPrivateFieldGet(this, _YargsInstance_shim, "f").y18n.getLocale();
		}
		__classPrivateFieldSet(this, _YargsInstance_detectLocale, false, "f");
		__classPrivateFieldGet(this, _YargsInstance_shim, "f").y18n.setLocale(locale);
		return this;
	}
	middleware(callback, applyBeforeValidation, global$1) {
		return __classPrivateFieldGet(this, _YargsInstance_globalMiddleware, "f").addMiddleware(callback, !!applyBeforeValidation, global$1);
	}
	nargs(key, value) {
		argsert("<string|object|array> [number]", [key, value], arguments.length);
		this[kPopulateParserHintSingleValueDictionary](this.nargs.bind(this), "narg", key, value);
		return this;
	}
	normalize(keys) {
		argsert("<array|string>", [keys], arguments.length);
		this[kPopulateParserHintArray]("normalize", keys);
		return this;
	}
	number(keys) {
		argsert("<array|string>", [keys], arguments.length);
		this[kPopulateParserHintArray]("number", keys);
		this[kTrackManuallySetKeys](keys);
		return this;
	}
	option(key, opt) {
		argsert("<string|object> [object]", [key, opt], arguments.length);
		if (typeof key === "object") Object.keys(key).forEach((k) => {
			this.options(k, key[k]);
		});
		else {
			if (typeof opt !== "object") opt = {};
			this[kTrackManuallySetKeys](key);
			if (__classPrivateFieldGet(this, _YargsInstance_versionOpt, "f") && (key === "version" || (opt === null || opt === void 0 ? void 0 : opt.alias) === "version")) this[kEmitWarning]([
				"\"version\" is a reserved word.",
				"Please do one of the following:",
				"- Disable version with `yargs.version(false)` if using \"version\" as an option",
				"- Use the built-in `yargs.version` method instead (if applicable)",
				"- Use a different option key",
				"https://yargs.js.org/docs/#api-reference-version"
			].join("\n"), void 0, "versionWarning");
			__classPrivateFieldGet(this, _YargsInstance_options, "f").key[key] = true;
			if (opt.alias) this.alias(key, opt.alias);
			const deprecate = opt.deprecate || opt.deprecated;
			if (deprecate) this.deprecateOption(key, deprecate);
			const demand = opt.demand || opt.required || opt.require;
			if (demand) this.demand(key, demand);
			if (opt.demandOption) this.demandOption(key, typeof opt.demandOption === "string" ? opt.demandOption : void 0);
			if (opt.conflicts) this.conflicts(key, opt.conflicts);
			if ("default" in opt) this.default(key, opt.default);
			if (opt.implies !== void 0) this.implies(key, opt.implies);
			if (opt.nargs !== void 0) this.nargs(key, opt.nargs);
			if (opt.config) this.config(key, opt.configParser);
			if (opt.normalize) this.normalize(key);
			if (opt.choices) this.choices(key, opt.choices);
			if (opt.coerce) this.coerce(key, opt.coerce);
			if (opt.group) this.group(key, opt.group);
			if (opt.boolean || opt.type === "boolean") {
				this.boolean(key);
				if (opt.alias) this.boolean(opt.alias);
			}
			if (opt.array || opt.type === "array") {
				this.array(key);
				if (opt.alias) this.array(opt.alias);
			}
			if (opt.number || opt.type === "number") {
				this.number(key);
				if (opt.alias) this.number(opt.alias);
			}
			if (opt.string || opt.type === "string") {
				this.string(key);
				if (opt.alias) this.string(opt.alias);
			}
			if (opt.count || opt.type === "count") this.count(key);
			if (typeof opt.global === "boolean") this.global(key, opt.global);
			if (opt.defaultDescription) __classPrivateFieldGet(this, _YargsInstance_options, "f").defaultDescription[key] = opt.defaultDescription;
			if (opt.skipValidation) this.skipValidation(key);
			const desc = opt.describe || opt.description || opt.desc;
			const descriptions = __classPrivateFieldGet(this, _YargsInstance_usage, "f").getDescriptions();
			if (!Object.prototype.hasOwnProperty.call(descriptions, key) || typeof desc === "string") this.describe(key, desc);
			if (opt.hidden) this.hide(key);
			if (opt.requiresArg) this.requiresArg(key);
		}
		return this;
	}
	options(key, opt) {
		return this.option(key, opt);
	}
	parse(args, shortCircuit, _parseFn) {
		argsert("[string|array] [function|boolean|object] [function]", [
			args,
			shortCircuit,
			_parseFn
		], arguments.length);
		this[kFreeze]();
		if (typeof args === "undefined") args = __classPrivateFieldGet(this, _YargsInstance_processArgs, "f");
		if (typeof shortCircuit === "object") {
			__classPrivateFieldSet(this, _YargsInstance_parseContext, shortCircuit, "f");
			shortCircuit = _parseFn;
		}
		if (typeof shortCircuit === "function") {
			__classPrivateFieldSet(this, _YargsInstance_parseFn, shortCircuit, "f");
			shortCircuit = false;
		}
		if (!shortCircuit) __classPrivateFieldSet(this, _YargsInstance_processArgs, args, "f");
		if (__classPrivateFieldGet(this, _YargsInstance_parseFn, "f")) __classPrivateFieldSet(this, _YargsInstance_exitProcess, false, "f");
		const parsed = this[kRunYargsParserAndExecuteCommands](args, !!shortCircuit);
		const tmpParsed = this.parsed;
		__classPrivateFieldGet(this, _YargsInstance_completion, "f").setParsed(this.parsed);
		if (isPromise(parsed)) return parsed.then((argv$1) => {
			if (__classPrivateFieldGet(this, _YargsInstance_parseFn, "f")) __classPrivateFieldGet(this, _YargsInstance_parseFn, "f").call(this, __classPrivateFieldGet(this, _YargsInstance_exitError, "f"), argv$1, __classPrivateFieldGet(this, _YargsInstance_output, "f"));
			return argv$1;
		}).catch((err) => {
			if (__classPrivateFieldGet(this, _YargsInstance_parseFn, "f")) __classPrivateFieldGet(this, _YargsInstance_parseFn, "f")(err, this.parsed.argv, __classPrivateFieldGet(this, _YargsInstance_output, "f"));
			throw err;
		}).finally(() => {
			this[kUnfreeze]();
			this.parsed = tmpParsed;
		});
		else {
			if (__classPrivateFieldGet(this, _YargsInstance_parseFn, "f")) __classPrivateFieldGet(this, _YargsInstance_parseFn, "f").call(this, __classPrivateFieldGet(this, _YargsInstance_exitError, "f"), parsed, __classPrivateFieldGet(this, _YargsInstance_output, "f"));
			this[kUnfreeze]();
			this.parsed = tmpParsed;
		}
		return parsed;
	}
	parseAsync(args, shortCircuit, _parseFn) {
		const maybePromise = this.parse(args, shortCircuit, _parseFn);
		return !isPromise(maybePromise) ? Promise.resolve(maybePromise) : maybePromise;
	}
	parseSync(args, shortCircuit, _parseFn) {
		const maybePromise = this.parse(args, shortCircuit, _parseFn);
		if (isPromise(maybePromise)) throw new YError(".parseSync() must not be used with asynchronous builders, handlers, or middleware");
		return maybePromise;
	}
	parserConfiguration(config) {
		argsert("<object>", [config], arguments.length);
		__classPrivateFieldSet(this, _YargsInstance_parserConfig, config, "f");
		return this;
	}
	pkgConf(key, rootPath) {
		argsert("<string> [string]", [key, rootPath], arguments.length);
		let conf = null;
		const obj = this[kPkgUp](rootPath || __classPrivateFieldGet(this, _YargsInstance_cwd, "f"));
		if (obj[key] && typeof obj[key] === "object") {
			conf = applyExtends(obj[key], rootPath || __classPrivateFieldGet(this, _YargsInstance_cwd, "f"), this[kGetParserConfiguration]()["deep-merge-config"] || false, __classPrivateFieldGet(this, _YargsInstance_shim, "f"));
			__classPrivateFieldGet(this, _YargsInstance_options, "f").configObjects = (__classPrivateFieldGet(this, _YargsInstance_options, "f").configObjects || []).concat(conf);
		}
		return this;
	}
	positional(key, opts) {
		argsert("<string> <object>", [key, opts], arguments.length);
		const supportedOpts = [
			"default",
			"defaultDescription",
			"implies",
			"normalize",
			"choices",
			"conflicts",
			"coerce",
			"type",
			"describe",
			"desc",
			"description",
			"alias"
		];
		opts = objFilter(opts, (k, v) => {
			if (k === "type" && ![
				"string",
				"number",
				"boolean"
			].includes(v)) return false;
			return supportedOpts.includes(k);
		});
		const fullCommand = __classPrivateFieldGet(this, _YargsInstance_context, "f").fullCommands[__classPrivateFieldGet(this, _YargsInstance_context, "f").fullCommands.length - 1];
		const parseOptions = fullCommand ? __classPrivateFieldGet(this, _YargsInstance_command, "f").cmdToParseOptions(fullCommand) : {
			array: [],
			alias: {},
			default: {},
			demand: {}
		};
		objectKeys(parseOptions).forEach((pk) => {
			const parseOption = parseOptions[pk];
			if (Array.isArray(parseOption)) {
				if (parseOption.indexOf(key) !== -1) opts[pk] = true;
			} else if (parseOption[key] && !(pk in opts)) opts[pk] = parseOption[key];
		});
		this.group(key, __classPrivateFieldGet(this, _YargsInstance_usage, "f").getPositionalGroupName());
		return this.option(key, opts);
	}
	recommendCommands(recommend = true) {
		argsert("[boolean]", [recommend], arguments.length);
		__classPrivateFieldSet(this, _YargsInstance_recommendCommands, recommend, "f");
		return this;
	}
	required(keys, max, msg) {
		return this.demand(keys, max, msg);
	}
	require(keys, max, msg) {
		return this.demand(keys, max, msg);
	}
	requiresArg(keys) {
		argsert("<array|string|object> [number]", [keys], arguments.length);
		if (typeof keys === "string" && __classPrivateFieldGet(this, _YargsInstance_options, "f").narg[keys]) return this;
		else this[kPopulateParserHintSingleValueDictionary](this.requiresArg.bind(this), "narg", keys, NaN);
		return this;
	}
	showCompletionScript($0, cmd) {
		argsert("[string] [string]", [$0, cmd], arguments.length);
		$0 = $0 || this.$0;
		__classPrivateFieldGet(this, _YargsInstance_logger, "f").log(__classPrivateFieldGet(this, _YargsInstance_completion, "f").generateCompletionScript($0, cmd || __classPrivateFieldGet(this, _YargsInstance_completionCommand, "f") || "completion"));
		return this;
	}
	showHelp(level) {
		argsert("[string|function]", [level], arguments.length);
		__classPrivateFieldSet(this, _YargsInstance_hasOutput, true, "f");
		if (!__classPrivateFieldGet(this, _YargsInstance_usage, "f").hasCachedHelpMessage()) {
			if (!this.parsed) {
				const parse = this[kRunYargsParserAndExecuteCommands](__classPrivateFieldGet(this, _YargsInstance_processArgs, "f"), void 0, void 0, 0, true);
				if (isPromise(parse)) {
					parse.then(() => {
						__classPrivateFieldGet(this, _YargsInstance_usage, "f").showHelp(level);
					});
					return this;
				}
			}
			const builderResponse = __classPrivateFieldGet(this, _YargsInstance_command, "f").runDefaultBuilderOn(this);
			if (isPromise(builderResponse)) {
				builderResponse.then(() => {
					__classPrivateFieldGet(this, _YargsInstance_usage, "f").showHelp(level);
				});
				return this;
			}
		}
		__classPrivateFieldGet(this, _YargsInstance_usage, "f").showHelp(level);
		return this;
	}
	scriptName(scriptName) {
		this.customScriptName = true;
		this.$0 = scriptName;
		return this;
	}
	showHelpOnFail(enabled, message) {
		argsert("[boolean|string] [string]", [enabled, message], arguments.length);
		__classPrivateFieldGet(this, _YargsInstance_usage, "f").showHelpOnFail(enabled, message);
		return this;
	}
	showVersion(level) {
		argsert("[string|function]", [level], arguments.length);
		__classPrivateFieldGet(this, _YargsInstance_usage, "f").showVersion(level);
		return this;
	}
	skipValidation(keys) {
		argsert("<array|string>", [keys], arguments.length);
		this[kPopulateParserHintArray]("skipValidation", keys);
		return this;
	}
	strict(enabled) {
		argsert("[boolean]", [enabled], arguments.length);
		__classPrivateFieldSet(this, _YargsInstance_strict, enabled !== false, "f");
		return this;
	}
	strictCommands(enabled) {
		argsert("[boolean]", [enabled], arguments.length);
		__classPrivateFieldSet(this, _YargsInstance_strictCommands, enabled !== false, "f");
		return this;
	}
	strictOptions(enabled) {
		argsert("[boolean]", [enabled], arguments.length);
		__classPrivateFieldSet(this, _YargsInstance_strictOptions, enabled !== false, "f");
		return this;
	}
	string(keys) {
		argsert("<array|string>", [keys], arguments.length);
		this[kPopulateParserHintArray]("string", keys);
		this[kTrackManuallySetKeys](keys);
		return this;
	}
	terminalWidth() {
		argsert([], 0);
		return __classPrivateFieldGet(this, _YargsInstance_shim, "f").process.stdColumns;
	}
	updateLocale(obj) {
		return this.updateStrings(obj);
	}
	updateStrings(obj) {
		argsert("<object>", [obj], arguments.length);
		__classPrivateFieldSet(this, _YargsInstance_detectLocale, false, "f");
		__classPrivateFieldGet(this, _YargsInstance_shim, "f").y18n.updateLocale(obj);
		return this;
	}
	usage(msg, description, builder, handler) {
		argsert("<string|null|undefined> [string|boolean] [function|object] [function]", [
			msg,
			description,
			builder,
			handler
		], arguments.length);
		if (description !== void 0) {
			assertNotStrictEqual(msg, null, __classPrivateFieldGet(this, _YargsInstance_shim, "f"));
			if ((msg || "").match(/^\$0( |$)/)) return this.command(msg, description, builder, handler);
			else throw new YError(".usage() description must start with $0 if being used as alias for .command()");
		} else {
			__classPrivateFieldGet(this, _YargsInstance_usage, "f").usage(msg);
			return this;
		}
	}
	usageConfiguration(config) {
		argsert("<object>", [config], arguments.length);
		__classPrivateFieldSet(this, _YargsInstance_usageConfig, config, "f");
		return this;
	}
	version(opt, msg, ver) {
		const defaultVersionOpt = "version";
		argsert("[boolean|string] [string] [string]", [
			opt,
			msg,
			ver
		], arguments.length);
		if (__classPrivateFieldGet(this, _YargsInstance_versionOpt, "f")) {
			this[kDeleteFromParserHintObject](__classPrivateFieldGet(this, _YargsInstance_versionOpt, "f"));
			__classPrivateFieldGet(this, _YargsInstance_usage, "f").version(void 0);
			__classPrivateFieldSet(this, _YargsInstance_versionOpt, null, "f");
		}
		if (arguments.length === 0) {
			ver = this[kGuessVersion]();
			opt = defaultVersionOpt;
		} else if (arguments.length === 1) {
			if (opt === false) return this;
			ver = opt;
			opt = defaultVersionOpt;
		} else if (arguments.length === 2) {
			ver = msg;
			msg = void 0;
		}
		__classPrivateFieldSet(this, _YargsInstance_versionOpt, typeof opt === "string" ? opt : defaultVersionOpt, "f");
		msg = msg || __classPrivateFieldGet(this, _YargsInstance_usage, "f").deferY18nLookup("Show version number");
		__classPrivateFieldGet(this, _YargsInstance_usage, "f").version(ver || void 0);
		this.boolean(__classPrivateFieldGet(this, _YargsInstance_versionOpt, "f"));
		this.describe(__classPrivateFieldGet(this, _YargsInstance_versionOpt, "f"), msg);
		return this;
	}
	wrap(cols) {
		argsert("<number|null|undefined>", [cols], arguments.length);
		__classPrivateFieldGet(this, _YargsInstance_usage, "f").wrap(cols);
		return this;
	}
	[(_YargsInstance_command = /* @__PURE__ */ new WeakMap(), _YargsInstance_cwd = /* @__PURE__ */ new WeakMap(), _YargsInstance_context = /* @__PURE__ */ new WeakMap(), _YargsInstance_completion = /* @__PURE__ */ new WeakMap(), _YargsInstance_completionCommand = /* @__PURE__ */ new WeakMap(), _YargsInstance_defaultShowHiddenOpt = /* @__PURE__ */ new WeakMap(), _YargsInstance_exitError = /* @__PURE__ */ new WeakMap(), _YargsInstance_detectLocale = /* @__PURE__ */ new WeakMap(), _YargsInstance_emittedWarnings = /* @__PURE__ */ new WeakMap(), _YargsInstance_exitProcess = /* @__PURE__ */ new WeakMap(), _YargsInstance_frozens = /* @__PURE__ */ new WeakMap(), _YargsInstance_globalMiddleware = /* @__PURE__ */ new WeakMap(), _YargsInstance_groups = /* @__PURE__ */ new WeakMap(), _YargsInstance_hasOutput = /* @__PURE__ */ new WeakMap(), _YargsInstance_helpOpt = /* @__PURE__ */ new WeakMap(), _YargsInstance_isGlobalContext = /* @__PURE__ */ new WeakMap(), _YargsInstance_logger = /* @__PURE__ */ new WeakMap(), _YargsInstance_output = /* @__PURE__ */ new WeakMap(), _YargsInstance_options = /* @__PURE__ */ new WeakMap(), _YargsInstance_parentRequire = /* @__PURE__ */ new WeakMap(), _YargsInstance_parserConfig = /* @__PURE__ */ new WeakMap(), _YargsInstance_parseFn = /* @__PURE__ */ new WeakMap(), _YargsInstance_parseContext = /* @__PURE__ */ new WeakMap(), _YargsInstance_pkgs = /* @__PURE__ */ new WeakMap(), _YargsInstance_preservedGroups = /* @__PURE__ */ new WeakMap(), _YargsInstance_processArgs = /* @__PURE__ */ new WeakMap(), _YargsInstance_recommendCommands = /* @__PURE__ */ new WeakMap(), _YargsInstance_shim = /* @__PURE__ */ new WeakMap(), _YargsInstance_strict = /* @__PURE__ */ new WeakMap(), _YargsInstance_strictCommands = /* @__PURE__ */ new WeakMap(), _YargsInstance_strictOptions = /* @__PURE__ */ new WeakMap(), _YargsInstance_usage = /* @__PURE__ */ new WeakMap(), _YargsInstance_usageConfig = /* @__PURE__ */ new WeakMap(), _YargsInstance_versionOpt = /* @__PURE__ */ new WeakMap(), _YargsInstance_validation = /* @__PURE__ */ new WeakMap(), kCopyDoubleDash)](argv$1) {
		if (!argv$1._ || !argv$1["--"]) return argv$1;
		argv$1._.push.apply(argv$1._, argv$1["--"]);
		try {
			delete argv$1["--"];
		} catch (_err) {}
		return argv$1;
	}
	[kCreateLogger]() {
		return {
			log: (...args) => {
				if (!this[kHasParseCallback]()) console.log(...args);
				__classPrivateFieldSet(this, _YargsInstance_hasOutput, true, "f");
				if (__classPrivateFieldGet(this, _YargsInstance_output, "f").length) __classPrivateFieldSet(this, _YargsInstance_output, __classPrivateFieldGet(this, _YargsInstance_output, "f") + "\n", "f");
				__classPrivateFieldSet(this, _YargsInstance_output, __classPrivateFieldGet(this, _YargsInstance_output, "f") + args.join(" "), "f");
			},
			error: (...args) => {
				if (!this[kHasParseCallback]()) console.error(...args);
				__classPrivateFieldSet(this, _YargsInstance_hasOutput, true, "f");
				if (__classPrivateFieldGet(this, _YargsInstance_output, "f").length) __classPrivateFieldSet(this, _YargsInstance_output, __classPrivateFieldGet(this, _YargsInstance_output, "f") + "\n", "f");
				__classPrivateFieldSet(this, _YargsInstance_output, __classPrivateFieldGet(this, _YargsInstance_output, "f") + args.join(" "), "f");
			}
		};
	}
	[kDeleteFromParserHintObject](optionKey) {
		objectKeys(__classPrivateFieldGet(this, _YargsInstance_options, "f")).forEach((hintKey) => {
			if (((key) => key === "configObjects")(hintKey)) return;
			const hint = __classPrivateFieldGet(this, _YargsInstance_options, "f")[hintKey];
			if (Array.isArray(hint)) {
				if (hint.includes(optionKey)) hint.splice(hint.indexOf(optionKey), 1);
			} else if (typeof hint === "object") delete hint[optionKey];
		});
		delete __classPrivateFieldGet(this, _YargsInstance_usage, "f").getDescriptions()[optionKey];
	}
	[kEmitWarning](warning, type, deduplicationId) {
		if (!__classPrivateFieldGet(this, _YargsInstance_emittedWarnings, "f")[deduplicationId]) {
			__classPrivateFieldGet(this, _YargsInstance_shim, "f").process.emitWarning(warning, type);
			__classPrivateFieldGet(this, _YargsInstance_emittedWarnings, "f")[deduplicationId] = true;
		}
	}
	[kFreeze]() {
		__classPrivateFieldGet(this, _YargsInstance_frozens, "f").push({
			options: __classPrivateFieldGet(this, _YargsInstance_options, "f"),
			configObjects: __classPrivateFieldGet(this, _YargsInstance_options, "f").configObjects.slice(0),
			exitProcess: __classPrivateFieldGet(this, _YargsInstance_exitProcess, "f"),
			groups: __classPrivateFieldGet(this, _YargsInstance_groups, "f"),
			strict: __classPrivateFieldGet(this, _YargsInstance_strict, "f"),
			strictCommands: __classPrivateFieldGet(this, _YargsInstance_strictCommands, "f"),
			strictOptions: __classPrivateFieldGet(this, _YargsInstance_strictOptions, "f"),
			completionCommand: __classPrivateFieldGet(this, _YargsInstance_completionCommand, "f"),
			output: __classPrivateFieldGet(this, _YargsInstance_output, "f"),
			exitError: __classPrivateFieldGet(this, _YargsInstance_exitError, "f"),
			hasOutput: __classPrivateFieldGet(this, _YargsInstance_hasOutput, "f"),
			parsed: this.parsed,
			parseFn: __classPrivateFieldGet(this, _YargsInstance_parseFn, "f"),
			parseContext: __classPrivateFieldGet(this, _YargsInstance_parseContext, "f")
		});
		__classPrivateFieldGet(this, _YargsInstance_usage, "f").freeze();
		__classPrivateFieldGet(this, _YargsInstance_validation, "f").freeze();
		__classPrivateFieldGet(this, _YargsInstance_command, "f").freeze();
		__classPrivateFieldGet(this, _YargsInstance_globalMiddleware, "f").freeze();
	}
	[kGetDollarZero]() {
		let $0 = "";
		let default$0;
		if (/\b(node|iojs|electron)(\.exe)?$/.test(__classPrivateFieldGet(this, _YargsInstance_shim, "f").process.argv()[0])) default$0 = __classPrivateFieldGet(this, _YargsInstance_shim, "f").process.argv().slice(1, 2);
		else default$0 = __classPrivateFieldGet(this, _YargsInstance_shim, "f").process.argv().slice(0, 1);
		$0 = default$0.map((x) => {
			const b = this[kRebase](__classPrivateFieldGet(this, _YargsInstance_cwd, "f"), x);
			return x.match(/^(\/|([a-zA-Z]:)?\\)/) && b.length < x.length ? b : x;
		}).join(" ").trim();
		if (__classPrivateFieldGet(this, _YargsInstance_shim, "f").getEnv("_") && __classPrivateFieldGet(this, _YargsInstance_shim, "f").getProcessArgvBin() === __classPrivateFieldGet(this, _YargsInstance_shim, "f").getEnv("_")) $0 = __classPrivateFieldGet(this, _YargsInstance_shim, "f").getEnv("_").replace(`${__classPrivateFieldGet(this, _YargsInstance_shim, "f").path.dirname(__classPrivateFieldGet(this, _YargsInstance_shim, "f").process.execPath())}/`, "");
		return $0;
	}
	[kGetParserConfiguration]() {
		return __classPrivateFieldGet(this, _YargsInstance_parserConfig, "f");
	}
	[kGetUsageConfiguration]() {
		return __classPrivateFieldGet(this, _YargsInstance_usageConfig, "f");
	}
	[kGuessLocale]() {
		if (!__classPrivateFieldGet(this, _YargsInstance_detectLocale, "f")) return;
		const locale = __classPrivateFieldGet(this, _YargsInstance_shim, "f").getEnv("LC_ALL") || __classPrivateFieldGet(this, _YargsInstance_shim, "f").getEnv("LC_MESSAGES") || __classPrivateFieldGet(this, _YargsInstance_shim, "f").getEnv("LANG") || __classPrivateFieldGet(this, _YargsInstance_shim, "f").getEnv("LANGUAGE") || "en_US";
		this.locale(locale.replace(/[.:].*/, ""));
	}
	[kGuessVersion]() {
		return this[kPkgUp]().version || "unknown";
	}
	[kParsePositionalNumbers](argv$1) {
		const args = argv$1["--"] ? argv$1["--"] : argv$1._;
		for (let i = 0, arg; (arg = args[i]) !== void 0; i++) if (__classPrivateFieldGet(this, _YargsInstance_shim, "f").Parser.looksLikeNumber(arg) && Number.isSafeInteger(Math.floor(parseFloat(`${arg}`)))) args[i] = Number(arg);
		return argv$1;
	}
	[kPkgUp](rootPath) {
		const npath = rootPath || "*";
		if (__classPrivateFieldGet(this, _YargsInstance_pkgs, "f")[npath]) return __classPrivateFieldGet(this, _YargsInstance_pkgs, "f")[npath];
		let obj = {};
		try {
			let startDir = rootPath || __classPrivateFieldGet(this, _YargsInstance_shim, "f").mainFilename;
			if (__classPrivateFieldGet(this, _YargsInstance_shim, "f").path.extname(startDir)) startDir = __classPrivateFieldGet(this, _YargsInstance_shim, "f").path.dirname(startDir);
			const pkgJsonPath = __classPrivateFieldGet(this, _YargsInstance_shim, "f").findUp(startDir, (dir, names) => {
				if (names.includes("package.json")) return "package.json";
				else return;
			});
			assertNotStrictEqual(pkgJsonPath, void 0, __classPrivateFieldGet(this, _YargsInstance_shim, "f"));
			obj = JSON.parse(__classPrivateFieldGet(this, _YargsInstance_shim, "f").readFileSync(pkgJsonPath, "utf8"));
		} catch (_noop) {}
		__classPrivateFieldGet(this, _YargsInstance_pkgs, "f")[npath] = obj || {};
		return __classPrivateFieldGet(this, _YargsInstance_pkgs, "f")[npath];
	}
	[kPopulateParserHintArray](type, keys) {
		keys = [].concat(keys);
		keys.forEach((key) => {
			key = this[kSanitizeKey](key);
			__classPrivateFieldGet(this, _YargsInstance_options, "f")[type].push(key);
		});
	}
	[kPopulateParserHintSingleValueDictionary](builder, type, key, value) {
		this[kPopulateParserHintDictionary](builder, type, key, value, (type$1, key$1, value$1) => {
			__classPrivateFieldGet(this, _YargsInstance_options, "f")[type$1][key$1] = value$1;
		});
	}
	[kPopulateParserHintArrayDictionary](builder, type, key, value) {
		this[kPopulateParserHintDictionary](builder, type, key, value, (type$1, key$1, value$1) => {
			__classPrivateFieldGet(this, _YargsInstance_options, "f")[type$1][key$1] = (__classPrivateFieldGet(this, _YargsInstance_options, "f")[type$1][key$1] || []).concat(value$1);
		});
	}
	[kPopulateParserHintDictionary](builder, type, key, value, singleKeyHandler) {
		if (Array.isArray(key)) key.forEach((k) => {
			builder(k, value);
		});
		else if (((key$1) => typeof key$1 === "object")(key)) for (const k of objectKeys(key)) builder(k, key[k]);
		else singleKeyHandler(type, this[kSanitizeKey](key), value);
	}
	[kSanitizeKey](key) {
		if (key === "__proto__") return "___proto___";
		return key;
	}
	[kSetKey](key, set) {
		this[kPopulateParserHintSingleValueDictionary](this[kSetKey].bind(this), "key", key, set);
		return this;
	}
	[kUnfreeze]() {
		var _a$1, _b$1, _c$1, _d, _e, _f, _g, _h, _j, _k, _l, _m;
		const frozen = __classPrivateFieldGet(this, _YargsInstance_frozens, "f").pop();
		assertNotStrictEqual(frozen, void 0, __classPrivateFieldGet(this, _YargsInstance_shim, "f"));
		let configObjects;
		_a$1 = this, _b$1 = this, _c$1 = this, _d = this, _e = this, _f = this, _g = this, _h = this, _j = this, _k = this, _l = this, _m = this, {options: { set value(_o) {
			__classPrivateFieldSet(_a$1, _YargsInstance_options, _o, "f");
		} }.value, configObjects, exitProcess: { set value(_o) {
			__classPrivateFieldSet(_b$1, _YargsInstance_exitProcess, _o, "f");
		} }.value, groups: { set value(_o) {
			__classPrivateFieldSet(_c$1, _YargsInstance_groups, _o, "f");
		} }.value, output: { set value(_o) {
			__classPrivateFieldSet(_d, _YargsInstance_output, _o, "f");
		} }.value, exitError: { set value(_o) {
			__classPrivateFieldSet(_e, _YargsInstance_exitError, _o, "f");
		} }.value, hasOutput: { set value(_o) {
			__classPrivateFieldSet(_f, _YargsInstance_hasOutput, _o, "f");
		} }.value, parsed: this.parsed, strict: { set value(_o) {
			__classPrivateFieldSet(_g, _YargsInstance_strict, _o, "f");
		} }.value, strictCommands: { set value(_o) {
			__classPrivateFieldSet(_h, _YargsInstance_strictCommands, _o, "f");
		} }.value, strictOptions: { set value(_o) {
			__classPrivateFieldSet(_j, _YargsInstance_strictOptions, _o, "f");
		} }.value, completionCommand: { set value(_o) {
			__classPrivateFieldSet(_k, _YargsInstance_completionCommand, _o, "f");
		} }.value, parseFn: { set value(_o) {
			__classPrivateFieldSet(_l, _YargsInstance_parseFn, _o, "f");
		} }.value, parseContext: { set value(_o) {
			__classPrivateFieldSet(_m, _YargsInstance_parseContext, _o, "f");
		} }.value} = frozen;
		__classPrivateFieldGet(this, _YargsInstance_options, "f").configObjects = configObjects;
		__classPrivateFieldGet(this, _YargsInstance_usage, "f").unfreeze();
		__classPrivateFieldGet(this, _YargsInstance_validation, "f").unfreeze();
		__classPrivateFieldGet(this, _YargsInstance_command, "f").unfreeze();
		__classPrivateFieldGet(this, _YargsInstance_globalMiddleware, "f").unfreeze();
	}
	[kValidateAsync](validation$1, argv$1) {
		return maybeAsyncResult(argv$1, (result) => {
			validation$1(result);
			return result;
		});
	}
	getInternalMethods() {
		return {
			getCommandInstance: this[kGetCommandInstance].bind(this),
			getContext: this[kGetContext].bind(this),
			getHasOutput: this[kGetHasOutput].bind(this),
			getLoggerInstance: this[kGetLoggerInstance].bind(this),
			getParseContext: this[kGetParseContext].bind(this),
			getParserConfiguration: this[kGetParserConfiguration].bind(this),
			getUsageConfiguration: this[kGetUsageConfiguration].bind(this),
			getUsageInstance: this[kGetUsageInstance].bind(this),
			getValidationInstance: this[kGetValidationInstance].bind(this),
			hasParseCallback: this[kHasParseCallback].bind(this),
			isGlobalContext: this[kIsGlobalContext].bind(this),
			postProcess: this[kPostProcess].bind(this),
			reset: this[kReset].bind(this),
			runValidation: this[kRunValidation].bind(this),
			runYargsParserAndExecuteCommands: this[kRunYargsParserAndExecuteCommands].bind(this),
			setHasOutput: this[kSetHasOutput].bind(this)
		};
	}
	[kGetCommandInstance]() {
		return __classPrivateFieldGet(this, _YargsInstance_command, "f");
	}
	[kGetContext]() {
		return __classPrivateFieldGet(this, _YargsInstance_context, "f");
	}
	[kGetHasOutput]() {
		return __classPrivateFieldGet(this, _YargsInstance_hasOutput, "f");
	}
	[kGetLoggerInstance]() {
		return __classPrivateFieldGet(this, _YargsInstance_logger, "f");
	}
	[kGetParseContext]() {
		return __classPrivateFieldGet(this, _YargsInstance_parseContext, "f") || {};
	}
	[kGetUsageInstance]() {
		return __classPrivateFieldGet(this, _YargsInstance_usage, "f");
	}
	[kGetValidationInstance]() {
		return __classPrivateFieldGet(this, _YargsInstance_validation, "f");
	}
	[kHasParseCallback]() {
		return !!__classPrivateFieldGet(this, _YargsInstance_parseFn, "f");
	}
	[kIsGlobalContext]() {
		return __classPrivateFieldGet(this, _YargsInstance_isGlobalContext, "f");
	}
	[kPostProcess](argv$1, populateDoubleDash, calledFromCommand, runGlobalMiddleware) {
		if (calledFromCommand) return argv$1;
		if (isPromise(argv$1)) return argv$1;
		if (!populateDoubleDash) argv$1 = this[kCopyDoubleDash](argv$1);
		if (this[kGetParserConfiguration]()["parse-positional-numbers"] || this[kGetParserConfiguration]()["parse-positional-numbers"] === void 0) argv$1 = this[kParsePositionalNumbers](argv$1);
		if (runGlobalMiddleware) argv$1 = applyMiddleware(argv$1, this, __classPrivateFieldGet(this, _YargsInstance_globalMiddleware, "f").getMiddleware(), false);
		return argv$1;
	}
	[kReset](aliases = {}) {
		__classPrivateFieldSet(this, _YargsInstance_options, __classPrivateFieldGet(this, _YargsInstance_options, "f") || {}, "f");
		const tmpOptions = {};
		tmpOptions.local = __classPrivateFieldGet(this, _YargsInstance_options, "f").local || [];
		tmpOptions.configObjects = __classPrivateFieldGet(this, _YargsInstance_options, "f").configObjects || [];
		const localLookup = {};
		tmpOptions.local.forEach((l) => {
			localLookup[l] = true;
			(aliases[l] || []).forEach((a) => {
				localLookup[a] = true;
			});
		});
		Object.assign(__classPrivateFieldGet(this, _YargsInstance_preservedGroups, "f"), Object.keys(__classPrivateFieldGet(this, _YargsInstance_groups, "f")).reduce((acc, groupName) => {
			const keys = __classPrivateFieldGet(this, _YargsInstance_groups, "f")[groupName].filter((key) => !(key in localLookup));
			if (keys.length > 0) acc[groupName] = keys;
			return acc;
		}, {}));
		__classPrivateFieldSet(this, _YargsInstance_groups, {}, "f");
		const arrayOptions = [
			"array",
			"boolean",
			"string",
			"skipValidation",
			"count",
			"normalize",
			"number",
			"hiddenOptions"
		];
		const objectOptions = [
			"narg",
			"key",
			"alias",
			"default",
			"defaultDescription",
			"config",
			"choices",
			"demandedOptions",
			"demandedCommands",
			"deprecatedOptions"
		];
		arrayOptions.forEach((k) => {
			tmpOptions[k] = (__classPrivateFieldGet(this, _YargsInstance_options, "f")[k] || []).filter((k$1) => !localLookup[k$1]);
		});
		objectOptions.forEach((k) => {
			tmpOptions[k] = objFilter(__classPrivateFieldGet(this, _YargsInstance_options, "f")[k], (k$1) => !localLookup[k$1]);
		});
		tmpOptions.envPrefix = __classPrivateFieldGet(this, _YargsInstance_options, "f").envPrefix;
		__classPrivateFieldSet(this, _YargsInstance_options, tmpOptions, "f");
		__classPrivateFieldSet(this, _YargsInstance_usage, __classPrivateFieldGet(this, _YargsInstance_usage, "f") ? __classPrivateFieldGet(this, _YargsInstance_usage, "f").reset(localLookup) : usage(this, __classPrivateFieldGet(this, _YargsInstance_shim, "f")), "f");
		__classPrivateFieldSet(this, _YargsInstance_validation, __classPrivateFieldGet(this, _YargsInstance_validation, "f") ? __classPrivateFieldGet(this, _YargsInstance_validation, "f").reset(localLookup) : validation(this, __classPrivateFieldGet(this, _YargsInstance_usage, "f"), __classPrivateFieldGet(this, _YargsInstance_shim, "f")), "f");
		__classPrivateFieldSet(this, _YargsInstance_command, __classPrivateFieldGet(this, _YargsInstance_command, "f") ? __classPrivateFieldGet(this, _YargsInstance_command, "f").reset() : command(__classPrivateFieldGet(this, _YargsInstance_usage, "f"), __classPrivateFieldGet(this, _YargsInstance_validation, "f"), __classPrivateFieldGet(this, _YargsInstance_globalMiddleware, "f"), __classPrivateFieldGet(this, _YargsInstance_shim, "f")), "f");
		if (!__classPrivateFieldGet(this, _YargsInstance_completion, "f")) __classPrivateFieldSet(this, _YargsInstance_completion, completion(this, __classPrivateFieldGet(this, _YargsInstance_usage, "f"), __classPrivateFieldGet(this, _YargsInstance_command, "f"), __classPrivateFieldGet(this, _YargsInstance_shim, "f")), "f");
		__classPrivateFieldGet(this, _YargsInstance_globalMiddleware, "f").reset();
		__classPrivateFieldSet(this, _YargsInstance_completionCommand, null, "f");
		__classPrivateFieldSet(this, _YargsInstance_output, "", "f");
		__classPrivateFieldSet(this, _YargsInstance_exitError, null, "f");
		__classPrivateFieldSet(this, _YargsInstance_hasOutput, false, "f");
		this.parsed = false;
		return this;
	}
	[kRebase](base, dir) {
		return __classPrivateFieldGet(this, _YargsInstance_shim, "f").path.relative(base, dir);
	}
	[kRunYargsParserAndExecuteCommands](args, shortCircuit, calledFromCommand, commandIndex = 0, helpOnly = false) {
		var _a$1, _b$1, _c$1, _d;
		let skipValidation = !!calledFromCommand || helpOnly;
		args = args || __classPrivateFieldGet(this, _YargsInstance_processArgs, "f");
		__classPrivateFieldGet(this, _YargsInstance_options, "f").__ = __classPrivateFieldGet(this, _YargsInstance_shim, "f").y18n.__;
		__classPrivateFieldGet(this, _YargsInstance_options, "f").configuration = this[kGetParserConfiguration]();
		const populateDoubleDash = !!__classPrivateFieldGet(this, _YargsInstance_options, "f").configuration["populate--"];
		const config = Object.assign({}, __classPrivateFieldGet(this, _YargsInstance_options, "f").configuration, { "populate--": true });
		const parsed = __classPrivateFieldGet(this, _YargsInstance_shim, "f").Parser.detailed(args, Object.assign({}, __classPrivateFieldGet(this, _YargsInstance_options, "f"), { configuration: {
			"parse-positional-numbers": false,
			...config
		} }));
		const argv$1 = Object.assign(parsed.argv, __classPrivateFieldGet(this, _YargsInstance_parseContext, "f"));
		let argvPromise = void 0;
		const aliases = parsed.aliases;
		let helpOptSet = false;
		let versionOptSet = false;
		Object.keys(argv$1).forEach((key) => {
			if (key === __classPrivateFieldGet(this, _YargsInstance_helpOpt, "f") && argv$1[key]) helpOptSet = true;
			else if (key === __classPrivateFieldGet(this, _YargsInstance_versionOpt, "f") && argv$1[key]) versionOptSet = true;
		});
		argv$1.$0 = this.$0;
		this.parsed = parsed;
		if (commandIndex === 0) __classPrivateFieldGet(this, _YargsInstance_usage, "f").clearCachedHelpMessage();
		try {
			this[kGuessLocale]();
			if (shortCircuit) return this[kPostProcess](argv$1, populateDoubleDash, !!calledFromCommand, false);
			if (__classPrivateFieldGet(this, _YargsInstance_helpOpt, "f")) {
				if ([__classPrivateFieldGet(this, _YargsInstance_helpOpt, "f")].concat(aliases[__classPrivateFieldGet(this, _YargsInstance_helpOpt, "f")] || []).filter((k) => k.length > 1).includes("" + argv$1._[argv$1._.length - 1])) {
					argv$1._.pop();
					helpOptSet = true;
				}
			}
			__classPrivateFieldSet(this, _YargsInstance_isGlobalContext, false, "f");
			const handlerKeys = __classPrivateFieldGet(this, _YargsInstance_command, "f").getCommands();
			const requestCompletions = ((_a$1 = __classPrivateFieldGet(this, _YargsInstance_completion, "f")) === null || _a$1 === void 0 ? void 0 : _a$1.completionKey) ? [(_b$1 = __classPrivateFieldGet(this, _YargsInstance_completion, "f")) === null || _b$1 === void 0 ? void 0 : _b$1.completionKey, ...(_d = this.getAliases()[(_c$1 = __classPrivateFieldGet(this, _YargsInstance_completion, "f")) === null || _c$1 === void 0 ? void 0 : _c$1.completionKey]) !== null && _d !== void 0 ? _d : []].some((key) => Object.prototype.hasOwnProperty.call(argv$1, key)) : false;
			const skipRecommendation = helpOptSet || requestCompletions || helpOnly;
			if (argv$1._.length) {
				if (handlerKeys.length) {
					let firstUnknownCommand;
					for (let i = commandIndex || 0, cmd; argv$1._[i] !== void 0; i++) {
						cmd = String(argv$1._[i]);
						if (handlerKeys.includes(cmd) && cmd !== __classPrivateFieldGet(this, _YargsInstance_completionCommand, "f")) {
							const innerArgv = __classPrivateFieldGet(this, _YargsInstance_command, "f").runCommand(cmd, this, parsed, i + 1, helpOnly, helpOptSet || versionOptSet || helpOnly);
							return this[kPostProcess](innerArgv, populateDoubleDash, !!calledFromCommand, false);
						} else if (!firstUnknownCommand && cmd !== __classPrivateFieldGet(this, _YargsInstance_completionCommand, "f")) {
							firstUnknownCommand = cmd;
							break;
						}
					}
					if (!__classPrivateFieldGet(this, _YargsInstance_command, "f").hasDefaultCommand() && __classPrivateFieldGet(this, _YargsInstance_recommendCommands, "f") && firstUnknownCommand && !skipRecommendation) __classPrivateFieldGet(this, _YargsInstance_validation, "f").recommendCommands(firstUnknownCommand, handlerKeys);
				}
				if (__classPrivateFieldGet(this, _YargsInstance_completionCommand, "f") && argv$1._.includes(__classPrivateFieldGet(this, _YargsInstance_completionCommand, "f")) && !requestCompletions) {
					if (__classPrivateFieldGet(this, _YargsInstance_exitProcess, "f")) setBlocking(true);
					this.showCompletionScript();
					this.exit(0);
				}
			}
			if (__classPrivateFieldGet(this, _YargsInstance_command, "f").hasDefaultCommand() && !skipRecommendation) {
				const innerArgv = __classPrivateFieldGet(this, _YargsInstance_command, "f").runCommand(null, this, parsed, 0, helpOnly, helpOptSet || versionOptSet || helpOnly);
				return this[kPostProcess](innerArgv, populateDoubleDash, !!calledFromCommand, false);
			}
			if (requestCompletions) {
				if (__classPrivateFieldGet(this, _YargsInstance_exitProcess, "f")) setBlocking(true);
				args = [].concat(args);
				const completionArgs = args.slice(args.indexOf(`--${__classPrivateFieldGet(this, _YargsInstance_completion, "f").completionKey}`) + 1);
				__classPrivateFieldGet(this, _YargsInstance_completion, "f").getCompletion(completionArgs, (err, completions) => {
					if (err) throw new YError(err.message);
					(completions || []).forEach((completion$1) => {
						__classPrivateFieldGet(this, _YargsInstance_logger, "f").log(completion$1);
					});
					this.exit(0);
				});
				return this[kPostProcess](argv$1, !populateDoubleDash, !!calledFromCommand, false);
			}
			if (!__classPrivateFieldGet(this, _YargsInstance_hasOutput, "f")) {
				if (helpOptSet) {
					if (__classPrivateFieldGet(this, _YargsInstance_exitProcess, "f")) setBlocking(true);
					skipValidation = true;
					this.showHelp((message) => {
						__classPrivateFieldGet(this, _YargsInstance_logger, "f").log(message);
						this.exit(0);
					});
				} else if (versionOptSet) {
					if (__classPrivateFieldGet(this, _YargsInstance_exitProcess, "f")) setBlocking(true);
					skipValidation = true;
					__classPrivateFieldGet(this, _YargsInstance_usage, "f").showVersion("log");
					this.exit(0);
				}
			}
			if (!skipValidation && __classPrivateFieldGet(this, _YargsInstance_options, "f").skipValidation.length > 0) skipValidation = Object.keys(argv$1).some((key) => __classPrivateFieldGet(this, _YargsInstance_options, "f").skipValidation.indexOf(key) >= 0 && argv$1[key] === true);
			if (!skipValidation) {
				if (parsed.error) throw new YError(parsed.error.message);
				if (!requestCompletions) {
					const validation$1 = this[kRunValidation](aliases, {}, parsed.error);
					if (!calledFromCommand) argvPromise = applyMiddleware(argv$1, this, __classPrivateFieldGet(this, _YargsInstance_globalMiddleware, "f").getMiddleware(), true);
					argvPromise = this[kValidateAsync](validation$1, argvPromise !== null && argvPromise !== void 0 ? argvPromise : argv$1);
					if (isPromise(argvPromise) && !calledFromCommand) argvPromise = argvPromise.then(() => {
						return applyMiddleware(argv$1, this, __classPrivateFieldGet(this, _YargsInstance_globalMiddleware, "f").getMiddleware(), false);
					});
				}
			}
		} catch (err) {
			if (err instanceof YError) __classPrivateFieldGet(this, _YargsInstance_usage, "f").fail(err.message, err);
			else throw err;
		}
		return this[kPostProcess](argvPromise !== null && argvPromise !== void 0 ? argvPromise : argv$1, populateDoubleDash, !!calledFromCommand, true);
	}
	[kRunValidation](aliases, positionalMap, parseErrors, isDefaultCommand) {
		const demandedOptions = { ...this.getDemandedOptions() };
		return (argv$1) => {
			if (parseErrors) throw new YError(parseErrors.message);
			__classPrivateFieldGet(this, _YargsInstance_validation, "f").nonOptionCount(argv$1);
			__classPrivateFieldGet(this, _YargsInstance_validation, "f").requiredArguments(argv$1, demandedOptions);
			let failedStrictCommands = false;
			if (__classPrivateFieldGet(this, _YargsInstance_strictCommands, "f")) failedStrictCommands = __classPrivateFieldGet(this, _YargsInstance_validation, "f").unknownCommands(argv$1);
			if (__classPrivateFieldGet(this, _YargsInstance_strict, "f") && !failedStrictCommands) __classPrivateFieldGet(this, _YargsInstance_validation, "f").unknownArguments(argv$1, aliases, positionalMap, !!isDefaultCommand);
			else if (__classPrivateFieldGet(this, _YargsInstance_strictOptions, "f")) __classPrivateFieldGet(this, _YargsInstance_validation, "f").unknownArguments(argv$1, aliases, {}, false, false);
			__classPrivateFieldGet(this, _YargsInstance_validation, "f").limitedChoices(argv$1);
			__classPrivateFieldGet(this, _YargsInstance_validation, "f").implications(argv$1);
			__classPrivateFieldGet(this, _YargsInstance_validation, "f").conflicting(argv$1);
		};
	}
	[kSetHasOutput]() {
		__classPrivateFieldSet(this, _YargsInstance_hasOutput, true, "f");
	}
	[kTrackManuallySetKeys](keys) {
		if (typeof keys === "string") __classPrivateFieldGet(this, _YargsInstance_options, "f").key[keys] = true;
		else for (const k of keys) __classPrivateFieldGet(this, _YargsInstance_options, "f").key[k] = true;
	}
};
function isYargsInstance(y) {
	return !!y && typeof y.getInternalMethods === "function";
}

//#endregion
//#region node_modules/.pnpm/yargs@18.0.0/node_modules/yargs/index.mjs
const Yargs = YargsFactory(esm_default);
var yargs_default = Yargs;

//#endregion
//#region src/JSONFilterTransform.ts
/**
* Extracts JSON-RPC messages from a stream that may contain non-JSON output.
*
* Lines that start with '{' are passed through as-is. Lines that contain '{'
* but have a non-JSON prefix (e.g. Python warnings prepended to a JSON message)
* have the prefix stripped and the JSON portion extracted. Lines with no '{'
* are dropped entirely.
*/
var JSONFilterTransform = class extends Transform {
	buffer = "";
	constructor() {
		super({ objectMode: false });
	}
	_flush(callback) {
		const json = extractJson(this.buffer);
		if (json !== null) callback(null, Buffer.from(json));
		else callback(null, null);
	}
	_transform(chunk, _encoding, callback) {
		this.buffer += chunk.toString();
		const lines = this.buffer.split("\n");
		this.buffer = lines.pop() || "";
		const jsonLines = [];
		const nonJsonLines = [];
		for (const line of lines) {
			const json = extractJson(line);
			if (json !== null) jsonLines.push(json);
			else if (line.trim().length > 0) nonJsonLines.push(line);
		}
		if (nonJsonLines.length > 0) console.warn("[mcp-proxy] ignoring non-JSON output", nonJsonLines);
		if (jsonLines.length > 0) {
			const output = jsonLines.join("\n") + "\n";
			callback(null, Buffer.from(output));
		} else callback(null, null);
	}
};
/**
* Extracts the JSON portion from a line that may have a non-JSON prefix.
* Returns null if the line contains no '{'.
*/
function extractJson(line) {
	const trimmed = line.trim();
	if (trimmed.length === 0) return null;
	const braceIndex = trimmed.indexOf("{");
	if (braceIndex === -1) return null;
	if (braceIndex === 0) return trimmed;
	const jsonPart = trimmed.slice(braceIndex);
	console.warn("[mcp-proxy] stripped non-JSON prefix from output:", trimmed.slice(0, braceIndex));
	return jsonPart;
}

//#endregion
//#region src/StdioClientTransport.ts
/**
* Forked from https://github.com/modelcontextprotocol/typescript-sdk/blob/a1608a6513d18eb965266286904760f830de96fe/src/client/stdio.ts
*/
/**
* Client transport for stdio: this will connect to a server by spawning a process and communicating with it over stdin/stdout.
*
* This transport is only available in Node.js environments.
*/
var StdioClientTransport = class {
	onclose;
	onerror;
	onmessage;
	/**
	* The child process pid spawned by this transport.
	*
	* This is only available after the transport has been started.
	*/
	get pid() {
		return this._process?.pid ?? null;
	}
	/**
	* The stderr stream of the child process, if `StdioServerParameters.stderr` was set to "pipe" or "overlapped".
	*
	* If stderr piping was requested, a PassThrough stream is returned _immediately_, allowing callers to
	* attach listeners before the start method is invoked. This prevents loss of any early
	* error output emitted by the child process.
	*/
	get stderr() {
		if (this._stderrStream) return this._stderrStream;
		return this._process?.stderr ?? null;
	}
	_abortController = new AbortController();
	_process;
	_readBuffer = new ReadBuffer();
	_serverParams;
	_stderrStream = null;
	onEvent;
	constructor(server) {
		this._serverParams = server;
		if (server.stderr === "pipe" || server.stderr === "overlapped") this._stderrStream = new PassThrough();
		this.onEvent = server.onEvent;
	}
	async close() {
		this.onEvent?.({ type: "close" });
		this._abortController.abort();
		this._process = void 0;
		this._readBuffer.clear();
	}
	send(message) {
		return new Promise((resolve$1) => {
			if (!this._process?.stdin) throw new Error("Not connected");
			const json = serializeMessage(message);
			if (this._process.stdin.write(json)) resolve$1();
			else this._process.stdin.once("drain", resolve$1);
		});
	}
	/**
	* Starts the server process and prepares to communicate with it.
	*/
	async start() {
		if (this._process) throw new Error("StdioClientTransport already started! If using Client class, note that connect() calls start() automatically.");
		return new Promise((resolve$1, reject) => {
			this._process = spawn(this._serverParams.command, this._serverParams.args ?? [], {
				cwd: this._serverParams.cwd,
				env: this._serverParams.env,
				shell: this._serverParams.shell ?? false,
				signal: this._abortController.signal,
				stdio: [
					"pipe",
					"pipe",
					this._serverParams.stderr ?? "inherit"
				]
			});
			this._process.on("error", (error) => {
				if (error.name === "AbortError") {
					this.onclose?.();
					return;
				}
				reject(error);
				this.onerror?.(error);
			});
			this._process.on("spawn", () => {
				resolve$1();
			});
			this._process.on("close", (_code) => {
				this.onEvent?.({ type: "close" });
				this._process = void 0;
				this.onclose?.();
			});
			this._process.stdin?.on("error", (error) => {
				this.onEvent?.({
					error,
					type: "error"
				});
				this.onerror?.(error);
			});
			const jsonFilterTransform = new JSONFilterTransform();
			this._process.stdout?.pipe(jsonFilterTransform);
			jsonFilterTransform.on("data", (chunk) => {
				this.onEvent?.({
					chunk: chunk.toString(),
					type: "data"
				});
				this._readBuffer.append(chunk);
				this.processReadBuffer();
			});
			jsonFilterTransform.on("error", (error) => {
				this.onEvent?.({
					error,
					type: "error"
				});
				this.onerror?.(error);
			});
			if (this._stderrStream && this._process.stderr) this._process.stderr.pipe(this._stderrStream);
		});
	}
	processReadBuffer() {
		while (true) try {
			const message = this._readBuffer.readMessage();
			if (message === null) break;
			this.onEvent?.({
				message,
				type: "message"
			});
			this.onmessage?.(message);
		} catch (error) {
			this.onEvent?.({
				error,
				type: "error"
			});
			this.onerror?.(error);
		}
	}
};

//#endregion
//#region src/bin/mcp-proxy.ts
const packageJson = createRequire(import.meta.url)("../../package.json");
util.inspect.defaultOptions.depth = 8;
if (!("EventSource" in global)) global.EventSource = EventSource;
const argv = await yargs_default(hideBin(process.argv)).scriptName("mcp-proxy").version(packageJson.version).command("$0 [command] [args...]", "Proxy an MCP stdio server over HTTP").positional("command", {
	describe: "The command to run",
	type: "string"
}).positional("args", {
	array: true,
	describe: "The arguments to pass to the command",
	type: "string"
}).usage("$0 [options] -- <command> [args...]\n       $0 <command> [args...]").env("MCP_PROXY").parserConfiguration({ "populate--": true }).options({
	apiKey: {
		describe: "API key for authenticating requests (uses X-API-Key header)",
		type: "string"
	},
	connectionTimeout: {
		default: 6e4,
		describe: "The timeout (in milliseconds) for initial connection to the MCP server (default: 60 seconds)",
		type: "number"
	},
	debug: {
		default: false,
		describe: "Enable debug logging",
		type: "boolean"
	},
	endpoint: {
		describe: "The endpoint to listen on",
		type: "string"
	},
	gracefulShutdownTimeout: {
		default: 5e3,
		describe: "The timeout (in milliseconds) for graceful shutdown",
		type: "number"
	},
	host: {
		default: "::",
		describe: "The host to listen on",
		type: "string"
	},
	port: {
		default: 8080,
		describe: "The port to listen on",
		type: "number"
	},
	requestTimeout: {
		default: 3e5,
		describe: "The timeout (in milliseconds) for requests to the MCP server (default: 5 minutes)",
		type: "number"
	},
	server: {
		choices: ["sse", "stream"],
		describe: "The server type to use (sse or stream). By default, both are enabled",
		type: "string"
	},
	shell: {
		default: false,
		describe: "Spawn the server via the user's shell",
		type: "boolean"
	},
	sseEndpoint: {
		default: "/sse",
		describe: "The SSE endpoint to listen on",
		type: "string"
	},
	sslCa: {
		describe: "Filename to override the trusted CA certificates",
		type: "string"
	},
	sslCert: {
		describe: "Cert chains filename in PEM format",
		type: "string"
	},
	sslKey: {
		describe: "Private keys filename in PEM format",
		type: "string"
	},
	stateless: {
		default: false,
		describe: "Enable stateless mode for HTTP streamable transport (no session management)",
		type: "boolean"
	},
	streamEndpoint: {
		default: "/mcp",
		describe: "The stream endpoint to listen on",
		type: "string"
	},
	tunnel: {
		default: false,
		describe: "Expose the proxy via a public tunnel using tunnel.gla.ma",
		type: "boolean"
	},
	tunnelSubdomain: {
		describe: "Request a specific subdomain for the tunnel (availability not guaranteed)",
		type: "string"
	}
}).help().parseAsync();
const dashDashArgs = argv["--"];
let finalCommand;
let finalArgs;
if (dashDashArgs && dashDashArgs.length > 0) [finalCommand, ...finalArgs] = dashDashArgs;
else if (argv.command) {
	finalCommand = argv.command;
	finalArgs = argv.args || [];
} else {
	console.error("Error: No command specified.");
	console.error("Usage: mcp-proxy [options] -- <command> [args...]");
	console.error("   or: mcp-proxy <command> [args...]");
	console.error("");
	console.error("Examples:");
	console.error("  mcp-proxy --port 8080 -- node server.js --port 3000");
	console.error("  mcp-proxy node server.js");
	process.exit(1);
}
const connect = async (client, connectionTimeout) => {
	const transport = new StdioClientTransport({
		args: finalArgs,
		command: finalCommand,
		env: process.env,
		onEvent: (event) => {
			if (argv.debug) console.debug("transport event", event);
		},
		shell: argv.shell,
		stderr: "inherit"
	});
	await client.connect(transport, { timeout: connectionTimeout });
};
const proxy = async () => {
	const client = new Client({
		name: "mcp-proxy",
		version: "1.0.0"
	}, { capabilities: {} });
	await connect(client, argv.connectionTimeout);
	const serverVersion = client.getServerVersion();
	const serverCapabilities = client.getServerCapabilities();
	console.info("starting server on port %d", argv.port);
	const createServer = async () => {
		const server$1 = new Server(serverVersion, { capabilities: serverCapabilities });
		proxyServer({
			client,
			requestTimeout: argv.requestTimeout,
			server: server$1,
			serverCapabilities
		});
		return server$1;
	};
	const server = await startHTTPServer({
		apiKey: argv.apiKey,
		createServer,
		eventStore: new InMemoryEventStore(),
		host: argv.host,
		port: argv.port,
		sseEndpoint: argv.server && argv.server !== "sse" ? null : argv.sseEndpoint ?? argv.endpoint,
		sslCa: argv.sslCa,
		sslCert: argv.sslCert,
		sslKey: argv.sslKey,
		stateless: argv.stateless,
		streamEndpoint: argv.server && argv.server !== "stream" ? null : argv.streamEndpoint ?? argv.endpoint
	});
	let tunnel;
	if (argv.tunnel) {
		console.info("establishing tunnel via tunnel.gla.ma");
		tunnel = await pipenet({
			host: "https://tunnel.gla.ma",
			port: argv.port,
			subdomain: argv.tunnelSubdomain
		});
		console.info("tunnel established at %s", tunnel.url);
	}
	return { close: async () => {
		await server.close();
		if (tunnel) await tunnel.close();
	} };
};
const createGracefulShutdown = ({ server, timeout }) => {
	const gracefulShutdown = () => {
		console.info("received shutdown signal; shutting down");
		server.close();
		setTimeout$1(() => {
			process.exit(1);
		}, timeout).unref();
	};
	process.once("SIGTERM", gracefulShutdown);
	process.once("SIGINT", gracefulShutdown);
	return () => {
		server.close();
	};
};
const main = async () => {
	try {
		createGracefulShutdown({
			server: await proxy(),
			timeout: argv.gracefulShutdownTimeout
		});
	} catch (error) {
		console.error("could not start the proxy", error);
		setTimeout$1(() => {
			process.exit(1);
		}, 1e3);
	}
};
await main();

//#endregion
export {  };
//# sourceMappingURL=mcp-proxy.mjs.map