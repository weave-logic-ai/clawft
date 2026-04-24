/**
 * IoT Sensor Networks: Distributed Anomaly Detection
 *
 * Use Case: Real-time anomaly detection in IoT sensor networks
 * with limited compute resources.
 *
 * Optimization Priority: POWER EFFICIENCY + LATENCY
 */
import { UnifiedMetrics } from '../../types';
export declare const IOT_ATTENTION_CONFIG: {
    heads: number;
    forwardPassTargetMs: number;
    batchSize: number;
    precision: "int8";
    edgeOptimized: boolean;
    powerBudgetMw: number;
    hypergraph: {
        enabled: boolean;
        maxHyperedgeSize: number;
        compressionRatio: number;
        distributedProcessing: boolean;
    };
    dynamicK: {
        min: number;
        max: number;
        adaptationStrategy: "anomaly-severity";
    };
    selfHealing: {
        enabled: boolean;
        adaptationIntervalMs: number;
        degradationThreshold: number;
        networkResilience: boolean;
    };
};
export interface IoTMetrics extends UnifiedMetrics {
    anomalyDetectionRate: number;
    falseAlarmRate: number;
    powerConsumptionMw: number;
    networkLatencyMs: number;
    sensorCoverage: number;
}
export interface Sensor {
    id: string;
    reading: Float32Array;
    timestamp: number;
    batteryPercent: number;
}
export interface AnomalyAlert {
    sensorId: string;
    anomalyScore: number;
    severity: 'warning' | 'critical';
    correlatedSensors: string[];
    timestamp: number;
    latencyMs: number;
}
export declare function detectAnomalies(sensorReading: Float32Array & {
    id: string;
}, normalPatterns: any, // HNSWGraph type
neighborSensors: Sensor[], applyAttention: (data: Float32Array, config: any) => Promise<Float32Array>, createHyperedge: (readings: Float32Array[]) => Promise<Float32Array>, severityThreshold?: number): Promise<AnomalyAlert[]>;
export declare const IOT_PERFORMANCE_TARGETS: {
    p50LatencyMs: number;
    anomalyDetectionRate: number;
    falseAlarmRate: number;
    powerConsumptionMw: number;
    uptimePercent: number;
};
export declare const IOT_CONFIG_VARIATIONS: {
    esp32: {
        heads: number;
        precision: "int8";
        powerBudgetMw: number;
        forwardPassTargetMs: number;
        batchSize: number;
        edgeOptimized: boolean;
        hypergraph: {
            enabled: boolean;
            maxHyperedgeSize: number;
            compressionRatio: number;
            distributedProcessing: boolean;
        };
        dynamicK: {
            min: number;
            max: number;
            adaptationStrategy: "anomaly-severity";
        };
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
            networkResilience: boolean;
        };
    };
    raspberryPi: {
        heads: number;
        precision: "float16";
        powerBudgetMw: number;
        forwardPassTargetMs: number;
        batchSize: number;
        edgeOptimized: boolean;
        hypergraph: {
            enabled: boolean;
            maxHyperedgeSize: number;
            compressionRatio: number;
            distributedProcessing: boolean;
        };
        dynamicK: {
            min: number;
            max: number;
            adaptationStrategy: "anomaly-severity";
        };
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
            networkResilience: boolean;
        };
    };
    jetsonNano: {
        heads: number;
        precision: "float16";
        powerBudgetMw: number;
        forwardPassTargetMs: number;
        batchSize: number;
        edgeOptimized: boolean;
        hypergraph: {
            enabled: boolean;
            maxHyperedgeSize: number;
            compressionRatio: number;
            distributedProcessing: boolean;
        };
        dynamicK: {
            min: number;
            max: number;
            adaptationStrategy: "anomaly-severity";
        };
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
            networkResilience: boolean;
        };
    };
    coralTPU: {
        heads: number;
        precision: "int8";
        powerBudgetMw: number;
        forwardPassTargetMs: number;
        batchSize: number;
        edgeOptimized: boolean;
        hypergraph: {
            enabled: boolean;
            maxHyperedgeSize: number;
            compressionRatio: number;
            distributedProcessing: boolean;
        };
        dynamicK: {
            min: number;
            max: number;
            adaptationStrategy: "anomaly-severity";
        };
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
            networkResilience: boolean;
        };
    };
};
export declare function adaptConfigToDeployment(baseConfig: typeof IOT_ATTENTION_CONFIG, environment: 'urban' | 'industrial' | 'agricultural' | 'remote'): typeof IOT_ATTENTION_CONFIG;
export declare function adaptConfigToBattery(baseConfig: typeof IOT_ATTENTION_CONFIG, batteryPercent: number, chargingStatus: 'charging' | 'discharging' | 'solar'): typeof IOT_ATTENTION_CONFIG;
export interface NetworkTopology {
    nodeCount: number;
    averageDegree: number;
    meshDensity: number;
    gatewayDistance: number;
}
export declare function adaptConfigToTopology(baseConfig: typeof IOT_ATTENTION_CONFIG, topology: NetworkTopology): typeof IOT_ATTENTION_CONFIG;
//# sourceMappingURL=iot-sensor-networks.d.ts.map