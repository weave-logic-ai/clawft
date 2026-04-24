// Type declarations for optional @ruvector dependencies
// These allow TypeScript compilation without installing the packages

declare module '@ruvector/core' {
  export interface VectorEntry {
    id: string;
    vector: Float32Array | number[];
    metadata?: Record<string, any>;
  }

  export interface SearchQuery {
    vector: Float32Array | number[];
    k?: number;
    filter?: Record<string, any>;
    threshold?: number;
  }

  export interface SearchResult {
    id: string;
    score: number;
    vector: Float32Array | number[];
    metadata?: Record<string, any>;
  }

  export interface DbOptions {
    dimension?: number;
    dimensions?: number;
    metric?: 'cosine' | 'euclidean' | 'dot' | 'l2' | 'ip';
    maxElements?: number;
    efConstruction?: number;
    M?: number;
    m?: number;
    path?: string;
    autoPersist?: boolean;
    hnsw?: {
      m?: number;
      efConstruction?: number;
      efSearch?: number;
    };
  }

  export class VectorDB {
    constructor(config: DbOptions);
    insert(entry: VectorEntry): void;
    insertBatch(entries: VectorEntry[]): void;
    search(query: SearchQuery | number[], k?: number): Array<{ id: string; distance: number; score?: number }>;
    get(id: string): VectorEntry | null;
    delete(id: string): boolean;
    remove(id: string): boolean;
    count(): number;
    setEfSearch(ef: number): void;
    save(path?: string): void;
    load(path: string): void;
    clear(): void;
    buildIndex(): void;
    optimize(): void;
    stats(): any;
    memoryUsage?(): number;
  }

  export function isNative(): boolean;
}

declare module '@ruvector/gnn' {
  export class GNNLayer {
    constructor(inputDim: number, outputDim: number, heads: number);
    forward(
      query: number[],
      neighbors: number[][],
      weights: number[]
    ): number[];
    train(
      samples: Array<{ embedding: number[]; label: number }>,
      options: {
        epochs: number;
        learningRate: number;
        batchSize: number;
      }
    ): Promise<{ epochs: number; finalLoss: number }>;
    save(path: string): void;
    load(path: string): void;
  }
}

declare module '@ruvector/graph-node' {
  export interface GraphNode {
    id: string;
    labels: string[];
    properties: Record<string, any>;
  }

  export interface QueryResult {
    nodes: GraphNode[];
    relationships: any[];
  }

  export class GraphDB {
    execute(cypher: string, params?: Record<string, any>): Promise<QueryResult>;
    createNode(labels: string[], properties: Record<string, any>): Promise<string>;
    getNode(id: string): Promise<GraphNode | null>;
    deleteNode(id: string): Promise<boolean>;
  }
}
