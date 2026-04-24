"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.UndiciInstrumentation = void 0;
/*
 * Copyright The OpenTelemetry Authors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
const diagch = require("diagnostics_channel");
const url_1 = require("url");
const instrumentation_1 = require("@opentelemetry/instrumentation");
const api_1 = require("@opentelemetry/api");
const version_1 = require("./version");
const SemanticAttributes_1 = require("./enums/SemanticAttributes");
const core_1 = require("@opentelemetry/core");
// A combination of https://github.com/elastic/apm-agent-nodejs and
// https://github.com/gadget-inc/opentelemetry-instrumentations/blob/main/packages/opentelemetry-instrumentation-undici/src/index.ts
class UndiciInstrumentation extends instrumentation_1.InstrumentationBase {
    constructor(config = {}) {
        super(version_1.PACKAGE_NAME, version_1.PACKAGE_VERSION, config);
        this._recordFromReq = new WeakMap();
        this.setConfig(config);
    }
    // No need to instrument files/modules
    init() {
        return undefined;
    }
    disable() {
        if (!this._config.enabled) {
            return;
        }
        this._channelSubs.forEach(sub => sub.channel.unsubscribe(sub.onMessage));
        this._channelSubs.length = 0;
        this._config.enabled = false;
    }
    enable() {
        if (this._config.enabled) {
            return;
        }
        this._config.enabled = true;
        // This method is called by the `InstrumentationAbstract` constructor before
        // ours is called. So we need to ensure the property is initalized
        this._channelSubs = this._channelSubs || [];
        this.subscribeToChannel('undici:request:create', this.onRequestCreated.bind(this));
        this.subscribeToChannel('undici:client:sendHeaders', this.onRequestHeaders.bind(this));
        this.subscribeToChannel('undici:request:headers', this.onResponseHeaders.bind(this));
        this.subscribeToChannel('undici:request:trailers', this.onDone.bind(this));
        this.subscribeToChannel('undici:request:error', this.onError.bind(this));
    }
    setConfig(config = {}) {
        super.setConfig(config);
        if (config === null || config === void 0 ? void 0 : config.enabled) {
            this.enable();
        }
        else {
            this.disable();
        }
    }
    _updateMetricInstruments() {
        this._httpClientDurationHistogram = this.meter.createHistogram('http.client.request.duration', {
            description: 'Measures the duration of outbound HTTP requests.',
            unit: 's',
            valueType: api_1.ValueType.DOUBLE,
            advice: {
                explicitBucketBoundaries: [
                    0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1, 2.5, 5,
                    7.5, 10,
                ],
            },
        });
    }
    _getConfig() {
        return this._config;
    }
    subscribeToChannel(diagnosticChannel, onMessage) {
        const channel = diagch.channel(diagnosticChannel);
        channel.subscribe(onMessage);
        this._channelSubs.push({
            name: diagnosticChannel,
            channel,
            onMessage,
        });
    }
    // This is the 1st message we receive for each request (fired after request creation). Here we will
    // create the span and populate some atttributes, then link the span to the request for further
    // span processing
    onRequestCreated({ request }) {
        // Ignore if:
        // - instrumentation is disabled
        // - ignored by config
        // - method is 'CONNECT'
        const config = this._getConfig();
        const shouldIgnoreReq = (0, instrumentation_1.safeExecuteInTheMiddle)(() => {
            var _a;
            return !config.enabled ||
                request.method === 'CONNECT' ||
                ((_a = config.ignoreRequestHook) === null || _a === void 0 ? void 0 : _a.call(config, request));
        }, e => e && this._diag.error('caught ignoreRequestHook error: ', e), true);
        if (shouldIgnoreReq) {
            return;
        }
        const startTime = (0, core_1.hrTime)();
        const requestUrl = new url_1.URL(request.origin + request.path);
        const urlScheme = requestUrl.protocol.replace(':', '');
        const requestMethod = this.getRequestMethod(request.method);
        const attributes = {
            [SemanticAttributes_1.SemanticAttributes.HTTP_REQUEST_METHOD]: requestMethod,
            [SemanticAttributes_1.SemanticAttributes.HTTP_REQUEST_METHOD_ORIGINAL]: request.method,
            [SemanticAttributes_1.SemanticAttributes.URL_FULL]: requestUrl.toString(),
            [SemanticAttributes_1.SemanticAttributes.URL_PATH]: requestUrl.pathname,
            [SemanticAttributes_1.SemanticAttributes.URL_QUERY]: requestUrl.search,
            [SemanticAttributes_1.SemanticAttributes.URL_SCHEME]: urlScheme,
        };
        const schemePorts = { https: '443', http: '80' };
        const serverAddress = requestUrl.hostname;
        const serverPort = requestUrl.port || schemePorts[urlScheme];
        attributes[SemanticAttributes_1.SemanticAttributes.SERVER_ADDRESS] = serverAddress;
        if (serverPort && !isNaN(Number(serverPort))) {
            attributes[SemanticAttributes_1.SemanticAttributes.SERVER_PORT] = Number(serverPort);
        }
        // Get user agent from headers
        let userAgent;
        if (Array.isArray(request.headers)) {
            const idx = request.headers.findIndex(h => h.toLowerCase() === 'user-agent');
            userAgent = request.headers[idx + 1];
        }
        else if (typeof request.headers === 'string') {
            const headers = request.headers.split('\r\n');
            const uaHeader = headers.find(h => h.toLowerCase().startsWith('user-agent'));
            userAgent =
                uaHeader && uaHeader.substring(uaHeader.indexOf(':') + 1).trim();
        }
        if (userAgent) {
            attributes[SemanticAttributes_1.SemanticAttributes.USER_AGENT_ORIGINAL] = userAgent;
        }
        // Get attributes from the hook if present
        const hookAttributes = (0, instrumentation_1.safeExecuteInTheMiddle)(() => { var _a; return (_a = config.startSpanHook) === null || _a === void 0 ? void 0 : _a.call(config, request); }, e => e && this._diag.error('caught startSpanHook error: ', e), true);
        if (hookAttributes) {
            Object.entries(hookAttributes).forEach(([key, val]) => {
                attributes[key] = val;
            });
        }
        // Check if parent span is required via config and:
        // - if a parent is required but not present, we use a `NoopSpan` to still
        //   propagate context without recording it.
        // - create a span otherwise
        const activeCtx = api_1.context.active();
        const currentSpan = api_1.trace.getSpan(activeCtx);
        let span;
        if (config.requireParentforSpans && !currentSpan) {
            span = api_1.trace.wrapSpanContext(api_1.INVALID_SPAN_CONTEXT);
        }
        else {
            span = this.tracer.startSpan(requestMethod === '_OTHER' ? 'HTTP' : requestMethod, {
                kind: api_1.SpanKind.CLIENT,
                attributes: attributes,
            }, activeCtx);
        }
        // Execute the request hook if defined
        (0, instrumentation_1.safeExecuteInTheMiddle)(() => { var _a; return (_a = config.requestHook) === null || _a === void 0 ? void 0 : _a.call(config, span, request); }, e => e && this._diag.error('caught requestHook error: ', e), true);
        // Context propagation goes last so no hook can tamper
        // the propagation headers
        const requestContext = api_1.trace.setSpan(api_1.context.active(), span);
        const addedHeaders = {};
        api_1.propagation.inject(requestContext, addedHeaders);
        const headerEntries = Object.entries(addedHeaders);
        for (let i = 0; i < headerEntries.length; i++) {
            const [k, v] = headerEntries[i];
            if (typeof request.addHeader === 'function') {
                request.addHeader(k, v);
            }
            else if (typeof request.headers === 'string') {
                request.headers += `${k}: ${v}\r\n`;
            }
            else if (Array.isArray(request.headers)) {
                // undici@6.11.0 accidentally, briefly removed `request.addHeader()`.
                request.headers.push(k, v);
            }
        }
        this._recordFromReq.set(request, { span, attributes, startTime });
    }
    // This is the 2nd message we receive for each request. It is fired when connection with
    // the remote is established and about to send the first byte. Here we do have info about the
    // remote address and port so we can populate some `network.*` attributes into the span
    onRequestHeaders({ request, socket }) {
        var _a;
        const record = this._recordFromReq.get(request);
        if (!record) {
            return;
        }
        const config = this._getConfig();
        const { span } = record;
        const { remoteAddress, remotePort } = socket;
        const spanAttributes = {
            [SemanticAttributes_1.SemanticAttributes.NETWORK_PEER_ADDRESS]: remoteAddress,
            [SemanticAttributes_1.SemanticAttributes.NETWORK_PEER_PORT]: remotePort,
        };
        // After hooks have been processed (which may modify request headers)
        // we can collect the headers based on the configuration
        if ((_a = config.headersToSpanAttributes) === null || _a === void 0 ? void 0 : _a.requestHeaders) {
            const headersToAttribs = new Set(config.headersToSpanAttributes.requestHeaders.map(n => n.toLowerCase()));
            // headers could be in form
            // ['name: value', ...] for v5
            // ['name', 'value', ...] for v6
            const rawHeaders = Array.isArray(request.headers)
                ? request.headers
                : request.headers.split('\r\n');
            rawHeaders.forEach((h, idx) => {
                const sepIndex = h.indexOf(':');
                const hasSeparator = sepIndex !== -1;
                const name = (hasSeparator ? h.substring(0, sepIndex) : h).toLowerCase();
                const value = hasSeparator
                    ? h.substring(sepIndex + 1)
                    : rawHeaders[idx + 1];
                if (headersToAttribs.has(name)) {
                    spanAttributes[`http.request.header.${name}`] = value.trim();
                }
            });
        }
        span.setAttributes(spanAttributes);
    }
    // This is the 3rd message we get for each request and it's fired when the server
    // headers are received, body may not be accessible yet.
    // From the response headers we can set the status and content length
    onResponseHeaders({ request, response, }) {
        var _a, _b;
        const record = this._recordFromReq.get(request);
        if (!record) {
            return;
        }
        const { span, attributes } = record;
        const spanAttributes = {
            [SemanticAttributes_1.SemanticAttributes.HTTP_RESPONSE_STATUS_CODE]: response.statusCode,
        };
        const config = this._getConfig();
        const headersToAttribs = new Set();
        if ((_a = config.headersToSpanAttributes) === null || _a === void 0 ? void 0 : _a.responseHeaders) {
            (_b = config.headersToSpanAttributes) === null || _b === void 0 ? void 0 : _b.responseHeaders.forEach(name => headersToAttribs.add(name.toLowerCase()));
        }
        for (let idx = 0; idx < response.headers.length; idx = idx + 2) {
            const name = response.headers[idx].toString().toLowerCase();
            const value = response.headers[idx + 1];
            if (headersToAttribs.has(name)) {
                spanAttributes[`http.response.header.${name}`] = value.toString();
            }
            if (name === 'content-length') {
                const contentLength = Number(value.toString());
                if (!isNaN(contentLength)) {
                    spanAttributes['http.response.header.content-length'] = contentLength;
                }
            }
        }
        span.setAttributes(spanAttributes);
        span.setStatus({
            code: response.statusCode >= 400
                ? api_1.SpanStatusCode.ERROR
                : api_1.SpanStatusCode.UNSET,
        });
        record.attributes = Object.assign(attributes, spanAttributes);
    }
    // This is the last event we receive if the request went without any errors
    onDone({ request }) {
        const record = this._recordFromReq.get(request);
        if (!record) {
            return;
        }
        const { span, attributes, startTime } = record;
        // End the span
        span.end();
        this._recordFromReq.delete(request);
        // Record metrics
        this.recordRequestDuration(attributes, startTime);
    }
    // This is the event we get when something is wrong in the request like
    // - invalid options when calling `fetch` global API or any undici method for request
    // - connectivity errors such as unreachable host
    // - requests aborted through an `AbortController.signal`
    // NOTE: server errors are considered valid responses and it's the lib consumer
    // who should deal with that.
    onError({ request, error }) {
        const record = this._recordFromReq.get(request);
        if (!record) {
            return;
        }
        const { span, attributes, startTime } = record;
        // NOTE: in `undici@6.3.0` when request aborted the error type changes from
        // a custom error (`RequestAbortedError`) to a built-in `DOMException` carrying
        // some differences:
        // - `code` is from DOMEXception (ABORT_ERR: 20)
        // - `message` changes
        // - stacktrace is smaller and contains node internal frames
        span.recordException(error);
        span.setStatus({
            code: api_1.SpanStatusCode.ERROR,
            message: error.message,
        });
        span.end();
        this._recordFromReq.delete(request);
        // Record metrics (with the error)
        attributes[SemanticAttributes_1.SemanticAttributes.ERROR_TYPE] = error.message;
        this.recordRequestDuration(attributes, startTime);
    }
    recordRequestDuration(attributes, startTime) {
        // Time to record metrics
        const metricsAttributes = {};
        // Get the attribs already in span attributes
        const keysToCopy = [
            SemanticAttributes_1.SemanticAttributes.HTTP_RESPONSE_STATUS_CODE,
            SemanticAttributes_1.SemanticAttributes.HTTP_REQUEST_METHOD,
            SemanticAttributes_1.SemanticAttributes.SERVER_ADDRESS,
            SemanticAttributes_1.SemanticAttributes.SERVER_PORT,
            SemanticAttributes_1.SemanticAttributes.URL_SCHEME,
            SemanticAttributes_1.SemanticAttributes.ERROR_TYPE,
        ];
        keysToCopy.forEach(key => {
            if (key in attributes) {
                metricsAttributes[key] = attributes[key];
            }
        });
        // Take the duration and record it
        const durationSeconds = (0, core_1.hrTimeToMilliseconds)((0, core_1.hrTimeDuration)(startTime, (0, core_1.hrTime)())) / 1000;
        this._httpClientDurationHistogram.record(durationSeconds, metricsAttributes);
    }
    getRequestMethod(original) {
        const knownMethods = {
            CONNECT: true,
            OPTIONS: true,
            HEAD: true,
            GET: true,
            POST: true,
            PUT: true,
            PATCH: true,
            DELETE: true,
            TRACE: true,
        };
        if (original.toUpperCase() in knownMethods) {
            return original.toUpperCase();
        }
        return '_OTHER';
    }
}
exports.UndiciInstrumentation = UndiciInstrumentation;
//# sourceMappingURL=undici.js.map