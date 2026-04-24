/**
 * Custom simulation builder
 * Component registry and validation system
 */

import chalk from 'chalk';
import { HelpFormatter } from '../lib/help-formatter.js';

export interface Component {
  id: string;
  name: string;
  category: string;
  description: string;
  optimal: boolean;
  metrics?: string;
  compatibility?: string[];
}

export class ComponentRegistry {
  private static components: Component[] = [
    // Backends
    {
      id: 'ruvector',
      name: 'RuVector',
      category: 'backend',
      description: 'Native RuVector implementation',
      optimal: true,
      metrics: '8.2x speedup',
    },
    {
      id: 'hnswlib',
      name: 'hnswlib',
      category: 'backend',
      description: 'Baseline for comparison',
      optimal: false,
    },
    {
      id: 'faiss',
      name: 'FAISS',
      category: 'backend',
      description: 'Facebook AI Similarity Search',
      optimal: false,
    },

    // Attention
    {
      id: 'attention-8',
      name: '8-head Attention',
      category: 'attention',
      description: 'Multi-head attention with 8 heads',
      optimal: true,
      metrics: '+12.4% improvement',
    },
    {
      id: 'attention-4',
      name: '4-head Attention',
      category: 'attention',
      description: 'Multi-head attention with 4 heads',
      optimal: false,
    },
    {
      id: 'attention-16',
      name: '16-head Attention',
      category: 'attention',
      description: 'Multi-head attention with 16 heads',
      optimal: false,
    },
    {
      id: 'attention-none',
      name: 'No Attention',
      category: 'attention',
      description: 'Baseline without attention',
      optimal: false,
    },

    // Search
    {
      id: 'search-beam-5',
      name: 'Beam-5 Search',
      category: 'search',
      description: 'Beam search with width 5',
      optimal: true,
      metrics: '96.8% recall',
    },
    {
      id: 'search-dynamic-k',
      name: 'Dynamic-k Search',
      category: 'search',
      description: 'Adaptive k (5-20)',
      optimal: true,
      metrics: '-18.4% latency',
    },
    {
      id: 'search-greedy',
      name: 'Greedy Search',
      category: 'search',
      description: 'Baseline greedy search',
      optimal: false,
    },
    {
      id: 'search-astar',
      name: 'A* Search',
      category: 'search',
      description: 'A* pathfinding',
      optimal: false,
    },

    // Clustering
    {
      id: 'cluster-louvain',
      name: 'Louvain Clustering',
      category: 'clustering',
      description: 'Community detection with Louvain',
      optimal: true,
      metrics: 'Q=0.758 modularity',
    },
    {
      id: 'cluster-spectral',
      name: 'Spectral Clustering',
      category: 'clustering',
      description: 'Spectral graph clustering',
      optimal: false,
    },
    {
      id: 'cluster-hierarchical',
      name: 'Hierarchical Clustering',
      category: 'clustering',
      description: 'Hierarchical agglomerative clustering',
      optimal: false,
    },

    // Self-healing
    {
      id: 'self-healing-mpc',
      name: 'MPC Self-Healing',
      category: 'self-healing',
      description: 'Model Predictive Control adaptation',
      optimal: true,
      metrics: '97.9% uptime',
    },
    {
      id: 'self-healing-reactive',
      name: 'Reactive Self-Healing',
      category: 'self-healing',
      description: 'Reactive adaptation',
      optimal: false,
    },
    {
      id: 'self-healing-none',
      name: 'No Self-Healing',
      category: 'self-healing',
      description: 'No adaptation',
      optimal: false,
    },

    // Neural
    {
      id: 'neural-gnn-edges',
      name: 'GNN Edge Selection',
      category: 'neural',
      description: 'GNN-based edge selection',
      optimal: true,
      metrics: '-18% memory',
    },
    {
      id: 'neural-rl-nav',
      name: 'RL Navigation',
      category: 'neural',
      description: 'Reinforcement learning navigation',
      optimal: true,
      metrics: '-26% hops',
    },
    {
      id: 'neural-joint-opt',
      name: 'Joint Optimization',
      category: 'neural',
      description: 'Joint embedding-topology optimization',
      optimal: true,
      metrics: '+9.1% end-to-end',
    },
    {
      id: 'neural-full-pipeline',
      name: 'Full Neural Pipeline',
      category: 'neural',
      description: 'Complete neural augmentation',
      optimal: true,
      metrics: '+29.4% improvement',
      compatibility: ['neural-gnn-edges', 'neural-rl-nav', 'neural-joint-opt'],
    },
  ];

  static getByCategory(category: string): Component[] {
    return this.components.filter((c) => c.category === category);
  }

  static getById(id: string): Component | undefined {
    return this.components.find((c) => c.id === id);
  }

  static getAllCategories(): string[] {
    return [...new Set(this.components.map((c) => c.category))];
  }

  static getOptimalComponents(): Component[] {
    return this.components.filter((c) => c.optimal);
  }

  static validateCompatibility(componentIds: string[]): { valid: boolean; errors: string[] } {
    const errors: string[] = [];

    // Check full neural pipeline requirements
    if (componentIds.includes('neural-full-pipeline')) {
      const required = ['neural-gnn-edges', 'neural-rl-nav', 'neural-joint-opt'];
      const missing = required.filter((r) => !componentIds.includes(r));
      if (missing.length > 0) {
        errors.push(`Full neural pipeline requires: ${missing.join(', ')}`);
      }
    }

    return {
      valid: errors.length === 0,
      errors,
    };
  }
}

export async function runCustomBuilder(): Promise<void> {
  console.log(HelpFormatter.formatCustomHelp());

  console.log(chalk.yellow('\nCustom builder interactive mode coming soon!'));
  console.log(chalk.gray('Use --wizard for interactive simulation creation\n'));
}
