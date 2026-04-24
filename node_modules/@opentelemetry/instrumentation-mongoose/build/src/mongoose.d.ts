import { MongooseInstrumentationConfig } from './types';
import { InstrumentationBase, InstrumentationModuleDefinition } from '@opentelemetry/instrumentation';
export declare const _STORED_PARENT_SPAN: unique symbol;
export declare class MongooseInstrumentation extends InstrumentationBase {
    protected _config: MongooseInstrumentationConfig;
    constructor(config?: MongooseInstrumentationConfig);
    setConfig(config?: MongooseInstrumentationConfig): void;
    protected init(): InstrumentationModuleDefinition;
    private patch;
    private unpatch;
    private patchAggregateExec;
    private patchQueryExec;
    private patchOnModelMethods;
    private patchModelAggregate;
    private patchAndCaptureSpanContext;
    private _startSpan;
    private _handleResponse;
    private _callOriginalFunction;
}
//# sourceMappingURL=mongoose.d.ts.map