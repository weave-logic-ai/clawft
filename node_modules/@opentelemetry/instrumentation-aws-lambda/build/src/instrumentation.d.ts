import { InstrumentationBase, InstrumentationNodeModuleDefinition } from '@opentelemetry/instrumentation';
import { MeterProvider, TracerProvider } from '@opentelemetry/api';
import { AwsLambdaInstrumentationConfig } from './types';
export declare const traceContextEnvironmentKey = "_X_AMZN_TRACE_ID";
export declare class AwsLambdaInstrumentation extends InstrumentationBase {
    private _traceForceFlusher?;
    private _metricForceFlusher?;
    protected _config: AwsLambdaInstrumentationConfig;
    constructor(config?: AwsLambdaInstrumentationConfig);
    setConfig(config?: AwsLambdaInstrumentationConfig): void;
    init(): InstrumentationNodeModuleDefinition[];
    private _getHandler;
    private _getPatchHandler;
    setTracerProvider(tracerProvider: TracerProvider): void;
    private _traceForceFlush;
    setMeterProvider(meterProvider: MeterProvider): void;
    private _metricForceFlush;
    private _wrapCallback;
    private _endSpan;
    private _applyResponseHook;
    private static _extractAccountId;
    private static _defaultEventContextExtractor;
    private static _determineParent;
}
//# sourceMappingURL=instrumentation.d.ts.map