/**
 * TDD London School Tests for DeprecationWarnings
 */

import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { DeprecationWarnings } from '../../../compatibility/DeprecationWarnings';
import type { DeprecationConfig } from '../../../compatibility/types';
import * as fs from 'fs';

describe('DeprecationWarnings', () => {
  let warnings: DeprecationWarnings;
  let consoleWarnSpy: any;
  let consoleErrorSpy: any;

  beforeEach(() => {
    consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    consoleWarnSpy.mockRestore();
    consoleErrorSpy.mockRestore();
  });

  describe('constructor', () => {
    it('should initialize with default config', () => {
      warnings = new DeprecationWarnings();

      expect(warnings.getWarnings()).toEqual([]);
    });

    it('should accept custom config', () => {
      warnings = new DeprecationWarnings({
        emitWarnings: false,
        severity: 'silent'
      });

      warnings.warn('testAPI', {
        message: 'Test warning',
        migration: 'test migration',
        documentation: 'test doc'
      });

      expect(consoleWarnSpy).not.toHaveBeenCalled();
    });
  });

  describe('warn', () => {
    it('should emit warning to console by default', () => {
      warnings = new DeprecationWarnings();

      warnings.warn('initSwarm', {
        message: 'initSwarm() is deprecated',
        migration: 'Use swarms.create()',
        documentation: 'https://docs.example.com'
      });

      expect(consoleWarnSpy).toHaveBeenCalled();
      const call = consoleWarnSpy.mock.calls[0][0];
      expect(call).toContain('initSwarm');
      expect(call).toContain('deprecated');
    });

    it('should not emit when emitWarnings is false', () => {
      warnings = new DeprecationWarnings({ emitWarnings: false });

      warnings.warn('initSwarm', {
        message: 'initSwarm() is deprecated',
        migration: 'Use swarms.create()',
        documentation: 'https://docs.example.com'
      });

      expect(consoleWarnSpy).not.toHaveBeenCalled();
    });

    it('should store warning in history', () => {
      warnings = new DeprecationWarnings();

      warnings.warn('initSwarm', {
        message: 'initSwarm() is deprecated',
        migration: 'Use swarms.create()',
        documentation: 'https://docs.example.com'
      });

      const history = warnings.getWarnings();
      expect(history).toHaveLength(1);
      expect(history[0]).toContain('initSwarm');
    });

    it('should only warn once per API by default', () => {
      warnings = new DeprecationWarnings();

      warnings.warn('initSwarm', {
        message: 'Test',
        migration: 'Test',
        documentation: 'Test'
      });
      warnings.warn('initSwarm', {
        message: 'Test',
        migration: 'Test',
        documentation: 'Test'
      });

      expect(consoleWarnSpy).toHaveBeenCalledTimes(1);
    });

    it('should throw error in strict mode', () => {
      warnings = new DeprecationWarnings({ throwOnDeprecated: true });

      expect(() => {
        warnings.warn('initSwarm', {
          message: 'Test',
          migration: 'Test',
          documentation: 'Test'
        });
      }).toThrow();
    });

    it('should use silent severity', () => {
      warnings = new DeprecationWarnings({ severity: 'silent' });

      warnings.warn('initSwarm', {
        message: 'Test',
        migration: 'Test',
        documentation: 'Test'
      });

      expect(consoleWarnSpy).not.toHaveBeenCalled();
    });

    it('should use soft severity with minimal output', () => {
      warnings = new DeprecationWarnings({ severity: 'soft' });

      warnings.warn('initSwarm', {
        message: 'Test',
        migration: 'Test',
        documentation: 'Test'
      });

      expect(consoleWarnSpy).toHaveBeenCalled();
      const call = consoleWarnSpy.mock.calls[0][0];
      expect(call).not.toContain('Documentation:'); // Soft mode is minimal
    });

    it('should use prominent severity with full output', () => {
      warnings = new DeprecationWarnings({ severity: 'prominent' });

      warnings.warn('initSwarm', {
        message: 'initSwarm() is deprecated',
        migration: 'Use swarms.create()',
        documentation: 'https://docs.example.com'
      });

      expect(consoleErrorSpy).toHaveBeenCalled(); // Prominent uses error
      const call = consoleErrorSpy.mock.calls[0][0];
      expect(call).toContain('initSwarm');
      expect(call).toContain('Migration');
    });
  });

  describe('getWarnings', () => {
    it('should return all warning messages', () => {
      warnings = new DeprecationWarnings();

      warnings.warn('api1', {
        message: 'Warning 1',
        migration: 'Migration 1',
        documentation: 'Doc 1'
      });
      warnings.warn('api2', {
        message: 'Warning 2',
        migration: 'Migration 2',
        documentation: 'Doc 2'
      });

      const history = warnings.getWarnings();
      expect(history).toHaveLength(2);
    });
  });

  describe('clearWarnings', () => {
    it('should clear warning history', () => {
      warnings = new DeprecationWarnings();

      warnings.warn('api1', {
        message: 'Test',
        migration: 'Test',
        documentation: 'Test'
      });

      expect(warnings.getWarnings()).toHaveLength(1);

      warnings.clearWarnings();

      expect(warnings.getWarnings()).toEqual([]);
    });

    it('should reset seen APIs after clear', () => {
      warnings = new DeprecationWarnings();

      warnings.warn('api1', {
        message: 'Test',
        migration: 'Test',
        documentation: 'Test'
      });

      warnings.clearWarnings();

      warnings.warn('api1', {
        message: 'Test',
        migration: 'Test',
        documentation: 'Test'
      });

      expect(consoleWarnSpy).toHaveBeenCalledTimes(2); // Should warn again
    });
  });

  describe('hasWarnings', () => {
    it('should return false when no warnings', () => {
      warnings = new DeprecationWarnings();

      expect(warnings.hasWarnings()).toBe(false);
    });

    it('should return true when warnings exist', () => {
      warnings = new DeprecationWarnings();

      warnings.warn('api1', {
        message: 'Test',
        migration: 'Test',
        documentation: 'Test'
      });

      expect(warnings.hasWarnings()).toBe(true);
    });
  });

  describe('getWarningCount', () => {
    it('should return 0 when no warnings', () => {
      warnings = new DeprecationWarnings();

      expect(warnings.getWarningCount()).toBe(0);
    });

    it('should return correct count', () => {
      warnings = new DeprecationWarnings();

      warnings.warn('api1', {
        message: 'Test',
        migration: 'Test',
        documentation: 'Test'
      });
      warnings.warn('api2', {
        message: 'Test',
        migration: 'Test',
        documentation: 'Test'
      });

      expect(warnings.getWarningCount()).toBe(2);
    });
  });
});
