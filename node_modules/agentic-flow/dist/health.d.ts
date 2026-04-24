import http from 'http';
interface HealthStatus {
    status: 'healthy' | 'degraded' | 'unhealthy';
    timestamp: string;
    uptime: number;
    version: string;
    checks: {
        api: {
            status: 'pass' | 'fail';
            message?: string;
        };
        memory: {
            status: 'pass' | 'warn' | 'fail';
            usage: number;
            limit: number;
        };
        quic?: {
            status: 'pass' | 'warn' | 'fail';
            enabled: boolean;
            available?: boolean;
            connections?: number;
        };
    };
}
export declare function getHealthStatus(): Promise<HealthStatus>;
export declare function startHealthServer(port?: number): http.Server;
export {};
//# sourceMappingURL=health.d.ts.map