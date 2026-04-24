/**
 * Simulation Registry
 *
 * Auto-discovers and manages simulation scenarios with plugin support.
 * Provides validation, version compatibility checking, and dynamic loading.
 */

import * as fs from 'fs';
import * as path from 'path';
import { promisify } from 'util';
import * as semver from 'semver';

const readdir = promisify(fs.readdir);
const readFile = promisify(fs.readFile);
const stat = promisify(fs.stat);

// ============================================================================
// Types
// ============================================================================

export interface SimulationMetadata {
  id: string;
  name: string;
  version: string;
  category: 'core' | 'experimental' | 'plugin';
  description: string;
  author?: string;
  agentdbVersion: string; // Semver range
  tags?: string[];
  estimatedDuration?: number; // milliseconds
  requiredMemoryMB?: number;
}

export interface ValidationResult {
  valid: boolean;
  errors?: string[];
  warnings?: string[];
}

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
}

export interface SimulationResult {
  id?: number; // Assigned by report store
  scenario: string;
  timestamp: Date;
  config: AgentDBConfig;
  metrics: {
    recall: number;
    latency: number;
    throughput: number;
    memoryUsage: number;
    [key: string]: any;
  };
  insights: string[];
  recommendations: string[];
  iterations?: number;
  duration?: number;
}

export interface SimulationScenario {
  metadata: SimulationMetadata;

  // Main execution entry point
  execute(config: AgentDBConfig): Promise<SimulationResult>;

  // Validation (optional)
  validate?(config: AgentDBConfig): ValidationResult;

  // Cleanup (optional)
  cleanup?(): Promise<void>;
}

// ============================================================================
// Simulation Registry
// ============================================================================

export class SimulationRegistry {
  private scenarios: Map<string, SimulationScenario> = new Map();
  private discoveryPaths: string[] = [];
  private agentdbVersion: string;

  constructor(agentdbVersion: string = '2.0.0') {
    this.agentdbVersion = agentdbVersion;

    // Default discovery paths
    this.discoveryPaths = [
      path.join(__dirname, '../../simulation/scenarios'), // dist/src/cli/lib/../../simulation/scenarios (local dev)
      path.join(__dirname, '../../../simulation/scenarios'), // dist/simulation/scenarios (published package)
      path.join(process.env.HOME || '', '.agentdb', 'plugins'), // User plugins
      path.join(process.cwd(), 'agentdb-plugins') // Project-local plugins
    ];
  }

  // --------------------------------------------------------------------------
  // Discovery
  // --------------------------------------------------------------------------

  /**
   * Discover all simulation scenarios from configured paths.
   */
  async discover(): Promise<SimulationScenario[]> {
    const discovered: SimulationScenario[] = [];

    for (const searchPath of this.discoveryPaths) {
      if (!fs.existsSync(searchPath)) {
        continue;
      }

      const scenarios = await this.discoverInPath(searchPath);
      discovered.push(...scenarios);
    }

    return discovered;
  }

  /**
   * Discover scenarios in a specific directory.
   */
  private async discoverInPath(searchPath: string): Promise<SimulationScenario[]> {
    const discovered: SimulationScenario[] = [];

    try {
      const entries = await readdir(searchPath);

      for (const entry of entries) {
        const fullPath = path.join(searchPath, entry);
        const stats = await stat(fullPath);

        if (stats.isDirectory()) {
          // Check for metadata.json or package.json
          const metadataPath = path.join(fullPath, 'metadata.json');
          const packagePath = path.join(fullPath, 'package.json');

          let metadata: SimulationMetadata | null = null;

          if (fs.existsSync(metadataPath)) {
            const content = await readFile(metadataPath, 'utf-8');
            metadata = JSON.parse(content);
          } else if (fs.existsSync(packagePath)) {
            const content = await readFile(packagePath, 'utf-8');
            const pkg = JSON.parse(content);
            metadata = this.extractMetadataFromPackage(pkg);
          }

          if (metadata) {
            // Load scenario implementation
            const scenario = await this.loadScenario(fullPath, metadata);

            if (scenario && this.isCompatible(scenario)) {
              discovered.push(scenario);
              this.scenarios.set(scenario.metadata.id, scenario);
            }
          }
        }
      }
    } catch (error) {
      console.warn(`Failed to discover scenarios in ${searchPath}:`, error);
    }

    return discovered;
  }

  /**
   * Load a scenario implementation from a directory.
   */
  private async loadScenario(
    scenarioPath: string,
    metadata: SimulationMetadata
  ): Promise<SimulationScenario | null> {
    try {
      // Try to load index.ts, index.js, or main file from package.json
      const possibleFiles = ['index.ts', 'index.js', 'scenario.ts', 'scenario.js'];

      for (const file of possibleFiles) {
        const filePath = path.join(scenarioPath, file);

        if (fs.existsSync(filePath)) {
          // Dynamic import
          const module = await import(filePath);
          const scenario: SimulationScenario = module.default || module;

          // Merge metadata
          scenario.metadata = { ...metadata };

          return scenario;
        }
      }

      console.warn(`No implementation found for scenario: ${metadata.id}`);
      return null;
    } catch (error) {
      console.error(`Failed to load scenario ${metadata.id}:`, error);
      return null;
    }
  }

  /**
   * Extract metadata from package.json.
   */
  private extractMetadataFromPackage(pkg: any): SimulationMetadata | null {
    if (!pkg.agentdb || !pkg.agentdb.scenario) {
      return null;
    }

    const scenario = pkg.agentdb.scenario;

    return {
      id: scenario.id || pkg.name,
      name: scenario.name || pkg.name,
      version: pkg.version || '1.0.0',
      category: scenario.category || 'plugin',
      description: pkg.description || scenario.description || '',
      author: pkg.author || scenario.author,
      agentdbVersion: scenario.agentdbVersion || pkg.engines?.agentdb || '^2.0.0',
      tags: scenario.tags || pkg.keywords,
      estimatedDuration: scenario.estimatedDuration,
      requiredMemoryMB: scenario.requiredMemoryMB
    };
  }

  // --------------------------------------------------------------------------
  // Registry Management
  // --------------------------------------------------------------------------

  /**
   * Get scenario by ID.
   */
  get(id: string): SimulationScenario | undefined {
    return this.scenarios.get(id);
  }

  /**
   * List all scenarios.
   */
  list(): SimulationScenario[] {
    return Array.from(this.scenarios.values());
  }

  /**
   * Filter scenarios by category.
   */
  listByCategory(category: SimulationMetadata['category']): SimulationScenario[] {
    return this.list().filter(s => s.metadata.category === category);
  }

  /**
   * Search scenarios by tags.
   */
  searchByTags(tags: string[]): SimulationScenario[] {
    return this.list().filter(scenario => {
      const scenarioTags = scenario.metadata.tags || [];
      return tags.some(tag => scenarioTags.includes(tag));
    });
  }

  /**
   * Register a scenario manually (for testing or runtime registration).
   */
  register(scenario: SimulationScenario): void {
    // Validate before registering
    const validation = this.validate(scenario);

    if (!validation.valid) {
      throw new Error(
        `Invalid scenario: ${validation.errors?.join(', ')}`
      );
    }

    if (!this.isCompatible(scenario)) {
      throw new Error(
        `Incompatible scenario: requires AgentDB ${scenario.metadata.agentdbVersion}, ` +
        `but current version is ${this.agentdbVersion}`
      );
    }

    this.scenarios.set(scenario.metadata.id, scenario);
  }

  /**
   * Unregister a scenario.
   */
  unregister(id: string): boolean {
    return this.scenarios.delete(id);
  }

  /**
   * Add a custom discovery path.
   */
  addDiscoveryPath(path: string): void {
    if (!this.discoveryPaths.includes(path)) {
      this.discoveryPaths.push(path);
    }
  }

  // --------------------------------------------------------------------------
  // Validation
  // --------------------------------------------------------------------------

  /**
   * Validate scenario implementation.
   */
  validate(scenario: SimulationScenario): ValidationResult {
    const errors: string[] = [];
    const warnings: string[] = [];

    // Check metadata
    if (!scenario.metadata) {
      errors.push('Missing metadata');
    } else {
      if (!scenario.metadata.id) errors.push('Missing metadata.id');
      if (!scenario.metadata.name) errors.push('Missing metadata.name');
      if (!scenario.metadata.version) errors.push('Missing metadata.version');
      if (!scenario.metadata.category) errors.push('Missing metadata.category');
      if (!scenario.metadata.description) warnings.push('Missing metadata.description');
      if (!scenario.metadata.agentdbVersion) {
        errors.push('Missing metadata.agentdbVersion');
      }
    }

    // Check execute function
    if (typeof scenario.execute !== 'function') {
      errors.push('Missing execute() function');
    }

    // Check optional functions
    if (scenario.validate && typeof scenario.validate !== 'function') {
      warnings.push('validate property is not a function');
    }

    if (scenario.cleanup && typeof scenario.cleanup !== 'function') {
      warnings.push('cleanup property is not a function');
    }

    return {
      valid: errors.length === 0,
      errors: errors.length > 0 ? errors : undefined,
      warnings: warnings.length > 0 ? warnings : undefined
    };
  }

  /**
   * Check version compatibility.
   */
  isCompatible(scenario: SimulationScenario): boolean {
    try {
      return semver.satisfies(
        this.agentdbVersion,
        scenario.metadata.agentdbVersion
      );
    } catch (error) {
      console.warn(
        `Invalid semver range: ${scenario.metadata.agentdbVersion}`,
        error
      );
      return false;
    }
  }

  // --------------------------------------------------------------------------
  // Utilities
  // --------------------------------------------------------------------------

  /**
   * Get scenarios grouped by category.
   */
  getGroupedByCategory(): Record<string, SimulationScenario[]> {
    const grouped: Record<string, SimulationScenario[]> = {
      core: [],
      experimental: [],
      plugin: []
    };

    for (const scenario of this.list()) {
      grouped[scenario.metadata.category].push(scenario);
    }

    return grouped;
  }

  /**
   * Get scenario statistics.
   */
  getStats(): {
    total: number;
    byCategory: Record<string, number>;
    compatible: number;
    incompatible: number;
  } {
    const scenarios = this.list();

    return {
      total: scenarios.length,
      byCategory: {
        core: this.listByCategory('core').length,
        experimental: this.listByCategory('experimental').length,
        plugin: this.listByCategory('plugin').length
      },
      compatible: scenarios.filter(s => this.isCompatible(s)).length,
      incompatible: scenarios.filter(s => !this.isCompatible(s)).length
    };
  }

  /**
   * Generate registry report.
   */
  generateReport(): string {
    const stats = this.getStats();
    const scenarios = this.list();

    let report = '# Simulation Registry Report\n\n';
    report += `**Total Scenarios**: ${stats.total}\n`;
    report += `**Core**: ${stats.byCategory.core}\n`;
    report += `**Experimental**: ${stats.byCategory.experimental}\n`;
    report += `**Plugins**: ${stats.byCategory.plugin}\n`;
    report += `**Compatible**: ${stats.compatible}\n`;
    report += `**Incompatible**: ${stats.incompatible}\n\n`;

    report += '## Scenarios\n\n';

    for (const scenario of scenarios) {
      const compatible = this.isCompatible(scenario) ? '✅' : '❌';

      report += `### ${scenario.metadata.name} (${scenario.metadata.id})\n`;
      report += `- **Version**: ${scenario.metadata.version}\n`;
      report += `- **Category**: ${scenario.metadata.category}\n`;
      report += `- **Compatible**: ${compatible}\n`;
      report += `- **Description**: ${scenario.metadata.description}\n`;

      if (scenario.metadata.author) {
        report += `- **Author**: ${scenario.metadata.author}\n`;
      }

      if (scenario.metadata.tags && scenario.metadata.tags.length > 0) {
        report += `- **Tags**: ${scenario.metadata.tags.join(', ')}\n`;
      }

      if (scenario.metadata.estimatedDuration) {
        report += `- **Estimated Duration**: ${scenario.metadata.estimatedDuration}ms\n`;
      }

      if (scenario.metadata.requiredMemoryMB) {
        report += `- **Required Memory**: ${scenario.metadata.requiredMemoryMB}MB\n`;
      }

      report += '\n';
    }

    return report;
  }
}

// ============================================================================
// Factory
// ============================================================================

/**
 * Create and initialize a registry with auto-discovery.
 */
export async function createRegistry(
  agentdbVersion?: string
): Promise<SimulationRegistry> {
  const registry = new SimulationRegistry(agentdbVersion);
  await registry.discover();
  return registry;
}
