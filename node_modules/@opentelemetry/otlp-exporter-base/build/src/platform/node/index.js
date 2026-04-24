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
exports.CompressionAlgorithm = exports.configureCompression = exports.createHttpAgent = exports.sendWithHttp = exports.OTLPExporterNodeBase = void 0;
var OTLPExporterNodeBase_1 = require("./OTLPExporterNodeBase");
Object.defineProperty(exports, "OTLPExporterNodeBase", { enumerable: true, get: function () { return OTLPExporterNodeBase_1.OTLPExporterNodeBase; } });
var util_1 = require("./util");
Object.defineProperty(exports, "sendWithHttp", { enumerable: true, get: function () { return util_1.sendWithHttp; } });
Object.defineProperty(exports, "createHttpAgent", { enumerable: true, get: function () { return util_1.createHttpAgent; } });
Object.defineProperty(exports, "configureCompression", { enumerable: true, get: function () { return util_1.configureCompression; } });
var types_1 = require("./types");
Object.defineProperty(exports, "CompressionAlgorithm", { enumerable: true, get: function () { return types_1.CompressionAlgorithm; } });
//# sourceMappingURL=index.js.map