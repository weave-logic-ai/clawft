import { InstrumentationBase, InstrumentationNodeModuleDefinition } from '@opentelemetry/instrumentation';
import { PinoInstrumentationConfig } from './types';
export declare class PinoInstrumentation extends InstrumentationBase {
    constructor(config?: PinoInstrumentationConfig);
    protected init(): InstrumentationNodeModuleDefinition[];
    getConfig(): PinoInstrumentationConfig;
    setConfig(config?: PinoInstrumentationConfig): void;
    private _callHook;
    private _getMixinFunction;
    private _combineOptions;
}
//# sourceMappingURL=instrumentation.d.ts.map