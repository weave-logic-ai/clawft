interface LogContext {
    agent?: string;
    task?: string;
    duration?: number;
    error?: unknown;
    [key: string]: any;
}
declare class Logger {
    private context;
    setContext(ctx: Record<string, any>): void;
    private log;
    debug(message: string, data?: LogContext): void;
    info(message: string, data?: LogContext): void;
    warn(message: string, data?: LogContext): void;
    error(message: string, data?: LogContext): void;
}
export declare const logger: Logger;
export {};
//# sourceMappingURL=logger.d.ts.map