"use strict";
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
Object.defineProperty(exports, "__esModule", { value: true });
exports.configureCompression = exports.getCredentialsFromEnvironment = exports.configureCredentials = exports.validateAndNormalizeUrl = exports.DEFAULT_COLLECTOR_URL = void 0;
const api_1 = require("@opentelemetry/api");
const core_1 = require("@opentelemetry/core");
const path = require("path");
const url_1 = require("url");
const fs = require("fs");
const otlp_exporter_base_1 = require("@opentelemetry/otlp-exporter-base");
const grpc_exporter_transport_1 = require("./grpc-exporter-transport");
exports.DEFAULT_COLLECTOR_URL = 'http://localhost:4317';
function validateAndNormalizeUrl(url) {
    var _a;
    const hasProtocol = url.match(/^([\w]{1,8}):\/\//);
    if (!hasProtocol) {
        url = `https://${url}`;
    }
    const target = new url_1.URL(url);
    if (target.protocol === 'unix:') {
        return url;
    }
    if (target.pathname && target.pathname !== '/') {
        api_1.diag.warn('URL path should not be set when using grpc, the path part of the URL will be ignored.');
    }
    if (target.protocol !== '' && !((_a = target.protocol) === null || _a === void 0 ? void 0 : _a.match(/^(http)s?:$/))) {
        api_1.diag.warn('URL protocol should be http(s)://. Using http://.');
    }
    return target.host;
}
exports.validateAndNormalizeUrl = validateAndNormalizeUrl;
function configureCredentials(credentials, endpoint) {
    let insecure;
    if (credentials) {
        return credentials;
    }
    else if (endpoint.startsWith('https://')) {
        insecure = false;
    }
    else if (endpoint.startsWith('http://') ||
        endpoint === exports.DEFAULT_COLLECTOR_URL) {
        insecure = true;
    }
    else {
        insecure = getSecurityFromEnv();
    }
    if (insecure) {
        return (0, grpc_exporter_transport_1.createInsecureCredentials)();
    }
    else {
        return getCredentialsFromEnvironment();
    }
}
exports.configureCredentials = configureCredentials;
function getSecurityFromEnv() {
    const definedInsecure = (0, core_1.getEnv)().OTEL_EXPORTER_OTLP_TRACES_INSECURE ||
        (0, core_1.getEnv)().OTEL_EXPORTER_OTLP_INSECURE;
    if (definedInsecure) {
        return definedInsecure.toLowerCase() === 'true';
    }
    else {
        return false;
    }
}
/**
 * Exported for testing
 */
function getCredentialsFromEnvironment() {
    const rootCert = retrieveRootCert();
    const privateKey = retrievePrivateKey();
    const certChain = retrieveCertChain();
    return (0, grpc_exporter_transport_1.createSslCredentials)(rootCert, privateKey, certChain);
}
exports.getCredentialsFromEnvironment = getCredentialsFromEnvironment;
function retrieveRootCert() {
    const rootCertificate = (0, core_1.getEnv)().OTEL_EXPORTER_OTLP_TRACES_CERTIFICATE ||
        (0, core_1.getEnv)().OTEL_EXPORTER_OTLP_CERTIFICATE;
    if (rootCertificate) {
        try {
            return fs.readFileSync(path.resolve(process.cwd(), rootCertificate));
        }
        catch (_a) {
            api_1.diag.warn('Failed to read root certificate file');
            return undefined;
        }
    }
    else {
        return undefined;
    }
}
function retrievePrivateKey() {
    const clientKey = (0, core_1.getEnv)().OTEL_EXPORTER_OTLP_TRACES_CLIENT_KEY ||
        (0, core_1.getEnv)().OTEL_EXPORTER_OTLP_CLIENT_KEY;
    if (clientKey) {
        try {
            return fs.readFileSync(path.resolve(process.cwd(), clientKey));
        }
        catch (_a) {
            api_1.diag.warn('Failed to read client certificate private key file');
            return undefined;
        }
    }
    else {
        return undefined;
    }
}
function retrieveCertChain() {
    const clientChain = (0, core_1.getEnv)().OTEL_EXPORTER_OTLP_TRACES_CLIENT_CERTIFICATE ||
        (0, core_1.getEnv)().OTEL_EXPORTER_OTLP_CLIENT_CERTIFICATE;
    if (clientChain) {
        try {
            return fs.readFileSync(path.resolve(process.cwd(), clientChain));
        }
        catch (_a) {
            api_1.diag.warn('Failed to read client certificate chain file');
            return undefined;
        }
    }
    else {
        return undefined;
    }
}
function configureCompression(compression) {
    if (compression != null) {
        return compression;
    }
    const envCompression = (0, core_1.getEnv)().OTEL_EXPORTER_OTLP_TRACES_COMPRESSION ||
        (0, core_1.getEnv)().OTEL_EXPORTER_OTLP_COMPRESSION;
    if (envCompression === 'gzip') {
        return otlp_exporter_base_1.CompressionAlgorithm.GZIP;
    }
    else if (envCompression === 'none') {
        return otlp_exporter_base_1.CompressionAlgorithm.NONE;
    }
    api_1.diag.warn('Unknown compression "' + envCompression + '", falling back to "none"');
    return otlp_exporter_base_1.CompressionAlgorithm.NONE;
}
exports.configureCompression = configureCompression;
//# sourceMappingURL=util.js.map