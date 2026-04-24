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
var __extends = (this && this.__extends) || (function () {
    var extendStatics = function (d, b) {
        extendStatics = Object.setPrototypeOf ||
            ({ __proto__: [] } instanceof Array && function (d, b) { d.__proto__ = b; }) ||
            function (d, b) { for (var p in b) if (Object.prototype.hasOwnProperty.call(b, p)) d[p] = b[p]; };
        return extendStatics(d, b);
    };
    return function (d, b) {
        if (typeof b !== "function" && b !== null)
            throw new TypeError("Class extends value " + String(b) + " is not a constructor or null");
        extendStatics(d, b);
        function __() { this.constructor = d; }
        d.prototype = b === null ? Object.create(b) : (__.prototype = b.prototype, new __());
    };
})();
import { baggageUtils, getEnv } from '@opentelemetry/core';
import { OTLPMetricExporterBase } from '../../OTLPMetricExporterBase';
import { OTLPExporterBrowserBase, appendResourcePathToUrl, appendRootPathToUrlIfNeeded, } from '@opentelemetry/otlp-exporter-base';
import { JsonMetricsSerializer, } from '@opentelemetry/otlp-transformer';
var DEFAULT_COLLECTOR_RESOURCE_PATH = 'v1/metrics';
var DEFAULT_COLLECTOR_URL = "http://localhost:4318/" + DEFAULT_COLLECTOR_RESOURCE_PATH;
var OTLPExporterBrowserProxy = /** @class */ (function (_super) {
    __extends(OTLPExporterBrowserProxy, _super);
    function OTLPExporterBrowserProxy(config) {
        var _this = _super.call(this, config, JsonMetricsSerializer, 'application/json') || this;
        _this._headers = Object.assign(_this._headers, baggageUtils.parseKeyPairsIntoRecord(getEnv().OTEL_EXPORTER_OTLP_METRICS_HEADERS));
        return _this;
    }
    OTLPExporterBrowserProxy.prototype.getDefaultUrl = function (config) {
        return typeof config.url === 'string'
            ? config.url
            : getEnv().OTEL_EXPORTER_OTLP_METRICS_ENDPOINT.length > 0
                ? appendRootPathToUrlIfNeeded(getEnv().OTEL_EXPORTER_OTLP_METRICS_ENDPOINT)
                : getEnv().OTEL_EXPORTER_OTLP_ENDPOINT.length > 0
                    ? appendResourcePathToUrl(getEnv().OTEL_EXPORTER_OTLP_ENDPOINT, DEFAULT_COLLECTOR_RESOURCE_PATH)
                    : DEFAULT_COLLECTOR_URL;
    };
    return OTLPExporterBrowserProxy;
}(OTLPExporterBrowserBase));
/**
 * Collector Metric Exporter for Web
 */
var OTLPMetricExporter = /** @class */ (function (_super) {
    __extends(OTLPMetricExporter, _super);
    function OTLPMetricExporter(config) {
        return _super.call(this, new OTLPExporterBrowserProxy(config), config) || this;
    }
    return OTLPMetricExporter;
}(OTLPMetricExporterBase));
export { OTLPMetricExporter };
//# sourceMappingURL=OTLPMetricExporter.js.map