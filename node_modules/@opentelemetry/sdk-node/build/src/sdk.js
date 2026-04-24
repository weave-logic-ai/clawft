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
exports.NodeSDK = void 0;
const api_1 = require("@opentelemetry/api");
const api_logs_1 = require("@opentelemetry/api-logs");
const instrumentation_1 = require("@opentelemetry/instrumentation");
const resources_1 = require("@opentelemetry/resources");
const sdk_logs_1 = require("@opentelemetry/sdk-logs");
const sdk_metrics_1 = require("@opentelemetry/sdk-metrics");
const sdk_trace_base_1 = require("@opentelemetry/sdk-trace-base");
const sdk_trace_node_1 = require("@opentelemetry/sdk-trace-node");
const semantic_conventions_1 = require("@opentelemetry/semantic-conventions");
const TracerProviderWithEnvExporter_1 = require("./TracerProviderWithEnvExporter");
const core_1 = require("@opentelemetry/core");
const utils_1 = require("./utils");
class NodeSDK {
    /**
     * Create a new NodeJS SDK instance
     */
    constructor(configuration = {}) {
        var _a, _b, _c, _d, _e, _f, _g;
        const env = (0, core_1.getEnv)();
        const envWithoutDefaults = (0, core_1.getEnvWithoutDefaults)();
        if (env.OTEL_SDK_DISABLED) {
            this._disabled = true;
            // Functions with possible side-effects are set
            // to no-op via the _disabled flag
        }
        // Default is INFO, use environment without defaults to check
        // if the user originally set the environment variable.
        if (envWithoutDefaults.OTEL_LOG_LEVEL) {
            api_1.diag.setLogger(new api_1.DiagConsoleLogger(), {
                logLevel: envWithoutDefaults.OTEL_LOG_LEVEL,
            });
        }
        this._configuration = configuration;
        this._resource = (_a = configuration.resource) !== null && _a !== void 0 ? _a : new resources_1.Resource({});
        let defaultDetectors = [];
        if (process.env.OTEL_NODE_RESOURCE_DETECTORS != null) {
            defaultDetectors = (0, utils_1.getResourceDetectorsFromEnv)();
        }
        else {
            defaultDetectors = [resources_1.envDetector, resources_1.processDetector, resources_1.hostDetector];
        }
        this._resourceDetectors =
            (_b = configuration.resourceDetectors) !== null && _b !== void 0 ? _b : defaultDetectors;
        this._serviceName = configuration.serviceName;
        this._autoDetectResources = (_c = configuration.autoDetectResources) !== null && _c !== void 0 ? _c : true;
        // If a tracer provider can be created from manual configuration, create it
        if (configuration.traceExporter ||
            configuration.spanProcessor ||
            configuration.spanProcessors) {
            const tracerProviderConfig = {};
            if (configuration.sampler) {
                tracerProviderConfig.sampler = configuration.sampler;
            }
            if (configuration.spanLimits) {
                tracerProviderConfig.spanLimits = configuration.spanLimits;
            }
            if (configuration.idGenerator) {
                tracerProviderConfig.idGenerator = configuration.idGenerator;
            }
            if (configuration.spanProcessor) {
                api_1.diag.warn("The 'spanProcessor' option is deprecated. Please use 'spanProcessors' instead.");
            }
            const spanProcessor = (_d = configuration.spanProcessor) !== null && _d !== void 0 ? _d : 
            // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
            new sdk_trace_base_1.BatchSpanProcessor(configuration.traceExporter);
            const spanProcessors = (_e = configuration.spanProcessors) !== null && _e !== void 0 ? _e : [spanProcessor];
            this._tracerProviderConfig = {
                tracerConfig: tracerProviderConfig,
                spanProcessors,
                contextManager: configuration.contextManager,
                textMapPropagator: configuration.textMapPropagator,
            };
        }
        if (configuration.logRecordProcessor) {
            this._loggerProviderConfig = {
                logRecordProcessor: configuration.logRecordProcessor,
            };
        }
        if (configuration.metricReader || configuration.views) {
            const meterProviderConfig = {};
            if (configuration.metricReader) {
                meterProviderConfig.reader = configuration.metricReader;
            }
            if (configuration.views) {
                meterProviderConfig.views = configuration.views;
            }
            this._meterProviderConfig = meterProviderConfig;
        }
        this._instrumentations = (_g = (_f = configuration.instrumentations) === null || _f === void 0 ? void 0 : _f.flat()) !== null && _g !== void 0 ? _g : [];
    }
    /**
     * Call this method to construct SDK components and register them with the OpenTelemetry API.
     */
    start() {
        var _a, _b, _c, _d, _e, _f;
        if (this._disabled) {
            return;
        }
        (0, instrumentation_1.registerInstrumentations)({
            instrumentations: this._instrumentations,
        });
        if (this._autoDetectResources) {
            const internalConfig = {
                detectors: this._resourceDetectors,
            };
            this._resource = this._resource.merge((0, resources_1.detectResourcesSync)(internalConfig));
        }
        this._resource =
            this._serviceName === undefined
                ? this._resource
                : this._resource.merge(new resources_1.Resource({
                    [semantic_conventions_1.SEMRESATTRS_SERVICE_NAME]: this._serviceName,
                }));
        // if there is a tracerProviderConfig (traceExporter/spanProcessor was set manually) or the traceExporter is set manually, use NodeTracerProvider
        const Provider = this._tracerProviderConfig
            ? sdk_trace_node_1.NodeTracerProvider
            : TracerProviderWithEnvExporter_1.TracerProviderWithEnvExporters;
        // If the Provider is configured with Env Exporters, we need to check if the SDK had any manual configurations and set them here
        const tracerProvider = new Provider(Object.assign(Object.assign({}, this._configuration), { resource: this._resource }));
        this._tracerProvider = tracerProvider;
        if (this._tracerProviderConfig) {
            for (const spanProcessor of this._tracerProviderConfig.spanProcessors) {
                tracerProvider.addSpanProcessor(spanProcessor);
            }
        }
        tracerProvider.register({
            contextManager: (_b = (_a = this._tracerProviderConfig) === null || _a === void 0 ? void 0 : _a.contextManager) !== null && _b !== void 0 ? _b : 
            // _tracerProviderConfig may be undefined if trace-specific settings are not provided - fall back to raw config
            (_c = this._configuration) === null || _c === void 0 ? void 0 : _c.contextManager,
            propagator: (_d = this._tracerProviderConfig) === null || _d === void 0 ? void 0 : _d.textMapPropagator,
        });
        if (this._loggerProviderConfig) {
            const loggerProvider = new sdk_logs_1.LoggerProvider({
                resource: this._resource,
            });
            loggerProvider.addLogRecordProcessor(this._loggerProviderConfig.logRecordProcessor);
            this._loggerProvider = loggerProvider;
            api_logs_1.logs.setGlobalLoggerProvider(loggerProvider);
        }
        if (this._meterProviderConfig) {
            const readers = [];
            if (this._meterProviderConfig.reader) {
                readers.push(this._meterProviderConfig.reader);
            }
            const meterProvider = new sdk_metrics_1.MeterProvider({
                resource: this._resource,
                views: (_f = (_e = this._meterProviderConfig) === null || _e === void 0 ? void 0 : _e.views) !== null && _f !== void 0 ? _f : [],
                readers: readers,
            });
            this._meterProvider = meterProvider;
            api_1.metrics.setGlobalMeterProvider(meterProvider);
            // TODO: This is a workaround to fix https://github.com/open-telemetry/opentelemetry-js/issues/3609
            // If the MeterProvider is not yet registered when instrumentations are registered, all metrics are dropped.
            // This code is obsolete once https://github.com/open-telemetry/opentelemetry-js/issues/3622 is implemented.
            for (const instrumentation of this._instrumentations) {
                instrumentation.setMeterProvider(api_1.metrics.getMeterProvider());
            }
        }
    }
    shutdown() {
        const promises = [];
        if (this._tracerProvider) {
            promises.push(this._tracerProvider.shutdown());
        }
        if (this._loggerProvider) {
            promises.push(this._loggerProvider.shutdown());
        }
        if (this._meterProvider) {
            promises.push(this._meterProvider.shutdown());
        }
        return (Promise.all(promises)
            // return void instead of the array from Promise.all
            .then(() => { }));
    }
}
exports.NodeSDK = NodeSDK;
//# sourceMappingURL=sdk.js.map