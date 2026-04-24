/**
 * Backend Factory - Automatic Backend Detection and Selection
 *
 * Detects available vector backends and creates appropriate instances.
 * Priority: RuVector (native/WASM) > HNSWLib (Node.js)
 *
 * Features:
 * - Automatic detection of @ruvector packages
 * - Native vs WASM detection for RuVector
 * - GNN and Graph capabilities detection
 * - Graceful fallback to HNSWLib
 * - Clear error messages for missing dependencies
 */

import type { VectorBackend, VectorConfig } from './VectorBackend.js';
import { RuVectorBackend } from './ruvector/RuVectorBackend.js';

// Note: HNSWLibBackend is lazy-loaded to avoid import failures on systems
// without build tools. The import happens in createHNSWLibBackend().

export type BackendType = 'auto' | 'ruvector' | 'hnswlib';

export interface BackendDetection {
  available: 'ruvector' | 'hnswlib' | 'none';
  ruvector: {
    core: boolean;
    gnn: boolean;
    graph: boolean;
    native: boolean;
  };
  hnswlib: boolean;
}

/**
 * Detect available vector backends
 */
export async function detectBackends(): Promise<BackendDetection> {
  const result: BackendDetection = {
    available: 'none',
    ruvector: {
      core: false,
      gnn: false,
      graph: false,
      native: false
    },
    hnswlib: false
  };

  // Check RuVector packages (main package or scoped packages)
  try {
    // Try main ruvector package first
    const ruvector = await import('ruvector');
    result.ruvector.core = true;
    result.ruvector.gnn = true; // Main package includes GNN
    result.ruvector.graph = true; // Main package includes Graph
    result.ruvector.native = ruvector.isNative?.() ?? false;
    result.available = 'ruvector';
  } catch {
    // Try scoped packages as fallback
    try {
      const core = await import('@ruvector/core');
      result.ruvector.core = true;
      result.ruvector.native = core.isNative?.() ?? false;
      result.available = 'ruvector';

      // Check optional packages
      try {
        await import('@ruvector/gnn');
        result.ruvector.gnn = true;
      } catch {
        // GNN not installed - this is optional
      }

      try {
        await import('@ruvector/graph-node');
        result.ruvector.graph = true;
      } catch {
        // Graph not installed - this is optional
      }
    } catch {
      // RuVector not installed - will try fallback
    }
  }

  // Check HNSWLib
  try {
    await import('hnswlib-node');
    result.hnswlib = true;

    if (result.available === 'none') {
      result.available = 'hnswlib';
    }
  } catch {
    // HNSWLib not installed
  }

  return result;
}

/**
 * Lazy-load HNSWLibBackend to avoid import failures on systems without build tools
 */
async function createHNSWLibBackend(config: VectorConfig): Promise<VectorBackend> {
  const { HNSWLibBackend } = await import('./hnswlib/HNSWLibBackend.js');
  return new HNSWLibBackend(config);
}

/**
 * Create vector backend with automatic detection
 *
 * @param type - Backend type: 'auto', 'ruvector', or 'hnswlib'
 * @param config - Vector configuration
 * @returns Initialized VectorBackend instance
 */
export async function createBackend(
  type: BackendType,
  config: VectorConfig
): Promise<VectorBackend> {
  const detection = await detectBackends();

  let backend: VectorBackend;

  // Handle explicit backend selection
  if (type === 'ruvector') {
    if (!detection.ruvector.core) {
      throw new Error(
        'RuVector not available.\n' +
        'Install with: npm install @ruvector/core\n' +
        'Optional GNN support: npm install @ruvector/gnn\n' +
        'Optional Graph support: npm install @ruvector/graph-node'
      );
    }
    backend = new RuVectorBackend(config);
  } else if (type === 'hnswlib') {
    if (!detection.hnswlib) {
      throw new Error(
        'HNSWLib not available.\n' +
        'Install with: npm install hnswlib-node'
      );
    }
    // Lazy-load HNSWLibBackend to avoid module-level import failures
    backend = await createHNSWLibBackend(config);
  } else {
    // Auto-detect best available backend
    if (detection.ruvector.core) {
      backend = new RuVectorBackend(config);
      console.log(
        `[AgentDB] Using RuVector backend (${detection.ruvector.native ? 'native' : 'WASM'})`
      );

      // Try to initialize RuVector, fallback to HNSWLib if it fails
      try {
        await (backend as any).initialize();
        return backend;
      } catch (error) {
        const errorMessage = (error as Error).message;

        // If RuVector fails due to :memory: path or other initialization issues,
        // try falling back to HNSWLib
        if (detection.hnswlib) {
          console.log('[AgentDB] RuVector initialization failed, falling back to HNSWLib');
          console.log(`[AgentDB] Reason: ${errorMessage.split('\n')[0]}`);
          // Lazy-load HNSWLibBackend for fallback
          backend = await createHNSWLibBackend(config);
          console.log('[AgentDB] Using HNSWLib backend (fallback)');
        } else {
          // No fallback available, re-throw error
          throw error;
        }
      }
    } else if (detection.hnswlib) {
      // Lazy-load HNSWLibBackend when it's the only option
      backend = await createHNSWLibBackend(config);
      console.log('[AgentDB] Using HNSWLib backend (fallback)');
    } else {
      throw new Error(
        'No vector backend available.\n' +
        'Install one of:\n' +
        '  - npm install @ruvector/core (recommended)\n' +
        '  - npm install hnswlib-node (fallback)'
      );
    }
  }

  // Initialize the backend (if not already initialized)
  // Note: RuVector may already be initialized in the try block above
  try {
    await (backend as any).initialize();
  } catch (error) {
    // Ignore if already initialized
    if (!(error as Error).message.includes('already initialized')) {
      throw error;
    }
  }

  return backend;
}

/**
 * Get recommended backend type based on environment
 */
export async function getRecommendedBackend(): Promise<BackendType> {
  const detection = await detectBackends();

  if (detection.ruvector.core) {
    return 'ruvector';
  } else if (detection.hnswlib) {
    return 'hnswlib';
  } else {
    return 'auto'; // Will throw error in createBackend
  }
}

/**
 * Check if a specific backend is available
 */
export async function isBackendAvailable(backend: 'ruvector' | 'hnswlib'): Promise<boolean> {
  const detection = await detectBackends();

  if (backend === 'ruvector') {
    return detection.ruvector.core;
  }

  return detection.hnswlib;
}

/**
 * Get installation instructions for a backend
 */
export function getInstallCommand(backend: 'ruvector' | 'hnswlib'): string {
  return backend === 'ruvector'
    ? 'npm install ruvector'
    : 'npm install hnswlib-node';
}
