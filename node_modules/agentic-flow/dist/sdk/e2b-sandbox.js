/**
 * E2B Sandbox Integration - Secure cloud sandboxes for AI code execution
 *
 * Provides isolated execution environments using E2B's Firecracker-based sandboxes.
 * Supports Python, JavaScript, and shell code execution with file system access.
 *
 * @see https://e2b.dev/docs
 */
import { logger } from "../utils/logger.js";
// Lazy-loaded E2B SDK
let e2bModule = null;
let codeInterpreterModule = null;
/**
 * Initialize E2B SDK (lazy load)
 */
async function getE2B() {
    if (!e2bModule) {
        try {
            e2bModule = await import('e2b');
        }
        catch (error) {
            logger.warn('E2B SDK not installed. Run: npm install e2b @e2b/code-interpreter');
            return null;
        }
    }
    return e2bModule;
}
/**
 * Initialize Code Interpreter SDK (lazy load)
 */
async function getCodeInterpreter() {
    if (!codeInterpreterModule) {
        try {
            codeInterpreterModule = await import('@e2b/code-interpreter');
        }
        catch (error) {
            logger.warn('E2B Code Interpreter not installed. Run: npm install @e2b/code-interpreter');
            return null;
        }
    }
    return codeInterpreterModule;
}
/**
 * E2B Sandbox Manager - manages sandbox lifecycle and execution
 */
export class E2BSandboxManager {
    sandbox = null;
    config;
    isCodeInterpreter = false;
    constructor(config = {}) {
        this.config = {
            apiKey: config.apiKey || process.env.E2B_API_KEY,
            template: config.template || 'base',
            timeout: config.timeout || 300000,
            cwd: config.cwd || '/home/user',
            envVars: config.envVars || {},
            metadata: config.metadata || {}
        };
    }
    /**
     * Create and start a new sandbox
     */
    async create(useCodeInterpreter = true) {
        try {
            if (!this.config.apiKey) {
                logger.error('E2B_API_KEY not configured');
                return false;
            }
            if (useCodeInterpreter) {
                const ci = await getCodeInterpreter();
                if (!ci)
                    return false;
                this.sandbox = await ci.Sandbox.create({
                    apiKey: this.config.apiKey,
                    timeout: this.config.timeout,
                    metadata: this.config.metadata,
                    envVars: this.config.envVars
                });
                this.isCodeInterpreter = true;
            }
            else {
                const e2b = await getE2B();
                if (!e2b)
                    return false;
                this.sandbox = await e2b.Sandbox.create({
                    apiKey: this.config.apiKey,
                    template: this.config.template,
                    timeout: this.config.timeout,
                    metadata: this.config.metadata,
                    envVars: this.config.envVars
                });
                this.isCodeInterpreter = false;
            }
            logger.info('E2B sandbox created', {
                id: this.sandbox.sandboxId,
                type: useCodeInterpreter ? 'code-interpreter' : 'base'
            });
            return true;
        }
        catch (error) {
            logger.error('Failed to create E2B sandbox', { error: error.message });
            return false;
        }
    }
    /**
     * Run Python code in the sandbox
     */
    async runPython(code) {
        if (!this.sandbox) {
            return { success: false, output: '', error: 'Sandbox not initialized', logs: [] };
        }
        try {
            if (this.isCodeInterpreter) {
                const result = await this.sandbox.runCode(code);
                // Extract logs from various formats
                const logs = [];
                if (Array.isArray(result.logs)) {
                    for (const log of result.logs) {
                        if (typeof log === 'string') {
                            logs.push(log);
                        }
                        else if (log?.line) {
                            logs.push(log.line);
                        }
                        else if (log?.text) {
                            logs.push(log.text);
                        }
                        else if (log?.type === 'stdout' && log?.text) {
                            logs.push(log.text);
                        }
                        else {
                            logs.push(String(log));
                        }
                    }
                }
                // Extract artifacts
                const artifacts = Array.isArray(result.artifacts)
                    ? result.artifacts.map((a) => typeof a === 'string' ? a : a?.path || String(a))
                    : [];
                // Build output from multiple sources (improved capture)
                let output = '';
                // Check result.text first (primary output)
                if (result.text) {
                    output = result.text;
                }
                // Check result.output
                else if (result.output) {
                    output = result.output;
                }
                // Check result.results array (Code Interpreter returns this)
                else if (Array.isArray(result.results) && result.results.length > 0) {
                    output = result.results
                        .map((r) => r?.text || r?.html || r?.data || String(r))
                        .join('\n');
                }
                // Fall back to joined logs
                else if (logs.length > 0) {
                    output = logs.join('\n');
                }
                return {
                    success: !result.error,
                    output,
                    error: result.error?.message || result.error?.name || (typeof result.error === 'string' ? result.error : undefined),
                    logs,
                    artifacts
                };
            }
            else {
                // Use shell for base sandbox (more reliable output)
                const proc = await this.sandbox.process.start({
                    cmd: 'python3',
                    args: ['-c', code],
                    cwd: this.config.cwd
                });
                await proc.wait();
                return {
                    success: proc.exitCode === 0,
                    output: proc.stdout || '',
                    error: proc.stderr || undefined,
                    exitCode: proc.exitCode,
                    logs: []
                };
            }
        }
        catch (error) {
            return {
                success: false,
                output: '',
                error: error.message,
                logs: []
            };
        }
    }
    /**
     * Run JavaScript/TypeScript code in the sandbox
     */
    async runJavaScript(code) {
        if (!this.sandbox) {
            return { success: false, output: '', error: 'Sandbox not initialized', logs: [] };
        }
        try {
            // Check if sandbox has process API (base sandbox) or use commands API
            if (this.sandbox.process?.start) {
                const proc = await this.sandbox.process.start({
                    cmd: 'node',
                    args: ['-e', code],
                    cwd: this.config.cwd
                });
                await proc.wait();
                return {
                    success: proc.exitCode === 0,
                    output: proc.stdout || '',
                    error: proc.stderr || undefined,
                    exitCode: proc.exitCode,
                    logs: []
                };
            }
            else if (this.sandbox.commands?.run) {
                const result = await this.sandbox.commands.run(`node -e '${code.replace(/'/g, "\\'")}'`);
                return {
                    success: result.exitCode === 0,
                    output: result.stdout || '',
                    error: result.stderr || undefined,
                    exitCode: result.exitCode,
                    logs: []
                };
            }
            else {
                return { success: false, output: '', error: 'No execution API available', logs: [] };
            }
        }
        catch (error) {
            return {
                success: false,
                output: '',
                error: error.message,
                logs: []
            };
        }
    }
    /**
     * Run shell command in the sandbox
     */
    async runCommand(command, args = []) {
        if (!this.sandbox) {
            return { success: false, output: '', error: 'Sandbox not initialized', logs: [] };
        }
        try {
            // Check if sandbox has process API (base sandbox) or use commands API
            if (this.sandbox.process?.start) {
                const proc = await this.sandbox.process.start({
                    cmd: command,
                    args,
                    cwd: this.config.cwd
                });
                await proc.wait();
                return {
                    success: proc.exitCode === 0,
                    output: proc.stdout || '',
                    error: proc.stderr || undefined,
                    exitCode: proc.exitCode,
                    logs: []
                };
            }
            else if (this.sandbox.commands?.run) {
                const fullCmd = args.length > 0 ? `${command} ${args.join(' ')}` : command;
                const result = await this.sandbox.commands.run(fullCmd);
                return {
                    success: result.exitCode === 0,
                    output: result.stdout || '',
                    error: result.stderr || undefined,
                    exitCode: result.exitCode,
                    logs: []
                };
            }
            else {
                return { success: false, output: '', error: 'No execution API available', logs: [] };
            }
        }
        catch (error) {
            return {
                success: false,
                output: '',
                error: error.message,
                logs: []
            };
        }
    }
    /**
     * Write file to sandbox
     */
    async writeFile(path, content) {
        if (!this.sandbox) {
            return { success: false, path, error: 'Sandbox not initialized' };
        }
        try {
            await this.sandbox.filesystem.write(path, content);
            return { success: true, path };
        }
        catch (error) {
            return { success: false, path, error: error.message };
        }
    }
    /**
     * Read file from sandbox
     */
    async readFile(path) {
        if (!this.sandbox) {
            return { success: false, path, error: 'Sandbox not initialized' };
        }
        try {
            const content = await this.sandbox.filesystem.read(path);
            return { success: true, path, content };
        }
        catch (error) {
            return { success: false, path, error: error.message };
        }
    }
    /**
     * List files in sandbox directory
     */
    async listFiles(path = '/') {
        if (!this.sandbox) {
            return [];
        }
        try {
            const files = await this.sandbox.filesystem.list(path);
            return files.map((f) => f.name);
        }
        catch (error) {
            logger.warn('Failed to list files', { path, error: error.message });
            return [];
        }
    }
    /**
     * Install packages in sandbox
     */
    async installPackages(packages, manager = 'pip') {
        const cmd = manager === 'pip' ? 'pip' : 'npm';
        const args = manager === 'pip' ? ['install', ...packages] : ['install', ...packages];
        return this.runCommand(cmd, args);
    }
    /**
     * Get sandbox ID
     */
    getSandboxId() {
        return this.sandbox?.sandboxId || null;
    }
    /**
     * Keep sandbox alive (extend timeout)
     */
    async keepAlive(duration = 300000) {
        if (!this.sandbox)
            return false;
        try {
            await this.sandbox.keepAlive(duration);
            return true;
        }
        catch (error) {
            logger.warn('Failed to keep sandbox alive', { error: error.message });
            return false;
        }
    }
    /**
     * Close and destroy sandbox
     */
    async close() {
        if (this.sandbox) {
            try {
                await this.sandbox.kill();
                logger.info('E2B sandbox closed', { id: this.sandbox.sandboxId });
            }
            catch (error) {
                logger.warn('Error closing sandbox', { error: error.message });
            }
            this.sandbox = null;
        }
    }
}
// Singleton instance for convenience
let defaultSandbox = null;
/**
 * Get or create the default sandbox manager
 */
export function getE2BSandbox(config) {
    if (!defaultSandbox) {
        defaultSandbox = new E2BSandboxManager(config);
    }
    return defaultSandbox;
}
/**
 * Quick helper to run code in E2B sandbox
 */
export async function runInE2BSandbox(code, language = 'python') {
    const sandbox = getE2BSandbox();
    // Create if not exists
    if (!sandbox.getSandboxId()) {
        const created = await sandbox.create(language === 'python');
        if (!created) {
            return { success: false, output: '', error: 'Failed to create sandbox', logs: [] };
        }
    }
    switch (language) {
        case 'python':
            return sandbox.runPython(code);
        case 'javascript':
            return sandbox.runJavaScript(code);
        case 'shell':
            return sandbox.runCommand('sh', ['-c', code]);
    }
}
/**
 * Check if E2B is available
 */
export async function isE2BAvailable() {
    const e2b = await getE2B();
    return e2b !== null && !!process.env.E2B_API_KEY;
}
//# sourceMappingURL=e2b-sandbox.js.map