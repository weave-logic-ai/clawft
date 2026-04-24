/**
 * Debug Streaming System for Federation
 *
 * Provides detailed, real-time visibility into agent operations
 * with multiple verbosity levels and customizable output formats.
 *
 * Features:
 * - Multiple debug levels (SILENT, BASIC, DETAILED, VERBOSE, TRACE)
 * - Real-time event streaming
 * - Performance metrics and timing
 * - Stack traces and context
 * - Customizable formatters
 * - File and console output
 * - JSON and human-readable formats
 */
import { EventEmitter } from 'events';
import { WriteStream } from 'fs';
export declare enum DebugLevel {
    SILENT = 0,// No output
    BASIC = 1,// Major events only (init, shutdown, errors)
    DETAILED = 2,// Include operations (queries, inserts, updates)
    VERBOSE = 3,// Include all events (presence, messages, tasks)
    TRACE = 4
}
export interface DebugEvent {
    timestamp: string;
    level: DebugLevel;
    category: string;
    operation: string;
    agentId?: string;
    tenantId?: string;
    duration?: number;
    data?: any;
    error?: Error;
    stackTrace?: string;
    metadata?: Record<string, any>;
}
export interface DebugStreamConfig {
    level: DebugLevel;
    output: 'console' | 'file' | 'both' | 'stream';
    format: 'human' | 'json' | 'compact';
    outputFile?: string;
    includeTimestamps?: boolean;
    includeStackTraces?: boolean;
    includeMetadata?: boolean;
    colorize?: boolean;
    filterCategories?: string[];
    filterAgents?: string[];
    customStream?: WriteStream;
}
export declare class DebugStream extends EventEmitter {
    private config;
    private fileStream?;
    private eventBuffer;
    private metrics;
    constructor(config?: Partial<DebugStreamConfig>);
    /**
     * Log a debug event
     */
    log(event: Omit<DebugEvent, 'timestamp'>): void;
    /**
     * Log connection events
     */
    logConnection(operation: string, data?: any, error?: Error): void;
    /**
     * Log database operations
     */
    logDatabase(operation: string, data?: any, duration?: number, error?: Error): void;
    /**
     * Log realtime events
     */
    logRealtime(operation: string, agentId?: string, data?: any, duration?: number): void;
    /**
     * Log memory operations
     */
    logMemory(operation: string, agentId?: string, tenantId?: string, data?: any, duration?: number): void;
    /**
     * Log task operations
     */
    logTask(operation: string, agentId?: string, tenantId?: string, data?: any, duration?: number): void;
    /**
     * Log internal state changes
     */
    logTrace(operation: string, data?: any): void;
    /**
     * Output event to configured destinations
     */
    private outputEvent;
    /**
     * Format event for output
     */
    private formatEvent;
    /**
     * Format event in human-readable format
     */
    private formatHuman;
    /**
     * Format event in compact format
     */
    private formatCompact;
    /**
     * Get level string
     */
    private getLevelString;
    /**
     * Get color for level
     */
    private getLevelColor;
    /**
     * Colorize text
     */
    private colorize;
    /**
     * Capture stack trace
     */
    private captureStackTrace;
    /**
     * Get metrics summary
     */
    getMetrics(): Record<string, {
        count: number;
        avgDuration: number;
    }>;
    /**
     * Print metrics summary
     */
    printMetrics(): void;
    /**
     * Get event buffer
     */
    getEvents(filter?: {
        category?: string;
        agentId?: string;
        since?: Date;
    }): DebugEvent[];
    /**
     * Clear event buffer
     */
    clearEvents(): void;
    /**
     * Clear metrics
     */
    clearMetrics(): void;
    /**
     * Close file stream
     */
    close(): void;
}
/**
 * Create debug stream with sensible defaults
 */
export declare function createDebugStream(config?: Partial<DebugStreamConfig>): DebugStream;
/**
 * Get debug level from environment variable
 */
export declare function getDebugLevelFromEnv(): DebugLevel;
//# sourceMappingURL=debug-stream.d.ts.map