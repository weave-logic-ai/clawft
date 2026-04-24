"use strict";
/**
 * Core module exports
 *
 * These wrappers provide safe, type-flexible interfaces to the underlying
 * native packages, handling array type conversions automatically.
 */
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __exportStar = (this && this.__exportStar) || function(m, exports) {
    for (var p in m) if (p !== "default" && !Object.prototype.hasOwnProperty.call(exports, p)) __createBinding(exports, m, p);
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.NeuralSubstrate = exports.AdaptiveEmbedder = exports.LearningEngine = exports.TensorCompress = exports.ASTParser = exports.CodeParser = exports.RuvectorCluster = exports.CodeGraph = exports.SemanticRouter = exports.ExtendedWorkerPool = exports.ParallelIntelligence = exports.OptimizedOnnxEmbedder = exports.OnnxEmbedder = exports.IntelligenceEngine = exports.Sona = exports.agentdbFast = exports.attentionFallbacks = exports.gnnWrapper = void 0;
__exportStar(require("./gnn-wrapper"), exports);
__exportStar(require("./attention-fallbacks"), exports);
__exportStar(require("./agentdb-fast"), exports);
__exportStar(require("./sona-wrapper"), exports);
__exportStar(require("./intelligence-engine"), exports);
__exportStar(require("./onnx-embedder"), exports);
__exportStar(require("./onnx-optimized"), exports);
__exportStar(require("./parallel-intelligence"), exports);
__exportStar(require("./parallel-workers"), exports);
__exportStar(require("./router-wrapper"), exports);
__exportStar(require("./graph-wrapper"), exports);
__exportStar(require("./cluster-wrapper"), exports);
__exportStar(require("./ast-parser"), exports);
__exportStar(require("./diff-embeddings"), exports);
__exportStar(require("./coverage-router"), exports);
__exportStar(require("./graph-algorithms"), exports);
__exportStar(require("./tensor-compress"), exports);
__exportStar(require("./learning-engine"), exports);
__exportStar(require("./adaptive-embedder"), exports);
__exportStar(require("./neural-embeddings"), exports);
__exportStar(require("./neural-perf"), exports);
__exportStar(require("./rvf-wrapper"), exports);
// Analysis module (consolidated security, complexity, patterns)
__exportStar(require("../analysis"), exports);
// Re-export default objects for convenience
var gnn_wrapper_1 = require("./gnn-wrapper");
Object.defineProperty(exports, "gnnWrapper", { enumerable: true, get: function () { return __importDefault(gnn_wrapper_1).default; } });
var attention_fallbacks_1 = require("./attention-fallbacks");
Object.defineProperty(exports, "attentionFallbacks", { enumerable: true, get: function () { return __importDefault(attention_fallbacks_1).default; } });
var agentdb_fast_1 = require("./agentdb-fast");
Object.defineProperty(exports, "agentdbFast", { enumerable: true, get: function () { return __importDefault(agentdb_fast_1).default; } });
var sona_wrapper_1 = require("./sona-wrapper");
Object.defineProperty(exports, "Sona", { enumerable: true, get: function () { return __importDefault(sona_wrapper_1).default; } });
var intelligence_engine_1 = require("./intelligence-engine");
Object.defineProperty(exports, "IntelligenceEngine", { enumerable: true, get: function () { return __importDefault(intelligence_engine_1).default; } });
var onnx_embedder_1 = require("./onnx-embedder");
Object.defineProperty(exports, "OnnxEmbedder", { enumerable: true, get: function () { return __importDefault(onnx_embedder_1).default; } });
var onnx_optimized_1 = require("./onnx-optimized");
Object.defineProperty(exports, "OptimizedOnnxEmbedder", { enumerable: true, get: function () { return __importDefault(onnx_optimized_1).default; } });
var parallel_intelligence_1 = require("./parallel-intelligence");
Object.defineProperty(exports, "ParallelIntelligence", { enumerable: true, get: function () { return __importDefault(parallel_intelligence_1).default; } });
var parallel_workers_1 = require("./parallel-workers");
Object.defineProperty(exports, "ExtendedWorkerPool", { enumerable: true, get: function () { return __importDefault(parallel_workers_1).default; } });
var router_wrapper_1 = require("./router-wrapper");
Object.defineProperty(exports, "SemanticRouter", { enumerable: true, get: function () { return __importDefault(router_wrapper_1).default; } });
var graph_wrapper_1 = require("./graph-wrapper");
Object.defineProperty(exports, "CodeGraph", { enumerable: true, get: function () { return __importDefault(graph_wrapper_1).default; } });
var cluster_wrapper_1 = require("./cluster-wrapper");
Object.defineProperty(exports, "RuvectorCluster", { enumerable: true, get: function () { return __importDefault(cluster_wrapper_1).default; } });
var ast_parser_1 = require("./ast-parser");
Object.defineProperty(exports, "CodeParser", { enumerable: true, get: function () { return __importDefault(ast_parser_1).default; } });
// Alias for backward compatibility
var ast_parser_2 = require("./ast-parser");
Object.defineProperty(exports, "ASTParser", { enumerable: true, get: function () { return ast_parser_2.CodeParser; } });
// New v2.1 modules
var tensor_compress_1 = require("./tensor-compress");
Object.defineProperty(exports, "TensorCompress", { enumerable: true, get: function () { return __importDefault(tensor_compress_1).default; } });
var learning_engine_1 = require("./learning-engine");
Object.defineProperty(exports, "LearningEngine", { enumerable: true, get: function () { return __importDefault(learning_engine_1).default; } });
var adaptive_embedder_1 = require("./adaptive-embedder");
Object.defineProperty(exports, "AdaptiveEmbedder", { enumerable: true, get: function () { return __importDefault(adaptive_embedder_1).default; } });
var neural_embeddings_1 = require("./neural-embeddings");
Object.defineProperty(exports, "NeuralSubstrate", { enumerable: true, get: function () { return __importDefault(neural_embeddings_1).default; } });
