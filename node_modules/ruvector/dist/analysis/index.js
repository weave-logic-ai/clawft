"use strict";
/**
 * Analysis Module - Consolidated code analysis utilities
 *
 * Single source of truth for:
 * - Security scanning
 * - Complexity analysis
 * - Pattern extraction
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
exports.patterns = exports.complexity = exports.security = void 0;
__exportStar(require("./security"), exports);
__exportStar(require("./complexity"), exports);
__exportStar(require("./patterns"), exports);
// Re-export defaults for convenience
var security_1 = require("./security");
Object.defineProperty(exports, "security", { enumerable: true, get: function () { return __importDefault(security_1).default; } });
var complexity_1 = require("./complexity");
Object.defineProperty(exports, "complexity", { enumerable: true, get: function () { return __importDefault(complexity_1).default; } });
var patterns_1 = require("./patterns");
Object.defineProperty(exports, "patterns", { enumerable: true, get: function () { return __importDefault(patterns_1).default; } });
