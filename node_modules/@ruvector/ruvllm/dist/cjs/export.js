"use strict";
/**
 * Export/Serialization for SONA Models
 *
 * Support for SafeTensors, JSON, and other export formats.
 *
 * @example
 * ```typescript
 * import { ModelExporter, SafeTensorsWriter } from '@ruvector/ruvllm';
 *
 * // Export model to SafeTensors format
 * const exporter = new ModelExporter();
 * const buffer = exporter.toSafeTensors({
 *   weights: loraAdapter.getWeights(),
 *   config: loraAdapter.getConfig(),
 * });
 *
 * // Save to file
 * fs.writeFileSync('model.safetensors', buffer);
 * ```
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.DatasetExporter = exports.ModelImporter = exports.ModelExporter = exports.SafeTensorsReader = exports.SafeTensorsWriter = void 0;
/**
 * SafeTensors Writer
 *
 * Writes tensors in SafeTensors format for compatibility with
 * HuggingFace ecosystem.
 */
class SafeTensorsWriter {
    constructor() {
        this.tensors = new Map();
        this.metadata = {};
    }
    /**
     * Add a tensor
     */
    addTensor(name, data, shape) {
        this.tensors.set(name, { data, shape });
        return this;
    }
    /**
     * Add 2D tensor from number array
     */
    add2D(name, data) {
        const rows = data.length;
        const cols = data[0]?.length || 0;
        const flat = new Float32Array(rows * cols);
        for (let i = 0; i < rows; i++) {
            for (let j = 0; j < cols; j++) {
                flat[i * cols + j] = data[i][j];
            }
        }
        return this.addTensor(name, flat, [rows, cols]);
    }
    /**
     * Add 1D tensor from number array
     */
    add1D(name, data) {
        return this.addTensor(name, new Float32Array(data), [data.length]);
    }
    /**
     * Add metadata
     */
    addMetadata(key, value) {
        this.metadata[key] = value;
        return this;
    }
    /**
     * Build SafeTensors buffer
     */
    build() {
        // Build header
        const header = {};
        let offset = 0;
        const tensorData = [];
        for (const [name, { data, shape }] of this.tensors) {
            const bytes = new Uint8Array(data.buffer);
            const dataLength = bytes.length;
            header[name] = {
                dtype: 'F32',
                shape,
                data_offsets: [offset, offset + dataLength],
            };
            tensorData.push(bytes);
            offset += dataLength;
        }
        // Add metadata
        if (Object.keys(this.metadata).length > 0) {
            header['__metadata__'] = this.metadata;
        }
        // Encode header
        const headerJson = JSON.stringify(header);
        const headerBytes = new TextEncoder().encode(headerJson);
        // Pad header to 8-byte alignment
        const headerPadding = (8 - (headerBytes.length % 8)) % 8;
        const paddedHeaderLength = headerBytes.length + headerPadding;
        // Build final buffer
        const totalLength = 8 + paddedHeaderLength + offset;
        const buffer = new Uint8Array(totalLength);
        const view = new DataView(buffer.buffer);
        // Write header length (8 bytes, little-endian)
        view.setBigUint64(0, BigInt(paddedHeaderLength), true);
        // Write header
        buffer.set(headerBytes, 8);
        // Write tensor data
        let dataOffset = 8 + paddedHeaderLength;
        for (const data of tensorData) {
            buffer.set(data, dataOffset);
            dataOffset += data.length;
        }
        return buffer;
    }
    /**
     * Clear all tensors and metadata
     */
    clear() {
        this.tensors.clear();
        this.metadata = {};
    }
}
exports.SafeTensorsWriter = SafeTensorsWriter;
/**
 * SafeTensors Reader
 *
 * Reads tensors from SafeTensors format.
 */
class SafeTensorsReader {
    constructor(buffer) {
        this.header = {};
        this.dataOffset = 0;
        this.buffer = buffer;
        this.parseHeader();
    }
    /**
     * Get tensor names
     */
    getTensorNames() {
        return Object.keys(this.header).filter(k => k !== '__metadata__');
    }
    /**
     * Get tensor by name
     */
    getTensor(name) {
        const entry = this.header[name];
        if (!entry || typeof entry === 'object' && 'dtype' in entry === false) {
            return null;
        }
        const tensorHeader = entry;
        const [start, end] = tensorHeader.data_offsets;
        const bytes = this.buffer.slice(this.dataOffset + start, this.dataOffset + end);
        return {
            data: new Float32Array(bytes.buffer, bytes.byteOffset, bytes.length / 4),
            shape: tensorHeader.shape,
        };
    }
    /**
     * Get tensor as 2D array
     */
    getTensor2D(name) {
        const tensor = this.getTensor(name);
        if (!tensor || tensor.shape.length !== 2)
            return null;
        const [rows, cols] = tensor.shape;
        const result = [];
        for (let i = 0; i < rows; i++) {
            const row = [];
            for (let j = 0; j < cols; j++) {
                row.push(tensor.data[i * cols + j]);
            }
            result.push(row);
        }
        return result;
    }
    /**
     * Get tensor as 1D array
     */
    getTensor1D(name) {
        const tensor = this.getTensor(name);
        if (!tensor)
            return null;
        return Array.from(tensor.data);
    }
    /**
     * Get metadata
     */
    getMetadata() {
        const meta = this.header['__metadata__'];
        if (!meta || typeof meta !== 'object')
            return {};
        return meta;
    }
    parseHeader() {
        const view = new DataView(this.buffer.buffer, this.buffer.byteOffset);
        const headerLength = Number(view.getBigUint64(0, true));
        const headerBytes = this.buffer.slice(8, 8 + headerLength);
        const headerJson = new TextDecoder().decode(headerBytes);
        this.header = JSON.parse(headerJson.replace(/\0+$/, '')); // Remove padding nulls
        this.dataOffset = 8 + headerLength;
    }
}
exports.SafeTensorsReader = SafeTensorsReader;
/**
 * Model Exporter
 *
 * Unified export interface for SONA models.
 */
class ModelExporter {
    /**
     * Export to SafeTensors format
     */
    toSafeTensors(model) {
        const writer = new SafeTensorsWriter();
        // Add metadata
        writer.addMetadata('name', model.metadata.name);
        writer.addMetadata('version', model.metadata.version);
        writer.addMetadata('architecture', model.metadata.architecture);
        if (model.metadata.training) {
            writer.addMetadata('training_steps', String(model.metadata.training.steps));
            writer.addMetadata('training_loss', String(model.metadata.training.loss));
        }
        // Add LoRA weights
        if (model.loraWeights) {
            writer.add2D('lora.A', model.loraWeights.loraA);
            writer.add2D('lora.B', model.loraWeights.loraB);
            writer.add1D('lora.scaling', [model.loraWeights.scaling]);
        }
        // Add patterns as embeddings
        if (model.patterns && model.patterns.length > 0) {
            const embeddings = model.patterns.map(p => p.embedding);
            writer.add2D('patterns.embeddings', embeddings);
            const successRates = model.patterns.map(p => p.successRate);
            writer.add1D('patterns.success_rates', successRates);
        }
        // Add raw tensors
        if (model.tensors) {
            for (const [name, data] of model.tensors) {
                writer.addTensor(name, data, [data.length]);
            }
        }
        return writer.build();
    }
    /**
     * Export to JSON format
     */
    toJSON(model) {
        return JSON.stringify({
            metadata: model.metadata,
            loraConfig: model.loraConfig,
            loraWeights: model.loraWeights,
            patterns: model.patterns,
            ewcStats: model.ewcStats,
        }, null, 2);
    }
    /**
     * Export to compact binary format
     */
    toBinary(model) {
        const json = this.toJSON(model);
        const jsonBytes = new TextEncoder().encode(json);
        // Simple format: [4-byte length][json bytes]
        const buffer = new Uint8Array(4 + jsonBytes.length);
        const view = new DataView(buffer.buffer);
        view.setUint32(0, jsonBytes.length, true);
        buffer.set(jsonBytes, 4);
        return buffer;
    }
    /**
     * Export for HuggingFace Hub compatibility
     */
    toHuggingFace(model) {
        const safetensors = this.toSafeTensors(model);
        const config = JSON.stringify({
            model_type: 'sona-lora',
            ...model.metadata,
            lora_config: model.loraConfig,
        }, null, 2);
        const readme = `---
license: mit
tags:
- sona
- lora
- ruvector
---

# ${model.metadata.name}

${model.metadata.architecture} model trained with SONA adaptive learning.

## Usage

\`\`\`typescript
import { LoraAdapter, SafeTensorsReader } from '@ruvector/ruvllm';

const reader = new SafeTensorsReader(buffer);
const adapter = new LoraAdapter();
adapter.setWeights({
  loraA: reader.getTensor2D('lora.A'),
  loraB: reader.getTensor2D('lora.B'),
  scaling: reader.getTensor1D('lora.scaling')[0],
});
\`\`\`

## Training Info

- Steps: ${model.metadata.training?.steps || 'N/A'}
- Final Loss: ${model.metadata.training?.loss || 'N/A'}
`;
        return { safetensors, config, readme };
    }
}
exports.ModelExporter = ModelExporter;
/**
 * Model Importer
 *
 * Import models from various formats.
 */
class ModelImporter {
    /**
     * Import from SafeTensors format
     */
    fromSafeTensors(buffer) {
        const reader = new SafeTensorsReader(buffer);
        const metadata = reader.getMetadata();
        const result = {
            metadata: {
                name: metadata.name || 'unknown',
                version: metadata.version || '1.0.0',
                architecture: metadata.architecture || 'sona-lora',
                training: metadata.training_steps ? {
                    steps: parseInt(metadata.training_steps),
                    loss: parseFloat(metadata.training_loss || '0'),
                    learningRate: 0,
                } : undefined,
            },
        };
        // Load LoRA weights
        const loraA = reader.getTensor2D('lora.A');
        const loraB = reader.getTensor2D('lora.B');
        const loraScaling = reader.getTensor1D('lora.scaling');
        if (loraA && loraB && loraScaling) {
            result.loraWeights = {
                loraA,
                loraB,
                scaling: loraScaling[0],
            };
        }
        // Load patterns
        const patternEmbeddings = reader.getTensor2D('patterns.embeddings');
        const patternRates = reader.getTensor1D('patterns.success_rates');
        if (patternEmbeddings && patternRates) {
            result.patterns = patternEmbeddings.map((embedding, i) => ({
                id: `imported-${i}`,
                type: 'query_response',
                embedding,
                successRate: patternRates[i] || 0,
                useCount: 0,
                lastUsed: new Date(),
            }));
        }
        return result;
    }
    /**
     * Import from JSON format
     */
    fromJSON(json) {
        return JSON.parse(json);
    }
    /**
     * Import from binary format
     */
    fromBinary(buffer) {
        const view = new DataView(buffer.buffer, buffer.byteOffset);
        const length = view.getUint32(0, true);
        const jsonBytes = buffer.slice(4, 4 + length);
        const json = new TextDecoder().decode(jsonBytes);
        return this.fromJSON(json);
    }
}
exports.ModelImporter = ModelImporter;
/**
 * Dataset Exporter
 *
 * Export training data in various formats.
 */
class DatasetExporter {
    /**
     * Export to JSONL format (one JSON per line)
     */
    toJSONL(data) {
        return data
            .map(item => JSON.stringify({
            input: item.input,
            output: item.output,
            quality: item.quality,
        }))
            .join('\n');
    }
    /**
     * Export to CSV format
     */
    toCSV(data) {
        const header = 'quality,input,output';
        const rows = data.map(item => `${item.quality},"${item.input.join(',')}","${item.output.join(',')}"`);
        return [header, ...rows].join('\n');
    }
    /**
     * Export patterns for pre-training
     */
    toPretrain(patterns) {
        return patterns
            .filter(p => p.successRate >= 0.7)
            .map(p => JSON.stringify({
            embedding: p.embedding,
            type: p.type,
            quality: p.successRate,
        }))
            .join('\n');
    }
}
exports.DatasetExporter = DatasetExporter;
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiZXhwb3J0LmpzIiwic291cmNlUm9vdCI6IiIsInNvdXJjZXMiOlsiLi4vLi4vc3JjL2V4cG9ydC50cyJdLCJuYW1lcyI6W10sIm1hcHBpbmdzIjoiO0FBQUE7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7R0FtQkc7OztBQWdDSDs7Ozs7R0FLRztBQUNILE1BQWEsaUJBQWlCO0lBQTlCO1FBQ1UsWUFBTyxHQUF5RCxJQUFJLEdBQUcsRUFBRSxDQUFDO1FBQzFFLGFBQVEsR0FBMkIsRUFBRSxDQUFDO0lBMkdoRCxDQUFDO0lBekdDOztPQUVHO0lBQ0gsU0FBUyxDQUFDLElBQVksRUFBRSxJQUFrQixFQUFFLEtBQWU7UUFDekQsSUFBSSxDQUFDLE9BQU8sQ0FBQyxHQUFHLENBQUMsSUFBSSxFQUFFLEVBQUUsSUFBSSxFQUFFLEtBQUssRUFBRSxDQUFDLENBQUM7UUFDeEMsT0FBTyxJQUFJLENBQUM7SUFDZCxDQUFDO0lBRUQ7O09BRUc7SUFDSCxLQUFLLENBQUMsSUFBWSxFQUFFLElBQWdCO1FBQ2xDLE1BQU0sSUFBSSxHQUFHLElBQUksQ0FBQyxNQUFNLENBQUM7UUFDekIsTUFBTSxJQUFJLEdBQUcsSUFBSSxDQUFDLENBQUMsQ0FBQyxFQUFFLE1BQU0sSUFBSSxDQUFDLENBQUM7UUFDbEMsTUFBTSxJQUFJLEdBQUcsSUFBSSxZQUFZLENBQUMsSUFBSSxHQUFHLElBQUksQ0FBQyxDQUFDO1FBRTNDLEtBQUssSUFBSSxDQUFDLEdBQUcsQ0FBQyxFQUFFLENBQUMsR0FBRyxJQUFJLEVBQUUsQ0FBQyxFQUFFLEVBQUUsQ0FBQztZQUM5QixLQUFLLElBQUksQ0FBQyxHQUFHLENBQUMsRUFBRSxDQUFDLEdBQUcsSUFBSSxFQUFFLENBQUMsRUFBRSxFQUFFLENBQUM7Z0JBQzlCLElBQUksQ0FBQyxDQUFDLEdBQUcsSUFBSSxHQUFHLENBQUMsQ0FBQyxHQUFHLElBQUksQ0FBQyxDQUFDLENBQUMsQ0FBQyxDQUFDLENBQUMsQ0FBQztZQUNsQyxDQUFDO1FBQ0gsQ0FBQztRQUVELE9BQU8s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WixPQUFPLEVBQUUsQ0FBQyxDQUFDLFdBQVc7U0FDdkIsQ0FBQyxDQUFDO2FBQ0YsSUFBSSxDQUFDLElBQUksQ0FBQyxDQUFDO0lBQ2hCLENBQUM7Q0FDRjtBQXRDRCwwQ0FzQ0MiLCJzb3VyY2VzQ29udGVudCI6WyIvKipcbiAqIEV4cG9ydC9TZXJpYWxpemF0aW9uIGZvciBTT05BIE1vZGVsc1xuICpcbiAqIFN1cHBvcnQgZm9yIFNhZmVUZW5zb3JzLCBKU09OLCBhbmQgb3RoZXIgZXhwb3J0IGZvcm1hdHMuXG4gKlxuICogQGV4YW1wbGVcbiAqIGBgYHR5cGVzY3JpcHRcbiAqIGltcG9ydCB7IE1vZGVsRXhwb3J0ZXIsIFNhZmVUZW5zb3JzV3JpdGVyIH0gZnJvbSAnQHJ1dmVjdG9yL3J1dmxsbSc7XG4gKlxuICogLy8gRXhwb3J0IG1vZGVsIHRvIFNhZmVUZW5zb3JzIGZvcm1hdFxuICogY29uc3QgZXhwb3J0ZXIgPSBuZXcgTW9kZWxFeHBvcnRlcigpO1xuICogY29uc3QgYnVmZmVyID0gZXhwb3J0ZXIudG9TYWZlVGVuc29ycyh7XG4gKiAgIHdlaWdodHM6IGxvcmFBZGFwdGVyLmdldFdlaWdodHMoKSxcbiAqICAgY29uZmlnOiBsb3JhQWRhcHRlci5nZXRDb25maWcoKSxcbiAqIH0pO1xuICpcbiAqIC8vIFNhdmUgdG8gZmlsZVxuICogZnMud3JpdGVGaWxlU3luYygnbW9kZWwuc2FmZXRlbnNvcnMnLCBidWZmZXIpO1xuICogYGBgXG4gKi9cblxuaW1wb3J0IHsgTG9SQUNvbmZpZywgTGVhcm5lZFBhdHRlcm4sIEV3Y1N0YXRzLCBFbWJlZGRpbmcsIE1vZGVsTWV0YWRhdGEgfSBmcm9tICcuL3R5cGVzJztcbmltcG9ydCB7IExvcmFXZWlnaHRzIH0gZnJvbSAnLi9sb3JhJztcblxuLyoqXG4gKiBFeHBvcnRhYmxlIG1vZGVsIGRhdGFcbiAqL1xuZXhwb3J0IGludGVyZmFjZSBFeHBvcnRhYmxlTW9kZWwge1xuICAvKiogTW9kZWwgbWV0YWRhdGEgKi9cbiAgbWV0YWRhdGE6IE1vZGVsTWV0YWRhdGE7XG4gIC8qKiBMb1JBIHdlaWdodHMgKGlmIGFwcGxpY2FibGUpICovXG4gIGxvcmFXZWlnaHRzPzogTG9yYVdlaWdodHM7XG4gIC8qKiBMb1JBIGNvbmZpZyAqL1xuICBsb3JhQ29uZmlnPzogTG9SQUNvbmZpZztcbiAgLyoqIExlYXJuZWQgcGF0dGVybnMgKi9cbiAgcGF0dGVybnM/OiBMZWFybmVkUGF0dGVybltdO1xuICAvKiogRVdDIHN0YXRpc3RpY3MgKi9cbiAgZXdjU3RhdHM/OiBFd2NTdGF0cztcbiAgLyoqIFJhdyB0ZW5zb3JzICovXG4gIHRlbnNvcnM/OiBNYXA8c3RyaW5nLCBGbG9hdDMyQXJyYXk+O1xufVxuXG4vKipcbiAqIFNhZmVUZW5zb3JzIGhlYWRlciBlbnRyeVxuICovXG5pbnRlcmZhY2UgU2FmZVRlbnNvcnNIZWFkZXIge1xuICBkdHlwZTogc3RyaW5nO1xuICBzaGFwZTogbnVtYmVyW107XG4gIGRhdGFfb2Zmc2V0czogW251bWJlciwgbnVtYmVyXTtcbn1cblxuLyoqXG4gKiBTYWZlVGVuc29ycyBXcml0ZXJcbiAqXG4gKiBXcml0ZXMgdGVuc29ycyBpbiBTYWZlVGVuc29ycyBmb3JtYXQgZm9yIGNvbXBhdGliaWxpdHkgd2l0aFxuICogSHVnZ2luZ0ZhY2UgZWNvc3lzdGVtLlxuICovXG5leHBvcnQgY2xhc3MgU2FmZVRlbnNvcnNXcml0ZXIge1xuICBwcml2YXRlIHRlbnNvcnM6IE1hcDxzdHJpbmcsIHsgZGF0YTogRmxvYXQzMkFycmF5OyBzaGFwZTogbnVtYmVyW10gfT4gPSBuZXcgTWFwKCk7XG4gIHByaXZhdGUgbWV0YWRhdGE6IFJlY29yZDxzdHJpbmcsIHN0cmluZz4gPSB7fTtcblxuICAvKipcbiAgICogQWRkIGEgdGVuc29yXG4gICAqL1xuICBhZGRUZW5zb3IobmFtZTogc3RyaW5nLCBkYXRhOiBGbG9hdDMyQXJyYXksIHNoYXBlOiBudW1iZXJbXSk6IHRoaXMge1xuICAgIHRoaXMudGVuc29ycy5zZXQobmFtZSwgeyBkYXRhLCBzaGFwZSB9KTtcbiAgICByZXR1cm4gdGhpcztcbiAgfVxuXG4gIC8qKlxuICAgKiBBZGQgMkQgdGVuc29yIGZyb20gbnVtYmVyIGFycmF5XG4gICAqL1xuICBhZGQyRChuYW1lOiBzdHJpbmcsIGRhdGE6IG51bWJlcltdW10pOiB0aGlzIHtcbiAgICBjb25zdCByb3dzID0gZGF0YS5sZW5ndGg7XG4gICAgY29uc3QgY29scyA9IGRhdGFbMF0/Lmxlbmd0aCB8fCAwO1xuICAgIGNvbnN0IGZsYXQgPSBuZXcgRmxvYXQzMkFycmF5KHJvd3MgKiBjb2xzKTtcblxuICAgIGZvciAobGV0IGkgPSAwOyBpIDwgcm93czsgaSsrKSB7XG4gICAgICBmb3IgKGxldCBqID0gMDsgaiA8IGNvbHM7IGorKykge1xuICAgICAgICBmbGF0W2kgKiBjb2xzICsgal0gPSBkYXRhW2ldW2pdO1xuICAgICAgfVxuICAgIH1cblxuICAgIHJldHVybiB0aGlzLmFkZFRlbnNvcihuYW1lLCBmbGF0LCBbcm93cywgY29sc10pO1xuICB9XG5cbiAgLyoqXG4gICAqIEFkZCAxRCB0ZW5zb3IgZnJvbSBudW1iZXIgYXJyYXlcbiAgICovXG4gIGFkZDFEKG5hbWU6IHN0cmluZywgZGF0YTogbnVtYmVyW10pOiB0aGlzIHtcbiAgICByZXR1cm4gdGhpcy5hZGRUZW5zb3IobmFtZSwgbmV3IEZsb2F0MzJBcnJheShkYXRhKSwgW2RhdGEubGVuZ3RoXSk7XG4gIH1cblxuICAvKipcbiAgICogQWRkIG1ldGFkYXRhXG4gICAqL1xuICBhZGRNZXRhZGF0YShrZXk6IHN0cmluZywgdmFsdWU6IHN0cmluZyk6IHRoaXMge1xuICAgIHRoaXMubWV0YWRhdGFba2V5XSA9IHZhbHVlO1xuICAgIHJldHVybiB0aGlzO1xuICB9XG5cbiAgLyoqXG4gICAqIEJ1aWxkIFNhZmVUZW5zb3JzIGJ1ZmZlclxuICAgKi9cbiAgYnVpbGQoKTogVWludDhBcnJheSB7XG4gICAgLy8gQnVpbGQgaGVhZGVyXG4gICAgY29uc3QgaGVhZGVyOiBSZWNvcmQ8c3RyaW5nLCBTYWZlVGVuc29yc0hlYWRlciB8IFJlY29yZDxzdHJpbmcsIHN0cmluZz4+ID0ge307XG4gICAgbGV0IG9mZnNldCA9IDA7XG5cbiAgICBjb25zdCB0ZW5zb3JEYXRhOiBVaW50OEFycmF5W10gPSBbXTtcblxuICAgIGZvciAoY29uc3QgW25hbWUsIHsgZGF0YSwgc2hhcGUgfV0gb2YgdGhpcy50ZW5zb3JzKSB7XG4gICAgICBjb25zdCBieXRlcyA9IG5ldyBVaW50OEFycmF5KGRhdGEuYnVmZmVyKTtcbiAgICAgIGNvbnN0IGRhdGFMZW5ndGggPSBieXRlcy5sZW5ndGg7XG5cbiAgICAgIGhlYWRlcltuYW1lXSA9IHtcbiAgICAgICAgZHR5cGU6ICdGMzInLFxuICAgICAgICBzaGFwZSxcbiAgICAgICAgZGF0YV9vZmZzZXRzOiBbb2Zmc2V0LCBvZmZzZXQgKyBkYXRhTGVuZ3RoXSxcbiAgICAgIH07XG5cbiAgICAgIHRlbnNvckRhdGEucHVzaChieXRlcyk7XG4gICAgICBvZmZzZXQgKz0gZGF0YUxlbmd0aDtcbiAgICB9XG5cbiAgICAvLyBBZGQgbWV0YWRhdGFcbiAgICBpZiAoT2JqZWN0LmtleXModGhpcy5tZXRhZGF0YSkubGVuZ3RoID4gMCkge1xuICAgICAgaGVhZGVyWydfX21ldGFkYXRhX18nXSA9IHRoaXMubWV0YWRhdGE7XG4gICAgfVxuXG4gICAgLy8gRW5jb2RlIGhlYWRlclxuICAgIGNvbnN0IGhlYWRlckpzb24gPSBKU09OLnN0cmluZ2lmeShoZWFkZXIpO1xuICAgIGNvbnN0IGhlYWRlckJ5dGVzID0gbmV3IFRleHRFbmNvZGVyKCkuZW5jb2RlKGhlYWRlckpzb24pO1xuXG4gICAgLy8gUGFkIGhlYWRlciB0byA4LWJ5dGUgYWxpZ25tZW50XG4gICAgY29uc3QgaGVhZGVyUGFkZGluZyA9ICg4IC0gKGhlYWRlckJ5dGVzLmxlbmd0aCAlIDgpKSAlIDg7XG4gICAgY29uc3QgcGFkZGVkSGVhZGVyTGVuZ3RoID0gaGVhZGVyQnl0ZXMubGVuZ3RoICsgaGVhZGVyUGFkZGluZztcblxuICAgIC8vIEJ1aWxkIGZpbmFsIGJ1ZmZlclxuICAgIGNvbnN0IHRvdGFsTGVuZ3RoID0gOCArIHBhZGRlZEhlYWRlckxlbmd0aCArIG9mZnNldDtcbiAgICBjb25zdCBidWZmZXIgPSBuZXcgVWludDhBcnJheSh0b3RhbExlbmd0aCk7XG4gICAgY29uc3QgdmlldyA9IG5ldyBEYXRhVmlldyhidWZmZXIuYnVmZmVyKTtcblxuICAgIC8vIFdyaXRlIGhlYWRlciBsZW5ndGggKDggYnl0ZXMsIGxpdHRsZS1lbmRpYW4pXG4gICAgdmlldy5zZXRCaWdVaW50NjQoMCwgQmlnSW50KHBhZGRlZEhlYWRlckxlbmd0aCksIHRydWUpO1xuXG4gICAgLy8gV3JpdGUgaGVhZGVyXG4gICAgYnVmZmVyLnNldChoZWFkZXJCeXRlcywgOCk7XG5cbiAgICAvLyBXcml0ZSB0ZW5zb3IgZGF0YVxuICAgIGxldCBkYXRhT2Zmc2V0ID0gOCArIHBhZGRlZEhlYWRlckxlbmd0aDtcbiAgICBmb3IgKGNvbnN0IGRhdGEgb2YgdGVuc29yRGF0YSkge1xuICAgICAgYnVmZmVyLnNldChkYXRhLCBkYXRhT2Zmc2V0KTtcbiAgICAgIGRhdGFPZmZzZXQgKz0gZGF0YS5sZW5ndGg7XG4gICAgfVxuXG4gICAgcmV0dXJuIGJ1ZmZlcjtcbiAgfVxuXG4gIC8qKlxuICAgKiBDbGVhciBhbGwgdGVuc29ycyBhbmQgbWV0YWRhdGFcbiAgICovXG4gIGNsZWFyKCk6IHZvaWQge1xuICAgIHRoaXMudGVuc29ycy5jbGVhcigpO1xuICAgIHRoaXMubWV0YWRhdGEgPSB7fTtcbiAgfVxufVxuXG4vKipcbiAqIFNhZmVUZW5zb3JzIFJlYWRlclxuICpcbiAqIFJlYWRzIHRlbnNvcnMgZnJvbSBTYWZlVGVuc29ycyBmb3JtYXQuXG4gKi9cbmV4cG9ydCBjbGFzcyBTYWZlVGVuc29yc1JlYWRlciB7XG4gIHByaXZhdGUgYnVmZmVyOiBVaW50OEFycmF5O1xuICBwcml2YXRlIGhlYWRlcjogUmVjb3JkPHN0cmluZywgU2FmZVRlbnNvcnNIZWFkZXIgfCBSZWNvcmQ8c3RyaW5nLCBzdHJpbmc+PiA9IHt9O1xuICBwcml2YXRlIGRhdGFPZmZzZXQ6IG51bWJlciA9IDA7XG5cbiAgY29uc3RydWN0b3IoYnVmZmVyOiBVaW50OEFycmF5KSB7XG4gICAgdGhpcy5idWZmZXIgPSBidWZmZXI7XG4gICAgdGhpcy5wYXJzZUhlYWRlcigpO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCB0ZW5zb3IgbmFtZXNcbiAgICovXG4gIGdldFRlbnNvck5hbWVzKCk6IHN0cmluZ1tdIHtcbiAgICByZXR1cm4gT2JqZWN0LmtleXModGhpcy5oZWFkZXIpLmZpbHRlcihrID0+IGsgIT09ICdfX21ldGFkYXRhX18nKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgdGVuc29yIGJ5IG5hbWVcbiAgICovXG4gIGdldFRlbnNvcihuYW1lOiBzdHJpbmcpOiB7IGRhdGE6IEZsb2F0MzJBcnJheTsgc2hhcGU6IG51bWJlcltdIH0gfCBudWxsIHtcbiAgICBjb25zdCBlbnRyeSA9IHRoaXMuaGVhZGVyW25hbWVdO1xuICAgIGlmICghZW50cnkgfHwgdHlwZW9mIGVudHJ5ID09PSAnb2JqZWN0JyAmJiAnZHR5cGUnIGluIGVudHJ5ID09PSBmYWxzZSkge1xuICAgICAgcmV0dXJuIG51bGw7XG4gICAgfVxuXG4gICAgY29uc3QgdGVuc29ySGVhZGVyID0gZW50cnkgYXMgU2FmZVRlbnNvcnNIZWFkZXI7XG4gICAgY29uc3QgW3N0YXJ0LCBlbmRdID0gdGVuc29ySGVhZGVyLmRhdGFfb2Zmc2V0cztcbiAgICBjb25zdCBieXRlcyA9IHRoaXMuYnVmZmVyLnNsaWNlKHRoaXMuZGF0YU9mZnNldCArIHN0YXJ0LCB0aGlzLmRhdGFPZmZzZXQgKyBlbmQpO1xuXG4gICAgcmV0dXJuIHtcbiAgICAgIGRhdGE6IG5ldyBGbG9hdDMyQXJyYXkoYnl0ZXMuYnVmZmVyLCBieXRlcy5ieXRlT2Zmc2V0LCBieXRlcy5sZW5ndGggLyA0KSxcbiAgICAgIHNoYXBlOiB0ZW5zb3JIZWFkZXIuc2hhcGUsXG4gICAgfTtcbiAgfVxuXG4gIC8qKlxuICAgKiBHZXQgdGVuc29yIGFzIDJEIGFycmF5XG4gICAqL1xuICBnZXRUZW5zb3IyRChuYW1lOiBzdHJpbmcpOiBudW1iZXJbXVtdIHwgbnVsbCB7XG4gICAgY29uc3QgdGVuc29yID0gdGhpcy5nZXRUZW5zb3IobmFtZSk7XG4gICAgaWYgKCF0ZW5zb3IgfHwgdGVuc29yLnNoYXBlLmxlbmd0aCAhPT0gMikgcmV0dXJuIG51bGw7XG5cbiAgICBjb25zdCBbcm93cywgY29sc10gPSB0ZW5zb3Iuc2hhcGU7XG4gICAgY29uc3QgcmVzdWx0OiBudW1iZXJbXVtdID0gW107XG5cbiAgICBmb3IgKGxldCBpID0gMDsgaSA8IHJvd3M7IGkrKykge1xuICAgICAgY29uc3Qgcm93OiBudW1iZXJbXSA9IFtdO1xuICAgICAgZm9yIChsZXQgaiA9IDA7IGogPCBjb2xzOyBqKyspIHtcbiAgICAgICAgcm93LnB1c2godGVuc29yLmRhdGFbaSAqIGNvbHMgKyBqXSk7XG4gICAgICB9XG4gICAgICByZXN1bHQucHVzaChyb3cpO1xuICAgIH1cblxuICAgIHJldHVybiByZXN1bHQ7XG4gIH1cblxuICAvKipcbiAgICogR2V0IHRlbnNvciBhcyAxRCBhcnJheVxuICAgKi9cbiAgZ2V0VGVuc29yMUQobmFtZTogc3RyaW5nKTogbnVtYmVyW10gfCBudWxsIHtcbiAgICBjb25zdCB0ZW5zb3IgPSB0aGlzLmdldFRlbnNvcihuYW1lKTtcbiAgICBpZiAoIXRlbnNvcikgcmV0dXJuIG51bGw7XG4gICAgcmV0dXJuIEFycmF5LmZyb20odGVuc29yLmRhdGEpO1xuICB9XG5cbiAgLyoqXG4gICAqIEdldCBtZXRhZGF0YVxuICAgKi9cbiAgZ2V0TWV0YWRhdGEoKTogUmVjb3JkPHN0cmluZywgc3RyaW5nPiB7XG4gICAgY29uc3QgbWV0YSA9IHRoaXMuaGVhZGVyWydfX21ldGFkYXRhX18nXTtcbiAgICBpZiAoIW1ldGEgfHwgdHlwZW9mIG1ldGEgIT09ICdvYmplY3QnKSByZXR1cm4ge307XG4gICAgcmV0dXJuIG1ldGEgYXMgUmVjb3JkPHN0cmluZywgc3RyaW5nPjtcbiAgfVxuXG4gIHByaXZhdGUgcGFyc2VIZWFkZXIoKTogdm9pZCB7XG4gICAgY29uc3QgdmlldyA9IG5ldyBEYXRhVmlldyh0aGlzLmJ1ZmZlci5idWZmZXIsIHRoaXMuYnVmZmVyLmJ5dGVPZmZzZXQpO1xuICAgIGNvbnN0IGhlYWRlckxlbmd0aCA9IE51bWJlcih2aWV3LmdldEJpZ1VpbnQ2NCgwLCB0cnVlKSk7XG5cbiAgICBjb25zdCBoZWFkZXJCeXRlcyA9IHRoaXMuYnVmZmVyLnNsaWNlKDgsIDggKyBoZWFkZXJMZW5ndGgpO1xuICAgIGNvbnN0IGhlYWRlckpzb24gPSBuZXcgVGV4dERlY29kZXIoKS5kZWNvZGUoaGVhZGVyQnl0ZXMpO1xuICAgIHRoaXMuaGVhZGVyID0gSlNPTi5wYXJzZShoZWFkZXJKc29uLnJlcGxhY2UoL1xcMCskLywgJycpKTsgLy8gUmVtb3ZlIHBhZGRpbmcgbnVsbHNcblxuICAgIHRoaXMuZGF0YU9mZnNldCA9IDggKyBoZWFkZXJMZW5ndGg7XG4gIH1cbn1cblxuLyoqXG4gKiBNb2RlbCBFeHBvcnRlclxuICpcbiAqIFVuaWZpZWQgZXhwb3J0IGludGVyZmFjZSBmb3IgU09OQSBtb2RlbHMuXG4gKi9cbmV4cG9ydCBjbGFzcyBNb2RlbEV4cG9ydGVyIHtcbiAgLyoqXG4gICAqIEV4cG9ydCB0byBTYWZlVGVuc29ycyBmb3JtYXRcbiAgICovXG4gIHRvU2FmZVRlbnNvcnMobW9kZWw6IEV4cG9ydGFibGVNb2RlbCk6IFVpbnQ4QXJyYXkge1xuICAgIGNvbnN0IHdyaXRlciA9IG5ldyBTYWZlVGVuc29yc1dyaXRlcigpO1xuXG4gICAgLy8gQWRkIG1ldGFkYXRhXG4gICAgd3JpdGVyLmFkZE1ldGFkYXRhKCduYW1lJywgbW9kZWwubWV0YWRhdGEubmFtZSk7XG4gICAgd3JpdGVyLmFkZE1ldGFkYXRhKCd2ZXJzaW9uJywgbW9kZWwubWV0YWRhdGEudmVyc2lvbik7XG4gICAgd3JpdGVyLmFkZE1ldGFkYXRhKCdhcmNoaXRlY3R1cmUnLCBtb2RlbC5tZXRhZGF0YS5hcmNoaXRlY3R1cmUpO1xuXG4gICAgaWYgKG1vZGVsLm1ldGFkYXRhLnRyYWluaW5nKSB7XG4gICAgICB3cml0ZXIuYWRkTWV0YWRhdGEoJ3RyYWluaW5nX3N0ZXBzJywgU3RyaW5nKG1vZGVsLm1ldGFkYXRhLnRyYWluaW5nLnN0ZXBzKSk7XG4gICAgICB3cml0ZXIuYWRkTWV0YWRhdGEoJ3RyYWluaW5nX2xvc3MnLCBTdHJpbmcobW9kZWwubWV0YWRhdGEudHJhaW5pbmcubG9zcykpO1xuICAgIH1cblxuICAgIC8vIEFkZCBMb1JBIHdlaWdodHNcbiAgICBpZiAobW9kZWwubG9yYVdlaWdodHMpIHtcbiAgICAgIHdyaXRlci5hZGQyRCgnbG9yYS5BJywgbW9kZWwubG9yYVdlaWdodHMubG9yYUEpO1xuICAgICAgd3JpdGVyLmFkZDJEKCdsb3JhLkInLCBtb2RlbC5sb3JhV2VpZ2h0cy5sb3JhQik7XG4gICAgICB3cml0ZXIuYWRkMUQoJ2xvcmEuc2NhbGluZycsIFttb2RlbC5sb3JhV2VpZ2h0cy5zY2FsaW5nXSk7XG4gICAgfVxuXG4gICAgLy8gQWRkIHBhdHRlcm5zIGFzIGVtYmVkZGluZ3NcbiAgICBpZiAobW9kZWwucGF0dGVybnMgJiYgbW9kZWwucGF0dGVybnMubGVuZ3RoID4gMCkge1xuICAgICAgY29uc3QgZW1iZWRkaW5nczogbnVtYmVyW11bXSA9IG1vZGVsLnBhdHRlcm5zLm1hcChwID0+IHAuZW1iZWRkaW5nKTtcbiAgICAgIHdyaXRlci5hZGQyRCgncGF0dGVybnMuZW1iZWRkaW5ncycsIGVtYmVkZGluZ3MpO1xuXG4gICAgICBjb25zdCBzdWNjZXNzUmF0ZXMgPSBtb2RlbC5wYXR0ZXJucy5tYXAocCA9PiBwLnN1Y2Nlc3NSYXRlKTtcbiAgICAgIHdyaXRlci5hZGQxRCgncGF0dGVybnMuc3VjY2Vzc19yYXRlcycsIHN1Y2Nlc3NSYXRlcyk7XG4gICAgfVxuXG4gICAgLy8gQWRkIHJhdyB0ZW5zb3JzXG4gICAgaWYgKG1vZGVsLnRlbnNvcnMpIHtcbiAgICAgIGZvciAoY29uc3QgW25hbWUsIGRhdGFdIG9mIG1vZGVsLnRlbnNvcnMpIHtcbiAgICAgICAgd3JpdGVyLmFkZFRlbnNvcihuYW1lLCBkYXRhLCBbZGF0YS5sZW5ndGhdKTtcbiAgICAgIH1cbiAgICB9XG5cbiAgICByZXR1cm4gd3JpdGVyLmJ1aWxkKCk7XG4gIH1cblxuICAvKipcbiAgICogRXhwb3J0IHRvIEpTT04gZm9ybWF0XG4gICAqL1xuICB0b0pTT04obW9kZWw6IEV4cG9ydGFibGVNb2RlbCk6IHN0cmluZyB7XG4gICAgcmV0dXJuIEpTT04uc3RyaW5naWZ5KHtcbiAgICAgIG1ldGFkYXRhOiBtb2RlbC5tZXRhZGF0YSxcbiAgICAgIGxvcmFDb25maWc6IG1vZGVsLmxvcmFDb25maWcsXG4gICAgICBsb3JhV2VpZ2h0czogbW9kZWwubG9yYVdlaWdodHMsXG4gICAgICBwYXR0ZXJuczogbW9kZWwucGF0dGVybnMsXG4gICAgICBld2NTdGF0czogbW9kZWwuZXdjU3RhdHMsXG4gICAgfSwgbnVsbCwgMik7XG4gIH1cblxuICAvKipcbiAgICogRXhwb3J0IHRvIGNvbXBhY3QgYmluYXJ5IGZvcm1hdFxuICAgKi9cbiAgdG9CaW5hcnkobW9kZWw6IEV4cG9ydGFibGVNb2RlbCk6IFVpbnQ4QXJyYXkge1xuICAgIGNvbnN0IGpzb24gPSB0aGlzLnRvSlNPTihtb2RlbCk7XG4gICAgY29uc3QganNvbkJ5dGVzID0gbmV3IFRleHRFbmNvZGVyKCkuZW5jb2RlKGpzb24pO1xuXG4gICAgLy8gU2ltcGxlIGZvcm1hdDogWzQtYnl0ZSBsZW5ndGhdW2pzb24gYnl0ZXNdXG4gICAgY29uc3QgYnVmZmVyID0gbmV3IFVpbnQ4QXJyYXkoNCArIGpzb25CeXRlcy5sZW5ndGgpO1xuICAgIGNvbnN0IHZpZXcgPSBuZXcgRGF0YVZpZXcoYnVmZmVyLmJ1ZmZlcik7XG4gICAgdmlldy5zZXRVaW50MzIoMCwganNvbkJ5dGVzLmxlbmd0aCwgdHJ1ZSk7XG4gICAgYnVmZmVyLnNldChqc29uQnl0ZXMsIDQpO1xuXG4gICAgcmV0dXJuIGJ1ZmZlcjtcbiAgfVxuXG4gIC8qKlxuICAgKiBFeHBvcnQgZm9yIEh1Z2dpbmdGYWNlIEh1YiBjb21wYXRpYmlsaXR5XG4gICAqL1xuICB0b0h1Z2dpbmdGYWNlKG1vZGVsOiBFeHBvcnRhYmxlTW9kZWwpOiB7XG4gICAgc2FmZXRlbnNvcnM6IFVpbnQ4QXJyYXk7XG4gICAgY29uZmlnOiBzdHJpbmc7XG4gICAgcmVhZG1lOiBzdHJpbmc7XG4gIH0ge1xuICAgIGNvbnN0IHNhZmV0ZW5zb3JzID0gdGhpcy50b1NhZmVUZW5zb3JzKG1vZGVsKTtcblxuICAgIGNvbnN0IGNvbmZpZyA9IEpTT04uc3RyaW5naWZ5KHtcbiAgICAgIG1vZGVsX3R5cGU6ICdzb25hLWxvcmEnLFxuICAgICAgLi4ubW9kZWwubWV0YWRhdGEsXG4gICAgICBsb3JhX2NvbmZpZzogbW9kZWwubG9yYUNvbmZpZyxcbiAgICB9LCBudWxsLCAyKTtcblxuICAgIGNvbnN0IHJlYWRtZSA9IGAtLS1cbmxpY2Vuc2U6IG1pdFxudGFnczpcbi0gc29uYVxuLSBsb3JhXG4tIHJ1dmVjdG9yXG4tLS1cblxuIyAke21vZGVsLm1ldGFkYXRhLm5hbWV9XG5cbiR7bW9kZWwubWV0YWRhdGEuYXJjaGl0ZWN0dXJlfSBtb2RlbCB0cmFpbmVkIHdpdGggU09OQSBhZGFwdGl2ZSBsZWFybmluZy5cblxuIyMgVXNhZ2VcblxuXFxgXFxgXFxgdHlwZXNjcmlwdFxuaW1wb3J0IHsgTG9yYUFkYXB0ZXIsIFNhZmVUZW5zb3JzUmVhZGVyIH0gZnJvbSAnQHJ1dmVjdG9yL3J1dmxsbSc7XG5cbmNvbnN0IHJlYWRlciA9IG5ldyBTYWZlVGVuc29yc1JlYWRlcihidWZmZXIpO1xuY29uc3QgYWRhcHRlciA9IG5ldyBMb3JhQWRhcHRlcigpO1xuYWRhcHRlci5zZXRXZWlnaHRzKHtcbiAgbG9yYUE6IHJlYWRlci5nZXRUZW5zb3IyRCgnbG9yYS5BJyksXG4gIGxvcmFCOiByZWFkZXIuZ2V0VGVuc29yMkQoJ2xvcmEuQicpLFxuICBzY2FsaW5nOiByZWFkZXIuZ2V0VGVuc29yMUQoJ2xvcmEuc2NhbGluZycpWzBdLFxufSk7XG5cXGBcXGBcXGBcblxuIyMgVHJhaW5pbmcgSW5mb1xuXG4tIFN0ZXBzOiAke21vZGVsLm1ldGFkYXRhLnRyYWluaW5nPy5zdGVwcyB8fCAnTi9BJ31cbi0gRmluYWwgTG9zczogJHttb2RlbC5tZXRhZGF0YS50cmFpbmluZz8ubG9zcyB8fCAnTi9BJ31cbmA7XG5cbiAgICByZXR1cm4geyBzYWZldGVuc29ycywgY29uZmlnLCByZWFkbWUgfTtcbiAgfVxufVxuXG4vKipcbiAqIE1vZGVsIEltcG9ydGVyXG4gKlxuICogSW1wb3J0IG1vZGVscyBmcm9tIHZhcmlvdXMgZm9ybWF0cy5cbiAqL1xuZXhwb3J0IGNsYXNzIE1vZGVsSW1wb3J0ZXIge1xuICAvKipcbiAgICogSW1wb3J0IGZyb20gU2FmZVRlbnNvcnMgZm9ybWF0XG4gICAqL1xuICBmcm9tU2FmZVRlbnNvcnMoYnVmZmVyOiBVaW50OEFycmF5KTogUGFydGlhbDxFeHBvcnRhYmxlTW9kZWw+IHtcbiAgICBjb25zdCByZWFkZXIgPSBuZXcgU2FmZVRlbnNvcnNSZWFkZXIoYnVmZmVyKTtcbiAgICBjb25zdCBtZXRhZGF0YSA9IHJlYWRlci5nZXRNZXRhZGF0YSgpO1xuXG4gICAgY29uc3QgcmVzdWx0OiBQYXJ0aWFsPEV4cG9ydGFibGVNb2RlbD4gPSB7XG4gICAgICBtZXRhZGF0YToge1xuICAgICAgICBuYW1lOiBtZXRhZGF0YS5uYW1lIHx8ICd1bmtub3duJyxcbiAgICAgICAgdmVyc2lvbjogbWV0YWRhdGEudmVyc2lvbiB8fCAnMS4wLjAnLFxuICAgICAgICBhcmNoaXRlY3R1cmU6IG1ldGFkYXRhLmFyY2hpdGVjdHVyZSB8fCAnc29uYS1sb3JhJyxcbiAgICAgICAgdHJhaW5pbmc6IG1ldGFkYXRhLnRyYWluaW5nX3N0ZXBzID8ge1xuICAgICAgICAgIHN0ZXBzOiBwYXJzZUludChtZXRhZGF0YS50cmFpbmluZ19zdGVwcyksXG4gICAgICAgICAgbG9zczogcGFyc2VGbG9hdChtZXRhZGF0YS50cmFpbmluZ19sb3NzIHx8ICcwJyksXG4gICAgICAgICAgbGVhcm5pbmdSYXRlOiAwLFxuICAgICAgICB9IDogdW5kZWZpbmVkLFxuICAgICAgfSxcbiAgICB9O1xuXG4gICAgLy8gTG9hZCBMb1JBIHdlaWdodHNcbiAgICBjb25zdCBsb3JhQSA9IHJlYWRlci5nZXRUZW5zb3IyRCgnbG9yYS5BJyk7XG4gICAgY29uc3QgbG9yYUIgPSByZWFkZXIuZ2V0VGVuc29yMkQoJ2xvcmEuQicpO1xuICAgIGNvbnN0IGxvcmFTY2FsaW5nID0gcmVhZGVyLmdldFRlbnNvcjFEKCdsb3JhLnNjYWxpbmcnKTtcblxuICAgIGlmIChsb3JhQSAmJiBsb3JhQiAmJiBsb3JhU2NhbGluZykge1xuICAgICAgcmVzdWx0LmxvcmFXZWlnaHRzID0ge1xuICAgICAgICBsb3JhQSxcbiAgICAgICAgbG9yYUIsXG4gICAgICAgIHNjYWxpbmc6IGxvcmFTY2FsaW5nWzBdLFxuICAgICAgfTtcbiAgICB9XG5cbiAgICAvLyBMb2FkIHBhdHRlcm5zXG4gICAgY29uc3QgcGF0dGVybkVtYmVkZGluZ3MgPSByZWFkZXIuZ2V0VGVuc29yMkQoJ3BhdHRlcm5zLmVtYmVkZGluZ3MnKTtcbiAgICBjb25zdCBwYXR0ZXJuUmF0ZXMgPSByZWFkZXIuZ2V0VGVuc29yMUQoJ3BhdHRlcm5zLnN1Y2Nlc3NfcmF0ZXMnKTtcblxuICAgIGlmIChwYXR0ZXJuRW1iZWRkaW5ncyAmJiBwYXR0ZXJuUmF0ZXMpIHtcbiAgICAgIHJlc3VsdC5wYXR0ZXJucyA9IHBhdHRlcm5FbWJlZGRpbmdzLm1hcCgoZW1iZWRkaW5nLCBpKSA9PiAoe1xuICAgICAgICBpZDogYGltcG9ydGVkLSR7aX1gLFxuICAgICAgICB0eXBlOiAncXVlcnlfcmVzcG9uc2UnIGFzIGNvbnN0LFxuICAgICAgICBlbWJlZGRpbmcsXG4gICAgICAgIHN1Y2Nlc3NSYXRlOiBwYXR0ZXJuUmF0ZXNbaV0gfHwgMCxcbiAgICAgICAgdXNlQ291bnQ6IDAsXG4gICAgICAgIGxhc3RVc2VkOiBuZXcgRGF0ZSgpLFxuICAgICAgfSkpO1xuICAgIH1cblxuICAgIHJldHVybiByZXN1bHQ7XG4gIH1cblxuICAvKipcbiAgICogSW1wb3J0IGZyb20gSlNPTiBmb3JtYXRcbiAgICovXG4gIGZyb21KU09OKGpzb246IHN0cmluZyk6IFBhcnRpYWw8RXhwb3J0YWJsZU1vZGVsPiB7XG4gICAgcmV0dXJuIEpTT04ucGFyc2UoanNvbik7XG4gIH1cblxuICAvKipcbiAgICogSW1wb3J0IGZyb20gYmluYXJ5IGZvcm1hdFxuICAgKi9cbiAgZnJvbUJpbmFyeShidWZmZXI6IFVpbnQ4QXJyYXkpOiBQYXJ0aWFsPEV4cG9ydGFibGVNb2RlbD4ge1xuICAgIGNvbnN0IHZpZXcgPSBuZXcgRGF0YVZpZXcoYnVmZmVyLmJ1ZmZlciwgYnVmZmVyLmJ5dGVPZmZzZXQpO1xuICAgIGNvbnN0IGxlbmd0aCA9IHZpZXcuZ2V0VWludDMyKDAsIHRydWUpO1xuICAgIGNvbnN0IGpzb25CeXRlcyA9IGJ1ZmZlci5zbGljZSg0LCA0ICsgbGVuZ3RoKTtcbiAgICBjb25zdCBqc29uID0gbmV3IFRleHREZWNvZGVyKCkuZGVjb2RlKGpzb25CeXRlcyk7XG4gICAgcmV0dXJuIHRoaXMuZnJvbUpTT04oanNvbik7XG4gIH1cbn1cblxuLyoqXG4gKiBEYXRhc2V0IEV4cG9ydGVyXG4gKlxuICogRXhwb3J0IHRyYWluaW5nIGRhdGEgaW4gdmFyaW91cyBmb3JtYXRzLlxuICovXG5leHBvcnQgY2xhc3MgRGF0YXNldEV4cG9ydGVyIHtcbiAgLyoqXG4gICAqIEV4cG9ydCB0byBKU09OTCBmb3JtYXQgKG9uZSBKU09OIHBlciBsaW5lKVxuICAgKi9cbiAgdG9KU09OTChkYXRhOiBBcnJheTx7IGlucHV0OiBFbWJlZGRpbmc7IG91dHB1dDogRW1iZWRkaW5nOyBxdWFsaXR5OiBudW1iZXIgfT4pOiBzdHJpbmcge1xuICAgIHJldHVybiBkYXRhXG4gICAgICAubWFwKGl0ZW0gPT4gSlNPTi5zdHJpbmdpZnkoe1xuICAgICAgICBpbnB1dDogaXRlbS5pbnB1dCxcbiAgICAgICAgb3V0cHV0OiBpdGVtLm91dHB1dCxcbiAgICAgICAgcXVhbGl0eTogaXRlbS5xdWFsaXR5LFxuICAgICAgfSkpXG4gICAgICAuam9pbignXFxuJyk7XG4gIH1cblxuICAvKipcbiAgICogRXhwb3J0IHRvIENTViBmb3JtYXRcbiAgICovXG4gIHRvQ1NWKGRhdGE6IEFycmF5PHsgaW5wdXQ6IEVtYmVkZGluZzsgb3V0cHV0OiBFbWJlZGRpbmc7IHF1YWxpdHk6IG51bWJlciB9Pik6IHN0cmluZyB7XG4gICAgY29uc3QgaGVhZGVyID0gJ3F1YWxpdHksaW5wdXQsb3V0cHV0JztcbiAgICBjb25zdCByb3dzID0gZGF0YS5tYXAoaXRlbSA9PlxuICAgICAgYCR7aXRlbS5xdWFsaXR5fSxcIiR7aXRlbS5pbnB1dC5qb2luKCcsJyl9XCIsXCIke2l0ZW0ub3V0cHV0LmpvaW4oJywnKX1cImBcbiAgICApO1xuICAgIHJldHVybiBbaGVhZGVyLCAuLi5yb3dzXS5qb2luKCdcXG4nKTtcbiAgfVxuXG4gIC8qKlxuICAgKiBFeHBvcnQgcGF0dGVybnMgZm9yIHByZS10cmFpbmluZ1xuICAgKi9cbiAgdG9QcmV0cmFpbihwYXR0ZXJuczogTGVhcm5lZFBhdHRlcm5bXSk6IHN0cmluZyB7XG4gICAgcmV0dXJuIHBhdHRlcm5zXG4gICAgICAuZmlsdGVyKHAgPT4gcC5zdWNjZXNzUmF0ZSA+PSAwLjcpXG4gICAgICAubWFwKHAgPT4gSlNPTi5zdHJpbmdpZnkoe1xuICAgICAgICBlbWJlZGRpbmc6IHAuZW1iZWRkaW5nLFxuICAgICAgICB0eXBlOiBwLnR5cGUsXG4gICAgICAgIHF1YWxpdHk6IHAuc3VjY2Vzc1JhdGUsXG4gICAgICB9KSlcbiAgICAgIC5qb2luKCdcXG4nKTtcbiAgfVxufVxuIl19