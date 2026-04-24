/**
 * AgentDB v2 Path Security Utilities
 *
 * Prevents path traversal attacks and ensures safe file operations:
 * - Path validation and canonicalization
 * - Symlink detection and handling
 * - Safe file read/write operations
 * - Temporary file cleanup
 */
import * as fs from 'fs';
/**
 * Validate and sanitize file path
 * Prevents path traversal attacks
 */
export declare function validatePath(filePath: string, baseDir: string): string;
/**
 * Check if path is a symbolic link
 */
export declare function isSymbolicLink(filePath: string): Promise<boolean>;
/**
 * Secure file write operation
 * Prevents writing to symbolic links and validates paths
 */
export declare function secureWrite(filePath: string, data: Buffer | string, baseDir: string, options?: {
    overwrite?: boolean;
    encoding?: BufferEncoding;
}): Promise<void>;
/**
 * Secure file read operation
 * Validates paths and prevents symlink attacks
 */
export declare function secureRead(filePath: string, baseDir: string, options?: {
    encoding?: BufferEncoding;
    followSymlinks?: boolean;
}): Promise<Buffer | string>;
/**
 * Secure directory listing
 * Prevents path traversal and filters out sensitive files
 */
export declare function secureListDir(dirPath: string, baseDir: string, options?: {
    recursive?: boolean;
    includeDotFiles?: boolean;
}): Promise<string[]>;
/**
 * Secure file deletion
 * Validates paths and prevents symlink attacks
 */
export declare function secureDelete(filePath: string, baseDir: string, options?: {
    force?: boolean;
}): Promise<void>;
/**
 * Temporary file manager with automatic cleanup
 */
export declare class TempFileManager {
    private tempFiles;
    private tempDir;
    private cleanupScheduled;
    constructor(baseDir: string);
    /**
     * Initialize temp directory
     */
    init(): Promise<void>;
    /**
     * Create a temporary file
     */
    createTempFile(prefix?: string): Promise<string>;
    /**
     * Write to temporary file
     */
    writeTempFile(data: Buffer | string, prefix?: string): Promise<string>;
    /**
     * Delete a specific temp file
     */
    deleteTempFile(tempPath: string): Promise<void>;
    /**
     * Clean up all temporary files
     */
    cleanup(): Promise<void>;
    /**
     * Synchronous cleanup for process exit
     */
    private cleanupSync;
    /**
     * Get list of managed temp files
     */
    getTempFiles(): string[];
}
/**
 * Ensure directory exists with safe permissions
 */
export declare function ensureDir(dirPath: string, baseDir: string): Promise<string>;
/**
 * Get safe file stats without following symlinks
 */
export declare function safeStats(filePath: string, baseDir: string): Promise<fs.Stats>;
/**
 * Check if path exists within base directory
 */
export declare function pathExists(filePath: string, baseDir: string): Promise<boolean>;
//# sourceMappingURL=path-security.d.ts.map