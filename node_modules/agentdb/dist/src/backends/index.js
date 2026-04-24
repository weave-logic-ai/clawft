/**
 * AgentDB Backends - Unified Vector Storage Interface
 *
 * Provides automatic backend selection between RuVector and HNSWLib
 * with graceful fallback and clear error messages.
 */
// Backend implementations
export { RuVectorBackend } from './ruvector/RuVectorBackend.js';
export { RuVectorLearning } from './ruvector/RuVectorLearning.js';
export { HNSWLibBackend } from './hnswlib/HNSWLibBackend.js';
// Factory and detection
export { createBackend, detectBackends, getRecommendedBackend, isBackendAvailable, getInstallCommand } from './factory.js';
//# sourceMappingURL=index.js.map