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
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __generator = (this && this.__generator) || function (thisArg, body) {
    var _ = { label: 0, sent: function() { if (t[0] & 1) throw t[1]; return t[1]; }, trys: [], ops: [] }, f, y, t, g;
    return g = { next: verb(0), "throw": verb(1), "return": verb(2) }, typeof Symbol === "function" && (g[Symbol.iterator] = function() { return this; }), g;
    function verb(n) { return function (v) { return step([n, v]); }; }
    function step(op) {
        if (f) throw new TypeError("Generator is already executing.");
        while (_) try {
            if (f = 1, y && (t = op[0] & 2 ? y["return"] : op[0] ? y["throw"] || ((t = y["return"]) && t.call(y), 0) : y.next) && !(t = t.call(y, op[1])).done) return t;
            if (y = 0, t) op = [op[0] & 2, t.value];
            switch (op[0]) {
                case 0: case 1: t = op; break;
                case 4: _.label++; return { value: op[1], done: false };
                case 5: _.label++; y = op[1]; op = [0]; continue;
                case 7: op = _.ops.pop(); _.trys.pop(); continue;
                default:
                    if (!(t = _.trys, t = t.length > 0 && t[t.length - 1]) && (op[0] === 6 || op[0] === 2)) { _ = 0; continue; }
                    if (op[0] === 3 && (!t || (op[1] > t[0] && op[1] < t[3]))) { _.label = op[1]; break; }
                    if (op[0] === 6 && _.label < t[1]) { _.label = t[1]; t = op; break; }
                    if (t && _.label < t[2]) { _.label = t[2]; _.ops.push(op); break; }
                    if (t[2]) _.ops.pop();
                    _.trys.pop(); continue;
            }
            op = body.call(thisArg, _);
        } catch (e) { op = [6, e]; y = 0; } finally { f = t = 0; }
        if (op[0] & 5) throw op[1]; return { value: op[0] ? op[1] : void 0, done: true };
    }
};
import { getEnv } from '@opentelemetry/core';
import { AggregationTemporality, InstrumentType, Aggregation, } from '@opentelemetry/sdk-metrics';
import { AggregationTemporalityPreference, } from './OTLPMetricExporterOptions';
import { diag } from '@opentelemetry/api';
export var CumulativeTemporalitySelector = function () { return AggregationTemporality.CUMULATIVE; };
export var DeltaTemporalitySelector = function (instrumentType) {
    switch (instrumentType) {
        case InstrumentType.COUNTER:
        case InstrumentType.OBSERVABLE_COUNTER:
        case InstrumentType.GAUGE:
        case InstrumentType.HISTOGRAM:
        case InstrumentType.OBSERVABLE_GAUGE:
            return AggregationTemporality.DELTA;
        case InstrumentType.UP_DOWN_COUNTER:
        case InstrumentType.OBSERVABLE_UP_DOWN_COUNTER:
            return AggregationTemporality.CUMULATIVE;
    }
};
export var LowMemoryTemporalitySelector = function (instrumentType) {
    switch (instrumentType) {
        case InstrumentType.COUNTER:
        case InstrumentType.HISTOGRAM:
            return AggregationTemporality.DELTA;
        case InstrumentType.GAUGE:
        case InstrumentType.UP_DOWN_COUNTER:
        case InstrumentType.OBSERVABLE_UP_DOWN_COUNTER:
        case InstrumentType.OBSERVABLE_COUNTER:
        case InstrumentType.OBSERVABLE_GAUGE:
            return AggregationTemporality.CUMULATIVE;
    }
};
function chooseTemporalitySelectorFromEnvironment() {
    var env = getEnv();
    var configuredTemporality = env.OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE.trim().toLowerCase();
    if (configuredTemporality === 'cumulative') {
        return CumulativeTemporalitySelector;
    }
    if (configuredTemporality === 'delta') {
        return DeltaTemporalitySelector;
    }
    if (configuredTemporality === 'lowmemory') {
        return LowMemoryTemporalitySelector;
    }
    diag.warn("OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE is set to '" + env.OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE + "', but only 'cumulative' and 'delta' are allowed. Using default ('cumulative') instead.");
    return CumulativeTemporalitySelector;
}
function chooseTemporalitySelector(temporalityPreference) {
    // Directly passed preference has priority.
    if (temporalityPreference != null) {
        if (temporalityPreference === AggregationTemporalityPreference.DELTA) {
            return DeltaTemporalitySelector;
        }
        else if (temporalityPreference === AggregationTemporalityPreference.LOWMEMORY) {
            return LowMemoryTemporalitySelector;
        }
        return CumulativeTemporalitySelector;
    }
    return chooseTemporalitySelectorFromEnvironment();
}
function chooseAggregationSelector(config) {
    if (config === null || config === void 0 ? void 0 : config.aggregationPreference) {
        return config.aggregationPreference;
    }
    else {
        return function (_instrumentType) { return Aggregation.Default(); };
    }
}
var OTLPMetricExporterBase = /** @class */ (function () {
    function OTLPMetricExporterBase(exporter, config) {
        this._otlpExporter = exporter;
        this._aggregationSelector = chooseAggregationSelector(config);
        this._aggregationTemporalitySelector = chooseTemporalitySelector(config === null || config === void 0 ? void 0 : config.temporalityPreference);
    }
    OTLPMetricExporterBase.prototype.export = function (metrics, resultCallback) {
        this._otlpExporter.export([metrics], resultCallback);
    };
    OTLPMetricExporterBase.prototype.shutdown = function () {
        return __awaiter(this, void 0, void 0, function () {
            return __generator(this, function (_a) {
                switch (_a.label) {
                    case 0: return [4 /*yield*/, this._otlpExporter.shutdown()];
                    case 1:
                        _a.sent();
                        return [2 /*return*/];
                }
            });
        });
    };
    OTLPMetricExporterBase.prototype.forceFlush = function () {
        return Promise.resolve();
    };
    OTLPMetricExporterBase.prototype.selectAggregation = function (instrumentType) {
        return this._aggregationSelector(instrumentType);
    };
    OTLPMetricExporterBase.prototype.selectAggregationTemporality = function (instrumentType) {
        return this._aggregationTemporalitySelector(instrumentType);
    };
    return OTLPMetricExporterBase;
}());
export { OTLPMetricExporterBase };
//# sourceMappingURL=OTLPMetricExporterBase.js.map