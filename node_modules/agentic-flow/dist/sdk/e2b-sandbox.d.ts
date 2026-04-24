/**
 * E2B Sandbox Integration - Secure cloud sandboxes for AI code execution
 *
 * Provides isolated execution environments using E2B's Firecracker-based sandboxes.
 * Supports Python, JavaScript, and shell code execution with file system access.
 *
 * @see https://e2b.dev/docs
 */
/**
 * E2B Sandbox configuration
 */
export interface E2BSandboxConfig {
    apiKey?: string;
    template?: string;
    timeout?: number;
    cwd?: string;
    envVars?: Record<string, string>;
    metadata?: Record<string, string>;
}
/**
 * Code execution result
 */
export interface ExecutionResult {
    success: boolean;
    output: string;
    error?: string;
    exitCode?: number;
    logs: string[];
    artifacts?: string[];
}
/**
 * File operation result
 */
export interface FileResult {
    success: boolean;
    path: string;
    content?: string;
    error?: string;
}
/**
 * E2B Sandbox Manager - manages sandbox lifecycle and execution
 */
export declare class E2BSandboxManager {
    private sandbox;
    private config;
    private isCodeInterpreter;
    constructor(config?: E2BSandboxConfig);
    /**
     * Create and start a new sandbox
     */
    create(useCodeInterpreter?: boolean): Promise<boolean>;
    /**
     * Run Python code in the sandbox
     */
    runPython(code: string): Promise<ExecutionResult>;
    /**
     * Run JavaScript/TypeScript code in the sandbox
     */
    runJavaScript(code: string): Promise<ExecutionResult>;
    /**
     * Run shell command in the sandbox
     */
    runCommand(command: string, args?: string[]): Promise<ExecutionResult>;
    /**
     * Write file to sandbox
     */
    writeFile(path: string, content: string): Promise<FileResult>;
    /**
     * Read file from sandbox
     */
    readFile(path: string): Promise<FileResult>;
    /**
     * List files in sandbox directory
     */
    listFiles(path?: string): Promise<string[]>;
    /**
     * Install packages in sandbox
     */
    installPackages(packages: string[], manager?: 'pip' | 'npm'): Promise<ExecutionResult>;
    /**
     * Get sandbox ID
     */
    getSandboxId(): string | null;
    /**
     * Keep sandbox alive (extend timeout)
     */
    keepAlive(duration?: number): Promise<boolean>;
    /**
     * Close and destroy sandbox
     */
    close(): Promise<void>;
}
/**
 * Get or create the default sandbox manager
 */
export declare function getE2BSandbox(config?: E2BSandboxConfig): E2BSandboxManager;
/**
 * Quick helper to run code in E2B sandbox
 */
export declare function runInE2BSandbox(code: string, language?: 'python' | 'javascript' | 'shell'): Promise<ExecutionResult>;
/**
 * Check if E2B is available
 */
export declare function isE2BAvailable(): Promise<boolean>;
//# sourceMappingURL=e2b-sandbox.d.ts.map