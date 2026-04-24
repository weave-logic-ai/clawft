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
var __read = (this && this.__read) || function (o, n) {
    var m = typeof Symbol === "function" && o[Symbol.iterator];
    if (!m) return o;
    var i = m.call(o), r, ar = [], e;
    try {
        while ((n === void 0 || n-- > 0) && !(r = i.next()).done) ar.push(r.value);
    }
    catch (error) { e = { error: error }; }
    finally {
        try {
            if (r && !r.done && (m = i["return"])) m.call(i);
        }
        finally { if (e) throw e.error; }
    }
    return ar;
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
import { diag } from '@opentelemetry/api';
import { OTLPExporterError } from '../../types';
import { DEFAULT_EXPORT_MAX_ATTEMPTS, DEFAULT_EXPORT_INITIAL_BACKOFF, DEFAULT_EXPORT_BACKOFF_MULTIPLIER, DEFAULT_EXPORT_MAX_BACKOFF, isExportRetryable, parseRetryAfterToMills, } from '../../util';
/**
 * Send metrics/spans using browser navigator.sendBeacon
 * @param body
 * @param url
 * @param blobPropertyBag
 * @param onSuccess
 * @param onError
 */
export function sendWithBeacon(body, url, blobPropertyBag, onSuccess, onError) {
    if (navigator.sendBeacon(url, new Blob([body], blobPropertyBag))) {
        diag.debug('sendBeacon - can send', body);
        onSuccess();
    }
    else {
        var error = new OTLPExporterError("sendBeacon - cannot send " + body);
        onError(error);
    }
}
/**
 * function to send metrics/spans using browser XMLHttpRequest
 *     used when navigator.sendBeacon is not available
 * @param body
 * @param url
 * @param headers
 * @param onSuccess
 * @param onError
 */
export function sendWithXhr(body, url, headers, exporterTimeout, onSuccess, onError) {
    var retryTimer;
    var xhr;
    var reqIsDestroyed = false;
    var exporterTimer = setTimeout(function () {
        clearTimeout(retryTimer);
        reqIsDestroyed = true;
        if (xhr.readyState === XMLHttpRequest.DONE) {
            var err = new OTLPExporterError('Request Timeout');
            onError(err);
        }
        else {
            xhr.abort();
        }
    }, exporterTimeout);
    var sendWithRetry = function (retries, minDelay) {
        if (retries === void 0) { retries = DEFAULT_EXPORT_MAX_ATTEMPTS; }
        if (minDelay === void 0) { minDelay = DEFAULT_EXPORT_INITIAL_BACKOFF; }
        xhr = new XMLHttpRequest();
        xhr.open('POST', url);
        var defaultHeaders = {
            Accept: 'application/json',
            'Content-Type': 'application/json',
        };
        Object.entries(__assign(__assign({}, defaultHeaders), headers)).forEach(function (_a) {
            var _b = __read(_a, 2), k = _b[0], v = _b[1];
            xhr.setRequestHeader(k, v);
        });
        xhr.send(body);
        xhr.onreadystatechange = function () {
            if (xhr.readyState === XMLHttpRequest.DONE && reqIsDestroyed === false) {
                if (xhr.status >= 200 && xhr.status <= 299) {
                    diag.debug('xhr success', body);
                    onSuccess();
                    clearTimeout(exporterTimer);
                    clearTimeout(retryTimer);
                }
                else if (xhr.status && isExportRetryable(xhr.status) && retries > 0) {
                    var retryTime = void 0;
                    minDelay = DEFAULT_EXPORT_BACKOFF_MULTIPLIER * minDelay;
                    // retry after interval specified in Retry-After header
                    if (xhr.getResponseHeader('Retry-After')) {
                        retryTime = parseRetryAfterToMills(xhr.getResponseHeader('Retry-After'));
                    }
                    else {
                        // exponential backoff with jitter
                        retryTime = Math.round(Math.random() * (DEFAULT_EXPORT_MAX_BACKOFF - minDelay) + minDelay);
                    }
                    retryTimer = setTimeout(function () {
                        sendWithRetry(retries - 1, minDelay);
                    }, retryTime);
                }
                else {
                    var error = new OTLPExporterError("Failed to export with XHR (status: " + xhr.status + ")", xhr.status);
                    onError(error);
                    clearTimeout(exporterTimer);
                    clearTimeout(retryTimer);
                }
            }
        };
        xhr.onabort = function () {
            if (reqIsDestroyed) {
                var err = new OTLPExporterError('Request Timeout');
                onError(err);
            }
            clearTimeout(exporterTimer);
            clearTimeout(retryTimer);
        };
        xhr.onerror = function () {
            if (reqIsDestroyed) {
                var err = new OTLPExporterError('Request Timeout');
                onError(err);
            }
            clearTimeout(exporterTimer);
            clearTimeout(retryTimer);
        };
    };
    sendWithRetry();
}
//# sourceMappingURL=util.js.map