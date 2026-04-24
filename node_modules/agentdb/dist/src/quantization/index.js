/**
 * Vector Quantization Module
 *
 * Exports all quantization utilities for AgentDB.
 */
export { 
// Scalar Quantization Functions
quantize8bit, quantize4bit, dequantize8bit, dequantize4bit, calculateQuantizationError, getQuantizationStats, 
// Product Quantization
ProductQuantizer, 
// Quantized Vector Store
QuantizedVectorStore, 
// Factory Functions
createScalar8BitStore, createScalar4BitStore, createProductQuantizedStore, } from './vector-quantization.js';
//# sourceMappingURL=index.js.map