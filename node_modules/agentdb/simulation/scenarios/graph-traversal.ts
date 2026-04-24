/**
 * Graph Traversal Simulation
 *
 * Tests Cypher queries and graph operations
 */

import { createUnifiedDatabase } from '../../src/db-unified.js';
import { EmbeddingService } from '../../src/controllers/EmbeddingService.js';
import * as path from 'path';

export default {
  description: 'Graph database traversal and Cypher query performance',

  async run(config: any) {
    const { verbosity = 2 } = config;

    if (verbosity >= 2) {
      console.log('   📊 Initializing Graph Traversal Simulation');
    }

    // Initialize AgentDB
    const embedder = new EmbeddingService({
      model: 'Xenova/all-MiniLM-L6-v2',
      dimension: 384,
      provider: 'transformers'
    });
    await embedder.initialize();

    const db = await createUnifiedDatabase(
      path.join(process.cwd(), 'simulation', 'data', 'graph-traversal.graph'),
      embedder,
      { forceMode: 'graph' }
    );

    // Get GraphDatabaseAdapter (not raw graph database)
    const graphDb = db.getGraphDatabase()!;

    // Check if we have GraphDatabaseAdapter methods
    if (!('createNode' in graphDb)) {
      throw new Error('Graph database does not support GraphDatabaseAdapter API. Use RuVector graph mode.');
    }

    const results = {
      nodesCreated: 0,
      edgesCreated: 0,
      queriesExecuted: 0,
      avgQueryTime: 0,
      totalTime: 0
    };

    const startTime = performance.now();

    // Create graph nodes using GraphDatabaseAdapter API
    const nodeIds: string[] = [];
    for (let i = 0; i < 50; i++) {
      const embedding = new Float32Array(384).map(() => Math.random());

      // Use GraphDatabaseAdapter.createNode API
      const id = await (graphDb as any).createNode({
        id: `test-node-${i}`,
        embedding,
        labels: ['TestNode'],
        properties: {
          nodeIndex: i.toString(),  // "index" is a reserved keyword in Cypher
          type: i % 2 === 0 ? 'even' : 'odd'
        }
      });

      nodeIds.push(id);
      results.nodesCreated++;
    }

    // Create edges using GraphDatabaseAdapter API
    for (let i = 0; i < 45; i++) {
      const embedding = new Float32Array(384).map(() => Math.random());

      await (graphDb as any).createEdge({
        from: nodeIds[i],
        to: nodeIds[i + 1],
        description: 'NEXT',
        embedding,
        confidence: 0.9
      });

      results.edgesCreated++;
    }

    // Execute Cypher queries
    const queries = [
      'MATCH (n:TestNode) RETURN n LIMIT 10',
      'MATCH (n:TestNode) WHERE n.type = "even" RETURN n LIMIT 10',
      'MATCH (n:TestNode)-[r:NEXT]->(m) RETURN n, r, m LIMIT 10',
      'MATCH (n:TestNode) RETURN count(n)',
      'MATCH (n:TestNode) WHERE n.nodeIndex > "20" RETURN n LIMIT 10'  // "index" is a reserved keyword
    ];

    let totalQueryTime = 0;
    for (const query of queries) {
      const queryStart = performance.now();
      const result = await (graphDb as any).query(query);
      const queryEnd = performance.now();

      totalQueryTime += (queryEnd - queryStart);
      results.queriesExecuted++;

      if (verbosity >= 3) {
        console.log(`      ✅ Query: ${query.substring(0, 50)}... (${(queryEnd - queryStart).toFixed(2)}ms)`);
        console.log(`         Results: ${result.nodes?.length || 0} nodes`);
      }
    }

    results.avgQueryTime = totalQueryTime / queries.length;

    const endTime = performance.now();
    results.totalTime = endTime - startTime;

    db.close();

    if (verbosity >= 2) {
      console.log(`      📊 Nodes Created: ${results.nodesCreated}`);
      console.log(`      📊 Edges Created: ${results.edgesCreated}`);
      console.log(`      📊 Queries Executed: ${results.queriesExecuted}`);
      console.log(`      📊 Avg Query Time: ${results.avgQueryTime.toFixed(2)}ms`);
      console.log(`      ⏱️  Duration: ${results.totalTime.toFixed(2)}ms`);
    }

    return results;
  }
};
