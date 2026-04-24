/**
 * Vector Quantization Module
 *
 * Exports all quantization utilities for AgentDB.
 */

export {
  // Types
  type QuantizationStats,
  type QuantizedVector,
  type ProductQuantizerConfig,
  type PQEncodedVector,
  type QuantizedVectorStoreConfig,
  type QuantizedSearchResult,
  // Scalar Quantization Functions
  quantize8bit,
  quantize4bit,
  dequantize8bit,
  dequantize4bit,
  calculateQuantizationError,
  getQuantizationStats,
  // Product Quantization
  ProductQuantizer,
  // Quantized Vector Store
  QuantizedVectorStore,
  // Factory Functions
  createScalar8BitStore,
  createScalar4BitStore,
  createProductQuantizedStore,
} from './vector-quantization.js';
