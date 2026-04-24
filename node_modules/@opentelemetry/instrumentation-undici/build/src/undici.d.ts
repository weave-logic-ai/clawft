import { InstrumentationBase } from '@opentelemetry/instrumentation';
import { UndiciInstrumentationConfig } from './types';
export declare class UndiciInstrumentation extends InstrumentationBase {
    private _channelSubs;
    private _recordFromReq;
    private _httpClientDurationHistogram;
    constructor(config?: UndiciInstrumentationConfig);
    protected init(): undefined;
    disable(): void;
    enable(): void;
    setConfig(config?: UndiciInstrumentationConfig): void;
    protected _updateMetricInstruments(): void;
    private _getConfig;
    private subscribeToChannel;
    private onRequestCreated;
    private onRequestHeaders;
    private onResponseHeaders;
    private onDone;
    private onError;
    private recordRequestDuration;
    private getRequestMethod;
}
//# sourceMappingURL=undici.d.ts.map