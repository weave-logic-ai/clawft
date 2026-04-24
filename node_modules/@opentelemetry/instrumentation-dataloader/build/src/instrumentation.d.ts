import { InstrumentationBase, InstrumentationNodeModuleDefinition } from '@opentelemetry/instrumentation';
import { DataloaderInstrumentationConfig } from './types';
export declare class DataloaderInstrumentation extends InstrumentationBase {
    constructor(config?: DataloaderInstrumentationConfig);
    protected init(): InstrumentationNodeModuleDefinition[];
    getConfig(): DataloaderInstrumentationConfig;
    setConfig(config?: DataloaderInstrumentationConfig): void;
    private shouldCreateSpans;
    private getSpanName;
    private _getPatchedConstructor;
    private _patchLoad;
    private _getPatchedLoad;
    private _patchLoadMany;
    private _getPatchedLoadMany;
}
//# sourceMappingURL=instrumentation.d.ts.map