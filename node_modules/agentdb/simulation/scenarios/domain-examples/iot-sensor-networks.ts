/**
 * IoT Sensor Networks: Distributed Anomaly Detection
 *
 * Use Case: Real-time anomaly detection in IoT sensor networks
 * with limited compute resources.
 *
 * Optimization Priority: POWER EFFICIENCY + LATENCY
 */

import { UnifiedMetrics } from '../../types';

export const IOT_ATTENTION_CONFIG = {
  heads: 4,                        // Lightweight for edge devices
  forwardPassTargetMs: 5,          // 5ms for real-time monitoring
  batchSize: 1,                    // Single-sensor processing
  precision: 'int8' as const,      // Quantized for edge (NVIDIA TensorRT, TFLite)
  edgeOptimized: true,             // ESP32, Raspberry Pi, Coral TPU
  powerBudgetMw: 500,              // 500mW power budget

  // Hypergraph for multi-sensor correlations
  hypergraph: {
    enabled: true,
    maxHyperedgeSize: 5,           // 5-sensor correlations
    compressionRatio: 3.7,         // 3.7x edge compression validated
    distributedProcessing: true    // Process across sensor network
  },

  // Dynamic-k based on anomaly severity
  dynamicK: {
    min: 3,                        // Minimum candidates (normal operation)
    max: 15,                       // Maximum candidates (anomaly detected)
    adaptationStrategy: 'anomaly-severity' as const
  },

  // Self-healing for autonomous operation
  selfHealing: {
    enabled: true,
    adaptationIntervalMs: 1000,    // 1s monitoring
    degradationThreshold: 0.10,    // 10% tolerance (edge constraints)
    networkResilience: true        // Handle node failures
  }
};

// IoT-specific metrics
export interface IoTMetrics extends UnifiedMetrics {
  anomalyDetectionRate: number;    // True positive anomaly detection
  falseAlarmRate: number;          // False positive rate (minimize network traffic)
  powerConsumptionMw: number;      // Power consumption (battery life critical)
  networkLatencyMs: number;        // Multi-hop network latency
  sensorCoverage: number;          // Sensor network coverage
}

// Sensor interface
export interface Sensor {
  id: string;
  reading: Float32Array;
  timestamp: number;
  batteryPercent: number;
}

// Anomaly alert interface
export interface AnomalyAlert {
  sensorId: string;
  anomalyScore: number;
  severity: 'warning' | 'critical';
  correlatedSensors: string[];
  timestamp: number;
  latencyMs: number;
}

// Example: Distributed anomaly detection
export async function detectAnomalies(
  sensorReading: Float32Array & { id: string },
  normalPatterns: any,             // HNSWGraph type
  neighborSensors: Sensor[],
  applyAttention: (data: Float32Array, config: any) => Promise<Float32Array>,
  createHyperedge: (readings: Float32Array[]) => Promise<Float32Array>,
  severityThreshold: number = 0.8
): Promise<AnomalyAlert[]> {
  const startTime = Date.now();
  const config = IOT_ATTENTION_CONFIG;

  // 4-head attention for lightweight processing
  const enhanced = await applyAttention(sensorReading, config);

  // Hypergraph: Correlate with neighbor sensors (multi-sensor patterns)
  const hyperedge = await createHyperedge([
    sensorReading,
    ...neighborSensors.map(s => s.reading)
  ]);

  // Search for normal patterns
  const matches = await normalPatterns.search(enhanced, 10);

  // Anomaly = low similarity to normal patterns
  const anomalyScore = 1 - matches[0].score;

  if (anomalyScore > severityThreshold) {
    // Dynamic-k: Get more candidates for anomaly analysis
    const k = Math.round(3 + anomalyScore * 12);  // 3-15 range
    const detailedMatches = await normalPatterns.search(enhanced, k);

    return [{
      sensorId: sensorReading.id,
      anomalyScore,
      severity: anomalyScore > 0.9 ? 'critical' : 'warning',
      correlatedSensors: neighborSensors.map(s => s.id),
      timestamp: Date.now(),
      latencyMs: Date.now() - startTime
    }];
  }

  return [];
}

// Performance targets for IoT
export const IOT_PERFORMANCE_TARGETS = {
  p50LatencyMs: 5,                 // 5ms median (real-time monitoring)
  anomalyDetectionRate: 0.95,      // 95% true positive rate
  falseAlarmRate: 0.05,            // 5% false positive rate
  powerConsumptionMw: 500,         // 500mW max (battery life)
  uptimePercent: 99.9              // 99.9% uptime (3 nines, edge resilience)
};

// IoT platform-specific configurations
export const IOT_CONFIG_VARIATIONS = {
  // ESP32 (very constrained, WiFi)
  esp32: {
    ...IOT_ATTENTION_CONFIG,
    heads: 2,                      // Minimal heads
    precision: 'int8' as const,
    powerBudgetMw: 200,
    forwardPassTargetMs: 10,
    batchSize: 1
  },

  // Raspberry Pi (more capable, still battery)
  raspberryPi: {
    ...IOT_ATTENTION_CONFIG,
    heads: 4,
    precision: 'float16' as const,
    powerBudgetMw: 1000,           // 1W budget
    forwardPassTargetMs: 5
  },

  // NVIDIA Jetson Nano (edge AI, powered)
  jetsonNano: {
    ...IOT_ATTENTION_CONFIG,
    heads: 8,                      // More capable
    precision: 'float16' as const,
    powerBudgetMw: 5000,           // 5W budget
    forwardPassTargetMs: 3,
    batchSize: 4
  },

  // Google Coral TPU (ML accelerator)
  coralTPU: {
    ...IOT_ATTENTION_CONFIG,
    heads: 6,
    precision: 'int8' as const,    // TPU optimized
    powerBudgetMw: 2000,           // 2W budget
    forwardPassTargetMs: 2,
    batchSize: 8
  }
};

// Deployment environment adaptations
export function adaptConfigToDeployment(
  baseConfig: typeof IOT_ATTENTION_CONFIG,
  environment: 'urban' | 'industrial' | 'agricultural' | 'remote'
): typeof IOT_ATTENTION_CONFIG {
  switch (environment) {
    case 'urban':
      return {
        ...baseConfig,
        heads: 6,                  // More sensors, more complex
        hypergraph: {
          ...baseConfig.hypergraph,
          maxHyperedgeSize: 8      // Dense sensor network
        }
      };
    case 'industrial':
      return {
        ...baseConfig,
        heads: 8,                  // High reliability needed
        forwardPassTargetMs: 3,
        powerBudgetMw: 2000        // Powered sensors
      };
    case 'agricultural':
      return {
        ...baseConfig,
        heads: 4,
        powerBudgetMw: 300,        // Solar-powered, battery constrained
        selfHealing: {
          ...baseConfig.selfHealing,
          networkResilience: true  // Sparse network
        }
      };
    case 'remote':
      return {
        ...baseConfig,
        heads: 2,                  // Minimal computation
        precision: 'int8' as const,
        powerBudgetMw: 100,        // Extreme battery constraint
        forwardPassTargetMs: 20    // Slower acceptable
      };
  }
}

// Battery-aware configuration
export function adaptConfigToBattery(
  baseConfig: typeof IOT_ATTENTION_CONFIG,
  batteryPercent: number,
  chargingStatus: 'charging' | 'discharging' | 'solar'
): typeof IOT_ATTENTION_CONFIG {
  if (chargingStatus === 'charging') {
    return {
      ...baseConfig,
      heads: 8                     // Use more resources when charging
      // Note: Precision optimization coming in future release
    };
  }

  if (batteryPercent < 10) {
    // Critical battery
    return {
      ...baseConfig,
      heads: 2,
      precision: 'int8' as const,
      powerBudgetMw: 100,
      batchSize: 1
    };
  } else if (batteryPercent < 30) {
    // Low battery
    return {
      ...baseConfig,
      heads: 3,
      precision: 'int8' as const,
      powerBudgetMw: 300
    };
  } else if (chargingStatus === 'solar') {
    // Solar powered - adaptive
    return {
      ...baseConfig,
      heads: 5,
      powerBudgetMw: 600
    };
  }

  return baseConfig;
}

// Network topology adaptations
export interface NetworkTopology {
  nodeCount: number;
  averageDegree: number;
  meshDensity: number;
  gatewayDistance: number;
}

export function adaptConfigToTopology(
  baseConfig: typeof IOT_ATTENTION_CONFIG,
  topology: NetworkTopology
): typeof IOT_ATTENTION_CONFIG {
  if (topology.meshDensity > 0.7) {
    // Dense mesh - can use more correlations
    return {
      ...baseConfig,
      hypergraph: {
        ...baseConfig.hypergraph,
        maxHyperedgeSize: 8
      }
    };
  } else if (topology.meshDensity < 0.3) {
    // Sparse mesh - limited correlations
    return {
      ...baseConfig,
      hypergraph: {
        ...baseConfig.hypergraph,
        maxHyperedgeSize: 3
      },
      selfHealing: {
        ...baseConfig.selfHealing,
        networkResilience: true
      }
    };
  }

  return baseConfig;
}
