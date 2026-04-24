/**
 * P2P Swarm V2 MCP Tools
 *
 * Production-grade P2P swarm coordination exposed as MCP tools.
 * Provides decentralized coordination, task execution, and learning sync.
 *
 * Features:
 * - Ed25519/X25519 cryptography
 * - GunDB relay coordination
 * - Task execution with claim resolution
 * - Heartbeat-based liveness
 * - Verified member registry
 */
import { z } from 'zod';
import { P2PSwarmV2 } from '../../../../swarm/p2p-swarm-v2.js';
/**
 * P2P Swarm Connect Tool
 */
export declare const p2pSwarmConnectTool: {
    name: string;
    description: string;
    schema: z.ZodObject<{
        agentId: z.ZodOptional<z.ZodString>;
        swarmKey: z.ZodOptional<z.ZodString>;
        enableExecutor: z.ZodDefault<z.ZodOptional<z.ZodBoolean>>;
    }, "strip", z.ZodTypeAny, {
        agentId?: string;
        swarmKey?: string;
        enableExecutor?: boolean;
    }, {
        agentId?: string;
        swarmKey?: string;
        enableExecutor?: boolean;
    }>;
    execute: (params: {
        agentId?: string;
        swarmKey?: string;
        enableExecutor?: boolean;
    }) => Promise<{
        success: boolean;
        connected: boolean;
        agentId: string;
        swarmId: string;
        swarmKey: string;
        relays: {
            total: number;
            healthy: number;
            avgLatency: number;
        };
        executorEnabled: boolean;
        error?: undefined;
    } | {
        success: boolean;
        error: string;
        connected?: undefined;
        agentId?: undefined;
        swarmId?: undefined;
        swarmKey?: undefined;
        relays?: undefined;
        executorEnabled?: undefined;
    }>;
};
/**
 * P2P Swarm Status Tool
 */
export declare const p2pSwarmStatusTool: {
    name: string;
    description: string;
    schema: z.ZodObject<{}, "strip", z.ZodTypeAny, {}, {}>;
    execute: () => Promise<{
        success: boolean;
        error: string;
    } | {
        liveMembers: {
            agentId: string;
            capabilities: string[];
            lastSeen: number;
            isAlive: boolean;
        }[];
        liveMemberCount: number;
        connected: boolean;
        swarmId: string;
        agentId: string;
        publicKey: string;
        relays: {
            total: number;
            healthy: number;
            avgLatency: number;
        };
        success: boolean;
        error?: undefined;
    }>;
};
/**
 * P2P Swarm Members Tool
 */
export declare const p2pSwarmMembersTool: {
    name: string;
    description: string;
    schema: z.ZodObject<{
        includeOffline: z.ZodDefault<z.ZodOptional<z.ZodBoolean>>;
    }, "strip", z.ZodTypeAny, {
        includeOffline?: boolean;
    }, {
        includeOffline?: boolean;
    }>;
    execute: (params: {
        includeOffline?: boolean;
    }) => Promise<{
        success: boolean;
        error: string;
        count?: undefined;
        liveCount?: undefined;
        members?: undefined;
    } | {
        success: boolean;
        count: number;
        liveCount: number;
        members: {
            agentId: string;
            capabilities: string[];
            isAlive: boolean;
            lastSeenAgo: number;
        }[];
        error?: undefined;
    }>;
};
/**
 * P2P Swarm Publish Tool
 */
export declare const p2pSwarmPublishTool: {
    name: string;
    description: string;
    schema: z.ZodObject<{
        topic: z.ZodString;
        payload: z.ZodAny;
    }, "strip", z.ZodTypeAny, {
        topic?: string;
        payload?: any;
    }, {
        topic?: string;
        payload?: any;
    }>;
    execute: (params: {
        topic: string;
        payload: any;
    }) => Promise<{
        success: boolean;
        error: string;
        messageId?: undefined;
        topic?: undefined;
    } | {
        success: boolean;
        messageId: string;
        topic: string;
        error?: undefined;
    }>;
};
/**
 * P2P Swarm Subscribe Tool
 */
export declare const p2pSwarmSubscribeTool: {
    name: string;
    description: string;
    schema: z.ZodObject<{
        topic: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        topic?: string;
    }, {
        topic?: string;
    }>;
    execute: (params: {
        topic: string;
    }) => Promise<{
        success: boolean;
        error: string;
        subscribed?: undefined;
        note?: undefined;
    } | {
        success: boolean;
        subscribed: string;
        note: string;
        error?: undefined;
    }>;
};
/**
 * P2P Swarm Sync Q-Table Tool
 */
export declare const p2pSwarmSyncQTableTool: {
    name: string;
    description: string;
    schema: z.ZodObject<{
        qTable: z.ZodArray<z.ZodArray<z.ZodNumber, "many">, "many">;
    }, "strip", z.ZodTypeAny, {
        qTable?: number[][];
    }, {
        qTable?: number[][];
    }>;
    execute: (params: {
        qTable: number[][];
    }) => Promise<{
        success: boolean;
        error: string;
        cid?: undefined;
        dimensions?: undefined;
        checksum?: undefined;
        timestamp?: undefined;
    } | {
        success: boolean;
        cid: string;
        dimensions: string;
        checksum: string;
        timestamp: number;
        error?: undefined;
    }>;
};
/**
 * P2P Swarm Sync Memory Tool
 */
export declare const p2pSwarmSyncMemoryTool: {
    name: string;
    description: string;
    schema: z.ZodObject<{
        vectors: z.ZodArray<z.ZodArray<z.ZodNumber, "many">, "many">;
        namespace: z.ZodDefault<z.ZodOptional<z.ZodString>>;
    }, "strip", z.ZodTypeAny, {
        namespace?: string;
        vectors?: number[][];
    }, {
        namespace?: string;
        vectors?: number[][];
    }>;
    execute: (params: {
        vectors: number[][];
        namespace?: string;
    }) => Promise<{
        success: boolean;
        error: string;
        cid?: undefined;
        namespace?: undefined;
        dimensions?: undefined;
        checksum?: undefined;
    } | {
        success: boolean;
        cid: string;
        namespace: string;
        dimensions: string;
        checksum: string;
        error?: undefined;
    }>;
};
/**
 * P2P Swarm Submit Task Tool
 */
export declare const p2pSwarmSubmitTaskTool: {
    name: string;
    description: string;
    schema: z.ZodObject<{
        moduleCID: z.ZodString;
        inputCID: z.ZodString;
        entrypoint: z.ZodDefault<z.ZodOptional<z.ZodString>>;
        fuelLimit: z.ZodDefault<z.ZodOptional<z.ZodNumber>>;
        memoryMB: z.ZodDefault<z.ZodOptional<z.ZodNumber>>;
        timeoutMs: z.ZodDefault<z.ZodOptional<z.ZodNumber>>;
    }, "strip", z.ZodTypeAny, {
        moduleCID?: string;
        entrypoint?: string;
        inputCID?: string;
        fuelLimit?: number;
        memoryMB?: number;
        timeoutMs?: number;
    }, {
        moduleCID?: string;
        entrypoint?: string;
        inputCID?: string;
        fuelLimit?: number;
        memoryMB?: number;
        timeoutMs?: number;
    }>;
    execute: (params: {
        moduleCID: string;
        inputCID: string;
        entrypoint?: string;
        fuelLimit?: number;
        memoryMB?: number;
        timeoutMs?: number;
    }) => Promise<{
        success: boolean;
        error: string;
        taskId?: undefined;
        messageId?: undefined;
        moduleCID?: undefined;
        inputCID?: undefined;
    } | {
        success: boolean;
        taskId: string;
        messageId: string;
        moduleCID: string;
        inputCID: string;
        error?: undefined;
    }>;
};
/**
 * P2P Swarm Start Executor Tool
 */
export declare const p2pSwarmStartExecutorTool: {
    name: string;
    description: string;
    schema: z.ZodObject<{}, "strip", z.ZodTypeAny, {}, {}>;
    execute: () => Promise<{
        success: boolean;
        error: string;
        message?: undefined;
    } | {
        success: boolean;
        message: string;
        error?: undefined;
    }>;
};
/**
 * P2P Swarm Stop Executor Tool
 */
export declare const p2pSwarmStopExecutorTool: {
    name: string;
    description: string;
    schema: z.ZodObject<{}, "strip", z.ZodTypeAny, {}, {}>;
    execute: () => Promise<{
        success: boolean;
        error: string;
        message?: undefined;
    } | {
        success: boolean;
        message: string;
        error?: undefined;
    }>;
};
/**
 * P2P Swarm Disconnect Tool
 */
export declare const p2pSwarmDisconnectTool: {
    name: string;
    description: string;
    schema: z.ZodObject<{}, "strip", z.ZodTypeAny, {}, {}>;
    execute: () => Promise<{
        success: boolean;
        message: string;
    }>;
};
/**
 * P2P Swarm Keygen Tool
 */
export declare const p2pSwarmKeygenTool: {
    name: string;
    description: string;
    schema: z.ZodObject<{}, "strip", z.ZodTypeAny, {}, {}>;
    execute: () => Promise<{
        success: boolean;
        swarmKey: string;
        usage: string;
    }>;
};
export declare const p2pSwarmTools: ({
    name: string;
    description: string;
    schema: z.ZodObject<{
        agentId: z.ZodOptional<z.ZodString>;
        swarmKey: z.ZodOptional<z.ZodString>;
        enableExecutor: z.ZodDefault<z.ZodOptional<z.ZodBoolean>>;
    }, "strip", z.ZodTypeAny, {
        agentId?: string;
        swarmKey?: string;
        enableExecutor?: boolean;
    }, {
        agentId?: string;
        swarmKey?: string;
        enableExecutor?: boolean;
    }>;
    execute: (params: {
        agentId?: string;
        swarmKey?: string;
        enableExecutor?: boolean;
    }) => Promise<{
        success: boolean;
        connected: boolean;
        agentId: string;
        swarmId: string;
        swarmKey: string;
        relays: {
            total: number;
            healthy: number;
            avgLatency: number;
        };
        executorEnabled: boolean;
        error?: undefined;
    } | {
        success: boolean;
        error: string;
        connected?: undefined;
        agentId?: undefined;
        swarmId?: undefined;
        swarmKey?: undefined;
        relays?: undefined;
        executorEnabled?: undefined;
    }>;
} | {
    name: string;
    description: string;
    schema: z.ZodObject<{}, "strip", z.ZodTypeAny, {}, {}>;
    execute: () => Promise<{
        success: boolean;
        error: string;
    } | {
        liveMembers: {
            agentId: string;
            capabilities: string[];
            lastSeen: number;
            isAlive: boolean;
        }[];
        liveMemberCount: number;
        connected: boolean;
        swarmId: string;
        agentId: string;
        publicKey: string;
        relays: {
            total: number;
            healthy: number;
            avgLatency: number;
        };
        success: boolean;
        error?: undefined;
    }>;
} | {
    name: string;
    description: string;
    schema: z.ZodObject<{
        includeOffline: z.ZodDefault<z.ZodOptional<z.ZodBoolean>>;
    }, "strip", z.ZodTypeAny, {
        includeOffline?: boolean;
    }, {
        includeOffline?: boolean;
    }>;
    execute: (params: {
        includeOffline?: boolean;
    }) => Promise<{
        success: boolean;
        error: string;
        count?: undefined;
        liveCount?: undefined;
        members?: undefined;
    } | {
        success: boolean;
        count: number;
        liveCount: number;
        members: {
            agentId: string;
            capabilities: string[];
            isAlive: boolean;
            lastSeenAgo: number;
        }[];
        error?: undefined;
    }>;
} | {
    name: string;
    description: string;
    schema: z.ZodObject<{
        topic: z.ZodString;
        payload: z.ZodAny;
    }, "strip", z.ZodTypeAny, {
        topic?: string;
        payload?: any;
    }, {
        topic?: string;
        payload?: any;
    }>;
    execute: (params: {
        topic: string;
        payload: any;
    }) => Promise<{
        success: boolean;
        error: string;
        messageId?: undefined;
        topic?: undefined;
    } | {
        success: boolean;
        messageId: string;
        topic: string;
        error?: undefined;
    }>;
} | {
    name: string;
    description: string;
    schema: z.ZodObject<{
        topic: z.ZodString;
    }, "strip", z.ZodTypeAny, {
        topic?: string;
    }, {
        topic?: string;
    }>;
    execute: (params: {
        topic: string;
    }) => Promise<{
        success: boolean;
        error: string;
        subscribed?: undefined;
        note?: undefined;
    } | {
        success: boolean;
        subscribed: string;
        note: string;
        error?: undefined;
    }>;
} | {
    name: string;
    description: string;
    schema: z.ZodObject<{
        qTable: z.ZodArray<z.ZodArray<z.ZodNumber, "many">, "many">;
    }, "strip", z.ZodTypeAny, {
        qTable?: number[][];
    }, {
        qTable?: number[][];
    }>;
    execute: (params: {
        qTable: number[][];
    }) => Promise<{
        success: boolean;
        error: string;
        cid?: undefined;
        dimensions?: undefined;
        checksum?: undefined;
        timestamp?: undefined;
    } | {
        success: boolean;
        cid: string;
        dimensions: string;
        checksum: string;
        timestamp: number;
        error?: undefined;
    }>;
} | {
    name: string;
    description: string;
    schema: z.ZodObject<{
        vectors: z.ZodArray<z.ZodArray<z.ZodNumber, "many">, "many">;
        namespace: z.ZodDefault<z.ZodOptional<z.ZodString>>;
    }, "strip", z.ZodTypeAny, {
        namespace?: string;
        vectors?: number[][];
    }, {
        namespace?: string;
        vectors?: number[][];
    }>;
    execute: (params: {
        vectors: number[][];
        namespace?: string;
    }) => Promise<{
        success: boolean;
        error: string;
        cid?: undefined;
        namespace?: undefined;
        dimensions?: undefined;
        checksum?: undefined;
    } | {
        success: boolean;
        cid: string;
        namespace: string;
        dimensions: string;
        checksum: string;
        error?: undefined;
    }>;
} | {
    name: string;
    description: string;
    schema: z.ZodObject<{
        moduleCID: z.ZodString;
        inputCID: z.ZodString;
        entrypoint: z.ZodDefault<z.ZodOptional<z.ZodString>>;
        fuelLimit: z.ZodDefault<z.ZodOptional<z.ZodNumber>>;
        memoryMB: z.ZodDefault<z.ZodOptional<z.ZodNumber>>;
        timeoutMs: z.ZodDefault<z.ZodOptional<z.ZodNumber>>;
    }, "strip", z.ZodTypeAny, {
        moduleCID?: string;
        entrypoint?: string;
        inputCID?: string;
        fuelLimit?: number;
        memoryMB?: number;
        timeoutMs?: number;
    }, {
        moduleCID?: string;
        entrypoint?: string;
        inputCID?: string;
        fuelLimit?: number;
        memoryMB?: number;
        timeoutMs?: number;
    }>;
    execute: (params: {
        moduleCID: string;
        inputCID: string;
        entrypoint?: string;
        fuelLimit?: number;
        memoryMB?: number;
        timeoutMs?: number;
    }) => Promise<{
        success: boolean;
        error: string;
        taskId?: undefined;
        messageId?: undefined;
        moduleCID?: undefined;
        inputCID?: undefined;
    } | {
        success: boolean;
        taskId: string;
        messageId: string;
        moduleCID: string;
        inputCID: string;
        error?: undefined;
    }>;
} | {
    name: string;
    description: string;
    schema: z.ZodObject<{}, "strip", z.ZodTypeAny, {}, {}>;
    execute: () => Promise<{
        success: boolean;
        error: string;
        message?: undefined;
    } | {
        success: boolean;
        message: string;
        error?: undefined;
    }>;
} | {
    name: string;
    description: string;
    schema: z.ZodObject<{}, "strip", z.ZodTypeAny, {}, {}>;
    execute: () => Promise<{
        success: boolean;
        message: string;
    }>;
} | {
    name: string;
    description: string;
    schema: z.ZodObject<{}, "strip", z.ZodTypeAny, {}, {}>;
    execute: () => Promise<{
        success: boolean;
        swarmKey: string;
        usage: string;
    }>;
})[];
export declare function getP2PSwarmInstance(): P2PSwarmV2 | null;
export declare function setP2PSwarmInstance(instance: P2PSwarmV2): void;
//# sourceMappingURL=p2p-swarm-tools.d.ts.map