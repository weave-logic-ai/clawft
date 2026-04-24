/**
 * Type definitions for v1.x to v2.0 compatibility layer
 */

export type APIVersion = '1.x' | '2.0';

export interface CompatibilityConfig {
  // Version control
  version?: APIVersion;
  preferV2?: boolean;

  // Warning configuration
  deprecationWarnings?: boolean;
  strictMode?: boolean;
  logDeprecations?: string; // File path

  // Feature flags
  features?: {
    agentDB?: boolean;
    attention?: boolean;
    gnnLearning?: boolean;
    causalReasoning?: boolean;
    reasoningBank?: boolean;
  };

  // Performance
  enableOptimizations?: boolean;
  preserveV1Behavior?: boolean;
}

export interface V1Config {
  // Memory
  memoryPath?: string;
  optimizeMemory?: boolean;

  // Swarm
  topology?: 'mesh' | 'hierarchical' | 'ring' | 'star';
  maxAgents?: number;
  strategy?: string;

  // LLM
  provider?: string;
  model?: string;

  // Compatibility
  deprecationWarnings?: boolean;
  strictMode?: boolean;
}

export interface V2Config {
  backend: 'agentdb';
  version: '2.0';

  memory: {
    path: string;
    backend: 'ruvector' | 'sqlite';
    enableHNSW?: boolean;
    enableQuantization?: boolean;
    cacheSize?: number;
    batchSize?: number;
  };

  swarm: {
    topology: 'mesh' | 'hierarchical' | 'ring' | 'star';
    maxAgents: number;
    strategy: 'auto' | 'balanced' | 'specialized';
  };

  routing: {
    provider: string;
    model: string;
    optimization: 'quality' | 'balanced' | 'cost' | 'speed';
  };

  intelligence?: {
    attentionMechanisms?: string[];
    gnnLearning?: boolean;
    causalReasoning?: boolean;
    reasoningBank?: boolean;
  };

  compatibility?: {
    v1Mode: boolean;
    preserveV1Behavior: boolean;
    emitWarnings: boolean;
  };
}

export interface DeprecationWarning {
  api: string;
  message: string;
  migration: string;
  documentation: string;
  severity: 'info' | 'warning' | 'error';
  timestamp: number;
}

export interface DeprecationConfig {
  emitWarnings?: boolean;
  throwOnDeprecated?: boolean;
  logToFile?: string;
  severity?: 'silent' | 'soft' | 'prominent';
}

export interface MigrationReport {
  v1APIsFound: Array<{
    api: string;
    occurrences: number;
    replacement: string;
  }>;
  suggestedMigrations: Array<{
    from: string;
    to: string;
    difficulty: 'easy' | 'medium' | 'hard';
    automatic: boolean;
  }>;
  estimatedEffort: 'none' | 'low' | 'medium' | 'high';
}

export interface ValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

export interface VersionDetectionResult {
  version: APIVersion;
  confidence: number;
  indicators: string[];
}
