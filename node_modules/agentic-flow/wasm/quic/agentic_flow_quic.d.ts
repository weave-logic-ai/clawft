/* tslint:disable */
/* eslint-disable */
/**
 * Create QUIC message from JavaScript
 */
export function createQuicMessage(id: string, msg_type: string, payload: Uint8Array, metadata: any): any;
/**
 * Create default connection config
 */
export function defaultConfig(): any;
/**
 * WASM wrapper for QuicClient
 */
export class WasmQuicClient {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create new WASM QUIC client
   */
  constructor(config: any);
  /**
   * Send message to server
   */
  sendMessage(addr: string, message: any): Promise<void>;
  /**
   * Receive message from server
   */
  recvMessage(addr: string): Promise<any>;
  /**
   * Get pool statistics
   */
  poolStats(): Promise<any>;
  /**
   * Close all connections
   */
  close(): Promise<void>;
}
