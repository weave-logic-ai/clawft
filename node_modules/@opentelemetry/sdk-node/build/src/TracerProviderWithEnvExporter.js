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
var _a;
Object.defineProperty(exports, "__esModule", { value: true });
exports.TracerProviderWithEnvExporters = void 0;
const api_1 = require("@opentelemetry/api");
const core_1 = require("@opentelemetry/core");
const sdk_trace_base_1 = require("@opentelemetry/sdk-trace-base");
const sdk_trace_node_1 = require("@opentelemetry/sdk-trace-node");
const exporter_trace_otlp_proto_1 = require("@opentelemetry/exporter-trace-otlp-proto");
const exporter_trace_otlp_http_1 = require("@opentelemetry/exporter-trace-otlp-http");
const exporter_trace_otlp_grpc_1 = require("@opentelemetry/exporter-trace-otlp-grpc");
const exporter_zipkin_1 = require("@opentelemetry/exporter-zipkin");
class TracerProviderWithEnvExporters extends sdk_trace_node_1.NodeTracerProvider {
    constructor(config = {}) {
        super(config);
        this._configuredExporters = [];
        this._hasSpanProcessors = false;
        let traceExportersList = this.filterBlanksAndNulls(Array.from(new Set((0, core_1.getEnv)().OTEL_TRACES_EXPORTER.split(','))));
        if (traceExportersList[0] === 'none') {
            api_1.diag.warn('OTEL_TRACES_EXPORTER contains "none". SDK will not be initialized.');
        }
        else if (traceExportersList.length === 0) {
            api_1.diag.warn('OTEL_TRACES_EXPORTER is empty. Using default otlp exporter.');
            traceExportersList = ['otlp'];
            this.createExportersFromList(traceExportersList);
            this._spanProcessors = this.configureSpanProcessors(this._configuredExporters);
            this._spanProcessors.forEach(processor => {
                this.addSpanProcessor(processor);
            });
        }
        else {
            if (traceExportersList.length > 1 &&
                traceExportersList.includes('none')) {
                api_1.diag.warn('OTEL_TRACES_EXPORTER contains "none" along with other exporters. Using default otlp exporter.');
                traceExportersList = ['otlp'];
            }
            this.createExportersFromList(traceExportersList);
            if (this._configuredExporters.length > 0) {
                this._spanProcessors = this.configureSpanProcessors(this._configuredExporters);
                this._spanProcessors.forEach(processor => {
                    this.addSpanProcessor(processor);
                });
            }
            else {
                api_1.diag.warn('Unable to set up trace exporter(s) due to invalid exporter and/or protocol values.');
            }
        }
    }
    static configureOtlp() {
        const protocol = this.getOtlpProtocol();
        switch (protocol) {
            case 'grpc':
                return new exporter_trace_otlp_grpc_1.OTLPTraceExporter();
            case 'http/json':
                return new exporter_trace_otlp_http_1.OTLPTraceExporter();
            case 'http/protobuf':
                return new exporter_trace_otlp_proto_1.OTLPTraceExporter();
            default:
                api_1.diag.warn(`Unsupported OTLP traces protocol: ${protocol}. Using http/protobuf.`);
                return new exporter_trace_otlp_proto_1.OTLPTraceExporter();
        }
    }
    static getOtlpProtocol() {
        var _b, _c, _d;
        const parsedEnvValues = (0, core_1.getEnvWithoutDefaults)();
        return ((_d = (_c = (_b = parsedEnvValues.OTEL_EXPORTER_OTLP_TRACES_PROTOCOL) !== null && _b !== void 0 ? _b : parsedEnvValues.OTEL_EXPORTER_OTLP_PROTOCOL) !== null && _c !== void 0 ? _c : (0, core_1.getEnv)().OTEL_EXPORTER_OTLP_TRACES_PROTOCOL) !== null && _d !== void 0 ? _d : (0, core_1.getEnv)().OTEL_EXPORTER_OTLP_PROTOCOL);
    }
    static configureJaeger() {
        // The JaegerExporter does not support being required in bundled
        // environments. By delaying the require statement to here, we only crash when
        // the exporter is actually used in such an environment.
        try {
            // eslint-disable-next-line @typescript-eslint/no-var-requires
            const { JaegerExporter } = require('@opentelemetry/exporter-jaeger');
            return new JaegerExporter();
        }
        catch (e) {
            throw new Error(`Could not instantiate JaegerExporter. This could be due to the JaegerExporter's lack of support for bundling. If possible, use @opentelemetry/exporter-trace-otlp-proto instead. Original Error: ${e}`);
        }
    }
    addSpanProcessor(spanProcessor) {
        super.addSpanProcessor(spanProcessor);
        this._hasSpanProcessors = true;
    }
    register(config) {
        if (this._hasSpanProcessors) {
            super.register(config);
        }
    }
    createExportersFromList(exporterList) {
        exporterList.forEach(exporterName => {
            const exporter = this._getSpanExporter(exporterName);
            if (exporter) {
                this._configuredExporters.push(exporter);
            }
            else {
                api_1.diag.warn(`Unrecognized OTEL_TRACES_EXPORTER value: ${exporterName}.`);
            }
        });
    }
    configureSpanProcessors(exporters) {
        return exporters.map(exporter => {
            if (exporter instanceof sdk_trace_base_1.ConsoleSpanExporter) {
                return new sdk_trace_base_1.SimpleSpanProcessor(exporter);
            }
            else {
                return new sdk_trace_base_1.BatchSpanProcessor(exporter);
            }
        });
    }
    filterBlanksAndNulls(list) {
        return list.map(item => item.trim()).filter(s => s !== 'null' && s !== '');
    }
}
exports.TracerProviderWithEnvExporters = TracerProviderWithEnvExporters;
_a = TracerProviderWithEnvExporters;
TracerProviderWithEnvExporters._registeredExporters = new Map([
    ['otlp', () => _a.configureOtlp()],
    ['zipkin', () => new exporter_zipkin_1.ZipkinExporter()],
    ['jaeger', () => _a.configureJaeger()],
    ['console', () => new sdk_trace_base_1.ConsoleSpanExporter()],
]);
//# sourceMappingURL=TracerProviderWithEnvExporter.js.map