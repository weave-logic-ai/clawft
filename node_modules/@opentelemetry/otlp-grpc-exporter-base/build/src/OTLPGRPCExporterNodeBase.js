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
exports.OTLPGRPCExporterNodeBase = void 0;
const api_1 = require("@opentelemetry/api");
const core_1 = require("@opentelemetry/core");
const otlp_exporter_base_1 = require("@opentelemetry/otlp-exporter-base");
const grpc_exporter_transport_1 = require("./grpc-exporter-transport");
const util_1 = require("./util");
/**
 * OTLP Exporter abstract base class
 */
class OTLPGRPCExporterNodeBase extends otlp_exporter_base_1.OTLPExporterBase {
    constructor(config = {}, signalSpecificMetadata, grpcName, grpcPath, serializer) {
        var _a;
        super(config);
        this.grpcQueue = [];
        this._serializer = serializer;
        if (config.headers) {
            api_1.diag.warn('Headers cannot be set when using grpc');
        }
        const nonSignalSpecificMetadata = core_1.baggageUtils.parseKeyPairsIntoRecord((0, core_1.getEnv)().OTEL_EXPORTER_OTLP_HEADERS);
        const rawMetadata = Object.assign({}, nonSignalSpecificMetadata, signalSpecificMetadata);
        let credentialProvider = () => {
            return (0, util_1.configureCredentials)(undefined, this.getUrlFromConfig(config));
        };
        if (config.credentials != null) {
            const credentials = config.credentials;
            credentialProvider = () => {
                return credentials;
            };
        }
        // Ensure we don't modify the original.
        const configMetadata = (_a = config.metadata) === null || _a === void 0 ? void 0 : _a.clone();
        const metadataProvider = () => {
            const metadata = configMetadata !== null && configMetadata !== void 0 ? configMetadata : (0, grpc_exporter_transport_1.createEmptyMetadata)();
            for (const [key, value] of Object.entries(rawMetadata)) {
                // only override with env var data if the key has no values.
                // not using Metadata.merge() as it will keep both values.
                if (metadata.get(key).length < 1) {
                    metadata.set(key, value);
                }
            }
            return metadata;
        };
        this.compression = (0, util_1.configureCompression)(config.compression);
        this._transport = new grpc_exporter_transport_1.GrpcExporterTransport({
            address: this.getDefaultUrl(config),
            compression: this.compression,
            credentials: credentialProvider,
            grpcName: grpcName,
            grpcPath: grpcPath,
            metadata: metadataProvider,
            timeoutMillis: this.timeoutMillis,
        });
    }
    onInit() {
        // Intentionally left empty; nothing to do.
    }
    onShutdown() {
        this._transport.shutdown();
    }
    send(objects, onSuccess, onError) {
        if (this._shutdownOnce.isCalled) {
            api_1.diag.debug('Shutdown already started. Cannot send objects');
            return;
        }
        const data = this._serializer.serializeRequest(objects);
        if (data == null) {
            onError(new Error('Could not serialize message'));
            return;
        }
        const promise = this._transport.send(data).then(response => {
            if (response.status === 'success') {
                onSuccess();
                return;
            }
            if (response.status === 'failure' && response.error) {
                onError(response.error);
            }
            onError(new otlp_exporter_base_1.OTLPExporterError('Export failed with unknown error'));
        }, onError);
        this._sendingPromises.push(promise);
        const popPromise = () => {
            const index = this._sendingPromises.indexOf(promise);
            this._sendingPromises.splice(index, 1);
        };
        promise.then(popPromise, popPromise);
    }
}
exports.OTLPGRPCExporterNodeBase = OTLPGRPCExporterNodeBase;
//# sourceMappingURL=OTLPGRPCExporterNodeBase.js.map