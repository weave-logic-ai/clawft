import { InstrumentationBase, InstrumentationNodeModuleDefinition } from '@opentelemetry/instrumentation';
import type { RestifyInstrumentationConfig } from './types';
export declare class RestifyInstrumentation extends InstrumentationBase {
    constructor(config?: RestifyInstrumentationConfig);
    private _moduleVersion?;
    private _isDisabled;
    setConfig(config?: RestifyInstrumentationConfig): void;
    getConfig(): RestifyInstrumentationConfig;
    init(): InstrumentationNodeModuleDefinition;
    private _middlewarePatcher;
    private _methodPatcher;
    private _handlerPatcher;
}
//# sourceMappingURL=instrumentation.d.ts.map