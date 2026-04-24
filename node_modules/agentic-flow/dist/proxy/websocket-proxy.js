/**
 * WebSocket Proxy for LLM Streaming
 *
 * Features:
 * - Bidirectional: Full-duplex communication
 * - Mobile-friendly: Better for unstable connections
 * - Lower overhead: No HTTP headers per message
 * - Reconnection: Automatic retry on disconnect
 * - Universal support: Works everywhere (browsers, mobile, desktop)
 *
 * Use Case: Fallback for unreliable connections (mobile, poor WiFi)
 */
import { WebSocketServer } from 'ws';
import { createServer } from 'http';
import { logger } from '../utils/logger.js';
export class WebSocketProxy {
    wss;
    server;
    config;
    clients = new Map();
    pingInterval;
    activeConnections = 0;
    constructor(config) {
        this.config = config;
        this.server = createServer();
        this.wss = new WebSocketServer({ server: this.server });
        this.setupHandlers();
        this.setupHeartbeat();
        logger.info('WebSocket proxy created', {
            port: config.port,
            pingInterval: config.pingInterval || 30000
        });
    }
    setupHandlers() {
        this.wss.on('connection', (ws, req) => {
            const clientIp = req.socket.remoteAddress;
            // Check connection limit (DoS protection)
            const maxConnections = this.config.maxConnections || 1000;
            if (this.activeConnections >= maxConnections) {
                logger.warn('Connection limit reached', {
                    activeConnections: this.activeConnections,
                    maxConnections
                });
                ws.close(1008, 'Server at capacity');
                return;
            }
            this.activeConnections++;
            logger.info('WebSocket client connected', {
                clientIp,
                activeConnections: this.activeConnections
            });
            // Initialize client state
            this.clients.set(ws, {
                ws,
                isAlive: true,
                lastPing: new Date(),
                requests: 0
            });
            // Handle incoming messages
            ws.on('message', async (data) => {
                const client = this.clients.get(ws);
                if (!client)
                    return;
                client.requests++;
                try {
                    const message = JSON.parse(data.toString());
                    logger.debug('WebSocket message received', { type: message.type });
                    if (message.type === 'ping') {
                        // Respond to ping
                        ws.send(JSON.stringify({ type: 'pong', timestamp: Date.now() }));
                        client.isAlive = true;
                        client.lastPing = new Date();
                    }
                    else if (message.type === 'streaming_request') {
                        // Handle LLM streaming request
                        await this.handleStreamingRequest(ws, message.data);
                    }
                    else if (message.type === 'non_streaming_request') {
                        // Handle non-streaming request
                        await this.handleNonStreamingRequest(ws, message.data);
                    }
                    else {
                        ws.send(JSON.stringify({
                            type: 'error',
                            error: `Unknown message type: ${message.type}`
                        }));
                    }
                }
                catch (error) {
                    logger.error('WebSocket message error', { error: error.message });
                    ws.send(JSON.stringify({
                        type: 'error',
                        error: error.message
                    }));
                }
            });
            // Handle pong responses
            ws.on('pong', () => {
                const client = this.clients.get(ws);
                if (client) {
                    client.isAlive = true;
                    client.lastPing = new Date();
                }
            });
            // Set connection timeout
            const connectionTimeout = this.config.connectionTimeout || 300000; // 5 minutes
            const timeoutHandle = setTimeout(() => {
                logger.warn('Connection timeout', { clientIp });
                ws.close(1000, 'Connection timeout');
            }, connectionTimeout);
            // Handle close
            ws.on('close', () => {
                logger.info('WebSocket client disconnected', { clientIp });
                this.clients.delete(ws);
                this.activeConnections--;
                clearTimeout(timeoutHandle);
            });
            // Handle errors
            ws.on('error', (error) => {
                logger.error('WebSocket error', { clientIp, error: error.message });
                this.clients.delete(ws);
                this.activeConnections--;
            });
            // Send initial handshake
            ws.send(JSON.stringify({
                type: 'connected',
                protocols: ['anthropic-messages-v1'],
                features: ['streaming', 'non-streaming', 'ping-pong']
            }));
        });
        this.wss.on('error', (error) => {
            logger.error('WebSocket server error', { error: error.message });
        });
    }
    setupHeartbeat() {
        const interval = this.config.pingInterval || 30000; // 30 seconds
        const timeout = this.config.pingTimeout || 60000; // 60 seconds
        this.pingInterval = setInterval(() => {
            const now = Date.now();
            this.clients.forEach((client, ws) => {
                // Check if client responded to last ping
                if (!client.isAlive) {
                    const timeSinceLastPing = now - client.lastPing.getTime();
                    if (timeSinceLastPing > timeout) {
                        logger.warn('Client did not respond to ping, terminating', {
                            timeSinceLastPing
                        });
                        ws.terminate();
                        this.clients.delete(ws);
                        return;
                    }
                }
                // Send ping
                client.isAlive = false;
                ws.ping();
            });
        }, interval);
    }
    async handleStreamingRequest(ws, anthropicReq) {
        try {
            logger.info('WebSocket streaming request', {
                model: anthropicReq.model,
                messageCount: anthropicReq.messages?.length
            });
            // Convert to Gemini format
            const geminiReq = this.convertAnthropicToGemini(anthropicReq);
            // Send streaming start event
            ws.send(JSON.stringify({
                type: 'message_start',
                message: {
                    id: `msg_${Date.now()}`,
                    role: 'assistant',
                    model: anthropicReq.model || 'gemini-2.0-flash-exp'
                }
            }));
            // Forward to Gemini
            const geminiBaseUrl = this.config.geminiBaseUrl || 'https://generativelanguage.googleapis.com/v1beta';
            const url = `${geminiBaseUrl}/models/gemini-2.0-flash-exp:streamGenerateContent?key=${this.config.geminiApiKey}&alt=sse`;
            const response = await fetch(url, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(geminiReq)
            });
            if (!response.ok) {
                const error = await response.text();
                throw new Error(`Gemini API error: ${error}`);
            }
            const reader = response.body?.getReader();
            if (!reader) {
                throw new Error('No response body');
            }
            const decoder = new TextDecoder();
            let chunkCount = 0;
            while (true) {
                const { done, value } = await reader.read();
                if (done)
                    break;
                const chunk = decoder.decode(value);
                chunkCount++;
                // Parse Gemini SSE and send as WebSocket messages
                const lines = chunk.split('\n').filter(line => line.trim());
                for (const line of lines) {
                    if (line.startsWith('data: ')) {
                        try {
                            const jsonStr = line.substring(6);
                            const parsed = JSON.parse(jsonStr);
                            const candidate = parsed.candidates?.[0];
                            const text = candidate?.content?.parts?.[0]?.text;
                            if (text) {
                                // Send text delta
                                ws.send(JSON.stringify({
                                    type: 'content_block_delta',
                                    delta: { type: 'text_delta', text }
                                }));
                            }
                            if (candidate?.finishReason) {
                                // Send completion
                                ws.send(JSON.stringify({
                                    type: 'message_stop',
                                    stop_reason: 'end_turn'
                                }));
                            }
                        }
                        catch (e) {
                            logger.debug('Failed to parse stream chunk', { line });
                        }
                    }
                }
            }
            logger.info('WebSocket stream complete', { totalChunks: chunkCount });
        }
        catch (error) {
            logger.error('WebSocket streaming error', { error: error.message });
            ws.send(JSON.stringify({
                type: 'error',
                error: error.message
            }));
        }
    }
    async handleNonStreamingRequest(ws, anthropicReq) {
        try {
            logger.info('WebSocket non-streaming request', {
                model: anthropicReq.model,
                messageCount: anthropicReq.messages?.length
            });
            // Convert to Gemini format
            const geminiReq = this.convertAnthropicToGemini(anthropicReq);
            // Forward to Gemini
            const geminiBaseUrl = this.config.geminiBaseUrl || 'https://generativelanguage.googleapis.com/v1beta';
            const url = `${geminiBaseUrl}/models/gemini-2.0-flash-exp:generateContent?key=${this.config.geminiApiKey}`;
            const response = await fetch(url, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(geminiReq)
            });
            if (!response.ok) {
                const error = await response.text();
                throw new Error(`Gemini API error: ${error}`);
            }
            const geminiRes = await response.json();
            const anthropicRes = this.convertGeminiToAnthropic(geminiRes);
            // Send complete response
            ws.send(JSON.stringify({
                type: 'message_complete',
                message: anthropicRes
            }));
        }
        catch (error) {
            logger.error('WebSocket non-streaming error', { error: error.message });
            ws.send(JSON.stringify({
                type: 'error',
                error: error.message
            }));
        }
    }
    convertAnthropicToGemini(anthropicReq) {
        const contents = [];
        let systemPrefix = '';
        if (anthropicReq.system) {
            systemPrefix = `System: ${anthropicReq.system}\n\n`;
        }
        for (let i = 0; i < anthropicReq.messages.length; i++) {
            const msg = anthropicReq.messages[i];
            let text;
            if (typeof msg.content === 'string') {
                text = msg.content;
            }
            else if (Array.isArray(msg.content)) {
                text = msg.content
                    .filter((block) => block.type === 'text')
                    .map((block) => block.text)
                    .join('\n');
            }
            else {
                text = '';
            }
            if (i === 0 && msg.role === 'user' && systemPrefix) {
                text = systemPrefix + text;
            }
            contents.push({
                role: msg.role === 'assistant' ? 'model' : 'user',
                parts: [{ text }]
            });
        }
        const geminiReq = { contents };
        if (anthropicReq.temperature !== undefined || anthropicReq.max_tokens !== undefined) {
            geminiReq.generationConfig = {};
            if (anthropicReq.temperature !== undefined) {
                geminiReq.generationConfig.temperature = anthropicReq.temperature;
            }
            if (anthropicReq.max_tokens !== undefined) {
                geminiReq.generationConfig.maxOutputTokens = anthropicReq.max_tokens;
            }
        }
        return geminiReq;
    }
    convertGeminiToAnthropic(geminiRes) {
        const candidate = geminiRes.candidates?.[0];
        if (!candidate) {
            throw new Error('No candidates in Gemini response');
        }
        const content = candidate.content;
        const parts = content?.parts || [];
        let rawText = '';
        for (const part of parts) {
            if (part.text) {
                rawText += part.text;
            }
        }
        return {
            id: `msg_${Date.now()}`,
            type: 'message',
            role: 'assistant',
            model: 'gemini-2.0-flash-exp',
            content: [
                {
                    type: 'text',
                    text: rawText
                }
            ],
            stop_reason: 'end_turn',
            usage: {
                input_tokens: geminiRes.usageMetadata?.promptTokenCount || 0,
                output_tokens: geminiRes.usageMetadata?.candidatesTokenCount || 0
            }
        };
    }
    start() {
        return new Promise((resolve) => {
            this.server.listen(this.config.port, () => {
                logger.info('WebSocket proxy started', {
                    port: this.config.port,
                    url: `ws://localhost:${this.config.port}`
                });
                console.log(`\n✅ WebSocket Proxy running at ws://localhost:${this.config.port}`);
                console.log(`   Protocol: WebSocket (fallback for unreliable connections)`);
                console.log(`   Features: Bidirectional, Mobile-friendly, Auto-reconnect\n`);
                resolve();
            });
        });
    }
    stop() {
        return new Promise((resolve) => {
            // Clear ping interval
            if (this.pingInterval) {
                clearInterval(this.pingInterval);
            }
            // Close all client connections
            this.clients.forEach((client, ws) => {
                ws.close(1000, 'Server shutting down');
            });
            this.clients.clear();
            // Close WebSocket server
            this.wss.close(() => {
                // Close HTTP server
                this.server.close(() => {
                    logger.info('WebSocket proxy stopped');
                    resolve();
                });
            });
        });
    }
}
// CLI entry point
if (import.meta.url === `file://${process.argv[1]}`) {
    const port = parseInt(process.env.PORT || '8080');
    const geminiApiKey = process.env.GOOGLE_GEMINI_API_KEY;
    if (!geminiApiKey) {
        console.error('❌ Error: GOOGLE_GEMINI_API_KEY environment variable required');
        process.exit(1);
    }
    const proxy = new WebSocketProxy({
        port,
        geminiApiKey,
        geminiBaseUrl: process.env.GEMINI_BASE_URL,
        pingInterval: 30000,
        pingTimeout: 60000
    });
    proxy.start();
    // Graceful shutdown
    process.on('SIGINT', async () => {
        console.log('\n🛑 Shutting down WebSocket proxy...');
        await proxy.stop();
        process.exit(0);
    });
}
//# sourceMappingURL=websocket-proxy.js.map