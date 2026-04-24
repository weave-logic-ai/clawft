// Health check endpoint with QUIC support
import http from 'http';
import { logger } from './utils/logger.js';
let serverStartTime = Date.now();
export async function getHealthStatus() {
    const memUsage = process.memoryUsage();
    const memLimit = 512 * 1024 * 1024; // 512MB
    const memPercent = (memUsage.heapUsed / memLimit) * 100;
    const apiKey = process.env.ANTHROPIC_API_KEY;
    const checks = {
        api: {
            status: apiKey && apiKey.startsWith('sk-ant-') ? 'pass' : 'fail',
            message: apiKey ? undefined : 'ANTHROPIC_API_KEY not configured'
        },
        memory: {
            status: memPercent > 90 ? 'fail' : memPercent > 75 ? 'warn' : 'pass',
            usage: Math.round(memUsage.heapUsed / 1024 / 1024),
            limit: Math.round(memLimit / 1024 / 1024)
        }
    };
    // Check QUIC availability if enabled
    const quicEnabled = process.env.AGENTIC_FLOW_ENABLE_QUIC === 'true';
    if (quicEnabled) {
        try {
            // Dynamic import to avoid loading QUIC module when not needed
            const { checkQuicAvailability } = await import('./config/quic.js');
            const availability = await checkQuicAvailability();
            checks.quic = {
                status: availability.available ? 'pass' : 'warn',
                enabled: true,
                available: availability.available,
                connections: 0
            };
        }
        catch (error) {
            logger.warn('QUIC health check failed', { error });
            checks.quic = {
                status: 'fail',
                enabled: true,
                available: false
            };
        }
    }
    let overallStatus = 'healthy';
    if (checks.memory.status === 'fail' || checks.api.status === 'fail') {
        overallStatus = 'unhealthy';
    }
    else if (checks.memory.status === 'warn' || checks.quic?.status === 'warn') {
        overallStatus = 'degraded';
    }
    return {
        status: overallStatus,
        timestamp: new Date().toISOString(),
        uptime: (Date.now() - serverStartTime) / 1000,
        version: process.env.npm_package_version || '1.5.10',
        checks
    };
}
export function startHealthServer(port = 8080) {
    const server = http.createServer(async (req, res) => {
        if (req.url === '/health' && req.method === 'GET') {
            const health = await getHealthStatus();
            const statusCode = health.status === 'healthy' ? 200 : health.status === 'degraded' ? 200 : 503;
            res.writeHead(statusCode, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify(health, null, 2));
            logger.debug('Health check requested', { status: health.status, quicEnabled: !!health.checks.quic });
        }
        else if (req.url === '/health/quic' && req.method === 'GET') {
            // QUIC-specific health endpoint
            const quicEnabled = process.env.AGENTIC_FLOW_ENABLE_QUIC === 'true';
            if (!quicEnabled) {
                res.writeHead(200, { 'Content-Type': 'application/json' });
                res.end(JSON.stringify({ enabled: false, message: 'QUIC transport is disabled' }));
                return;
            }
            try {
                const { checkQuicAvailability, loadQuicConfig } = await import('./config/quic.js');
                const availability = await checkQuicAvailability();
                const config = loadQuicConfig();
                res.writeHead(availability.available ? 200 : 503, { 'Content-Type': 'application/json' });
                res.end(JSON.stringify({
                    enabled: true,
                    available: availability.available,
                    reason: availability.reason,
                    config: {
                        host: config.host,
                        port: config.port,
                        maxConnections: config.maxConnections,
                        maxStreams: config.maxConcurrentStreams
                    }
                }, null, 2));
            }
            catch (error) {
                res.writeHead(503, { 'Content-Type': 'application/json' });
                res.end(JSON.stringify({
                    enabled: true,
                    available: false,
                    error: error.message
                }));
            }
        }
        else {
            res.writeHead(404, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ error: 'Not found' }));
        }
    });
    server.listen(port, () => {
        logger.info('Health check server started', { port });
    });
    return server;
}
//# sourceMappingURL=health.js.map