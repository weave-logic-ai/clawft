/**
 * Robotics Navigation: Adaptive Real-Time Path Planning
 *
 * Use Case: Autonomous robots need real-time similarity search
 * for environment matching and obstacle avoidance.
 *
 * Optimization Priority: BALANCED (latency + accuracy)
 */
import { UnifiedMetrics } from '../../types';
export declare const ROBOTICS_ATTENTION_CONFIG: {
    heads: number;
    forwardPassTargetMs: number;
    batchSize: number;
    precision: "float16";
    edgeOptimized: boolean;
    dynamicHeads: {
        enabled: boolean;
        simple: number;
        complex: number;
        adaptationStrategy: "scene-complexity";
    };
    dynamicK: {
        min: number;
        max: number;
        adaptationStrategy: "obstacle-density";
    };
    selfHealing: {
        enabled: boolean;
        adaptationIntervalMs: number;
        degradationThreshold: number;
        hardwareMonitoring: boolean;
    };
};
export interface RoboticsMetrics extends UnifiedMetrics {
    controlLoopLatencyMs: number;
    navigationAccuracy: number;
    obstacleDetectionRate: number;
    powerConsumptionW: number;
    temperatureCelsius: number;
}
export interface RobotContext {
    velocity: number;
    batteryPercent: number;
    temperatureCelsius: number;
    missionCriticality: 'low' | 'medium' | 'high';
}
export interface NavigationPlan {
    bestMatch: any;
    suggestedPath: any;
    confidence: number;
    latencyMs: number;
}
export declare function matchEnvironment(currentSensorData: Float32Array, // LIDAR, camera, IMU
knownEnvironments: any, // HNSWGraph type
robotContext: RobotContext, applyAttention: (data: Float32Array, config: any) => Promise<Float32Array>, analyzeSceneComplexity: (data: Float32Array) => number, calculateObstacleDensity: (data: Float32Array) => number, computePath: (matches: any[]) => any): Promise<NavigationPlan>;
export declare const ROBOTICS_PERFORMANCE_TARGETS: {
    controlLoopLatencyMs: number;
    navigationAccuracy: number;
    p99LatencyMs: number;
    powerConsumptionW: number;
    uptimePercent: number;
};
export declare const ROBOTICS_CONFIG_VARIATIONS: {
    highPerformance: {
        heads: number;
        forwardPassTargetMs: number;
        precision: "float32";
        powerConsumptionW: number;
        batchSize: number;
        edgeOptimized: boolean;
        dynamicHeads: {
            enabled: boolean;
            simple: number;
            complex: number;
            adaptationStrategy: "scene-complexity";
        };
        dynamicK: {
            min: number;
            max: number;
            adaptationStrategy: "obstacle-density";
        };
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
            hardwareMonitoring: boolean;
        };
    };
    consumerDrone: {
        heads: number;
        forwardPassTargetMs: number;
        precision: "float16";
        powerConsumptionW: number;
        batchSize: number;
        edgeOptimized: boolean;
        dynamicHeads: {
            enabled: boolean;
            simple: number;
            complex: number;
            adaptationStrategy: "scene-complexity";
        };
        dynamicK: {
            min: number;
            max: number;
            adaptationStrategy: "obstacle-density";
        };
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
            hardwareMonitoring: boolean;
        };
    };
    industrialAGV: {
        heads: number;
        forwardPassTargetMs: number;
        precision: "float32";
        dynamicK: {
            min: number;
            max: number;
            adaptationStrategy: "warehouse-density";
        };
        batchSize: number;
        edgeOptimized: boolean;
        dynamicHeads: {
            enabled: boolean;
            simple: number;
            complex: number;
            adaptationStrategy: "scene-complexity";
        };
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
            hardwareMonitoring: boolean;
        };
    };
    embedded: {
        heads: number;
        forwardPassTargetMs: number;
        precision: "int8";
        powerConsumptionW: number;
        batchSize: number;
        edgeOptimized: boolean;
        dynamicHeads: {
            enabled: boolean;
            simple: number;
            complex: number;
            adaptationStrategy: "scene-complexity";
        };
        dynamicK: {
            min: number;
            max: number;
            adaptationStrategy: "obstacle-density";
        };
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
            hardwareMonitoring: boolean;
        };
    };
};
export declare function adaptConfigToEnvironment(baseConfig: typeof ROBOTICS_ATTENTION_CONFIG, environment: 'indoor' | 'outdoor' | 'underground' | 'aerial'): typeof ROBOTICS_ATTENTION_CONFIG;
export declare function adaptConfigToPower(baseConfig: typeof ROBOTICS_ATTENTION_CONFIG, batteryPercent: number): typeof ROBOTICS_ATTENTION_CONFIG;
//# sourceMappingURL=robotics-navigation.d.ts.map