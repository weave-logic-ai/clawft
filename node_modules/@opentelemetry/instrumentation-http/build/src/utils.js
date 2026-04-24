"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.headerCapture = exports.getIncomingRequestMetricAttributesOnResponse = exports.getIncomingRequestAttributesOnResponse = exports.getIncomingRequestMetricAttributes = exports.getIncomingRequestAttributes = exports.getOutgoingRequestMetricAttributesOnResponse = exports.getOutgoingRequestAttributesOnResponse = exports.setAttributesFromHttpKind = exports.getOutgoingRequestMetricAttributes = exports.getOutgoingRequestAttributes = exports.extractHostnameAndPort = exports.isValidOptionsType = exports.getRequestInfo = exports.isCompressed = exports.setResponseContentLengthAttribute = exports.setRequestContentLengthAttribute = exports.setSpanWithError = exports.isIgnored = exports.satisfiesPattern = exports.parseResponseStatus = exports.getAbsoluteUrl = void 0;
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
const api_1 = require("@opentelemetry/api");
const semantic_conventions_1 = require("@opentelemetry/semantic-conventions");
const core_1 = require("@opentelemetry/core");
const url = require("url");
const AttributeNames_1 = require("./enums/AttributeNames");
/**
 * Get an absolute url
 */
const getAbsoluteUrl = (requestUrl, headers, fallbackProtocol = 'http:') => {
    const reqUrlObject = requestUrl || {};
    const protocol = reqUrlObject.protocol || fallbackProtocol;
    const port = (reqUrlObject.port || '').toString();
    const path = reqUrlObject.path || '/';
    let host = reqUrlObject.host || reqUrlObject.hostname || headers.host || 'localhost';
    // if there is no port in host and there is a port
    // it should be displayed if it's not 80 and 443 (default ports)
    if (host.indexOf(':') === -1 &&
        port &&
        port !== '80' &&
        port !== '443') {
        host += `:${port}`;
    }
    return `${protocol}//${host}${path}`;
};
exports.getAbsoluteUrl = getAbsoluteUrl;
/**
 * Parse status code from HTTP response. [More details](https://github.com/open-telemetry/opentelemetry-specification/blob/master/specification/data-http.md#status)
 */
const parseResponseStatus = (kind, statusCode) => {
    const upperBound = kind === api_1.SpanKind.CLIENT ? 400 : 500;
    // 1xx, 2xx, 3xx are OK on client and server
    // 4xx is OK on server
    if (statusCode && statusCode >= 100 && statusCode < upperBound) {
        return api_1.SpanStatusCode.UNSET;
    }
    // All other codes are error
    return api_1.SpanStatusCode.ERROR;
};
exports.parseResponseStatus = parseResponseStatus;
/**
 * Check whether the given obj match pattern
 * @param constant e.g URL of request
 * @param pattern Match pattern
 */
const satisfiesPattern = (constant, pattern) => {
    if (typeof pattern === 'string') {
        return pattern === constant;
    }
    else if (pattern instanceof RegExp) {
        return pattern.test(constant);
    }
    else if (typeof pattern === 'function') {
        return pattern(constant);
    }
    else {
        throw new TypeError('Pattern is in unsupported datatype');
    }
};
exports.satisfiesPattern = satisfiesPattern;
/**
 * Check whether the given request is ignored by configuration
 * It will not re-throw exceptions from `list` provided by the client
 * @param constant e.g URL of request
 * @param [list] List of ignore patterns
 * @param [onException] callback for doing something when an exception has
 *     occurred
 */
const isIgnored = (constant, list, onException) => {
    if (!list) {
        // No ignored urls - trace everything
        return false;
    }
    // Try/catch outside the loop for failing fast
    try {
        for (const pattern of list) {
            if ((0, exports.satisfiesPattern)(constant, pattern)) {
                return true;
            }
        }
    }
    catch (e) {
        if (onException) {
            onException(e);
        }
    }
    return false;
};
exports.isIgnored = isIgnored;
/**
 * Sets the span with the error passed in params
 * @param {Span} span the span that need to be set
 * @param {Error} error error that will be set to span
 */
const setSpanWithError = (span, error) => {
    const message = error.message;
    span.setAttribute(AttributeNames_1.AttributeNames.HTTP_ERROR_NAME, error.name);
    span.setAttribute(AttributeNames_1.AttributeNames.HTTP_ERROR_MESSAGE, message);
    span.setStatus({ code: api_1.SpanStatusCode.ERROR, message });
    span.recordException(error);
};
exports.setSpanWithError = setSpanWithError;
/**
 * Adds attributes for request content-length and content-encoding HTTP headers
 * @param { IncomingMessage } Request object whose headers will be analyzed
 * @param { SpanAttributes } SpanAttributes object to be modified
 */
const setRequestContentLengthAttribute = (request, attributes) => {
    const length = getContentLength(request.headers);
    if (length === null)
        return;
    if ((0, exports.isCompressed)(request.headers)) {
        attributes[semantic_conventions_1.SEMATTRS_HTTP_REQUEST_CONTENT_LENGTH] = length;
    }
    else {
        attributes[semantic_conventions_1.SEMATTRS_HTTP_REQUEST_CONTENT_LENGTH_UNCOMPRESSED] = length;
    }
};
exports.setRequestContentLengthAttribute = setRequestContentLengthAttribute;
/**
 * Adds attributes for response content-length and content-encoding HTTP headers
 * @param { IncomingMessage } Response object whose headers will be analyzed
 * @param { SpanAttributes } SpanAttributes object to be modified
 */
const setResponseContentLengthAttribute = (response, attributes) => {
    const length = getContentLength(response.headers);
    if (length === null)
        return;
    if ((0, exports.isCompressed)(response.headers)) {
        attributes[semantic_conventions_1.SEMATTRS_HTTP_RESPONSE_CONTENT_LENGTH] = length;
    }
    else {
        attributes[semantic_conventions_1.SEMATTRS_HTTP_RESPONSE_CONTENT_LENGTH_UNCOMPRESSED] = length;
    }
};
exports.setResponseContentLengthAttribute = setResponseContentLengthAttribute;
function getContentLength(headers) {
    const contentLengthHeader = headers['content-length'];
    if (contentLengthHeader === undefined)
        return null;
    const contentLength = parseInt(contentLengthHeader, 10);
    if (isNaN(contentLength))
        return null;
    return contentLength;
}
const isCompressed = (headers) => {
    const encoding = headers['content-encoding'];
    return !!encoding && encoding !== 'identity';
};
exports.isCompressed = isCompressed;
/**
 * Makes sure options is an url object
 * return an object with default value and parsed options
 * @param options original options for the request
 * @param [extraOptions] additional options for the request
 */
const getRequestInfo = (options, extraOptions) => {
    let pathname = '/';
    let origin = '';
    let optionsParsed;
    if (typeof options === 'string') {
        optionsParsed = url.parse(options);
        pathname = optionsParsed.pathname || '/';
        origin = `${optionsParsed.protocol || 'http:'}//${optionsParsed.host}`;
        if (extraOptions !== undefined) {
            Object.assign(optionsParsed, extraOptions);
        }
    }
    else if (options instanceof url.URL) {
        optionsParsed = {
            protocol: options.protocol,
            hostname: typeof options.hostname === 'string' && options.hostname.startsWith('[')
                ? options.hostname.slice(1, -1)
                : options.hostname,
            path: `${options.pathname || ''}${options.search || ''}`,
        };
        if (options.port !== '') {
            optionsParsed.port = Number(options.port);
        }
        if (options.username || options.password) {
            optionsParsed.auth = `${options.username}:${options.password}`;
        }
        pathname = options.pathname;
        origin = options.origin;
        if (extraOptions !== undefined) {
            Object.assign(optionsParsed, extraOptions);
        }
    }
    else {
        optionsParsed = Object.assign({ protocol: options.host ? 'http:' : undefined }, options);
        pathname = options.pathname;
        if (!pathname && optionsParsed.path) {
            pathname = url.parse(optionsParsed.path).pathname || '/';
        }
        const hostname = optionsParsed.host ||
            (optionsParsed.port != null
                ? `${optionsParsed.hostname}${optionsParsed.port}`
                : optionsParsed.hostname);
        origin = `${optionsParsed.protocol || 'http:'}//${hostname}`;
    }
    // some packages return method in lowercase..
    // ensure upperCase for consistency
    const method = optionsParsed.method
        ? optionsParsed.method.toUpperCase()
        : 'GET';
    return { origin, pathname, method, optionsParsed };
};
exports.getRequestInfo = getRequestInfo;
/**
 * Makes sure options is of type string or object
 * @param options for the request
 */
const isValidOptionsType = (options) => {
    if (!options) {
        return false;
    }
    const type = typeof options;
    return type === 'string' || (type === 'object' && !Array.isArray(options));
};
exports.isValidOptionsType = isValidOptionsType;
const extractHostnameAndPort = (requestOptions) => {
    var _a;
    if (requestOptions.hostname && requestOptions.port) {
        return { hostname: requestOptions.hostname, port: requestOptions.port };
    }
    const matches = ((_a = requestOptions.host) === null || _a === void 0 ? void 0 : _a.match(/^([^:/ ]+)(:\d{1,5})?/)) || null;
    const hostname = requestOptions.hostname || (matches === null ? 'localhost' : matches[1]);
    let port = requestOptions.port;
    if (!port) {
        if (matches && matches[2]) {
            // remove the leading ":". The extracted port would be something like ":8080"
            port = matches[2].substring(1);
        }
        else {
            port = requestOptions.protocol === 'https:' ? '443' : '80';
        }
    }
    return { hostname, port };
};
exports.extractHostnameAndPort = extractHostnameAndPort;
/**
 * Returns outgoing request attributes scoped to the options passed to the request
 * @param {ParsedRequestOptions} requestOptions the same options used to make the request
 * @param {{ component: string, hostname: string, hookAttributes?: SpanAttributes }} options used to pass data needed to create attributes
 */
const getOutgoingRequestAttributes = (requestOptions, options) => {
    var _a;
    const hostname = options.hostname;
    const port = options.port;
    const requestMethod = requestOptions.method;
    const method = requestMethod ? requestMethod.toUpperCase() : 'GET';
    const headers = requestOptions.headers || {};
    const userAgent = headers['user-agent'];
    const attributes = {
        [semantic_conventions_1.SEMATTRS_HTTP_URL]: (0, exports.getAbsoluteUrl)(requestOptions, headers, `${options.component}:`),
        [semantic_conventions_1.SEMATTRS_HTTP_METHOD]: method,
        [semantic_conventions_1.SEMATTRS_HTTP_TARGET]: requestOptions.path || '/',
        [semantic_conventions_1.SEMATTRS_NET_PEER_NAME]: hostname,
        [semantic_conventions_1.SEMATTRS_HTTP_HOST]: (_a = headers.host) !== null && _a !== void 0 ? _a : `${hostname}:${port}`,
    };
    if (userAgent !== undefined) {
        attributes[semantic_conventions_1.SEMATTRS_HTTP_USER_AGENT] = userAgent;
    }
    return Object.assign(attributes, options.hookAttributes);
};
exports.getOutgoingRequestAttributes = getOutgoingRequestAttributes;
/**
 * Returns outgoing request Metric attributes scoped to the request data
 * @param {SpanAttributes} spanAttributes the span attributes
 */
const getOutgoingRequestMetricAttributes = (spanAttributes) => {
    const metricAttributes = {};
    metricAttributes[semantic_conventions_1.SEMATTRS_HTTP_METHOD] = spanAttributes[semantic_conventions_1.SEMATTRS_HTTP_METHOD];
    metricAttributes[semantic_conventions_1.SEMATTRS_NET_PEER_NAME] =
        spanAttributes[semantic_conventions_1.SEMATTRS_NET_PEER_NAME];
    //TODO: http.url attribute, it should substitute any parameters to avoid high cardinality.
    return metricAttributes;
};
exports.getOutgoingRequestMetricAttributes = getOutgoingRequestMetricAttributes;
/**
 * Returns attributes related to the kind of HTTP protocol used
 * @param {string} [kind] Kind of HTTP protocol used: "1.0", "1.1", "2", "SPDY" or "QUIC".
 */
const setAttributesFromHttpKind = (kind, attributes) => {
    if (kind) {
        attributes[semantic_conventions_1.SEMATTRS_HTTP_FLAVOR] = kind;
        if (kind.toUpperCase() !== 'QUIC') {
            attributes[semantic_conventions_1.SEMATTRS_NET_TRANSPORT] = semantic_conventions_1.NETTRANSPORTVALUES_IP_TCP;
        }
        else {
            attributes[semantic_conventions_1.SEMATTRS_NET_TRANSPORT] = semantic_conventions_1.NETTRANSPORTVALUES_IP_UDP;
        }
    }
};
exports.setAttributesFromHttpKind = setAttributesFromHttpKind;
/**
 * Returns outgoing request attributes scoped to the response data
 * @param {IncomingMessage} response the response object
 * @param {{ hostname: string }} options used to pass data needed to create attributes
 */
const getOutgoingRequestAttributesOnResponse = (response) => {
    const { statusCode, statusMessage, httpVersion, socket } = response;
    const attributes = {};
    if (socket) {
        const { remoteAddress, remotePort } = socket;
        attributes[semantic_conventions_1.SEMATTRS_NET_PEER_IP] = remoteAddress;
        attributes[semantic_conventions_1.SEMATTRS_NET_PEER_PORT] = remotePort;
    }
    (0, exports.setResponseContentLengthAttribute)(response, attributes);
    if (statusCode) {
        attributes[semantic_conventions_1.SEMATTRS_HTTP_STATUS_CODE] = statusCode;
        attributes[AttributeNames_1.AttributeNames.HTTP_STATUS_TEXT] = (statusMessage || '').toUpperCase();
    }
    (0, exports.setAttributesFromHttpKind)(httpVersion, attributes);
    return attributes;
};
exports.getOutgoingRequestAttributesOnResponse = getOutgoingRequestAttributesOnResponse;
/**
 * Returns outgoing request Metric attributes scoped to the response data
 * @param {SpanAttributes} spanAttributes the span attributes
 */
const getOutgoingRequestMetricAttributesOnResponse = (spanAttributes) => {
    const metricAttributes = {};
    metricAttributes[semantic_conventions_1.SEMATTRS_NET_PEER_PORT] =
        spanAttributes[semantic_conventions_1.SEMATTRS_NET_PEER_PORT];
    metricAttributes[semantic_conventions_1.SEMATTRS_HTTP_STATUS_CODE] =
        spanAttributes[semantic_conventions_1.SEMATTRS_HTTP_STATUS_CODE];
    metricAttributes[semantic_conventions_1.SEMATTRS_HTTP_FLAVOR] = spanAttributes[semantic_conventions_1.SEMATTRS_HTTP_FLAVOR];
    return metricAttributes;
};
exports.getOutgoingRequestMetricAttributesOnResponse = getOutgoingRequestMetricAttributesOnResponse;
/**
 * Returns incoming request attributes scoped to the request data
 * @param {IncomingMessage} request the request object
 * @param {{ component: string, serverName?: string, hookAttributes?: SpanAttributes }} options used to pass data needed to create attributes
 */
const getIncomingRequestAttributes = (request, options) => {
    const headers = request.headers;
    const userAgent = headers['user-agent'];
    const ips = headers['x-forwarded-for'];
    const method = request.method || 'GET';
    const httpVersion = request.httpVersion;
    const requestUrl = request.url ? url.parse(request.url) : null;
    const host = (requestUrl === null || requestUrl === void 0 ? void 0 : requestUrl.host) || headers.host;
    const hostname = (requestUrl === null || requestUrl === void 0 ? void 0 : requestUrl.hostname) ||
        (host === null || host === void 0 ? void 0 : host.replace(/^(.*)(:[0-9]{1,5})/, '$1')) ||
        'localhost';
    const serverName = options.serverName;
    const attributes = {
        [semantic_conventions_1.SEMATTRS_HTTP_URL]: (0, exports.getAbsoluteUrl)(requestUrl, headers, `${options.component}:`),
        [semantic_conventions_1.SEMATTRS_HTTP_HOST]: host,
        [semantic_conventions_1.SEMATTRS_NET_HOST_NAME]: hostname,
        [semantic_conventions_1.SEMATTRS_HTTP_METHOD]: method,
        [semantic_conventions_1.SEMATTRS_HTTP_SCHEME]: options.component,
    };
    if (typeof ips === 'string') {
        attributes[semantic_conventions_1.SEMATTRS_HTTP_CLIENT_IP] = ips.split(',')[0];
    }
    if (typeof serverName === 'string') {
        attributes[semantic_conventions_1.SEMATTRS_HTTP_SERVER_NAME] = serverName;
    }
    if (requestUrl) {
        attributes[semantic_conventions_1.SEMATTRS_HTTP_TARGET] = requestUrl.path || '/';
    }
    if (userAgent !== undefined) {
        attributes[semantic_conventions_1.SEMATTRS_HTTP_USER_AGENT] = userAgent;
    }
    (0, exports.setRequestContentLengthAttribute)(request, attributes);
    (0, exports.setAttributesFromHttpKind)(httpVersion, attributes);
    return Object.assign(attributes, options.hookAttributes);
};
exports.getIncomingRequestAttributes = getIncomingRequestAttributes;
/**
 * Returns incoming request Metric attributes scoped to the request data
 * @param {SpanAttributes} spanAttributes the span attributes
 * @param {{ component: string }} options used to pass data needed to create attributes
 */
const getIncomingRequestMetricAttributes = (spanAttributes) => {
    const metricAttributes = {};
    metricAttributes[semantic_conventions_1.SEMATTRS_HTTP_SCHEME] = spanAttributes[semantic_conventions_1.SEMATTRS_HTTP_SCHEME];
    metricAttributes[semantic_conventions_1.SEMATTRS_HTTP_METHOD] = spanAttributes[semantic_conventions_1.SEMATTRS_HTTP_METHOD];
    metricAttributes[semantic_conventions_1.SEMATTRS_NET_HOST_NAME] =
        spanAttributes[semantic_conventions_1.SEMATTRS_NET_HOST_NAME];
    metricAttributes[semantic_conventions_1.SEMATTRS_HTTP_FLAVOR] = spanAttributes[semantic_conventions_1.SEMATTRS_HTTP_FLAVOR];
    //TODO: http.target attribute, it should substitute any parameters to avoid high cardinality.
    return metricAttributes;
};
exports.getIncomingRequestMetricAttributes = getIncomingRequestMetricAttributes;
/**
 * Returns incoming request attributes scoped to the response data
 * @param {(ServerResponse & { socket: Socket; })} response the response object
 */
const getIncomingRequestAttributesOnResponse = (request, response) => {
    // take socket from the request,
    // since it may be detached from the response object in keep-alive mode
    const { socket } = request;
    const { statusCode, statusMessage } = response;
    const rpcMetadata = (0, core_1.getRPCMetadata)(api_1.context.active());
    const attributes = {};
    if (socket) {
        const { localAddress, localPort, remoteAddress, remotePort } = socket;
        attributes[semantic_conventions_1.SEMATTRS_NET_HOST_IP] = localAddress;
        attributes[semantic_conventions_1.SEMATTRS_NET_HOST_PORT] = localPort;
        attributes[semantic_conventions_1.SEMATTRS_NET_PEER_IP] = remoteAddress;
        attributes[semantic_conventions_1.SEMATTRS_NET_PEER_PORT] = remotePort;
    }
    attributes[semantic_conventions_1.SEMATTRS_HTTP_STATUS_CODE] = statusCode;
    attributes[AttributeNames_1.AttributeNames.HTTP_STATUS_TEXT] = (statusMessage || '').toUpperCase();
    if ((rpcMetadata === null || rpcMetadata === void 0 ? void 0 : rpcMetadata.type) === core_1.RPCType.HTTP && rpcMetadata.route !== undefined) {
        attributes[semantic_conventions_1.SEMATTRS_HTTP_ROUTE] = rpcMetadata.route;
    }
    return attributes;
};
exports.getIncomingRequestAttributesOnResponse = getIncomingRequestAttributesOnResponse;
/**
 * Returns incoming request Metric attributes scoped to the request data
 * @param {SpanAttributes} spanAttributes the span attributes
 */
const getIncomingRequestMetricAttributesOnResponse = (spanAttributes) => {
    const metricAttributes = {};
    metricAttributes[semantic_conventions_1.SEMATTRS_HTTP_STATUS_CODE] =
        spanAttributes[semantic_conventions_1.SEMATTRS_HTTP_STATUS_CODE];
    metricAttributes[semantic_conventions_1.SEMATTRS_NET_HOST_PORT] =
        spanAttributes[semantic_conventions_1.SEMATTRS_NET_HOST_PORT];
    if (spanAttributes[semantic_conventions_1.SEMATTRS_HTTP_ROUTE] !== undefined) {
        metricAttributes[semantic_conventions_1.SEMATTRS_HTTP_ROUTE] = spanAttributes[semantic_conventions_1.SEMATTRS_HTTP_ROUTE];
    }
    return metricAttributes;
};
exports.getIncomingRequestMetricAttributesOnResponse = getIncomingRequestMetricAttributesOnResponse;
function headerCapture(type, headers) {
    const normalizedHeaders = new Map();
    for (let i = 0, len = headers.length; i < len; i++) {
        const capturedHeader = headers[i].toLowerCase();
        normalizedHeaders.set(capturedHeader, capturedHeader.replace(/-/g, '_'));
    }
    return (span, getHeader) => {
        for (const capturedHeader of normalizedHeaders.keys()) {
            const value = getHeader(capturedHeader);
            if (value === undefined) {
                continue;
            }
            const normalizedHeader = normalizedHeaders.get(capturedHeader);
            const key = `http.${type}.header.${normalizedHeader}`;
            if (typeof value === 'string') {
                span.setAttribute(key, [value]);
            }
            else if (Array.isArray(value)) {
                span.setAttribute(key, value);
            }
            else {
                span.setAttribute(key, [value]);
            }
        }
    };
}
exports.headerCapture = headerCapture;
//# sourceMappingURL=utils.js.map