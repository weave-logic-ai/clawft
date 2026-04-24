/**
 * Medical Imaging: High-Precision Image Similarity Search
 *
 * Use Case: Medical diagnosis requires highest possible accuracy
 * for similar case retrieval and diagnostic assistance.
 *
 * Optimization Priority: RECALL/PRECISION (latency trade-off acceptable)
 */
import { UnifiedMetrics } from '../../types';
export declare const MEDICAL_ATTENTION_CONFIG: {
    heads: number;
    forwardPassTargetMs: number;
    batchSize: number;
    precision: "float32";
    ensembleSize: number;
    recallTarget: number;
    precisionTarget: number;
    selfHealing: {
        enabled: boolean;
        adaptationIntervalMs: number;
        degradationThreshold: number;
        dataIntegrityChecks: boolean;
    };
};
export interface MedicalMetrics extends UnifiedMetrics {
    recallAt100: number;
    precisionAt10: number;
    diagnosticAgreement: number;
    falseNegativeRate: number;
    dataIntegrityScore: number;
}
export interface SimilarCase {
    caseId: string;
    diagnosis: string;
    similarity: number;
    radiologistNotes: string;
    confidence: number;
}
export declare function findSimilarCases(patientScan: Float32Array, // MRI/CT scan embeddings
medicalDatabase: any, // HNSWGraph type
applyAttention: (data: Float32Array, config: any) => Promise<Float32Array>, runEnsemble: (data: Float32Array, size: number) => Promise<any[]>, calculateEnsembleConfidence: (candidate: any, ensemble: any[]) => number, minConfidence?: number): Promise<SimilarCase[]>;
export declare const MEDICAL_PERFORMANCE_TARGETS: {
    recallAt100: number;
    precisionAt10: number;
    p50LatencyMs: number;
    falseNegativeRate: number;
    uptimePercent: number;
};
export declare const MEDICAL_CONFIG_VARIATIONS: {
    ctScans: {
        heads: number;
        recallTarget: number;
        ensembleSize: number;
        forwardPassTargetMs: number;
        batchSize: number;
        precision: "float32";
        precisionTarget: number;
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
            dataIntegrityChecks: boolean;
        };
    };
    mriScans: {
        heads: number;
        multiSequenceFusion: boolean;
        recallTarget: number;
        forwardPassTargetMs: number;
        batchSize: number;
        precision: "float32";
        ensembleSize: number;
        precisionTarget: number;
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
            dataIntegrityChecks: boolean;
        };
    };
    xrays: {
        heads: number;
        forwardPassTargetMs: number;
        recallTarget: number;
        batchSize: number;
        precision: "float32";
        ensembleSize: number;
        precisionTarget: number;
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
            dataIntegrityChecks: boolean;
        };
    };
    pathology: {
        heads: number;
        forwardPassTargetMs: number;
        recallTarget: number;
        hierarchicalProcessing: boolean;
        batchSize: number;
        precision: "float32";
        ensembleSize: number;
        precisionTarget: number;
        selfHealing: {
            enabled: boolean;
            adaptationIntervalMs: number;
            degradationThreshold: number;
            dataIntegrityChecks: boolean;
        };
    };
};
export declare function adaptConfigToUrgency(baseConfig: typeof MEDICAL_ATTENTION_CONFIG, urgency: 'routine' | 'urgent' | 'emergency'): typeof MEDICAL_ATTENTION_CONFIG;
export interface MedicalDataQuality {
    dicomCompliance: boolean;
    resolutionAdequate: boolean;
    contrastQuality: number;
    artifactScore: number;
    calibrationValid: boolean;
}
export declare function validateMedicalData(scan: any): MedicalDataQuality;
//# sourceMappingURL=medical-imaging.d.ts.map