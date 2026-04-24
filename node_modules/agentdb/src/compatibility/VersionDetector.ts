/**
 * VersionDetector - Automatically detects v1.x vs v2.0 API usage
 *
 * Analyzes configuration objects and method calls to determine
 * which API version is being used, enabling transparent compatibility.
 */

import type {
  APIVersion,
  V1Config,
  V2Config,
  CompatibilityConfig,
  VersionDetectionResult
} from './types';

export class VersionDetector {
  private static readonly V1_API_METHODS = new Set([
    'initSwarm',
    'spawnAgent',
    'orchestrateTask',
    'getMemory',
    'setMemory',
    'searchMemory',
    'getSwarmStatus',
    'destroySwarm',
    'getTaskStatus',
    'waitForTask'
  ]);

  private static readonly V1_TO_V2_MAPPING: Record<string, string> = {
    initSwarm: 'swarms.create',
    spawnAgent: 'agents.spawn',
    orchestrateTask: 'tasks.orchestrate',
    getMemory: 'memory.retrieve',
    setMemory: 'memory.store',
    searchMemory: 'memory.vectorSearch',
    getSwarmStatus: 'swarms.status',
    destroySwarm: 'swarms.destroy',
    getTaskStatus: 'tasks.status',
    waitForTask: 'tasks.wait'
  };

  /**
   * Detect API version from configuration object
   */
  static detect(
    config: V1Config | V2Config | CompatibilityConfig | any,
    context?: string
  ): VersionDetectionResult {
    const indicators: string[] = [];
    let confidence = 0.5; // Default neutral confidence

    // Explicit version takes precedence
    if ('version' in config) {
      if (config.version === '2.0') {
        return {
          version: '2.0',
          confidence: 1.0,
          indicators: ['explicit_version_2.0']
        };
      } else if (config.version === '1.x') {
        return {
          version: '1.x',
          confidence: 1.0,
          indicators: ['explicit_version_1.x']
        };
      }
    }

    // Check for v2.0 indicators
    if ('backend' in config && config.backend === 'agentdb') {
      indicators.push('v2_backend_field');
      confidence += 0.3;
    }

    if ('memory' in config && typeof config.memory === 'object') {
      const memoryConfig = config.memory;
      if ('backend' in memoryConfig || 'enableHNSW' in memoryConfig || 'enableQuantization' in memoryConfig) {
        indicators.push('v2_memory_structure');
        confidence += 0.3;
      }
    }

    if ('routing' in config && typeof config.routing === 'object') {
      indicators.push('v2_routing_structure');
      confidence += 0.2;
    }

    if ('intelligence' in config) {
      indicators.push('v2_intelligence_features');
      confidence += 0.2;
    }

    if ('swarm' in config && typeof config.swarm === 'object') {
      const swarmConfig = config.swarm;
      if ('strategy' in swarmConfig && typeof swarmConfig.strategy === 'string') {
        indicators.push('v2_swarm_structure');
        confidence += 0.2;
      }
    }

    // Check for v1.x indicators
    if ('memoryPath' in config) {
      indicators.push('v1_memoryPath_field');
      confidence -= 0.3;
    }

    if ('optimizeMemory' in config) {
      indicators.push('v1_optimizeMemory_field');
      confidence -= 0.2;
    }

    // Simple flat config suggests v1.x
    if ('topology' in config && typeof config.topology === 'string' && !('swarm' in config)) {
      indicators.push('v1_simple_config');
      confidence -= 0.2;
    }

    // Determine version based on confidence
    if (confidence >= 0.6) {
      return {
        version: '2.0',
        confidence: Math.min(confidence, 1.0),
        indicators
      };
    } else if (confidence <= 0.4) {
      return {
        version: '1.x',
        confidence: Math.min(1.0 - confidence, 1.0),
        indicators
      };
    }

    // Default to v1.x for backwards compatibility
    indicators.push('default_v1_compat');
    return {
      version: '1.x',
      confidence: 0.5,
      indicators
    };
  }

  /**
   * Check if a method name is a v1.x API
   */
  static isV1API(methodName: string): boolean {
    return this.V1_API_METHODS.has(methodName);
  }

  /**
   * Check if a method name is a v2.0 API (namespaced)
   */
  static isV2API(methodName: string): boolean {
    return methodName.includes('.') && !this.isV1API(methodName);
  }

  /**
   * Get v2.0 equivalent for v1.x API
   */
  static getAPIMapping(v1Method: string): string | undefined {
    return this.V1_TO_V2_MAPPING[v1Method];
  }

  /**
   * Get all v1.x APIs and their v2.0 equivalents
   */
  static getAllMappings(): Record<string, string> {
    return { ...this.V1_TO_V2_MAPPING };
  }
}
