/**
 * TDD London School Tests for VersionDetector
 *
 * Following London School TDD:
 * - Tests define behavior through interactions
 * - Heavy use of mocks and spies
 * - Test public interface, not implementation
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { VersionDetector } from '../../../compatibility/VersionDetector';
import type { V1Config, V2Config, CompatibilityConfig } from '../../../compatibility/types';

describe('VersionDetector', () => {
  describe('detect', () => {
    it('should detect v2.0 when version explicitly set to "2.0"', () => {
      const config: CompatibilityConfig = {
        version: '2.0'
      };

      const result = VersionDetector.detect(config, 'AgenticFlow');

      expect(result.version).toBe('2.0');
      expect(result.confidence).toBeGreaterThanOrEqual(1.0);
      expect(result.indicators).toContain('explicit_version_2.0');
    });

    it('should detect v1.x when version explicitly set to "1.x"', () => {
      const config: CompatibilityConfig = {
        version: '1.x'
      };

      const result = VersionDetector.detect(config, 'AgenticFlow');

      expect(result.version).toBe('1.x');
      expect(result.confidence).toBeGreaterThanOrEqual(1.0);
      expect(result.indicators).toContain('explicit_version_1.x');
    });

    it('should detect v2.0 from backend field presence', () => {
      const config = {
        backend: 'agentdb',
        memory: { path: './db' }
      } as V2Config;

      const result = VersionDetector.detect(config, 'AgenticFlow');

      expect(result.version).toBe('2.0');
      expect(result.indicators).toContain('v2_backend_field');
    });

    it('should detect v2.0 from structured memory config', () => {
      const config = {
        memory: {
          path: './db',
          backend: 'ruvector' as const,
          enableHNSW: true
        }
      };

      const result = VersionDetector.detect(config, 'AgenticFlow');

      expect(result.version).toBe('2.0');
      expect(result.indicators).toContain('v2_memory_structure');
    });

    it('should detect v1.x from memoryPath field', () => {
      const config: V1Config = {
        memoryPath: './swarm-memory.db',
        topology: 'mesh'
      };

      const result = VersionDetector.detect(config, 'AgenticFlow');

      expect(result.version).toBe('1.x');
      expect(result.indicators).toContain('v1_memoryPath_field');
    });

    it('should detect v1.x from simple topology string', () => {
      const config: V1Config = {
        topology: 'mesh',
        maxAgents: 8
      };

      const result = VersionDetector.detect(config, 'AgenticFlow');

      expect(result.version).toBe('1.x');
      expect(result.indicators).toContain('v1_simple_config');
    });

    it('should default to v1.x for empty config (backwards compatibility)', () => {
      const config = {};

      const result = VersionDetector.detect(config, 'AgenticFlow');

      expect(result.version).toBe('1.x');
      expect(result.indicators).toContain('default_v1_compat');
    });

    it('should detect v2.0 from routing config structure', () => {
      const config = {
        routing: {
          provider: 'anthropic',
          model: 'claude-3-opus',
          optimization: 'quality' as const
        }
      };

      const result = VersionDetector.detect(config, 'AgenticFlow');

      expect(result.version).toBe('2.0');
      expect(result.indicators).toContain('v2_routing_structure');
    });

    it('should detect v2.0 from intelligence features', () => {
      const config = {
        intelligence: {
          attentionMechanisms: ['hyperbolic'],
          gnnLearning: true
        }
      };

      const result = VersionDetector.detect(config, 'AgenticFlow');

      expect(result.version).toBe('2.0');
      expect(result.indicators).toContain('v2_intelligence_features');
    });

    it('should have high confidence for explicit version', () => {
      const config: CompatibilityConfig = {
        version: '2.0'
      };

      const result = VersionDetector.detect(config, 'AgenticFlow');

      expect(result.confidence).toBe(1.0);
    });

    it('should have lower confidence for inferred version', () => {
      const config: V1Config = {
        topology: 'mesh'
      };

      const result = VersionDetector.detect(config, 'AgenticFlow');

      expect(result.confidence).toBeLessThan(1.0);
      expect(result.confidence).toBeGreaterThan(0.5);
    });
  });

  describe('isV1API', () => {
    it('should return true for v1 method names', () => {
      expect(VersionDetector.isV1API('initSwarm')).toBe(true);
      expect(VersionDetector.isV1API('spawnAgent')).toBe(true);
      expect(VersionDetector.isV1API('orchestrateTask')).toBe(true);
      expect(VersionDetector.isV1API('getMemory')).toBe(true);
      expect(VersionDetector.isV1API('setMemory')).toBe(true);
      expect(VersionDetector.isV1API('searchMemory')).toBe(true);
      expect(VersionDetector.isV1API('getSwarmStatus')).toBe(true);
      expect(VersionDetector.isV1API('destroySwarm')).toBe(true);
    });

    it('should return false for v2 method names', () => {
      expect(VersionDetector.isV1API('swarms.create')).toBe(false);
      expect(VersionDetector.isV1API('agents.spawn')).toBe(false);
      expect(VersionDetector.isV1API('tasks.orchestrate')).toBe(false);
      expect(VersionDetector.isV1API('memory.retrieve')).toBe(false);
      expect(VersionDetector.isV1API('memory.store')).toBe(false);
    });
  });

  describe('isV2API', () => {
    it('should return true for v2 namespaced methods', () => {
      expect(VersionDetector.isV2API('swarms.create')).toBe(true);
      expect(VersionDetector.isV2API('agents.spawn')).toBe(true);
      expect(VersionDetector.isV2API('tasks.orchestrate')).toBe(true);
      expect(VersionDetector.isV2API('memory.store')).toBe(true);
    });

    it('should return false for v1 flat methods', () => {
      expect(VersionDetector.isV2API('initSwarm')).toBe(false);
      expect(VersionDetector.isV2API('spawnAgent')).toBe(false);
    });
  });

  describe('getAPIMapping', () => {
    it('should return v2 equivalent for v1 API', () => {
      expect(VersionDetector.getAPIMapping('initSwarm')).toBe('swarms.create');
      expect(VersionDetector.getAPIMapping('spawnAgent')).toBe('agents.spawn');
      expect(VersionDetector.getAPIMapping('orchestrateTask')).toBe('tasks.orchestrate');
      expect(VersionDetector.getAPIMapping('getMemory')).toBe('memory.retrieve');
      expect(VersionDetector.getAPIMapping('setMemory')).toBe('memory.store');
      expect(VersionDetector.getAPIMapping('searchMemory')).toBe('memory.vectorSearch');
      expect(VersionDetector.getAPIMapping('getSwarmStatus')).toBe('swarms.status');
      expect(VersionDetector.getAPIMapping('destroySwarm')).toBe('swarms.destroy');
    });

    it('should return undefined for unknown API', () => {
      expect(VersionDetector.getAPIMapping('unknownMethod')).toBeUndefined();
    });
  });
});
