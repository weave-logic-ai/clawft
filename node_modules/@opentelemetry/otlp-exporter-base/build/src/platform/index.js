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
exports.sendWithXhr = exports.OTLPExporterBrowserBase = exports.CompressionAlgorithm = exports.configureCompression = exports.createHttpAgent = exports.sendWithHttp = exports.OTLPExporterNodeBase = void 0;
var node_1 = require("./node");
Object.defineProperty(exports, "OTLPExporterNodeBase", { enumerable: true, get: function () { return node_1.OTLPExporterNodeBase; } });
Object.defineProperty(exports, "sendWithHttp", { enumerable: true, get: function () { return node_1.sendWithHttp; } });
Object.defineProperty(exports, "createHttpAgent", { enumerable: true, get: function () { return node_1.createHttpAgent; } });
Object.defineProperty(exports, "configureCompression", { enumerable: true, get: function () { return node_1.configureCompression; } });
Object.defineProperty(exports, "CompressionAlgorithm", { enumerable: true, get: function () { return node_1.CompressionAlgorithm; } });
var browser_1 = require("./browser");
Object.defineProperty(exports, "OTLPExporterBrowserBase", { enumerable: true, get: function () { return browser_1.OTLPExporterBrowserBase; } });
Object.defineProperty(exports, "sendWithXhr", { enumerable: true, get: function () { return browser_1.sendWithXhr; } });
//# sourceMappingURL=index.js.map