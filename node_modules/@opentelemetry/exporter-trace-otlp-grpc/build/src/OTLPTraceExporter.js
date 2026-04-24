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
exports.OTLPTraceExporter = void 0;
const core_1 = require("@opentelemetry/core");
const otlp_grpc_exporter_base_1 = require("@opentelemetry/otlp-grpc-exporter-base");
const otlp_transformer_1 = require("@opentelemetry/otlp-transformer");
const version_1 = require("./version");
const USER_AGENT = {
    'User-Agent': `OTel-OTLP-Exporter-JavaScript/${version_1.VERSION}`,
};
/**
 * OTLP Trace Exporter for Node
 */
class OTLPTraceExporter extends otlp_grpc_exporter_base_1.OTLPGRPCExporterNodeBase {
    constructor(config = {}) {
        const signalSpecificMetadata = Object.assign(Object.assign({}, USER_AGENT), core_1.baggageUtils.parseKeyPairsIntoRecord((0, core_1.getEnv)().OTEL_EXPORTER_OTLP_TRACES_HEADERS));
        super(config, signalSpecificMetadata, 'TraceExportService', '/opentelemetry.proto.collector.trace.v1.TraceService/Export', otlp_transformer_1.ProtobufTraceSerializer);
    }
    getDefaultUrl(config) {
        return (0, otlp_grpc_exporter_base_1.validateAndNormalizeUrl)(this.getUrlFromConfig(config));
    }
    getUrlFromConfig(config) {
        if (typeof config.url === 'string') {
            return config.url;
        }
        return ((0, core_1.getEnv)().OTEL_EXPORTER_OTLP_TRACES_ENDPOINT ||
            (0, core_1.getEnv)().OTEL_EXPORTER_OTLP_ENDPOINT ||
            otlp_grpc_exporter_base_1.DEFAULT_COLLECTOR_URL);
    }
}
exports.OTLPTraceExporter = OTLPTraceExporter;
//# sourceMappingURL=OTLPTraceExporter.js.map