/// <reference types="node" />
import type { Channel } from 'diagnostics_channel';
import { UndiciRequest, UndiciResponse } from './types';
export interface ListenerRecord {
    name: string;
    channel: Channel;
    onMessage: (message: any, name: string) => void;
}
export interface RequestMessage {
    request: UndiciRequest;
}
export interface RequestHeadersMessage {
    request: UndiciRequest;
    socket: any;
}
export interface ResponseHeadersMessage {
    request: UndiciRequest;
    response: UndiciResponse;
}
export interface RequestTrailersMessage {
    request: UndiciRequest;
    response: UndiciResponse;
}
export interface RequestErrorMessage {
    request: UndiciRequest;
    error: Error;
}
//# sourceMappingURL=internal-types.d.ts.map