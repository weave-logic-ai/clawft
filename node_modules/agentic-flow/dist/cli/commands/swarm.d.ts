#!/usr/bin/env node
/**
 * P2P Swarm V2 CLI Commands
 *
 * Production-grade P2P swarm coordination with:
 * - Ed25519/X25519 cryptography
 * - GunDB relay coordination
 * - Task execution with claim resolution
 * - Heartbeat-based liveness
 * - Verified member registry
 */
import { Command } from 'commander';
import { P2PSwarmV2 } from '../../swarm/p2p-swarm-v2.js';
export declare function createSwarmCommand(): Command;
export declare function getSwarmInstance(): P2PSwarmV2 | null;
export declare function setSwarmInstance(instance: P2PSwarmV2): void;
//# sourceMappingURL=swarm.d.ts.map