/**
 * SIMD Vector Operations Module
 *
 * Exports high-performance vector operations with SIMD acceleration.
 *
 * @module simd
 */
export { 
// SIMD Detection
detectSIMDSupport, detectSIMDSupportLegacy, 
// Core vector operations (8x unrolled with ILP)
cosineSimilaritySIMD, euclideanDistanceSIMD, euclideanDistanceSquaredSIMD, dotProductSIMD, l2NormSIMD, 
// Normalization
normalizeVector, normalizeVectorInPlace, 
// Batch operations
batchCosineSimilarity, batchEuclideanDistance, 
// Vector arithmetic
vectorAdd, vectorSub, vectorScale, 
// Utilities
toFloat32Array, randomUnitVector, 
// Class
SIMDVectorOps, 
// Default instance
defaultSIMDOps, 
// Default export
default as simdVectorOps, } from './simd-vector-ops';
//# sourceMappingURL=index.js.map