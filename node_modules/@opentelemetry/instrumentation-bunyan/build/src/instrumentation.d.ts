import { InstrumentationBase, InstrumentationNodeModuleDefinition } from '@opentelemetry/instrumentation';
import { BunyanInstrumentationConfig } from './types';
export declare class BunyanInstrumentation extends InstrumentationBase {
    constructor(config?: BunyanInstrumentationConfig);
    protected init(): InstrumentationNodeModuleDefinition[];
    getConfig(): BunyanInstrumentationConfig;
    setConfig(config?: BunyanInstrumentationConfig): void;
    private _getPatchedEmit;
    private _getPatchedCreateLogger;
    private _addStream;
    private _callHook;
}
//# sourceMappingURL=instrumentation.d.ts.map