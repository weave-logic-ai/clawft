export { QuicCoordinator, type SwarmAgent, type SwarmMessage, type SwarmState, type SwarmStats, type QuicCoordinatorConfig } from './quic-coordinator.js';
export { TransportRouter, type TransportProtocol, type TransportConfig, type TransportStats, type RouteResult } from './transport-router.js';
import { QuicCoordinator, SwarmAgent } from './quic-coordinator.js';
import { TransportRouter } from './transport-router.js';
export interface SwarmInitOptions {
    swarmId: string;
    topology: 'mesh' | 'hierarchical' | 'ring' | 'star';
    transport?: 'quic' | 'http2' | 'auto';
    maxAgents?: number;
    quicPort?: number;
    quicHost?: string;
    enableFallback?: boolean;
}
export interface SwarmInstance {
    swarmId: string;
    topology: 'mesh' | 'hierarchical' | 'ring' | 'star';
    transport: 'quic' | 'http2';
    coordinator?: QuicCoordinator;
    router: TransportRouter;
    registerAgent: (agent: SwarmAgent) => Promise<void>;
    unregisterAgent: (agentId: string) => Promise<void>;
    getStats: () => any;
    shutdown: () => Promise<void>;
}
/**
 * Initialize a multi-agent swarm with QUIC transport
 *
 * @example
 * ```typescript
 * const swarm = await initSwarm({
 *   swarmId: 'my-swarm',
 *   topology: 'mesh',
 *   transport: 'quic',
 *   quicPort: 4433
 * });
 *
 * await swarm.registerAgent({
 *   id: 'agent-1',
 *   role: 'worker',
 *   host: 'localhost',
 *   port: 4434,
 *   capabilities: ['compute', 'analyze']
 * });
 * ```
 */
export declare function initSwarm(options: SwarmInitOptions): Promise<SwarmInstance>;
/**
 * Check if QUIC transport is available
 */
export declare function checkQuicAvailability(): Promise<boolean>;
//# sourceMappingURL=index.d.ts.map