/**
 * Automatic Model Downloader for ONNX Phi-4
 *
 * Downloads Phi-4 ONNX model from HuggingFace on first use
 */
export interface DownloadProgress {
    downloaded: number;
    total: number;
    percentage: number;
}
export interface ModelInfo {
    repo: string;
    filename: string;
    localPath: string;
    sha256?: string;
}
export declare class ModelDownloader {
    private baseUrl;
    /**
     * Phi-4 Mini ONNX INT4 quantized model (CPU optimized)
     * Size: ~52MB model + ~4.86GB data = ~4.9GB total
     * Note: Requires TWO files - model.onnx and model.onnx.data
     */
    private phi4Model;
    private phi4ModelData;
    /**
     * Check if model exists locally
     */
    isModelDownloaded(modelPath?: string): boolean;
    /**
     * Get model download URL
     */
    private getDownloadUrl;
    /**
     * Download model with progress tracking
     */
    downloadModel(modelInfo?: ModelInfo, onProgress?: (progress: DownloadProgress) => void): Promise<string>;
    /**
     * Download Phi-4 ONNX model if needed (downloads BOTH .onnx and .onnx.data files)
     */
    ensurePhi4Model(onProgress?: (progress: DownloadProgress) => void): Promise<string>;
    /**
     * Verify model file integrity (optional)
     */
    verifyModel(modelPath: string, expectedSha256?: string): Promise<boolean>;
    /**
     * Get model info
     */
    getModelInfo(): ModelInfo;
    /**
     * Format download progress for display
     */
    static formatProgress(progress: DownloadProgress): string;
}
/**
 * Global singleton instance
 */
export declare const modelDownloader: ModelDownloader;
/**
 * Convenience function for downloading Phi-4 model
 */
export declare function ensurePhi4Model(onProgress?: (progress: DownloadProgress) => void): Promise<string>;
//# sourceMappingURL=model-downloader.d.ts.map