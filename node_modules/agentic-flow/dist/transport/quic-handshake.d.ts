export declare enum HandshakeState {
    Initial = "initial",
    Handshaking = "handshaking",
    Established = "established",
    Failed = "failed",
    Closed = "closed"
}
export interface QuicHandshakeContext {
    connectionId: string;
    state: HandshakeState;
    remoteAddr: string;
    startTime: number;
    wasmClient: any;
    createMessage: any;
    onEstablished?: () => void;
    onFailed?: (error: Error) => void;
}
/**
 * QUIC Handshake Manager
 * Implements connection establishment protocol using WASM sendMessage/recvMessage
 */
export declare class QuicHandshakeManager {
    private contexts;
    constructor();
    /**
     * Initiate QUIC handshake for a new connection
     */
    initiateHandshake(connectionId: string, remoteAddr: string, wasmClient: any, createMessage: any): Promise<boolean>;
    /**
     * Send QUIC Initial packet
     */
    private sendInitialPacket;
    /**
     * Wait for Server Hello response
     */
    private waitForServerHello;
    /**
     * Send Handshake Complete packet
     */
    private sendHandshakeComplete;
    /**
     * Create QUIC Initial packet payload
     */
    private createInitialPayload;
    /**
     * Create Handshake Complete payload
     */
    private createHandshakeCompletePayload;
    /**
     * Get handshake state for connection
     */
    getHandshakeState(connectionId: string): HandshakeState;
    /**
     * Check if connection is established
     */
    isEstablished(connectionId: string): boolean;
    /**
     * Close handshake context
     */
    closeHandshake(connectionId: string): void;
    /**
     * Get all active handshakes
     */
    getActiveHandshakes(): string[];
}
//# sourceMappingURL=quic-handshake.d.ts.map