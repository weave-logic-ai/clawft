/**
 * Automatic Model Downloader for ONNX Phi-4
 *
 * Downloads Phi-4 ONNX model from HuggingFace on first use
 */
import { createWriteStream, existsSync, mkdirSync } from 'fs';
import { dirname } from 'path';
import { createHash } from 'crypto';
import { readFileSync } from 'fs';
export class ModelDownloader {
    baseUrl = 'https://huggingface.co';
    /**
     * Phi-4 Mini ONNX INT4 quantized model (CPU optimized)
     * Size: ~52MB model + ~4.86GB data = ~4.9GB total
     * Note: Requires TWO files - model.onnx and model.onnx.data
     */
    phi4Model = {
        repo: 'microsoft/Phi-4-mini-instruct-onnx',
        filename: 'cpu_and_mobile/cpu-int4-rtn-block-32-acc-level-4/model.onnx',
        localPath: './models/phi-4-mini/cpu_and_mobile/cpu-int4-rtn-block-32-acc-level-4/model.onnx'
    };
    phi4ModelData = {
        repo: 'microsoft/Phi-4-mini-instruct-onnx',
        filename: 'cpu_and_mobile/cpu-int4-rtn-block-32-acc-level-4/model.onnx.data',
        localPath: './models/phi-4-mini/cpu_and_mobile/cpu-int4-rtn-block-32-acc-level-4/model.onnx.data'
    };
    /**
     * Check if model exists locally
     */
    isModelDownloaded(modelPath) {
        const path = modelPath || this.phi4Model.localPath;
        return existsSync(path);
    }
    /**
     * Get model download URL
     */
    getDownloadUrl(model) {
        return `${this.baseUrl}/${model.repo}/resolve/main/${model.filename}`;
    }
    /**
     * Download model with progress tracking
     */
    async downloadModel(modelInfo, onProgress) {
        const model = modelInfo || this.phi4Model;
        // Check if already downloaded
        if (this.isModelDownloaded(model.localPath)) {
            console.log(`✅ Model already exists at ${model.localPath}`);
            return model.localPath;
        }
        // Create directory
        const dir = dirname(model.localPath);
        if (!existsSync(dir)) {
            mkdirSync(dir, { recursive: true });
        }
        const url = this.getDownloadUrl(model);
        console.log(`📦 Downloading Phi-4 ONNX model from HuggingFace...`);
        console.log(`   URL: ${url}`);
        console.log(`   Destination: ${model.localPath}`);
        try {
            const response = await fetch(url);
            if (!response.ok) {
                throw new Error(`Download failed: ${response.statusText}`);
            }
            const totalSize = parseInt(response.headers.get('content-length') || '0', 10);
            let downloadedSize = 0;
            if (!response.body) {
                throw new Error('Response body is null');
            }
            const fileStream = createWriteStream(model.localPath);
            // Track progress
            const reader = response.body.getReader();
            const chunks = [];
            while (true) {
                const { done, value } = await reader.read();
                if (done)
                    break;
                if (value) {
                    chunks.push(value);
                    downloadedSize += value.length;
                    if (onProgress && totalSize > 0) {
                        onProgress({
                            downloaded: downloadedSize,
                            total: totalSize,
                            percentage: (downloadedSize / totalSize) * 100
                        });
                    }
                    // Write chunk
                    fileStream.write(value);
                }
            }
            fileStream.end();
            // Wait for file stream to finish
            await new Promise((resolve, reject) => {
                fileStream.on('finish', () => resolve());
                fileStream.on('error', reject);
            });
            console.log(`✅ Model downloaded successfully`);
            console.log(`   Size: ${(downloadedSize / (1024 * 1024)).toFixed(2)} MB`);
            console.log(`   Path: ${model.localPath}`);
            return model.localPath;
        }
        catch (error) {
            console.error(`❌ Model download failed:`, error);
            throw new Error(`Failed to download model: ${error}`);
        }
    }
    /**
     * Download Phi-4 ONNX model if needed (downloads BOTH .onnx and .onnx.data files)
     */
    async ensurePhi4Model(onProgress) {
        const mainFileExists = this.isModelDownloaded(this.phi4Model.localPath);
        const dataFileExists = this.isModelDownloaded(this.phi4ModelData.localPath);
        if (mainFileExists && dataFileExists) {
            return this.phi4Model.localPath;
        }
        console.log(`🔍 Phi-4-mini ONNX model not found locally`);
        console.log(`📥 Starting automatic download...`);
        console.log(`   This is a one-time download (~4.9GB total)`);
        console.log(`   Model: microsoft/Phi-4-mini-instruct-onnx (INT4 quantized)`);
        console.log(`   Files: model.onnx (~52MB) + model.onnx.data (~4.86GB)`);
        console.log(``);
        // Download main model file if missing
        if (!mainFileExists) {
            console.log(`📦 Downloading model.onnx...`);
            await this.downloadModel(this.phi4Model, onProgress);
            console.log(``);
        }
        // Download data file if missing
        if (!dataFileExists) {
            console.log(`📦 Downloading model.onnx.data (this is the large 4.86GB file)...`);
            await this.downloadModel(this.phi4ModelData, onProgress);
        }
        return this.phi4Model.localPath;
    }
    /**
     * Verify model file integrity (optional)
     */
    async verifyModel(modelPath, expectedSha256) {
        if (!expectedSha256) {
            return true; // Skip verification if no hash provided
        }
        try {
            const fileBuffer = readFileSync(modelPath);
            const hash = createHash('sha256').update(fileBuffer).digest('hex');
            if (hash !== expectedSha256) {
                console.error(`❌ Model verification failed`);
                console.error(`   Expected: ${expectedSha256}`);
                console.error(`   Got:      ${hash}`);
                return false;
            }
            console.log(`✅ Model verification passed`);
            return true;
        }
        catch (error) {
            console.error(`❌ Model verification error:`, error);
            return false;
        }
    }
    /**
     * Get model info
     */
    getModelInfo() {
        return this.phi4Model;
    }
    /**
     * Format download progress for display
     */
    static formatProgress(progress) {
        const mb = (bytes) => (bytes / (1024 * 1024)).toFixed(2);
        return `${progress.percentage.toFixed(1)}% (${mb(progress.downloaded)}/${mb(progress.total)} MB)`;
    }
}
/**
 * Global singleton instance
 */
export const modelDownloader = new ModelDownloader();
/**
 * Convenience function for downloading Phi-4 model
 */
export async function ensurePhi4Model(onProgress) {
    return modelDownloader.ensurePhi4Model(onProgress);
}
//# sourceMappingURL=model-downloader.js.map