import { InstrumentationBase, InstrumentationNodeModuleDefinition } from '@opentelemetry/instrumentation';
import * as types from './types';
export declare class KnexInstrumentation extends InstrumentationBase {
    constructor(config?: types.KnexInstrumentationConfig);
    init(): InstrumentationNodeModuleDefinition;
    private getRunnerNodeModuleFileInstrumentation;
    private getClientNodeModuleFileInstrumentation;
    private createQueryWrapper;
    private storeContext;
    ensureWrapped(obj: any, methodName: string, wrapper: (original: any) => any): void;
}
//# sourceMappingURL=instrumentation.d.ts.map