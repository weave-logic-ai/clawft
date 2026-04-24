/// <reference types="node" />
import type * as http from 'http';
import * as url from 'url';
import { Func, HttpInstrumentationConfig, HttpRequestArgs } from './types';
import { InstrumentationBase, InstrumentationNodeModuleDefinition } from '@opentelemetry/instrumentation';
/**
 * Http instrumentation instrumentation for Opentelemetry
 */
export declare class HttpInstrumentation extends InstrumentationBase<HttpInstrumentationConfig> {
    /** keep track on spans not ended */
    private readonly _spanNotEnded;
    private _headerCapture;
    private _httpServerDurationHistogram;
    private _httpClientDurationHistogram;
    constructor(config?: HttpInstrumentationConfig);
    protected _updateMetricInstruments(): void;
    setConfig(config?: HttpInstrumentationConfig): void;
    init(): [
        InstrumentationNodeModuleDefinition,
        InstrumentationNodeModuleDefinition
    ];
    private _getHttpInstrumentation;
    private _getHttpsInstrumentation;
    /**
     * Creates spans for incoming requests, restoring spans' context if applied.
     */
    protected _getPatchIncomingRequestFunction(component: 'http' | 'https'): (original: (event: string, ...args: unknown[]) => boolean) => (this: unknown, event: string, ...args: unknown[]) => boolean;
    /**
     * Creates spans for outgoing requests, sending spans' context for distributed
     * tracing.
     */
    protected _getPatchOutgoingRequestFunction(component: 'http' | 'https'): (original: Func<http.ClientRequest>) => Func<http.ClientRequest>;
    protected _getPatchOutgoingGetFunction(clientRequest: (options: http.RequestOptions | string | url.URL, ...args: HttpRequestArgs) => http.ClientRequest): (_original: Func<http.ClientRequest>) => Func<http.ClientRequest>;
    /** Patches HTTPS outgoing requests */
    private _getPatchHttpsOutgoingRequestFunction;
    private _setDefaultOptions;
    /** Patches HTTPS outgoing get requests */
    private _getPatchHttpsOutgoingGetFunction;
    /**
     * Attach event listeners to a client request to end span and add span attributes.
     *
     * @param request The original request object.
     * @param span representing the current operation
     * @param startTime representing the start time of the request to calculate duration in Metric
     * @param metricAttributes metric attributes
     */
    private _traceClientRequest;
    private _incomingRequestFunction;
    private _outgoingRequestFunction;
    private _onServerResponseFinish;
    private _onServerResponseError;
    private _startHttpSpan;
    private _closeHttpSpan;
    private _callResponseHook;
    private _callRequestHook;
    private _callStartSpanHook;
    private _createHeaderCapture;
}
//# sourceMappingURL=http.d.ts.map