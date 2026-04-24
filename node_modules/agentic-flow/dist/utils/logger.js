class Logger {
    context = {};
    setContext(ctx) {
        this.context = { ...this.context, ...ctx };
    }
    log(level, message, data) {
        // Skip all logs if QUIET mode is enabled (unless it's an error)
        if (process.env.QUIET === 'true' && level !== 'error') {
            return;
        }
        const timestamp = new Date().toISOString();
        const logEntry = {
            timestamp,
            level,
            message,
            ...this.context,
            ...data
        };
        // Structured JSON logging for production
        if (process.env.NODE_ENV === 'production') {
            console.log(JSON.stringify(logEntry));
        }
        else {
            // Human-readable for development
            const prefix = `[${timestamp}] ${level.toUpperCase()}`;
            const contextStr = Object.keys({ ...this.context, ...data }).length > 0
                ? ` ${JSON.stringify({ ...this.context, ...data })}`
                : '';
            console.log(`${prefix}: ${message}${contextStr}`);
        }
    }
    debug(message, data) {
        // Skip debug logs unless DEBUG or VERBOSE environment variable is set
        if (!process.env.DEBUG && !process.env.VERBOSE) {
            return;
        }
        this.log('debug', message, data);
    }
    info(message, data) {
        // Skip info logs unless VERBOSE is set
        if (!process.env.DEBUG && !process.env.VERBOSE) {
            return;
        }
        this.log('info', message, data);
    }
    warn(message, data) {
        // Skip warnings unless VERBOSE is set
        if (!process.env.DEBUG && !process.env.VERBOSE) {
            return;
        }
        this.log('warn', message, data);
    }
    error(message, data) {
        this.log('error', message, data);
    }
}
export const logger = new Logger();
//# sourceMappingURL=logger.js.map