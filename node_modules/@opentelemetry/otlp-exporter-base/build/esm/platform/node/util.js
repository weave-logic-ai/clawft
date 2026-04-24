var __assign = (this && this.__assign) || function () {
    __assign = Object.assign || function(t) {
        for (var s, i = 1, n = arguments.length; i < n; i++) {
            s = arguments[i];
            for (var p in s) if (Object.prototype.hasOwnProperty.call(s, p))
                t[p] = s[p];
        }
        return t;
    };
    return __assign.apply(this, arguments);
};
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
import * as url from 'url';
import * as http from 'http';
import * as https from 'https';
import * as zlib from 'zlib';
import { Readable } from 'stream';
import { diag } from '@opentelemetry/api';
import { CompressionAlgorithm } from './types';
import { getEnv } from '@opentelemetry/core';
import { OTLPExporterError } from '../../types';
import { DEFAULT_EXPORT_MAX_ATTEMPTS, DEFAULT_EXPORT_INITIAL_BACKOFF, DEFAULT_EXPORT_BACKOFF_MULTIPLIER, DEFAULT_EXPORT_MAX_BACKOFF, isExportRetryable, parseRetryAfterToMills, } from '../../util';
/**
 * Sends data using http
 * @param collector
 * @param data
 * @param contentType
 * @param onSuccess
 * @param onError
 */
export function sendWithHttp(collector, data, contentType, onSuccess, onError) {
    var exporterTimeout = collector.timeoutMillis;
    var parsedUrl = new url.URL(collector.url);
    var nodeVersion = Number(process.versions.node.split('.')[0]);
    var retryTimer;
    var req;
    var reqIsDestroyed = false;
    var exporterTimer = setTimeout(function () {
        clearTimeout(retryTimer);
        reqIsDestroyed = true;
        if (req.destroyed) {
            var err = new OTLPExporterError('Request Timeout');
            onError(err);
        }
        else {
            // req.abort() was deprecated since v14
            nodeVersion >= 14 ? req.destroy() : req.abort();
        }
    }, exporterTimeout);
    var options = {
        hostname: parsedUrl.hostname,
        port: parsedUrl.port,
        path: parsedUrl.pathname,
        method: 'POST',
        headers: __assign({ 'Content-Type': contentType }, collector.headers),
        agent: collector.agent,
    };
    var request = parsedUrl.protocol === 'http:' ? http.request : https.request;
    var sendWithRetry = function (retries, minDelay) {
        if (retries === void 0) { retries = DEFAULT_EXPORT_MAX_ATTEMPTS; }
        if (minDelay === void 0) { minDelay = DEFAULT_EXPORT_INITIAL_BACKOFF; }
        req = request(options, function (res) {
            var responseData = '';
            res.on('data', function (chunk) { return (responseData += chunk); });
            res.on('aborted', function () {
                if (reqIsDestroyed) {
                    var err = new OTLPExporterError('Request Timeout');
                    onError(err);
                }
            });
            res.on('end', function () {
                if (reqIsDestroyed === false) {
                    if (res.statusCode && res.statusCode < 299) {
                        diag.debug("statusCode: " + res.statusCode, responseData);
                        onSuccess();
                        // clear all timers since request was completed and promise was resolved
                        clearTimeout(exporterTimer);
                        clearTimeout(retryTimer);
                    }
                    else if (res.statusCode &&
                        isExportRetryable(res.statusCode) &&
                        retries > 0) {
                        var retryTime = void 0;
                        minDelay = DEFAULT_EXPORT_BACKOFF_MULTIPLIER * minDelay;
                        // retry after interval specified in Retry-After header
                        if (res.headers['retry-after']) {
                            retryTime = parseRetryAfterToMills(res.headers['retry-after']);
                        }
                        else {
                            // exponential backoff with jitter
                            retryTime = Math.round(Math.random() * (DEFAULT_EXPORT_MAX_BACKOFF - minDelay) +
                                minDelay);
                        }
                        retryTimer = setTimeout(function () {
                            sendWithRetry(retries - 1, minDelay);
                        }, retryTime);
                    }
                    else {
                        var error = new OTLPExporterError(res.statusMessage, res.statusCode, responseData);
                        onError(error);
                        // clear all timers since request was completed and promise was resolved
                        clearTimeout(exporterTimer);
                        clearTimeout(retryTimer);
                    }
                }
            });
        });
        req.on('error', function (error) {
            if (reqIsDestroyed) {
                var err = new OTLPExporterError('Request Timeout', error.code);
                onError(err);
            }
            else {
                onError(error);
            }
            clearTimeout(exporterTimer);
            clearTimeout(retryTimer);
        });
        req.on('abort', function () {
            if (reqIsDestroyed) {
                var err = new OTLPExporterError('Request Timeout');
                onError(err);
            }
            clearTimeout(exporterTimer);
            clearTimeout(retryTimer);
        });
        switch (collector.compression) {
            case CompressionAlgorithm.GZIP: {
                req.setHeader('Content-Encoding', 'gzip');
                var dataStream = readableFromUnit8Array(data);
                dataStream
                    .on('error', onError)
                    .pipe(zlib.createGzip())
                    .on('error', onError)
                    .pipe(req);
                break;
            }
            default:
                req.end(Buffer.from(data));
                break;
        }
    };
    sendWithRetry();
}
function readableFromUnit8Array(buff) {
    var readable = new Readable();
    readable.push(buff);
    readable.push(null);
    return readable;
}
export function createHttpAgent(config) {
    if (config.httpAgentOptions && config.keepAlive === false) {
        diag.warn('httpAgentOptions is used only when keepAlive is true');
        return undefined;
    }
    if (config.keepAlive === false || !config.url)
        return undefined;
    try {
        var parsedUrl = new url.URL(config.url);
        var Agent = parsedUrl.protocol === 'http:' ? http.Agent : https.Agent;
        return new Agent(__assign({ keepAlive: true }, config.httpAgentOptions));
    }
    catch (err) {
        diag.error("collector exporter failed to create http agent. err: " + err.message);
        return undefined;
    }
}
export function configureCompression(compression) {
    if (compression) {
        return compression;
    }
    else {
        var definedCompression = getEnv().OTEL_EXPORTER_OTLP_TRACES_COMPRESSION ||
            getEnv().OTEL_EXPORTER_OTLP_COMPRESSION;
        return definedCompression === CompressionAlgorithm.GZIP
            ? CompressionAlgorithm.GZIP
            : CompressionAlgorithm.NONE;
    }
}
//# sourceMappingURL=util.js.map