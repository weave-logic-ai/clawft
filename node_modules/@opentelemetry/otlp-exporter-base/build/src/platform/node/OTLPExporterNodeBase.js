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
exports.OTLPExporterNodeBase = void 0;
const OTLPExporterBase_1 = require("../../OTLPExporterBase");
const util_1 = require("../../util");
const util_2 = require("./util");
const api_1 = require("@opentelemetry/api");
const core_1 = require("@opentelemetry/core");
/**
 * Collector Metric Exporter abstract base class
 */
class OTLPExporterNodeBase extends OTLPExporterBase_1.OTLPExporterBase {
    constructor(config = {}, serializer, contentType) {
        super(config);
        this.DEFAULT_HEADERS = {};
        this._contentType = contentType;
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        if (config.metadata) {
            api_1.diag.warn('Metadata cannot be set when using http');
        }
        this.headers = Object.assign(this.DEFAULT_HEADERS, (0, util_1.parseHeaders)(config.headers), core_1.baggageUtils.parseKeyPairsIntoRecord((0, core_1.getEnv)().OTEL_EXPORTER_OTLP_HEADERS));
        this.agent = (0, util_2.createHttpAgent)(config);
        this.compression = (0, util_2.configureCompression)(config.compression);
        this._serializer = serializer;
    }
    onInit(_config) { }
    send(objects, onSuccess, onError) {
        if (this._shutdownOnce.isCalled) {
            api_1.diag.debug('Shutdown already started. Cannot send objects');
            return;
        }
        const promise = new Promise((resolve, reject) => {
            var _a;
            (0, util_2.sendWithHttp)(this, (_a = this._serializer.serializeRequest(objects)) !== null && _a !== void 0 ? _a : new Uint8Array(), this._contentType, resolve, reject);
        }).then(onSuccess, onError);
        this._sendingPromises.push(promise);
        const popPromise = () => {
            const index = this._sendingPromises.indexOf(promise);
            this._sendingPromises.splice(index, 1);
        };
        promise.then(popPromise, popPromise);
    }
    onShutdown() { }
}
exports.OTLPExporterNodeBase = OTLPExporterNodeBase;
//# sourceMappingURL=OTLPExporterNodeBase.js.map