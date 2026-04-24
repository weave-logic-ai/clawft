export interface AuthContext {
    userId: string;
    tier: 'free' | 'pro' | 'enterprise';
    role: 'user' | 'admin';
    permissions: string[];
}
export interface TransportConfig {
    type: 'stdio' | 'http';
    port?: number;
    host?: string;
    enableAuth?: boolean;
    enableCORS?: boolean;
}
export interface ToolContext {
    auth?: AuthContext;
    onProgress?: (update: ProgressUpdate) => void;
}
export interface ProgressUpdate {
    progress: number;
    message: string;
}
export interface ToolResult {
    success: boolean;
    [key: string]: any;
}
export interface ToolDefinition {
    name: string;
    description: string;
    parameters: any;
    canAccess?: (auth: AuthContext) => boolean;
    execute: (args: any, context: ToolContext) => Promise<any>;
}
//# sourceMappingURL=index.d.ts.map