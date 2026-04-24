/**
 * Configuration Manager
 *
 * Centralized configuration management with profiles, validation,
 * and environment variable support.
 */

import * as fs from 'fs';
import * as path from 'path';
import Ajv from 'ajv';
import type { JSONSchemaType } from 'ajv';

// ============================================================================
// Types
// ============================================================================

export interface AgentDBConfig {
  profile: 'production' | 'memory' | 'latency' | 'recall' | 'custom';
  hnsw: {
    M: number;
    efConstruction: number;
    efSearch: number;
  };
  attention: {
    heads: number;
    dimension: number;
  };
  traversal: {
    beamWidth: number;
    strategy: 'greedy' | 'beam' | 'dynamic';
  };
  clustering: {
    algorithm: 'louvain' | 'leiden' | 'spectral';
    resolution: number;
  };
  neural: {
    mode: 'none' | 'gnn-only' | 'full';
    reinforcementLearning: boolean;
  };
  hypergraph: {
    enabled: boolean;
    maxEdgeSize: number;
  };
  storage: {
    reportPath: string;
    autoBackup: boolean;
  };
  monitoring: {
    enabled: boolean;
    alertThresholds: {
      memoryMB: number;
      latencyMs: number;
    };
  };
  logging?: {
    level: 'debug' | 'info' | 'warn' | 'error';
    file?: string;
  };
}

export interface ValidationResult {
  valid: boolean;
  errors?: string[];
  warnings?: string[];
}

// ============================================================================
// Configuration Schema
// ============================================================================

const configSchema: JSONSchemaType<AgentDBConfig> = {
  type: 'object',
  properties: {
    profile: {
      type: 'string',
      enum: ['production', 'memory', 'latency', 'recall', 'custom']
    },
    hnsw: {
      type: 'object',
      properties: {
        M: { type: 'number', minimum: 4, maximum: 128 },
        efConstruction: { type: 'number', minimum: 50, maximum: 1000 },
        efSearch: { type: 'number', minimum: 10, maximum: 500 }
      },
      required: ['M', 'efConstruction', 'efSearch']
    },
    attention: {
      type: 'object',
      properties: {
        heads: { type: 'number', minimum: 1, maximum: 32 },
        dimension: { type: 'number', minimum: 16, maximum: 512 }
      },
      required: ['heads', 'dimension']
    },
    traversal: {
      type: 'object',
      properties: {
        beamWidth: { type: 'number', minimum: 1, maximum: 20 },
        strategy: { type: 'string', enum: ['greedy', 'beam', 'dynamic'] }
      },
      required: ['beamWidth', 'strategy']
    },
    clustering: {
      type: 'object',
      properties: {
        algorithm: { type: 'string', enum: ['louvain', 'leiden', 'spectral'] },
        resolution: { type: 'number', minimum: 0.1, maximum: 10.0 }
      },
      required: ['algorithm', 'resolution']
    },
    neural: {
      type: 'object',
      properties: {
        mode: { type: 'string', enum: ['none', 'gnn-only', 'full'] },
        reinforcementLearning: { type: 'boolean' }
      },
      required: ['mode', 'reinforcementLearning']
    },
    hypergraph: {
      type: 'object',
      properties: {
        enabled: { type: 'boolean' },
        maxEdgeSize: { type: 'number', minimum: 2, maximum: 100 }
      },
      required: ['enabled', 'maxEdgeSize']
    },
    storage: {
      type: 'object',
      properties: {
        reportPath: { type: 'string' },
        autoBackup: { type: 'boolean' }
      },
      required: ['reportPath', 'autoBackup']
    },
    monitoring: {
      type: 'object',
      properties: {
        enabled: { type: 'boolean' },
        alertThresholds: {
          type: 'object',
          properties: {
            memoryMB: { type: 'number', minimum: 0 },
            latencyMs: { type: 'number', minimum: 0 }
          },
          required: ['memoryMB', 'latencyMs']
        }
      },
      required: ['enabled', 'alertThresholds']
    },
    logging: {
      type: 'object',
      properties: {
        level: { type: 'string', enum: ['debug', 'info', 'warn', 'error'] },
        file: { type: 'string', nullable: true }
      },
      required: ['level'],
      nullable: true
    }
  },
  required: [
    'profile',
    'hnsw',
    'attention',
    'traversal',
    'clustering',
    'neural',
    'hypergraph',
    'storage',
    'monitoring'
  ]
};

// ============================================================================
// Preset Profiles
// ============================================================================

export const PRESET_PROFILES: Record<string, Omit<AgentDBConfig, 'profile'>> = {
  production: {
    // Optimal settings from simulation discoveries
    hnsw: {
      M: 32,              // 8.2x speedup from HNSW exploration
      efConstruction: 200,
      efSearch: 100
    },
    attention: {
      heads: 8,           // 12.4% accuracy boost from attention analysis
      dimension: 64
    },
    traversal: {
      beamWidth: 5,       // 96.8% recall from traversal optimization
      strategy: 'beam'
    },
    clustering: {
      algorithm: 'louvain', // Q=0.758 from clustering analysis
      resolution: 1.0
    },
    neural: {
      mode: 'full',       // 29.4% gain from neural augmentation
      reinforcementLearning: true
    },
    hypergraph: {
      enabled: true,      // 3.7x speedup from hypergraph exploration
      maxEdgeSize: 10
    },
    storage: {
      reportPath: path.join(process.cwd(), '.agentdb', 'reports.db'),
      autoBackup: true
    },
    monitoring: {
      enabled: true,
      alertThresholds: {
        memoryMB: 8192,
        latencyMs: 500
      }
    },
    logging: {
      level: 'info'
    }
  },

  memory: {
    // Memory-constrained settings
    hnsw: {
      M: 16,              // Reduced memory footprint
      efConstruction: 100,
      efSearch: 50
    },
    attention: {
      heads: 4,           // Fewer heads = less memory
      dimension: 32
    },
    traversal: {
      beamWidth: 3,       // Smaller beam = less memory
      strategy: 'greedy'
    },
    clustering: {
      algorithm: 'louvain',
      resolution: 1.0
    },
    neural: {
      mode: 'gnn-only',   // GNN edges only, no full neural
      reinforcementLearning: false
    },
    hypergraph: {
      enabled: false,     // Disabled to save memory
      maxEdgeSize: 5
    },
    storage: {
      reportPath: path.join(process.cwd(), '.agentdb', 'reports.db'),
      autoBackup: false
    },
    monitoring: {
      enabled: true,
      alertThresholds: {
        memoryMB: 2048,   // Lower threshold
        latencyMs: 1000
      }
    },
    logging: {
      level: 'warn'
    }
  },

  latency: {
    // Latency-critical settings
    hnsw: {
      M: 32,              // Fast search
      efConstruction: 150,
      efSearch: 75
    },
    attention: {
      heads: 4,           // Fewer heads for speed
      dimension: 32
    },
    traversal: {
      beamWidth: 3,       // Speed vs. recall tradeoff
      strategy: 'dynamic' // RL-based navigation
    },
    clustering: {
      algorithm: 'louvain', // Faster than Leiden
      resolution: 1.0
    },
    neural: {
      mode: 'gnn-only',   // GNN only for speed
      reinforcementLearning: true
    },
    hypergraph: {
      enabled: false,
      maxEdgeSize: 5
    },
    storage: {
      reportPath: path.join(process.cwd(), '.agentdb', 'reports.db'),
      autoBackup: false
    },
    monitoring: {
      enabled: true,
      alertThresholds: {
        memoryMB: 4096,
        latencyMs: 200    // Strict latency requirement
      }
    },
    logging: {
      level: 'error'      // Minimal logging overhead
    }
  },

  recall: {
    // High-recall settings
    hnsw: {
      M: 64,              // Maximum connectivity
      efConstruction: 400,
      efSearch: 200       // Exhaustive search
    },
    attention: {
      heads: 16,          // More heads for better accuracy
      dimension: 128
    },
    traversal: {
      beamWidth: 10,      // Wide beam for exhaustive search
      strategy: 'beam'
    },
    clustering: {
      algorithm: 'leiden', // Better quality than Louvain
      resolution: 0.8
    },
    neural: {
      mode: 'full',       // Full neural augmentation
      reinforcementLearning: true
    },
    hypergraph: {
      enabled: true,
      maxEdgeSize: 20
    },
    storage: {
      reportPath: path.join(process.cwd(), '.agentdb', 'reports.db'),
      autoBackup: true
    },
    monitoring: {
      enabled: true,
      alertThresholds: {
        memoryMB: 16384,  // High memory allowed
        latencyMs: 2000   // Relaxed latency
      }
    },
    logging: {
      level: 'debug'
    }
  }
};

// ============================================================================
// Configuration Manager
// ============================================================================

export class ConfigManager {
  private ajv: any;
  private validator: any;

  constructor() {
    this.ajv = new Ajv({ allErrors: true });
    this.validator = this.ajv.compile(configSchema);
  }

  // --------------------------------------------------------------------------
  // Loading
  // --------------------------------------------------------------------------

  /**
   * Load configuration from file.
   */
  loadFromFile(filePath: string): AgentDBConfig {
    if (!fs.existsSync(filePath)) {
      throw new Error(`Configuration file not found: ${filePath}`);
    }

    const content = fs.readFileSync(filePath, 'utf-8');
    const config = JSON.parse(content);

    return this.validate(config);
  }

  /**
   * Load configuration from preset profile.
   */
  loadProfile(profile: keyof typeof PRESET_PROFILES): AgentDBConfig {
    const preset = PRESET_PROFILES[profile];

    if (!preset) {
      throw new Error(`Unknown profile: ${profile}`);
    }

    return { ...preset, profile } as AgentDBConfig;
  }

  /**
   * Load configuration with environment variable overrides.
   */
  loadWithEnv(baseConfig: AgentDBConfig): AgentDBConfig {
    const config = { ...baseConfig };

    // HNSW overrides
    if (process.env.AGENTDB_HNSW_M) {
      config.hnsw.M = parseInt(process.env.AGENTDB_HNSW_M, 10);
    }
    if (process.env.AGENTDB_HNSW_EF_CONSTRUCTION) {
      config.hnsw.efConstruction = parseInt(process.env.AGENTDB_HNSW_EF_CONSTRUCTION, 10);
    }
    if (process.env.AGENTDB_HNSW_EF_SEARCH) {
      config.hnsw.efSearch = parseInt(process.env.AGENTDB_HNSW_EF_SEARCH, 10);
    }

    // Attention overrides
    if (process.env.AGENTDB_ATTENTION_HEADS) {
      config.attention.heads = parseInt(process.env.AGENTDB_ATTENTION_HEADS, 10);
    }
    if (process.env.AGENTDB_ATTENTION_DIM) {
      config.attention.dimension = parseInt(process.env.AGENTDB_ATTENTION_DIM, 10);
    }

    // Traversal overrides
    if (process.env.AGENTDB_BEAM_WIDTH) {
      config.traversal.beamWidth = parseInt(process.env.AGENTDB_BEAM_WIDTH, 10);
    }
    if (process.env.AGENTDB_TRAVERSAL_STRATEGY) {
      config.traversal.strategy = process.env.AGENTDB_TRAVERSAL_STRATEGY as any;
    }

    // Storage overrides
    if (process.env.AGENTDB_REPORT_PATH) {
      config.storage.reportPath = process.env.AGENTDB_REPORT_PATH;
    }

    // Monitoring overrides
    if (process.env.AGENTDB_MEMORY_THRESHOLD) {
      config.monitoring.alertThresholds.memoryMB =
        parseInt(process.env.AGENTDB_MEMORY_THRESHOLD, 10);
    }
    if (process.env.AGENTDB_LATENCY_THRESHOLD) {
      config.monitoring.alertThresholds.latencyMs =
        parseInt(process.env.AGENTDB_LATENCY_THRESHOLD, 10);
    }

    return this.validate(config);
  }

  /**
   * Load configuration from default locations.
   * Priority: CLI args > .agentdb.json > ~/.agentdb/config.json > defaults
   */
  loadDefault(profile: string = 'production'): AgentDBConfig {
    // 1. Check for project-level config
    const projectConfig = path.join(process.cwd(), '.agentdb.json');
    if (fs.existsSync(projectConfig)) {
      return this.loadWithEnv(this.loadFromFile(projectConfig));
    }

    // 2. Check for user-level config
    const userConfig = path.join(
      process.env.HOME || '',
      '.agentdb',
      'config.json'
    );
    if (fs.existsSync(userConfig)) {
      return this.loadWithEnv(this.loadFromFile(userConfig));
    }

    // 3. Use preset profile
    return this.loadWithEnv(this.loadProfile(profile as any));
  }

  // --------------------------------------------------------------------------
  // Saving
  // --------------------------------------------------------------------------

  /**
   * Save configuration to file.
   */
  save(config: AgentDBConfig, filePath: string): void {
    // Validate before saving
    this.validate(config);

    // Ensure directory exists
    const dir = path.dirname(filePath);
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }

    // Write file
    fs.writeFileSync(
      filePath,
      JSON.stringify(config, null, 2),
      'utf-8'
    );
  }

  // --------------------------------------------------------------------------
  // Validation
  // --------------------------------------------------------------------------

  /**
   * Validate configuration against schema.
   */
  validate(config: any): AgentDBConfig {
    const valid = this.validator(config);

    if (!valid) {
      const errors = this.validator.errors?.map((e: any) =>
        `${e.instancePath} ${e.message}`
      ).join(', ');

      throw new Error(`Invalid configuration: ${errors}`);
    }

    return config as AgentDBConfig;
  }

  /**
   * Validate with warnings (non-throwing).
   */
  validateWithWarnings(config: any): ValidationResult {
    const errors: string[] = [];
    const warnings: string[] = [];

    try {
      this.validate(config);
    } catch (error: any) {
      errors.push(error.message);
      return { valid: false, errors };
    }

    // Additional validation checks
    if (config.hnsw.M < 16) {
      warnings.push('HNSW M < 16 may result in poor recall');
    }

    if (config.traversal.beamWidth < 3) {
      warnings.push('Beam width < 3 may miss optimal paths');
    }

    if (config.neural.mode === 'full' && !config.monitoring.enabled) {
      warnings.push('Full neural mode recommended with monitoring enabled');
    }

    if (config.hypergraph.enabled && config.neural.mode === 'none') {
      warnings.push('Hypergraph works best with neural augmentation');
    }

    return {
      valid: true,
      warnings: warnings.length > 0 ? warnings : undefined
    };
  }

  // --------------------------------------------------------------------------
  // Utilities
  // --------------------------------------------------------------------------

  /**
   * Merge configurations (deep merge).
   */
  merge(base: AgentDBConfig, override: Partial<AgentDBConfig>): AgentDBConfig {
    const merged = {
      ...base,
      ...override,
      hnsw: { ...base.hnsw, ...override.hnsw },
      attention: { ...base.attention, ...override.attention },
      traversal: { ...base.traversal, ...override.traversal },
      clustering: { ...base.clustering, ...override.clustering },
      neural: { ...base.neural, ...override.neural },
      hypergraph: { ...base.hypergraph, ...override.hypergraph },
      storage: { ...base.storage, ...override.storage },
      monitoring: {
        ...base.monitoring,
        ...override.monitoring,
        alertThresholds: {
          ...base.monitoring.alertThresholds,
          ...override.monitoring?.alertThresholds
        }
      },
      logging: { ...base.logging, ...override.logging }
    };

    return this.validate(merged);
  }

  /**
   * Get configuration summary.
   */
  getSummary(config: AgentDBConfig): string {
    return `
Profile: ${config.profile}
HNSW: M=${config.hnsw.M}, efConstruction=${config.hnsw.efConstruction}, efSearch=${config.hnsw.efSearch}
Attention: heads=${config.attention.heads}, dim=${config.attention.dimension}
Traversal: beam=${config.traversal.beamWidth}, strategy=${config.traversal.strategy}
Clustering: ${config.clustering.algorithm}, resolution=${config.clustering.resolution}
Neural: ${config.neural.mode}, RL=${config.neural.reinforcementLearning}
Hypergraph: enabled=${config.hypergraph.enabled}
Monitoring: enabled=${config.monitoring.enabled}
`.trim();
  }

  /**
   * Export configuration as JSON string.
   */
  export(config: AgentDBConfig): string {
    return JSON.stringify(config, null, 2);
  }

  /**
   * Import configuration from JSON string.
   */
  import(json: string): AgentDBConfig {
    const config = JSON.parse(json);
    return this.validate(config);
  }
}

// ============================================================================
// Factory
// ============================================================================

/**
 * Create configuration manager instance.
 */
export function createConfigManager(): ConfigManager {
  return new ConfigManager();
}
